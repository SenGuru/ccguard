use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use ccguard_core::capture::CapturedSession;
use ccguard_core::classify::{classify, Allowlist};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

use crate::auth::AuthedTenant;
use crate::error::AppError;

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
    let allow = load_allowlist(&pool, &tenant_id).await?;
    let (class, _conf) = classify(
        s.repo.host.as_deref(),
        s.repo.org.as_deref(),
        s.repo.path.as_deref(),
        &allow,
    );

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

    Ok(StatusCode::ACCEPTED)
}
