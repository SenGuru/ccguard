use axum::Router;
use sqlx::PgPool;

#[allow(unused_imports)]
use crate::handlers::{ingest, summary};

pub fn app(pool: PgPool) -> Router {
    Router::new()
        // .route("/v1/events", axum::routing::post(ingest::ingest))          // added in Task 7
        // .route("/v1/orgs/:tenant/summary", axum::routing::get(summary::summary)) // added in Task 8
        .with_state(pool)
}
