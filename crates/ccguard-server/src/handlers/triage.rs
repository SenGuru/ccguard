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
use ccguard_core::gaming;
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
/// Evidence-channel caps (bounds prompt size; every channel degrades gracefully to empty).
const MAX_ASSISTANT_SNIPPETS: i64 = 2; // head snippets (+1 tail below)
const MAX_OUTCOMES: i64 = 5;
const MAX_RELATED: i64 = 5;
const LEXICON_TERMS: usize = 15;
const MAX_WORK_DOCS: i64 = 40;
const MAX_BACKGROUND_DOCS: i64 = 120;
/// A cwd shared by more than this many sessions is a hub (e.g. the home dir) and
/// carries no continuity signal.
const CWD_HUB_LIMIT: i64 = 10;
/// Same for shared files: a file edited across many sessions (memory indexes,
/// CLAUDE.md, dotfiles) is a hub — sharing it proves nothing. Live-caught
/// 2026-06-12: MEMORY.md (52 sessions) linked a non-work session to confirmed
/// work and the judge trusted the edge.
const FILE_HUB_LIMIT: i64 = 5;

/// SQL predicate excluding harness-injected "prompts" that are not the developer's
/// words (pasted skill bodies, local-command caveats, system reminders). They'd
/// pollute the judge's view of the session AND poison the learned work lexicon
/// (skill boilerplate scoring as "distinctive work vocabulary").
const NOT_BOILERPLATE: &str = "b.content not like '<local-command-caveat%' \
    and b.content not like 'Base directory for this skill%' \
    and b.content not like '<command-name>%' \
    and b.content not like '<system-reminder%'";

/// Tenant triage settings.
#[derive(Debug, Clone)]
pub struct TriageConfig {
    pub enabled: bool,
    /// Legacy free-text note (kept for back-compat / as a supplement).
    pub work_definition: String,
    pub model: String,
    pub work_domains: String,
    pub work_ticket_prefixes: String,
    pub approved_langs: String,
    /// "What the business does and what its work looks like in code."
    pub business_desc: String,
    /// "What Claude Code is allowed to be used for."
    pub work_allowed: String,
    /// Optional "what is NOT this business's work" — contrast examples only.
    pub personal_examples: String,
    pub policy_version: i32,
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
            business_desc: String::new(),
            work_allowed: String::new(),
            personal_examples: String::new(),
            policy_version: 1,
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

    /// The composed work-definition the judge reasons over: the admin's two
    /// plain-English fields (+ contrast examples), falling back to the legacy
    /// free-text note when the new fields are empty. `None` when nothing is set.
    pub fn work_def(&self) -> Option<String> {
        let composed = ccguard_core::triage::compose_work_definition(
            &self.business_desc,
            &self.work_allowed,
            &self.personal_examples,
        );
        if !composed.trim().is_empty() {
            Some(composed)
        } else if !self.work_definition.trim().is_empty() {
            Some(self.work_definition.clone())
        } else {
            None
        }
    }
}

