use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use ccguard_core::capture::CapturedSession;
use ccguard_core::classify::{classify, Allowlist};
use ccguard_core::event::Classification;
use ccguard_core::ontask::{self, OnTaskSignals};
use ccguard_core::roles::{self, Activity, JobRole};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::collections::BTreeSet;

use crate::auth::AuthedTenant;
use crate::error::AppError;

/// Parse the stored override classification text into a `Classification`.
/// Unknown / unexpected strings map to `Unknown` (conservative).
fn parse_classification(s: &str) -> Classification {
    match s {
        "work" => Classification::Work,
        "personal" => Classification::Personal,
        _ => Classification::Unknown,
    }
}

/// Look up an admin per-repo override for this repo (host+org+name all present).
/// Returns the override classification if a row exists, else `None`.
async fn repo_override(
    pool: &PgPool,
    tenant_id: &str,
    host: &str,
    org: &str,
    name: &str,
) -> Result<Option<Classification>, sqlx::Error> {
    let row = sqlx::query(
        "select classification from repo_overrides \
         where tenant_id=$1 and repo_host=$2 and repo_org=$3 and repo_name=$4",
    )
    .bind(tenant_id)
    .bind(host)
    .bind(org)
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| parse_classification(&r.get::<String, _>("classification"))))
}

async fn load_allowlist(pool: &PgPool, tenant_id: &str) -> Result<Allowlist, sqlx::Error> {
    let rows = sqlx::query("select kind, value from allowlist_rules where tenant_id = $1")
        .bind(tenant_id)
        .fetch_all(pool)
        .await?;
    let mut a = Allowlist::default();
    for r in rows {
        let kind: String = r.get("kind");
        let value: String = r.get("value");
        match kind.as_str() {
            "host" => a.hosts.push(value),
            "org" => a.orgs.push(value),
            "path_root" => a.path_roots.push(value),
            _ => {}
        }
    }
    Ok(a)
}

