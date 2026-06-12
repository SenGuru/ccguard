use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;

use crate::handlers::{
    capture, enforcement, findings, fleet, ingest, ontask, search, sessions, summary, tenants,
    timeline, triage, users,
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
        .route("/dashboard/triage", get(web::triage_page))
        .route("/dashboard/triage/config", post(web::triage_config_set))
        .route("/dashboard/triage/run", post(web::triage_run))
        .route(
            "/dashboard/triage/:session_id/confirm",
            post(web::triage_confirm),
        )
        .route(
            "/dashboard/triage/:session_id/relabel",
            post(web::triage_relabel),
        )
        .route(
            "/dashboard/signals",
            get(web::signals_page),
        )
        .route("/dashboard/signals/config", post(web::signals_config_set))
        .route("/dashboard/usage", get(web::usage_page))
        .route("/dashboard/usage/config", post(web::usage_config_set))
        .route("/dashboard/enforcement", get(web::enforcement_page))
        .route("/dashboard/enforcement/recompute", post(web::enforcement_recompute))
        .route("/dashboard/enforcement/arm", post(web::enforcement_arm))
        .route("/dashboard/enforcement/disarm", post(web::enforcement_disarm))
        .route(
            "/v1/enforcement/decision",
            get(enforcement::decision_endpoint),
        )
        .route("/v1/triage/pending", get(triage::pending_endpoint))
        .route("/v1/triage/verdict", post(triage::verdict_endpoint))
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
