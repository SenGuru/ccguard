use axum::extract::State;
use axum::Json;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

use crate::error::AppError;
use crate::passwords::verify_password;
use crate::tokens::generate_token;

#[derive(Deserialize)]
pub struct LoginReq {
    pub tenant_id: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResp {
    pub session_token: String,
    pub role: String,
}

pub async fn login(
    State(pool): State<PgPool>,
    Json(body): Json<LoginReq>,
) -> Result<Json<LoginResp>, AppError> {
    let row = sqlx::query(
        "select id, password_hash, role from users where tenant_id = $1 and email = $2",
    )
    .bind(&body.tenant_id)
    .bind(&body.email)
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::Unauthorized("invalid credentials"))?;

    let user_id: i64 = row.get("id");
    let password_hash: String = row.get("password_hash");
    let role: String = row.get("role");

    if !verify_password(&body.password, &password_hash) {
        return Err(AppError::Unauthorized("invalid credentials"));
    }

    let (token, token_hash) = generate_token();
    let expires_at = Utc::now() + Duration::days(30);
    sqlx::query("insert into sessions (user_id, token_hash, expires_at) values ($1,$2,$3)")
        .bind(user_id)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&pool)
        .await?;

    Ok(Json(LoginResp {
        session_token: token,
        role,
    }))
}
