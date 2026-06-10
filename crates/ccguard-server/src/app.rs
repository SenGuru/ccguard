use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;

use crate::handlers::{capture, ingest, sessions, summary, tenants, timeline, users};

pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/v1/tenants", post(tenants::create_tenant))
        .route("/v1/users", post(users::create_user))
        .route("/v1/auth/login", post(sessions::login))
        .route("/v1/events", post(ingest::ingest))
        .route("/v1/capture", post(capture::capture))
        .route("/v1/orgs/:tenant/summary", get(summary::summary))
        .route("/v1/orgs/:tenant/sessions", get(timeline::list_sessions))
        .route("/v1/sessions/:session_id/timeline", get(timeline::timeline))
        .with_state(pool)
}