/// Ingest one captured session (metadata + ordered events). Tenant comes from the ingest token.
/// Idempotent: re-posting the same session/seq is a no-op; content is sha256-deduped.
pub async fn capture(
    AuthedTenant(tenant_id): AuthedTenant,
    State(pool): State<PgPool>,
    Json(s): Json<CapturedSession>,
) -> Result<StatusCode, AppError> {
    // Per-repo override (admin work-definition) takes precedence over the org
    // allowlist when host/org/name are all known and an override row exists.
    let override_class = match (
        s.repo.host.as_deref(),
        s.repo.org.as_deref(),
        s.repo.name.as_deref(),
    ) {
        (Some(host), Some(org), Some(name)) => {
            repo_override(&pool, &tenant_id, host, org, name).await?
        }
        _ => None,
    };

    let class = match override_class {
        Some(c) => c,
        None => {
            let allow = load_allowlist(&pool, &tenant_id).await?;
            classify(
                s.repo.host.as_deref(),
                s.repo.org.as_deref(),
                s.repo.path.as_deref(),
                &allow,
            )
            .0
        }
    };

    let first_ts = s.events.iter().map(|e| e.ts).min();
    let last_ts = s.events.iter().map(|e| e.ts).max();

    sqlx::query(
        "insert into captured_sessions \
         (tenant_id, session_id, user_email, repo_host, repo_org, repo_name, repo_path, \
          classification, title, cwd, first_ts, last_ts, event_count) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) \
         on conflict (tenant_id, session_id) do update \
         set last_ts = excluded.last_ts, \
             event_count = excluded.event_count, \
             title = coalesce(excluded.title, captured_sessions.title)",
    )
    .bind(&tenant_id)
    .bind(&s.session_id)
    .bind(&s.user_email)
    .bind(&s.repo.host)
    .bind(&s.repo.org)
    .bind(&s.repo.name)
    .bind(&s.repo.path)
    .bind(class.as_str())
    .bind(&s.title)
    .bind(&s.cwd)
    .bind(first_ts)
    .bind(last_ts)
    .bind(s.events.len() as i32)
    .execute(&pool)
    .await?;

    for e in &s.events {
        let content_sha = match &e.content {
            Some(c) => {
                let sha = hex::encode(Sha256::digest(c.as_bytes()));
                sqlx::query(
                    "insert into content_blobs (tenant_id, sha256, content, bytes) \
                     values ($1,$2,$3,$4) on conflict (tenant_id, sha256) do nothing",
                )
                .bind(&tenant_id)
                .bind(&sha)
                .bind(c)
                .bind(c.len() as i32)
                .execute(&pool)
                .await?;

                // Scan this event's content for secrets / PII and store each
                // finding idempotently (only a redacted preview is persisted).
                for f in ccguard_core::findings::scan(c) {
                    sqlx::query(
                        "insert into findings \
                         (tenant_id, session_id, seq, kind, rule, severity, redacted) \
                         values ($1,$2,$3,$4,$5,$6,$7) \
                         on conflict (tenant_id, session_id, seq, rule, redacted) do nothing",
                    )
                    .bind(&tenant_id)
                    .bind(&s.session_id)
                    .bind(e.seq)
                    .bind(f.kind.as_str())
                    .bind(&f.rule)
                    .bind(f.severity.as_str())
                    .bind(&f.redacted)
                    .execute(&pool)
                    .await?;
                }

                Some(sha)
            }
            None => None,
        };
        sqlx::query(
            "insert into captured_events \
             (tenant_id, session_id, seq, ts, kind, model, tool_name, target, \
              content_sha, tokens_in, tokens_out, is_sidechain) \
             values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) \
             on conflict (tenant_id, session_id, seq) do nothing",
        )
        .bind(&tenant_id)
        .bind(&s.session_id)
        .bind(e.seq)
        .bind(e.ts)
        .bind(e.kind.as_str())
        .bind(&e.model)
        .bind(&e.tool_name)
        .bind(&e.target)
        .bind(&content_sha)
        .bind(e.tokens_in)
        .bind(e.tokens_out)
        .bind(e.is_sidechain)
        .execute(&pool)
        .await?;
    }

    // Recompute aggregates from ALL stored rows for this session (not just this batch),
    // so that chunked / idempotently re-posted batches produce correct cumulative totals.
    sqlx::query(
        "update captured_sessions s set \
           event_count = sub.cnt, \
           first_ts    = sub.min_ts, \
           last_ts     = sub.max_ts \
         from (select count(*)::int as cnt, min(ts) as min_ts, max(ts) as max_ts \
               from captured_events \
               where tenant_id = $1 and session_id = $2) sub \
         where s.tenant_id = $1 and s.session_id = $2",
    )
    .bind(&tenant_id)
    .bind(&s.session_id)
    .execute(&pool)
    .await?;

    // On-task scoring + role anomalies. Defensive: a scoring failure logs but
    // must NOT fail the capture (the events are already durably stored).
    if let Err(e) = score_session(&pool, &tenant_id, &s.session_id, class, &s.user_email).await {
        eprintln!(
            "on-task scoring failed for tenant={tenant_id} session={}: {e}",
            s.session_id
        );
    }

    Ok(StatusCode::ACCEPTED)
}

