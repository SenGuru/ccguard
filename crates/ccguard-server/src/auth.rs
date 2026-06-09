use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use sqlx::{PgPool, Row};

use crate::error::AppError;
use crate::tokens::hash_token;

/// Resolves `Authorization: Bearer <token>` to the owning tenant id, or 401.
pub struct AuthedTenant(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for AuthedTenant
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = PgPool::from_ref(state);
        let token = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized("missing bearer token"))?;

        let hash = hash_token(token);
        let row = sqlx::query(
            "select tenant_id from api_tokens where token_hash = $1 and revoked_at is null",
        )
        .bind(&hash)
        .fetch_optional(&pool)
        .await?;

        match row {
            Some(r) => Ok(AuthedTenant(r.get("tenant_id"))),
            None => Err(AppError::Unauthorized("invalid token")),
        }
    }
}
