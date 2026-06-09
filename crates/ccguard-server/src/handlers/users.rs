use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use sqlx::PgPool;

use crate::error::AppError;
use crate::passwords::hash_password;

const ROLES: [&str; 5] = ["owner", "admin", "manager", "auditor", "member"];

#[derive(Deserialize)]
pub struct NewUser {
    pub tenant_id: String,
    pub email: String,
    pub password: String,
    pub role: String,
}

/// Admin-gated user creation (bootstrap). Requires `X-Admin-Token` == `ADMIN_TOKEN`.
pub async fn create_user(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(body): Json<NewUser>,
) -> Result<StatusCode, AppError> {
    let admin = std::env::var("ADMIN_TOKEN").unwrap_or_default();
    let provided = headers
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if admin.is_empty() || provided != admin {
        return Err(AppError::Unauthorized("admin token required"));
    }
    if !ROLES.contains(&body.role.as_str()) {
        return Err(AppError::BadRequest("invalid role"));
    }

    let password_hash = hash_password(&body.password);
    sqlx::query("insert into users (tenant_id, email, password_hash, role) values ($1,$2,$3,$4)")
        .bind(&body.tenant_id)
        .bind(&body.email)
        .bind(&password_hash)
        .bind(&body.role)
        .execute(&pool)
        .await?;

    Ok(StatusCode::CREATED)
}
