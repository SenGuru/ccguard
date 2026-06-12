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

use axum::extract::{Query, State};
use axum::Json;
use ccguard_core::conformal::{self, Calibration, SelectiveDecision};
use ccguard_core::event::Classification;
use ccguard_core::triage::{self, StructuredPolicy, TriageInput, TriageLabel, TriageVerdict};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

use crate::auth::AuthedTenant;
use crate::error::AppError;
use crate::handlers::enforcement;
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
    pub work_domains: String,
    pub work_ticket_prefixes: String,
    pub approved_langs: String,
}

impl Default for TriageConfig {
    fn default() -> Self {
        TriageConfig {
            enabled: false,
            work_definition: String::new(),
            model: ccguard_core::triage::DEFAULT_MODEL.to_string(),
            work_domains: String::new(),
            work_ticket_prefixes: String::new(),
            approved_langs: String::new(),
        }
    }
}

impl TriageConfig {
    /// The structured (typed-predicate) policy the judge treats as authoritative.
    pub fn structured_policy(&self) -> StructuredPolicy {
        let parse = |s: &str| -> Vec<String> {
            s.split([',', '\n', '\r', ';'])
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect()
        };
        StructuredPolicy {
            work_domains: parse(&self.work_domains),
            work_ticket_prefixes: parse(&self.work_ticket_prefixes),
            approved_langs: parse(&self.approved_langs),
        }
    }
}

