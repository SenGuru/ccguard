use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;

use crate::handlers::{ingest, sessions, summary, tenants, users};

pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/v1/tenants", post(tenants::create_tenant))
        .route("/v1/users", post(users::create_user))
        .route("/v1/auth/login", post(sessions::login))
        .route("/v1/events", post(ingest::ingest))
        .route("/v1/orgs/:tenant/summary", get(summary::summary))
        .with_state(pool)
}
