use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::AppError;
use crate::tokens::generate_token;

#[derive(Deserialize)]
pub struct NewTenant {
    pub name: String,
}

#[derive(Serialize)]
pub struct TenantCreated {
    pub tenant_id: String,
    pub ingest_token: String,
}

fn random_tenant_id() -> String {
    let mut bytes = [0u8; 6];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("t_{}", hex::encode(bytes))
}

/// Admin-gated tenant provisioning. Requires `X-Admin-Token` matching the
/// `ADMIN_TOKEN` env var. Creates a tenant and its first ingest token; the token
/// plaintext is returned exactly once.
pub async fn create_tenant(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(body): Json<NewTenant>,
) -> Result<Json<TenantCreated>, AppError> {
    let admin = std::env::var("ADMIN_TOKEN").unwrap_or_default();
    let provided = headers
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if admin.is_empty() || provided != admin {
        return Err(AppError::Unauthorized("admin token required"));
    }

    let tenant_id = random_tenant_id();
    sqlx::query("insert into tenants (id, name) values ($1, $2)")
        .bind(&tenant_id)
        .bind(&body.name)
        .execute(&pool)
        .await?;

    let (token, hash) = generate_token();
    sqlx::query("insert into api_tokens (tenant_id, token_hash) values ($1, $2)")
        .bind(&tenant_id)
        .bind(&hash)
        .execute(&pool)
        .await?;

    Ok(Json(TenantCreated {
        tenant_id,
        ingest_token: token,
    }))
}