/// Load (or default) the tenant's triage config.
pub async fn load_config(pool: &PgPool, tenant_id: &str) -> Result<TriageConfig, sqlx::Error> {
    let row = sqlx::query(
        "select enabled, work_definition, model, work_domains, work_ticket_prefixes, approved_langs \
         from tenant_triage_config where tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?;
    Ok(match row {
        Some(r) => TriageConfig {
            enabled: r.get("enabled"),
            work_definition: r.get("work_definition"),
            model: r.get("model"),
            work_domains: r.get("work_domains"),
            work_ticket_prefixes: r.get("work_ticket_prefixes"),
            approved_langs: r.get("approved_langs"),
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
    /// Verdicts whose confidence fell below the conformal threshold — left
    /// unclassified for review rather than label-forced.
    pub abstained: usize,
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

/// Outcome of triaging one session.
pub struct TriageOutcome {
    pub verdict: TriageVerdict,
    /// True when a definite label was applied (mirrored). False for `unsure` or a
    /// conformal abstention.
    pub applied: bool,
    /// True when the verdict's confidence fell below the conformal threshold.
    pub abstained: bool,
}

/// Triage one session end-to-end: call Claude, apply the conformal selective
/// threshold, persist the verdict, and mirror a definite label onto the session.
/// A below-threshold verdict ABSTAINS (kept for review, not label-forced).
pub async fn triage_one(
    pool: &PgPool,
    client: &reqwest::Client,
    tenant_id: &str,
    session_id: &str,
    cfg: &TriageConfig,
    policy: &StructuredPolicy,
    calib: &Calibration,
) -> Result<TriageOutcome, TriageError> {
    let input = assemble_input(pool, tenant_id, session_id)
        .await?
        .ok_or(TriageError::SessionNotFound)?;

    let work_def = if cfg.work_definition.trim().is_empty() {
        None
    } else {
        Some(cfg.work_definition.as_str())
    };
    let verdict = triage_client::classify_session(client, &cfg.model, policy, work_def, &input)
        .await
        .map_err(TriageError::Client)?;

    Ok(apply_verdict(pool, tenant_id, session_id, &verdict, &cfg.model, calib).await?)
}

/// Persist a verdict and gate it — independent of HOW the verdict was produced
/// (the server-side Anthropic API, or the agent's local Claude Code CLI). Applies
/// the conformal selective threshold, the structural enforceability gate, and
/// mirrors a definite label onto the session. Pure-of-network.
pub async fn apply_verdict(
    pool: &PgPool,
    tenant_id: &str,
    session_id: &str,
    verdict: &TriageVerdict,
    model: &str,
    calib: &Calibration,
) -> Result<TriageOutcome, sqlx::Error> {
    // Conformal selective gate: once CALIBRATED, a below-threshold verdict abstains
    // to review. Before calibration (not enough human labels yet) the judge runs
    // uncalibrated and applies its label for visibility — otherwise it could never
    // produce the verdicts humans review to build the calibration set in the first
    // place. The abstain wrapper only suppresses application once it can vouch.
    let abstained = calib.usable
        && matches!(conformal::decide(verdict.confidence, calib), SelectiveDecision::Abstain)
        && verdict.label != TriageLabel::Unsure;

    // Structural corroboration → enforceability gate (never enforceable if abstained).
    let structural = structural_label(pool, tenant_id, session_id).await?;
    let llm_class = match verdict.label {
        TriageLabel::Work => Some(Classification::Work),
        TriageLabel::Personal => Some(Classification::Personal),
        TriageLabel::Unsure => None,
    };
    let applied = llm_class.is_some() && !abstained;
    let enforceable = !abstained
        && matches!(
            (structural, llm_class),
            (Classification::Work, Some(Classification::Work))
                | (Classification::Personal, Some(Classification::Personal))
        );
    let reason = if abstained {
        format!("{} [abstained: below calibration threshold]", verdict.reason)
    } else {
        verdict.reason.clone()
    };

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
    .bind(&reason)
    .bind(model)
    .bind(structural.as_str())
    .bind(enforceable)
    .execute(pool)
    .await?;

    // Mirror a definite label only when applied (not unsure, not abstained).
    if applied {
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
    }

    Ok(TriageOutcome { verdict: verdict.clone(), applied, abstained })
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

    // Structured policy + the conformal selective threshold, computed once per run.
    let policy = cfg.structured_policy();
    let calib = enforcement::load_calibration(pool, tenant_id).await?;

    let client = http_client();
    for r in &rows {
        let sid: String = r.get("session_id");
        summary.attempted += 1;
        match triage_one(pool, &client, tenant_id, &sid, cfg, &policy, &calib).await {
            Ok(o) => {
                if o.abstained {
                    summary.abstained += 1;
                } else {
                    match o.verdict.label {
                        TriageLabel::Work => summary.work += 1,
                        TriageLabel::Personal => summary.personal += 1,
                        TriageLabel::Unsure => summary.unsure += 1,
                    }
                }
            }
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

// ---- Agent-executed triage (via the employee's local Claude Code) ------------
//
// The judge can run two ways: server-side against the Anthropic API (a CCGuard
// key), OR — the preferred path — on the employee's own machine through the
// already-installed, already-logged-in Claude Code CLI, so it uses the company's
// existing Claude seat, costs nothing extra, and session content never leaves
// their tenancy. The server assembles the prompt; the agent runs it; the verdict
// comes back here and flows through the SAME conformal + structural gates.

/// One unclassified session the agent should classify, with the ready-to-run prompt.
#[derive(Debug, Serialize)]
pub struct PendingItem {
    pub session_id: String,
    pub prompt: String,
}

#[derive(Debug, Deserialize)]
pub struct PendingQuery {
    /// Restrict to one developer's sessions (the agent passes its own identity).
    #[serde(default)]
    pub seat: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

/// `GET /v1/triage/pending` — unclassified sessions with a server-built prompt for
/// the agent's local Claude Code to answer. Ingest-token (tenant) auth.
pub async fn pending_endpoint(
    AuthedTenant(tenant): AuthedTenant,
    State(pool): State<PgPool>,
    Query(q): Query<PendingQuery>,
) -> Result<Json<Vec<PendingItem>>, AppError> {
    let cfg = load_config(&pool, &tenant).await?;
    let policy = cfg.structured_policy();
    let work_def = if cfg.work_definition.trim().is_empty() {
        None
    } else {
        Some(cfg.work_definition.as_str())
    };
    let limit = q.limit.unwrap_or(25).clamp(1, 100);

    let rows = sqlx::query(
        "select s.session_id from captured_sessions s \
         left join session_triage t on t.tenant_id=s.tenant_id and t.session_id=s.session_id \
         where s.tenant_id=$1 and s.classification='unknown' and t.session_id is null \
           and ($2::text is null or s.user_email=$2) \
         order by s.last_ts desc nulls last limit $3",
    )
    .bind(&tenant)
    .bind(&q.seat)
    .bind(limit)
    .fetch_all(&pool)
    .await?;

    let system = triage::system_prompt(&policy, work_def);
    let mut out = Vec::new();
    for r in &rows {
        let sid: String = r.get("session_id");
        if let Some(input) = assemble_input(&pool, &tenant, &sid).await? {
            let prompt = format!("{system}\n\n{}", triage::user_prompt(&input));
            out.push(PendingItem { session_id: sid, prompt });
        }
    }
    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
pub struct VerdictBody {
    pub session_id: String,
    pub label: String,
    #[serde(default = "half")]
    pub confidence: f32,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub model: Option<String>,
}
fn half() -> f32 {
    0.5
}

/// `POST /v1/triage/verdict` — a verdict the agent produced via local Claude Code.
/// Flows through the same conformal + structural gates as the server-API path.
/// Ingest-token (tenant) auth.
pub async fn verdict_endpoint(
    AuthedTenant(tenant): AuthedTenant,
    State(pool): State<PgPool>,
    Json(b): Json<VerdictBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let verdict = TriageVerdict {
        label: TriageLabel::from_str(&b.label),
        confidence: b.confidence.clamp(0.0, 1.0),
        reason: b.reason,
    };
    let model = b.model.unwrap_or_else(|| "claude-code-local".to_string());
    let calib = enforcement::load_calibration(&pool, &tenant).await?;
    let outcome = apply_verdict(&pool, &tenant, &b.session_id, &verdict, &model, &calib).await?;
    Ok(Json(serde_json::json!({
        "session_id": b.session_id,
        "label": outcome.verdict.label.as_str(),
        "applied": outcome.applied,
        "abstained": outcome.abstained,
    })))
}
