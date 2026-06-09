use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;

use crate::handlers::{ingest, summary};

pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/v1/events", post(ingest::ingest))
        .route("/v1/orgs/:tenant/summary", get(summary::summary))
        .with_state(pool)
}
