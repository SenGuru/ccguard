use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use sqlx::{PgPool, Row};

use crate::auth::AuthedUser;
use crate::error::AppError;

#[derive(Serialize)]
pub struct SessionRow {
    pub session_id: String,
    pub user_email: String,
    pub classification: String,
    pub repo_org: Option<String>,
    pub repo_name: Option<String>,
    pub title: Option<String>,
    pub event_count: i32,
}

/// List captured sessions for the caller's tenant.
pub async fn list_sessions(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(tenant): Path<String>,
) -> Result<Json<Vec<SessionRow>>, AppError> {
    if user.tenant_id != tenant {
        return Err(AppError::Forbidden("cross-tenant access denied"));
    }
    let rows = sqlx::query(
        "select session_id, user_email, classification, repo_org, repo_name, title, event_count \
         from captured_sessions where tenant_id = $1 order by last_ts desc nulls last limit 500",
    )
    .bind(&tenant)
    .fetch_all(&pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| SessionRow {
                session_id: r.get("session_id"),
                user_email: r.get("user_email"),
                classification: r.get("classification"),
                repo_org: r.get("repo_org"),
                repo_name: r.get("repo_name"),
                title: r.get("title"),
                event_count: r.get("event_count"),
            })
            .collect(),
    ))
}

#[derive(Serialize)]
pub struct TimelineEvent {
    pub seq: i64,
    pub ts: chrono::DateTime<chrono::Utc>,
    pub kind: String,
    pub model: Option<String>,
    pub tool_name: Option<String>,
    pub target: Option<String>,
    pub content: Option<String>,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub is_sidechain: bool,
}

/// Full ordered timeline (with verbatim content) for one session, scoped to the caller's tenant.
pub async fn timeline(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<TimelineEvent>>, AppError> {
    let rows = sqlx::query(
        "select e.seq, e.ts, e.kind, e.model, e.tool_name, e.target, b.content, \
                e.tokens_in, e.tokens_out, e.is_sidechain \
         from captured_events e \
         left join content_blobs b \
           on b.tenant_id = e.tenant_id and b.sha256 = e.content_sha \
         where e.tenant_id = $1 and e.session_id = $2 \
         order by e.seq",
    )
    .bind(&user.tenant_id)
    .bind(&session_id)
    .fetch_all(&pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| TimelineEvent {
                seq: r.get("seq"),
                ts: r.get("ts"),
                kind: r.get("kind"),
                model: r.get("model"),
                tool_name: r.get("tool_name"),
                target: r.get("target"),
                content: r.get("content"),
                tokens_in: r.get("tokens_in"),
                tokens_out: r.get("tokens_out"),
                is_sidechain: r.get("is_sidechain"),
            })
            .collect(),
    ))
}
