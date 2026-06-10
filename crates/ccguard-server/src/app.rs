use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;

use crate::handlers::{
    capture, findings, fleet, ingest, ontask, search, sessions, summary, tenants, timeline, users,
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
        .route("/dashboard/fleet", get(web::fleet))
        .route(
            "/dashboard/policy",
            get(web::policy_get).post(web::policy_set),
        )
        .route(
            "/dashboard/policy/managed-settings.json",
            get(web::policy_download),
        )
        .route("/dashboard/review", get(web::review))
        .route(
            "/dashboard/indicators/:id/status",
            post(web::indicator_status),
        )
        .route(
            "/dashboard/roles",
            get(web::roles_get).post(web::roles_set),
        )
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
        .route(
            "/v1/orgs/:tenant/repo-overrides",
            post(ontask::set_repo_override),
        )
        .route("/v1/orgs/:tenant/roles", post(ontask::set_role))
        .route(
            "/v1/orgs/:tenant/indicators",
            get(ontask::list_indicators),
        )
        .route(
            "/v1/indicators/:id/status",
            post(ontask::set_indicator_status),
        )
        .route("/v1/orgs/:tenant/ontask", get(ontask::rollup))
        .with_state(pool)
}
