//! LLM-tier triage worker: resolve UNCLASSIFIED sessions with a Claude judge.
//!
//! Flow per session: assemble a bounded context (repo + prompts + touched files)
//! → call Claude (`triage_client`) → persist the verdict to `session_triage` and
//! mirror a work/personal label onto `captured_sessions.classification` so the
//! existing dashboard views light up. `unsure` leaves the session unclassified.
//!
//! Enforcement gating: a verdict is written `enforceable=false` unless the
//! deterministic structural classifier independently agrees. Confirming a verdict
//! for usage-limiting is a separate, human action (`confirm_triage`).

use ccguard_core::event::Classification;
use ccguard_core::triage::{TriageInput, TriageLabel, TriageVerdict};
use sqlx::{PgPool, Row};

use crate::triage_client::{self, TriageClientError};

/// Max prompts / touched-files fed to the judge per session (bounds cost + content).
const MAX_PROMPTS: i64 = 12;
const MAX_TARGETS: i64 = 20;

/// Tenant triage settings.
#[derive(Debug, Clone)]
pub struct TriageConfig {
    pub enabled: bool,
    pub work_definition: String,
    pub model: String,
}

impl Default for TriageConfig {
    fn default() -> Self {
        TriageConfig {
            enabled: false,
            work_definition: String::new(),
            model: ccguard_core::triage::DEFAULT_MODEL.to_string(),
        }
    }
}

/// Load (or default) the tenant's triage config.
pub async fn load_config(pool: &PgPool, tenant_id: &str) -> Result<TriageConfig, sqlx::Error> {
    let row = sqlx::query(
        "select enabled, work_definition, model from tenant_triage_config where tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?;
    Ok(match row {
        Some(r) => TriageConfig {
            enabled: r.get("enabled"),
            work_definition: r.get("work_definition"),
            model: r.get("model"),
        },
        None => TriageConfig::default(),
    })
}

/// Result of a bulk triage sweep, for surfacing back to the operator.
#[derive(Debug, Default)]
pub struct RunSummary {
    pub attempted: usize,
    pub work: usize,
    pub personal: usize,
    pub unsure: usize,
    pub errors: Vec<String>,
}

/// Build a reqwest client with a sane per-call timeout.
fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(40))
        .build()
        .unwrap_or_default()
}

/// Assemble the judge's view of one session from captured rows. `None` when the
/// session does not exist for this tenant.
pub async fn assemble_input(
    pool: &PgPool,
    tenant_id: &str,
    session_id: &str,
) -> Result<Option<TriageInput>, sqlx::Error> {
    let meta = sqlx::query(
        "select repo_org, repo_name, repo_path, cwd, title \
         from captured_sessions where tenant_id = $1 and session_id = $2",
    )
    .bind(tenant_id)
    .bind(session_id)
    .fetch_optional(pool)
    .await?;
    let meta = match meta {
        Some(m) => m,
        None => return Ok(None),
    };

    // First developer prompts in seq order.
    let prompt_rows = sqlx::query(
        "select b.content from captured_events e \
         join content_blobs b on b.tenant_id = e.tenant_id and b.sha256 = e.content_sha \
         where e.tenant_id = $1 and e.session_id = $2 and e.kind = 'user_prompt' \
         order by e.seq limit $3",
    )
    .bind(tenant_id)
    .bind(session_id)
    .bind(MAX_PROMPTS)
    .fetch_all(pool)
    .await?;
    let prompts: Vec<String> = prompt_rows
        .iter()
        .filter_map(|r| r.get::<Option<String>, _>("content"))
        .filter(|s| !s.trim().is_empty())
        .collect();

    // Touched files / shell targets (file_edit + tool_call), de-duped, first seen.
    let target_rows = sqlx::query(
        "select distinct target from captured_events \
         where tenant_id = $1 and session_id = $2 \
           and kind in ('file_edit','tool_call') and target is not null \
         order by target limit $3",
    )
    .bind(tenant_id)
    .bind(session_id)
    .bind(MAX_TARGETS)
    .fetch_all(pool)
    .await?;
    let tool_targets: Vec<String> = target_rows
        .iter()
        .filter_map(|r| r.get::<Option<String>, _>("target"))
        .collect();

    Ok(Some(TriageInput {
        repo_org: meta.get("repo_org"),
        repo_name: meta.get("repo_name"),
        repo_path: meta.get("repo_path"),
        cwd: meta.get("cwd"),
        title: meta.get("title"),
        prompts,
        tool_targets,
    }))
}

/// The deterministic structural label for a session, read from the provenance
/// cascade's recorded verdict. Used to corroborate the LLM verdict for enforcement
/// gating: an LLM verdict only becomes enforceable when this structural signal
/// independently agrees (work/work_provisional → Work; personal → Personal).
async fn structural_label(
    pool: &PgPool,
    tenant_id: &str,
    session_id: &str,
) -> Result<Classification, sqlx::Error> {
    let row = sqlx::query(
        "select class from session_provenance where tenant_id = $1 and session_id = $2",
    )
    .bind(tenant_id)
    .bind(session_id)
    .fetch_optional(pool)
    .await?;
    Ok(match row.map(|r| r.get::<String, _>("class")) {
        Some(c) => match c.as_str() {
            "work" | "work_provisional" => Classification::Work,
            "personal" => Classification::Personal,
            _ => Classification::Unknown,
        },
        None => Classification::Unknown,
    })
}

