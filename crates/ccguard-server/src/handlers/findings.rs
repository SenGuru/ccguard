use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use sqlx::{PgPool, Row};

use crate::auth::AuthedUser;
use crate::error::AppError;

#[derive(Serialize)]
pub struct FindingRow {
    pub session_id: String,
    pub seq: i64,
    pub kind: String,
    pub rule: String,
    pub severity: String,
    pub redacted: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// List secret / PII findings for the caller's tenant, newest first (limit 200).
/// Tenant-scoped: a caller may only read findings for their own tenant.
pub async fn list(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(tenant): Path<String>,
) -> Result<Json<Vec<FindingRow>>, AppError> {
    if user.tenant_id != tenant {
        return Err(AppError::Forbidden("cross-tenant access denied"));
    }
    let rows = sqlx::query(
        "select session_id, seq, kind, rule, severity, redacted, created_at \
         from findings where tenant_id = $1 order by created_at desc, id desc limit 200",
    )
    .bind(&tenant)
    .fetch_all(&pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| FindingRow {
                session_id: r.get("session_id"),
                seq: r.get("seq"),
                kind: r.get("kind"),
                rule: r.get("rule"),
                severity: r.get("severity"),
                redacted: r.get("redacted"),
                created_at: r.get("created_at"),
            })
            .collect(),
    ))
}
