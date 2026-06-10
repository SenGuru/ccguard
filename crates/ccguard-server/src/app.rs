use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;

use crate::handlers::{
    capture, findings, fleet, ingest, search, sessions, summary, tenants, timeline, users,
};
use crate::web;

pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/", get(web::root))
        .route("/login", get(web::login_get))
        .route("/web/login", post(web::login_post))
        .route("/dashboard", get(web::dashboard))
        .route("/dashboard/sessions/:session_id", get(web::session_view))
        .route("/dashboard/sessions/:session_id/export", get(web::export))
        .route("/dashboard/sessions/:session_id/hold", post(web::hold))
        .route("/dashboard/findings", get(web::findings))
        .route("/dashboard/search", get(web::search))
        .route("/v1/tenants", post(tenants::create_tenant))
        .route("/v1/users", post(users::create_user))
        .route("/v1/auth/login", post(sessions::login))
        .route("/v1/events", post(ingest::ingest))
        .route(
            "/v1/capture",
            post(capture::capture).layer(DefaultBodyLimit::max(64 * 1024 * 1024)),
        )
        .route("/v1/orgs/:tenant/policy", post(fleet::set_policy))
        .route("/v1/orgs/:tenant/fleet", get(fleet::list))
        .route("/v1/enroll", post(fleet::enroll))
        .route("/v1/attest", post(fleet::attest))
        .route("/v1/orgs/:tenant/summary", get(summary::summary))
        .route("/v1/orgs/:tenant/sessions", get(timeline::list_sessions))
        .route("/v1/orgs/:tenant/findings", get(findings::list))
        .route("/v1/orgs/:tenant/search", get(search::search))
        .route("/v1/sessions/:session_id/timeline", get(timeline::timeline))
        .with_state(pool)
}