/// Load (or default) the tenant's triage config.
pub async fn load_config(pool: &PgPool, tenant_id: &str) -> Result<TriageConfig, sqlx::Error> {
    let row = sqlx::query(
        "select enabled, work_definition, model, work_domains, work_ticket_prefixes, approved_langs, \
                business_desc, work_allowed, personal_examples, policy_version \
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
            business_desc: r.get("business_desc"),
            work_allowed: r.get("work_allowed"),
            personal_examples: r.get("personal_examples"),
            policy_version: r.get("policy_version"),
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

    // First developer prompts in seq order (boilerplate-injected "prompts" excluded).
    let prompt_rows = sqlx::query(&format!(
        "select b.content from captured_events e \
         join content_blobs b on b.tenant_id = e.tenant_id and b.sha256 = e.content_sha \
         where e.tenant_id = $1 and e.session_id = $2 and e.kind = 'user_prompt' \
           and {NOT_BOILERPLATE} \
         order by e.seq limit $3",
    ))
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

    // Touched files / shell targets, de-duped. Real file edits rank before generic
    // tool-call targets (web-search strings etc.) so they survive the cap.
    let target_rows = sqlx::query(
        "select target from ( \
           select target, min(case when kind = 'file_edit' then 0 else 1 end) as pri \
           from captured_events \
           where tenant_id = $1 and session_id = $2 \
             and kind in ('file_edit','tool_call') and target is not null \
           group by target \
         ) x order by pri, target limit $3",
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

    // The developer's current assignment (admin-set), looked up by the session's
    // user_email. Drives the off-assignment signal; None when unassigned.
    let assignment = sqlx::query_scalar::<_, Option<String>>(
        "select r.assignment from captured_sessions s \
         join employee_roles r on r.tenant_id = s.tenant_id and r.user_email = s.user_email \
         where s.tenant_id = $1 and s.session_id = $2",
    )
    .bind(tenant_id)
    .bind(session_id)
    .fetch_optional(pool)
    .await?
    .flatten()
    .filter(|a| !a.trim().is_empty());

    let mut input = TriageInput {
        repo_org: meta.get("repo_org"),
        repo_name: meta.get("repo_name"),
        repo_path: meta.get("repo_path"),
        cwd: meta.get("cwd"),
        title: meta.get("title"),
        prompts,
        tool_targets,
        assignment,
        ..Default::default()
    };
    gather_evidence(pool, tenant_id, session_id, &mut input).await?;
    Ok(Some(input))
}

/// Fill the judge's evidence channels (assistant output, git/PR outcomes, related
/// human-labeled sessions, learned work vocabulary + similarity). Every channel
/// degrades to empty — evidence enriches the judge's view, never gates it.
///
/// Grounding rule: the related-session and lexicon channels read ONLY
/// human-confirmed labels (`session_triage.human_reviewed`). AI labels would be
/// circular (the judge amplifying its own guesses) and would churn the triage
/// `input_digest` mid-sweep as verdicts post.
async fn gather_evidence(
    pool: &PgPool,
    tenant_id: &str,
    session_id: &str,
    input: &mut TriageInput,
) -> Result<(), sqlx::Error> {
    // Channel 5: assistant output samples — head 2 + tail 1 (substance often lives
    // in what was produced, not the developer's terse prompts).
    let head = sqlx::query_scalar::<_, String>(
        "select b.content from captured_events e \
         join content_blobs b on b.tenant_id = e.tenant_id and b.sha256 = e.content_sha \
         where e.tenant_id = $1 and e.session_id = $2 and e.kind = 'assistant_text' \
           and b.content is not null and length(trim(b.content)) > 0 \
         order by e.seq asc limit $3",
    )
    .bind(tenant_id)
    .bind(session_id)
    .bind(MAX_ASSISTANT_SNIPPETS)
    .fetch_all(pool)
    .await?;
    let tail = sqlx::query_scalar::<_, String>(
        "select b.content from captured_events e \
         join content_blobs b on b.tenant_id = e.tenant_id and b.sha256 = e.content_sha \
         where e.tenant_id = $1 and e.session_id = $2 and e.kind = 'assistant_text' \
           and b.content is not null and length(trim(b.content)) > 0 \
         order by e.seq desc limit 1",
    )
    .bind(tenant_id)
    .bind(session_id)
    .fetch_all(pool)
    .await?;
    let mut snippets = head;
    for t in tail {
        if !snippets.contains(&t) {
            snippets.push(t);
        }
    }
    input.assistant_snippets = snippets;

    // Channel 4: git/PR outcomes — PR URLs seen in the session plus the structural
    // provenance reason codes (push/remote evidence). Hardest channel to fake.
    let prs = sqlx::query_scalar::<_, String>(
        "select distinct target from captured_events \
         where tenant_id = $1 and session_id = $2 and kind = 'pr' and target is not null \
         limit $3",
    )
    .bind(tenant_id)
    .bind(session_id)
    .bind(MAX_OUTCOMES)
    .fetch_all(pool)
    .await?;
    let mut outcomes: Vec<String> = prs.into_iter().map(|u| format!("PR: {u}")).collect();
    if let Some(reasons) = sqlx::query_scalar::<_, Option<String>>(
        "select reasons from session_provenance where tenant_id = $1 and session_id = $2",
    )
    .bind(tenant_id)
    .bind(session_id)
    .fetch_optional(pool)
    .await?
    .flatten()
    .filter(|r| !r.trim().is_empty())
    {
        outcomes.push(format!("structural provenance signals: {reasons}"));
    }
    input.outcomes = outcomes;

    // Channel 3: continuity — human-labeled sessions sharing this one's working
    // directory (unless it's a hub cwd) or sharing edited files.
    let mut related: Vec<String> = sqlx::query(
        "with me as ( \
           select cwd from captured_sessions where tenant_id = $1 and session_id = $2 \
             and cwd is not null and cwd <> '' \
         ), hub as ( \
           select count(*) as c from captured_sessions s join me on s.cwd = me.cwd \
           where s.tenant_id = $1 \
         ) \
         select coalesce(nullif(s2.title,''),'(untitled)') as title, t2.human_label \
         from captured_sessions s2 \
         join me on s2.cwd = me.cwd \
         join session_triage t2 on t2.tenant_id = s2.tenant_id and t2.session_id = s2.session_id \
           and t2.human_reviewed and t2.human_label in ('work','personal') \
         where s2.tenant_id = $1 and s2.session_id <> $2 \
           and (select c from hub) <= $3 \
         limit $4",
    )
    .bind(tenant_id)
    .bind(session_id)
    .bind(CWD_HUB_LIMIT)
    .bind(MAX_RELATED)
    .fetch_all(pool)
    .await?
    .iter()
    .map(|r| {
        let title: String = r.get("title");
        let label: String = r.get("human_label");
        format!("\"{title}\" — human-confirmed {} (shares working directory)", label.to_uppercase())
    })
    .collect();
    if (related.len() as i64) < MAX_RELATED {
        let by_files = sqlx::query(
            "select distinct coalesce(nullif(s2.title,''),'(untitled)') as title, t2.human_label \
             from captured_events e1 \
             join captured_events e2 on e2.tenant_id = e1.tenant_id and e2.target = e1.target \
               and e2.kind = 'file_edit' and e2.session_id <> e1.session_id \
             join session_triage t2 on t2.tenant_id = e2.tenant_id and t2.session_id = e2.session_id \
               and t2.human_reviewed and t2.human_label in ('work','personal') \
             join captured_sessions s2 on s2.tenant_id = e2.tenant_id and s2.session_id = e2.session_id \
             where e1.tenant_id = $1 and e1.session_id = $2 and e1.kind = 'file_edit' \
               and e1.target is not null \
               and (select count(distinct e3.session_id) from captured_events e3 \
                    where e3.tenant_id = e1.tenant_id and e3.kind = 'file_edit' \
                      and e3.target = e1.target) <= $4 \
             limit $3",
        )
        .bind(tenant_id)
        .bind(session_id)
        .bind(MAX_RELATED - related.len() as i64)
        .bind(FILE_HUB_LIMIT)
        .fetch_all(pool)
        .await?;
        for r in &by_files {
            let title: String = r.get("title");
            let label: String = r.get("human_label");
            let line =
                format!("\"{title}\" — human-confirmed {} (edited the same files)", label.to_uppercase());
            if !related.contains(&line) {
                related.push(line);
            }
        }
    }
    input.related_sessions = related;

    // Channel 2: learned work vocabulary + similarity, grounded in human-confirmed
    // WORK sessions only. Docs are title + early prompts (bounded).
    // Doc shape MUST be identical for work and background (title + first 6 real
    // prompts) — asymmetric sampling inflates log-odds for long-prompt vocabulary.
    let doc_body = format!(
        "coalesce(( \
           select string_agg(left(x.content, 400), ' ' order by x.seq) \
           from (select e0.seq, b.content from captured_events e0 \
                 join content_blobs b on b.tenant_id = e0.tenant_id and b.sha256 = e0.content_sha \
                 where e0.tenant_id = s.tenant_id and e0.session_id = s.session_id \
                   and e0.kind = 'user_prompt' and {NOT_BOILERPLATE} \
                 order by e0.seq limit 6) x \
         ), '')"
    );
    let work_docs = sqlx::query(&format!(
        "select coalesce(nullif(s.title,''),'') as title, {doc_body} as body \
         from captured_sessions s \
         join session_triage t on t.tenant_id = s.tenant_id and t.session_id = s.session_id \
         where s.tenant_id = $1 and s.session_id <> $2 \
           and t.human_reviewed and t.human_label = 'work' \
         order by s.last_ts desc nulls last limit $3",
    ))
    .bind(tenant_id)
    .bind(session_id)
    .bind(MAX_WORK_DOCS)
    .fetch_all(pool)
    .await?;
    if !work_docs.is_empty() {
        let titles: Vec<String> = work_docs.iter().map(|r| r.get::<String, _>("title")).collect();
        let docs: Vec<String> = work_docs
            .iter()
            .map(|r| format!("{} {}", r.get::<String, _>("title"), r.get::<String, _>("body")))
            .collect();
        let background: Vec<String> = sqlx::query_scalar::<_, String>(&format!(
            "select coalesce(nullif(s.title,''),'') || ' ' || {doc_body} \
             from captured_sessions s \
             where s.tenant_id = $1 and s.session_id <> $2 \
               and not exists (select 1 from session_triage t \
                 where t.tenant_id = s.tenant_id and t.session_id = s.session_id \
                   and t.human_reviewed and t.human_label = 'work') \
             order by s.last_ts desc nulls last limit $3",
        ))
        .bind(tenant_id)
        .bind(session_id)
        .bind(MAX_BACKGROUND_DOCS)
        .fetch_all(pool)
        .await?;

        let lexicon = ccguard_core::lexicon::distinctive_terms(&docs, &background, LEXICON_TERMS);
        // Bound each prompt to the same 400-char window the corpus docs use before
        // tokenizing — a big session can have multi-MB prompts, and term_hits +
        // nearest_work each tokenize this whole blob, so the raw join made the
        // lexicon step take seconds. The distinctive terms live in the head anyway.
        let sample: String = input
            .prompts
            .iter()
            .map(|p| p.chars().take(400).collect::<String>())
            .collect::<Vec<_>>()
            .join(" ");
        let this_doc = format!(
            "{} {} {}",
            input.title.as_deref().unwrap_or(""),
            sample,
            input.tool_targets.join(" ")
        );
        input.work_term_hits = ccguard_core::lexicon::term_hits(&this_doc, &lexicon);
        if let Some((sim, idx)) = ccguard_core::lexicon::nearest_work(&this_doc, &docs) {
            input.work_similarity = Some(sim);
            input.nearest_work_title = titles.get(idx).cloned().filter(|t| !t.is_empty());
        }
    }
    Ok(())
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

    let wd = cfg.work_def();
    let verdict =
        triage_client::classify_session(client, &cfg.model, policy, wd.as_deref(), &input)
            .await
            .map_err(TriageError::Client)?;

    Ok(apply_verdict(pool, tenant_id, session_id, &verdict, &cfg.model, calib, None).await?)
}

/// Load the tenant's current policy version (stamps every verdict so labels bind
/// to the policy that produced them).
async fn policy_version(pool: &PgPool, tenant_id: &str) -> Result<i32, sqlx::Error> {
    Ok(sqlx::query_scalar::<_, i32>(
        "select policy_version from tenant_triage_config where tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(1))
}

/// Persist a verdict and gate it — independent of HOW it was produced (the server
/// Anthropic API, or the agent's local Claude Code). Applies the conformal selective
/// threshold + the structural enforceability gate, computes gaming flags, stamps the
/// policy version, and ALWAYS drives `captured_sessions.classification` to a terminal
/// value (so a 'pending' session never gets stuck). Pure-of-network.
pub async fn apply_verdict(
    pool: &PgPool,
    tenant_id: &str,
    session_id: &str,
    verdict: &TriageVerdict,
    model: &str,
    calib: &Calibration,
    input_digest: Option<&str>,
) -> Result<TriageOutcome, sqlx::Error> {
    // Conformal selective gate: once CALIBRATED, a below-threshold verdict abstains
    // to review. Before calibration (too few human labels) the judge runs
    // uncalibrated and applies its label for visibility — otherwise it could never
    // produce the verdicts humans review to build the calibration set.
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

    // Gaming flags (review-only; never alter the label). label_structure_conflict
    // fires when the AI says work but the structural cascade says confirmed personal.
    let prov_for_gaming = match structural {
        Classification::Personal => Some("personal"),
        Classification::Work => Some("work"),
        Classification::Unknown => None,
    };
    let gaming_flags = gaming::flags(verdict.label.as_str(), prov_for_gaming);
    let pv = policy_version(pool, tenant_id).await?;
    // The session's current event_count — the re-judge-on-growth baseline. Future
    // growth past this is what re-enqueues the session for another look.
    let ec: i32 = sqlx::query_scalar::<_, i32>(
        "select event_count from captured_sessions where tenant_id=$1 and session_id=$2",
    )
    .bind(tenant_id)
    .bind(session_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(0);

    sqlx::query(
        "insert into session_triage \
         (tenant_id, session_id, label, confidence, reason, model, resolved_by, structural, \
          enforceable, mixed, matched_clause, policy_version, gaming_flags, input_digest, \
          off_assignment, judged_event_count, next_retry_at, updated_at) \
         values ($1,$2,$3,$4,$5,$6,'llm',$7,$8,$9,$10,$11,$12,$13,$14,$15, null, now()) \
         on conflict (tenant_id, session_id) do update set \
           label = excluded.label, confidence = excluded.confidence, reason = excluded.reason, \
           model = excluded.model, resolved_by = 'llm', structural = excluded.structural, \
           enforceable = excluded.enforceable, mixed = excluded.mixed, \
           matched_clause = excluded.matched_clause, policy_version = excluded.policy_version, \
           gaming_flags = excluded.gaming_flags, input_digest = excluded.input_digest, \
           off_assignment = excluded.off_assignment, judged_event_count = excluded.judged_event_count, \
           next_retry_at = null, updated_at = now()",
    )
    .bind(tenant_id)
    .bind(session_id)
    .bind(verdict.label.as_str())
    .bind(verdict.confidence)
    .bind(&reason)
    .bind(model)
    .bind(structural.as_str())
    .bind(enforceable)
    .bind(verdict.mixed)
    .bind(&verdict.matched_clause)
    .bind(pv)
    .bind(&gaming_flags)
    .bind(input_digest)
    .bind(verdict.off_assignment)
    .bind(ec)
    .execute(pool)
    .await?;

    // ALWAYS drive classification to a terminal value — the drain that keeps a
    // 'pending' session from sticking forever. Applied → work|personal; unsure or
    // abstained → 'unknown' (terminal-safe, excluded from every meter, queued for review).
    let final_coarse = match (applied, llm_class) {
        (true, Some(c)) => c,
        _ => Classification::Unknown,
    };
    sqlx::query(
        "update captured_sessions set classification = $3 \
         where tenant_id = $1 and session_id = $2",
    )
    .bind(tenant_id)
    .bind(session_id)
    .bind(final_coarse.as_str())
    .execute(pool)
    .await?;

    // Re-run on-task scoring + indicators now that the real (AI) class is known —
    // at capture time the session was 'pending'/Unknown, so e.g. the personal_repo
    // indicator must (re)compute against the verdict. Defensive: a scoring failure
    // must not fail the verdict (it's already persisted).
    if let Ok(Some(email)) = sqlx::query_scalar::<_, String>(
        "select user_email from captured_sessions where tenant_id=$1 and session_id=$2",
    )
    .bind(tenant_id)
    .bind(session_id)
    .fetch_optional(pool)
    .await
    {
        if let Err(e) =
            crate::handlers::capture::score_session(pool, tenant_id, session_id, final_coarse, &email)
                .await
        {
            eprintln!("re-score after verdict failed for {session_id}: {e}");
        }

        // Off-assignment indicator: work that is not on this developer's lane.
        // Triage-derived (not capture-derived), so managed here — clear any stale
        // open one, then raise it when the applied verdict says off-assignment.
        let _ = sqlx::query(
            "delete from indicators where tenant_id=$1 and session_id=$2 \
             and kind='off_assignment' and status='open'",
        )
        .bind(tenant_id)
        .bind(session_id)
        .execute(pool)
        .await;
        if applied && verdict.off_assignment {
            let _ = sqlx::query(
                "insert into indicators (tenant_id, user_email, session_id, kind, detail, status) \
                 values ($1,$2,$3,'off_assignment',$4,'open') \
                 on conflict (tenant_id, session_id, kind) do nothing",
            )
            .bind(tenant_id)
            .bind(&email)
            .bind(session_id)
            .bind(format!("work, but off assignment: {}", verdict.reason))
            .execute(pool)
            .await;
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
         where s.tenant_id = $1 and ( \
             (s.classification = 'pending' \
               and (t.session_id is null or (t.next_retry_at is not null and t.next_retry_at <= now()))) \
             or (t.next_retry_at is not null and t.next_retry_at <= now() \
                 and not coalesce(t.human_reviewed, false)) \
         ) \
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
        // Triviality gate: drains trivial sessions (pending → unknown) without billing.
        if assemble_triageable(pool, tenant_id, &sid).await?.is_none() {
            continue;
        }
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

/// One unclassified session the agent should classify, with the ready-to-run prompt
/// and a content digest the agent echoes back so a verdict for stale content is rejected.
#[derive(Debug, Serialize)]
pub struct PendingItem {
    pub session_id: String,
    pub prompt: String,
    pub input_digest: String,
}

/// sha256 of the built prompt — the staleness key for a session's classified content.
fn input_digest(prompt: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(prompt.as_bytes()))
}

/// Assemble a session's judge input, but only if it's worth a (quota-costing)
/// classify call. Trivial/empty sessions are DRAINED `pending → 'unknown'`
/// (terminal-safe, never billed) and return `None`, so they don't stick at pending.
async fn assemble_triageable(
    pool: &PgPool,
    tenant_id: &str,
    session_id: &str,
) -> Result<Option<TriageInput>, sqlx::Error> {
    match assemble_input(pool, tenant_id, session_id).await? {
        Some(input) if triage::is_triageable(&input) => Ok(Some(input)),
        _ => {
            sqlx::query(
                "update captured_sessions set classification='unknown' \
                 where tenant_id=$1 and session_id=$2 and classification='pending'",
            )
            .bind(tenant_id)
            .bind(session_id)
            .execute(pool)
            .await?;
            // Clear any pending re-judge so a non-triageable grown session isn't
            // re-selected every sweep (the retry arm keys on next_retry_at).
            sqlx::query(
                "update session_triage set next_retry_at=null \
                 where tenant_id=$1 and session_id=$2",
            )
            .bind(tenant_id)
            .bind(session_id)
            .execute(pool)
            .await?;
            Ok(None)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PendingQuery {
    /// Restrict to one MACHINE's sessions (the seat = the computer). The agent passes
    /// its own `device_id`; this is the primary identity.
    #[serde(default)]
    pub device: Option<String>,
    /// Legacy: restrict by Claude Code login email. Kept for older agents; `device`
    /// takes precedence when both are sent.
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
    let wd = cfg.work_def();
    let work_def = wd.as_deref();
    let limit = q.limit.unwrap_or(25).clamp(1, 100);

    let rows = sqlx::query(
        "select s.session_id from captured_sessions s \
         left join session_triage t on t.tenant_id=s.tenant_id and t.session_id=s.session_id \
         where s.tenant_id=$1 \
           and ($2::text is null or s.device_id=$2) \
           and ($3::text is null or s.user_email=$3) and ( \
             (s.classification='pending' \
               and (t.session_id is null or (t.next_retry_at is not null and t.next_retry_at <= now()))) \
             or (t.next_retry_at is not null and t.next_retry_at <= now() \
                 and not coalesce(t.human_reviewed, false)) \
         ) \
         order by s.last_ts desc nulls last limit $4",
    )
    .bind(&tenant)
    .bind(&q.device)
    .bind(&q.seat)
    .bind(limit)
    .fetch_all(&pool)
    .await?;

    let system = triage::system_prompt(&policy, work_def);
    let mut out = Vec::new();
    for r in &rows {
        let sid: String = r.get("session_id");
        // Triviality gate (drains trivial sessions, never bills them).
        if let Some(input) = assemble_triageable(&pool, &tenant, &sid).await? {
            let prompt = format!("{system}\n\n{}", triage::user_prompt(&input));
            let dig = input_digest(&prompt);
            out.push(PendingItem { session_id: sid, prompt, input_digest: dig });
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
    #[serde(default)]
    pub mixed: bool,
    #[serde(default)]
    pub matched_clause: Option<String>,
    /// WORK that is not on this developer's assignment (off-assignment flag).
    #[serde(default)]
    pub off_assignment: bool,
    /// The digest the agent was given for the content it classified; rejected if the
    /// session's content has changed since (stale verdict → re-enqueue, don't apply).
    #[serde(default)]
    pub input_digest: Option<String>,
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
    // Staleness guard: if the agent's digest doesn't match the session's CURRENT
    // content, the verdict is for stale content — reject and re-enqueue.
    if let Some(posted) = b.input_digest.as_deref() {
        let cfg = load_config(&pool, &tenant).await?;
        let policy = cfg.structured_policy();
        let wd = cfg.work_def();
        let current = match assemble_input(&pool, &tenant, &b.session_id).await? {
            Some(input) => {
                let prompt = format!(
                    "{}\n\n{}",
                    triage::system_prompt(&policy, wd.as_deref()),
                    triage::user_prompt(&input)
                );
                Some(input_digest(&prompt))
            }
            None => None,
        };
        if current.as_deref() != Some(posted) {
            sqlx::query(
                "update session_triage set next_retry_at = now() \
                 where tenant_id=$1 and session_id=$2",
            )
            .bind(&tenant)
            .bind(&b.session_id)
            .execute(&pool)
            .await?;
            return Ok(Json(serde_json::json!({
                "session_id": b.session_id, "applied": false, "stale": true
            })));
        }
    }

    let label = TriageLabel::from_str(&b.label);
    let verdict = TriageVerdict {
        off_assignment: label == TriageLabel::Work && b.off_assignment,
        label,
        confidence: b.confidence.clamp(0.0, 1.0),
        reason: b.reason,
        mixed: b.mixed,
        matched_clause: b.matched_clause.filter(|s| !s.trim().is_empty()),
    };
    let model = b.model.unwrap_or_else(|| "claude-code-local".to_string());
    let calib = enforcement::load_calibration(&pool, &tenant).await?;
    let outcome = apply_verdict(
        &pool,
        &tenant,
        &b.session_id,
        &verdict,
        &model,
        &calib,
        b.input_digest.as_deref(),
    )
    .await?;
    Ok(Json(serde_json::json!({
        "session_id": b.session_id,
        "label": outcome.verdict.label.as_str(),
        "applied": outcome.applied,
        "abstained": outcome.abstained,
        "mixed": outcome.verdict.mixed,
    })))
}
