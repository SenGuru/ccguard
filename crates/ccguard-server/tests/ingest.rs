use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::{PgPool, Row};
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::tokens::generate_token;

/// Seed tenant + allowlist + an ingest token; return the plaintext token.
async fn seed(pool: &PgPool) -> String {
    sqlx::query("insert into tenants (id, name) values ('acme', 'Acme')")
        .execute(pool).await.unwrap();
    sqlx::query("insert into allowlist_rules (tenant_id, kind, value) values ('acme','host','github.com'),('acme','org','acme-corp')")
        .execute(pool).await.unwrap();
    let (token, hash) = generate_token();
    sqlx::query("insert into api_tokens (tenant_id, token_hash) values ('acme', $1)")
        .bind(&hash).execute(pool).await.unwrap();
    token
}

fn ev_body(session: &str, org: &str, cost: f64) -> String {
    serde_json::json!({
        "tenant_id": "ignored-by-server",
        "user": { "email": "dev@acme.com" },
        "tool": "claude-code",
        "session_id": session,
        "ts": "2026-06-10T10:00:00Z",
        "repo": { "host": "github.com", "org": org, "name": "r" },
        "source_layer": "endpoint_agent",
        "activity": { "type": "api_request", "cost_usd": cost }
    }).to_string()
}

#[sqlx::test(migrations = "./migrations")]
async fn ingest_classifies_company_repo_as_work(pool: PgPool) {
    let token = seed(&pool).await;
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/events")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(ev_body("s1", "acme-corp", 0.5)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let row = sqlx::query("select tenant_id, classification from events where session_id = 's1'")
        .fetch_one(&pool).await.unwrap();
    let tenant: String = row.get("tenant_id");
    let class: String = row.get("classification");
    assert_eq!(tenant, "acme"); // from the token, NOT the body's "ignored-by-server"
    assert_eq!(class, "work");
}

#[sqlx::test(migrations = "./migrations")]
async fn ingest_classifies_outside_repo_as_personal(pool: PgPool) {
    let token = seed(&pool).await;
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/events")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(ev_body("s2", "dev-personal", 0.3)))
                .unwrap(),
        )
        .await
        .unwrap();
    let _ = resp.collect().await;

    let row = sqlx::query("select classification from events where session_id = 's2'")
        .fetch_one(&pool).await.unwrap();
    let class: String = row.get("classification");
    assert_eq!(class, "personal");
}

#[sqlx::test(migrations = "./migrations")]
async fn ingest_without_token_is_unauthorized(pool: PgPool) {
    seed(&pool).await;
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/events")
                .header("content-type", "application/json")
                .body(Body::from(ev_body("s3", "acme-corp", 0.5)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // nothing stored
    let cnt = sqlx::query("select count(*) as c from events")
        .fetch_one(&pool).await.unwrap();
    let c: i64 = cnt.get("c");
    assert_eq!(c, 0);
}
