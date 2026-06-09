use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use ccguard_core::classify::{classify, Allowlist};
use ccguard_core::event::CcEvent;
use sqlx::{PgPool, Row};

use crate::error::AppError;

async fn load_allowlist(pool: &PgPool, tenant_id: &str) -> Result<Allowlist, sqlx::Error> {
    let rows = sqlx::query("select kind, value from allowlist_rules where tenant_id = $1")
        .bind(tenant_id)
        .fetch_all(pool)
        .await?;
    let mut allow = Allowlist::default();
    for row in rows {
        let kind: String = row.get("kind");
        let value: String = row.get("value");
        match kind.as_str() {
            "host" => allow.hosts.push(value),
            "org" => allow.orgs.push(value),
            "path_root" => allow.path_roots.push(value),
            _ => {}
        }
    }
    Ok(allow)
}

/// Ingest a CcEvent. The server is authoritative on classification: any
/// `repo.classification`/`repo.confidence` in the request payload is ignored and
/// recomputed from the tenant's allowlist.
pub async fn ingest(
    State(pool): State<PgPool>,
    Json(ev): Json<CcEvent>,
) -> Result<StatusCode, AppError> {
    let allow = load_allowlist(&pool, &ev.tenant_id).await?;
    let (class, confidence) = classify(
        ev.repo.host.as_deref(),
        ev.repo.org.as_deref(),
        ev.repo.path.as_deref(),
        &allow,
    );

    sqlx::query(
        "insert into events (tenant_id, user_email, seat_id, tool, session_id, ts, \
         repo_host, repo_org, repo_name, repo_path, classification, confidence, \
         activity_type, tokens_in, tokens_out, cost_usd, model, tool_name, content_ref, source_layer) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)",
    )
    .bind(&ev.tenant_id)
    .bind(&ev.user.email)
    .bind(&ev.user.seat_id)
    .bind(&ev.tool)
    .bind(&ev.session_id)
    .bind(ev.ts)
    .bind(&ev.repo.host)
    .bind(&ev.repo.org)
    .bind(&ev.repo.name)
    .bind(&ev.repo.path)
    .bind(class.as_str())
    .bind(confidence)
    .bind(&ev.activity.kind)
    .bind(ev.activity.tokens_in)
    .bind(ev.activity.tokens_out)
    .bind(ev.activity.cost_usd)
    .bind(&ev.activity.model)
    .bind(&ev.activity.tool_name)
    .bind(&ev.content_ref)
    .bind(&ev.source_layer)
    .execute(&pool)
    .await?;

    Ok(StatusCode::ACCEPTED)
}