/// Triage one session end-to-end: call Claude, persist the verdict, mirror the
/// label onto the session. Returns the verdict.
pub async fn triage_one(
    pool: &PgPool,
    client: &reqwest::Client,
    tenant_id: &str,
    session_id: &str,
    cfg: &TriageConfig,
) -> Result<TriageVerdict, TriageError> {
    let input = assemble_input(pool, tenant_id, session_id)
        .await?
        .ok_or(TriageError::SessionNotFound)?;

    let work_def = if cfg.work_definition.trim().is_empty() {
        None
    } else {
        Some(cfg.work_definition.as_str())
    };
    let verdict = triage_client::classify_session(client, &cfg.model, work_def, &input)
        .await
        .map_err(TriageError::Client)?;

    // Structural corroboration → enforceability gate.
    let structural = structural_label(pool, tenant_id, session_id).await?;
    let llm_class = match verdict.label {
        TriageLabel::Work => Some(Classification::Work),
        TriageLabel::Personal => Some(Classification::Personal),
        TriageLabel::Unsure => None,
    };
    let enforceable = matches!(
        (structural, llm_class),
        (Classification::Work, Some(Classification::Work))
            | (Classification::Personal, Some(Classification::Personal))
    );

    sqlx::query(
        "insert into session_triage \
         (tenant_id, session_id, label, confidence, reason, model, resolved_by, structural, enforceable, updated_at) \
         values ($1,$2,$3,$4,$5,$6,'llm',$7,$8, now()) \
         on conflict (tenant_id, session_id) do update set \
           label = excluded.label, confidence = excluded.confidence, reason = excluded.reason, \
           model = excluded.model, resolved_by = 'llm', structural = excluded.structural, \
           enforceable = excluded.enforceable, updated_at = now()",
    )
    .bind(tenant_id)
    .bind(session_id)
    .bind(verdict.label.as_str())
    .bind(verdict.confidence)
    .bind(&verdict.reason)
    .bind(&cfg.model)
    .bind(structural.as_str())
    .bind(enforceable)
    .execute(pool)
    .await?;

    // Mirror a definite label onto the session so the existing dashboard views
    // reflect it. `unsure` leaves the session unclassified.
    if let Some(c) = llm_class {
        sqlx::query(
            "update captured_sessions set classification = $3 \
             where tenant_id = $1 and session_id = $2",
        )
        .bind(tenant_id)
        .bind(session_id)
        .bind(c.as_str())
        .execute(pool)
        .await?;
    }

    Ok(verdict)
}

/// Sweep: triage up to `limit` currently-UNCLASSIFIED sessions that have no
/// verdict yet (newest first). Skips sessions already triaged so re-runs don't
/// re-bill. Stops early on a missing API key.
pub async fn run_unclassified(
    pool: &PgPool,
    tenant_id: &str,
    cfg: &TriageConfig,
    limit: i64,
) -> Result<RunSummary, sqlx::Error> {
    let mut summary = RunSummary::default();
    if !triage_client::api_key_present() {
        summary
            .errors
            .push("ANTHROPIC_API_KEY is not set — triage cannot run".into());
        return Ok(summary);
    }

    let rows = sqlx::query(
        "select s.session_id from captured_sessions s \
         left join session_triage t \
           on t.tenant_id = s.tenant_id and t.session_id = s.session_id \
         where s.tenant_id = $1 and s.classification = 'unknown' and t.session_id is null \
         order by s.last_ts desc nulls last limit $2",
    )
    .bind(tenant_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let client = http_client();
    for r in &rows {
        let sid: String = r.get("session_id");
        summary.attempted += 1;
        match triage_one(pool, &client, tenant_id, &sid, cfg).await {
            Ok(v) => match v.label {
                TriageLabel::Work => summary.work += 1,
                TriageLabel::Personal => summary.personal += 1,
                TriageLabel::Unsure => summary.unsure += 1,
            },
            Err(e) => {
                if summary.errors.len() < 5 {
                    summary
                        .errors
                        .push(format!("{}: {e}", sid.chars().take(8).collect::<String>()));
                }
                // A bad API key / auth error will recur for every session — bail early.
                if matches!(&e, TriageError::Client(TriageClientError::Status(401 | 403, _))) {
                    break;
                }
            }
        }
    }
    Ok(summary)
}

/// Errors from triaging a single session.
#[derive(Debug)]
pub enum TriageError {
    Db(sqlx::Error),
    Client(TriageClientError),
    SessionNotFound,
}
impl From<sqlx::Error> for TriageError {
    fn from(e: sqlx::Error) -> Self {
        TriageError::Db(e)
    }
}
impl std::fmt::Display for TriageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriageError::Db(e) => write!(f, "db error: {e}"),
            TriageError::Client(e) => write!(f, "{e}"),
            TriageError::SessionNotFound => write!(f, "session not found"),
        }
    }
}