/// Recompute the on-task score + indicators for a session from ALL stored
/// events (not just the current capture chunk), so chunked / re-posted batches
/// produce a correct full-session view. Idempotent: session_scores is upserted,
/// session_tickets/indicators use on-conflict-do-nothing, and this session's
/// open auto-indicators are cleared then re-inserted from the current view.
async fn score_session(
    pool: &PgPool,
    tenant_id: &str,
    session_id: &str,
    class: Classification,
    user_email: &str,
) -> Result<(), sqlx::Error> {
    // Pull every stored event for the session (joined to its verbatim content).
    let evrows = sqlx::query(
        "select e.kind, e.tool_name, e.target, b.content from captured_events e \
         left join content_blobs b on b.tenant_id=e.tenant_id and b.sha256=e.content_sha \
         where e.tenant_id=$1 and e.session_id=$2",
    )
    .bind(tenant_id)
    .bind(session_id)
    .fetch_all(pool)
    .await?;

    let total_events = evrows.len() as i64;
    let mut assistant_events: i64 = 0;
    let mut committed = false;
    let mut pr_opened = false;
    let mut code_events: i64 = 0;
    let mut tickets: BTreeSet<String> = BTreeSet::new();

    for r in &evrows {
        let kind: String = r.get("kind");
        let tool_name: Option<String> = r.get("tool_name");
        let target: Option<String> = r.get("target");
        let content: Option<String> = r.get("content");
        let tool = tool_name.as_deref().unwrap_or("");

        if kind == "assistant_text" {
            assistant_events += 1;
        }
        if kind == "pr" {
            pr_opened = true;
            committed = true;
        }
        // git commit / push detected from a Bash tool call's target or content.
        if kind == "tool_call" && tool == "Bash" {
            let hay = format!(
                "{} {}",
                target.as_deref().unwrap_or(""),
                content.as_deref().unwrap_or("")
            );
            if hay.contains("git commit") || hay.contains("git push") {
                committed = true;
            }
        }
        // Code-producing activity: a file edit, or a code/file/shell tool call.
        if kind == "file_edit"
            || (kind == "tool_call" && matches!(tool, "Edit" | "Write" | "Bash"))
        {
            code_events += 1;
        }
        // Ticket references from both content and target.
        for field in [content.as_deref(), target.as_deref()].into_iter().flatten() {
            for t in ontask::extract_ticket_refs(field) {
                tickets.insert(t);
            }
        }
    }

    // Persist every referenced ticket (idempotent).
    for t in &tickets {
        sqlx::query(
            "insert into session_tickets (tenant_id, session_id, ticket) \
             values ($1,$2,$3) on conflict do nothing",
        )
        .bind(tenant_id)
        .bind(session_id)
        .bind(t)
        .execute(pool)
        .await?;
    }

    let signals = OnTaskSignals {
        classification: class,
        committed,
        pr_opened,
        ticket_referenced: !tickets.is_empty(),
        total_events,
        assistant_events,
    };
    let (sc, label, reasons) = ontask::score(&signals);
    let reasons_joined = reasons.join("; ");

    sqlx::query(
        "insert into session_scores (tenant_id, session_id, score, label, reasons, updated_at) \
         values ($1,$2,$3,$4,$5, now()) \
         on conflict (tenant_id, session_id) do update set \
           score = excluded.score, \
           label = excluded.label, \
           reasons = excluded.reasons, \
           updated_at = now()",
    )
    .bind(tenant_id)
    .bind(session_id)
    .bind(sc)
    .bind(label.as_str())
    .bind(&reasons_joined)
    .execute(pool)
    .await?;

    // Re-raise auto-indicators from the current full-session view. Clear this
    // session's OPEN auto-indicators first so a chunk that flips a signal does
    // not leave a stale indicator behind, then insert-on-conflict-do-nothing.
    sqlx::query(
        "delete from indicators where tenant_id=$1 and session_id=$2 and status='open' \
         and kind in ('off_task','personal_repo','non_engineer_coding')",
    )
    .bind(tenant_id)
    .bind(session_id)
    .execute(pool)
    .await?;

    if label == ontask::OnTaskLabel::OffTask {
        insert_indicator(
            pool,
            tenant_id,
            session_id,
            user_email,
            "off_task",
            &reasons_joined,
        )
        .await?;
    }
    if class == Classification::Personal {
        insert_indicator(
            pool,
            tenant_id,
            session_id,
            user_email,
            "personal_repo",
            "personal repo on company tooling",
        )
        .await?;
    }

    // Role anomaly: if the employee has an assigned role, compare observed code
    // activity against what the role predicts.
    let role_row = sqlx::query(
        "select job_role from employee_roles where tenant_id=$1 and user_email=$2",
    )
    .bind(tenant_id)
    .bind(user_email)
    .fetch_optional(pool)
    .await?;
    if let Some(row) = role_row {
        let role = JobRole::from_str(&row.get::<String, _>("job_role"));
        let activity = Activity {
            code_events,
            total_events,
        };
        for ind in roles::role_anomalies(role, &activity) {
            insert_indicator(pool, tenant_id, session_id, user_email, &ind.kind, &ind.detail)
                .await?;
        }
    }

    Ok(())
}

/// Insert one open auto-indicator, idempotent on (tenant_id, session_id, kind).
async fn insert_indicator(
    pool: &PgPool,
    tenant_id: &str,
    session_id: &str,
    user_email: &str,
    kind: &str,
    detail: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "insert into indicators (tenant_id, user_email, session_id, kind, detail, status) \
         values ($1,$2,$3,$4,$5,'open') on conflict (tenant_id, session_id, kind) do nothing",
    )
    .bind(tenant_id)
    .bind(user_email)
    .bind(session_id)
    .bind(kind)
    .bind(detail)
    .execute(pool)
    .await?;
    Ok(())
}
