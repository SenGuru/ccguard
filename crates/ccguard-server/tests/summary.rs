use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

use ccguard_server::app::app;

async fn post_event(pool: &PgPool, session: &str, org: &str, cost: f64) {
    let body = serde_json::json!({
        "tenant_id": "acme",
        "user": { "email": "dev@acme.com" },
        "tool": "claude-code",
        "session_id": session,
        "ts": "2026-06-09T21:13:00Z",
        "repo": { "host": "github.com", "org": org, "name": "r" },
        "source_layer": "endpoint_agent",
        "activity": { "type": "api_request", "cost_usd": cost }
    });
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/events")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[sqlx::test(migrations = "./migrations")]
async fn summary_groups_spend_by_classification(pool: PgPool) {
    sqlx::query("insert into tenants (id, name) values ('acme','Acme')")
        .execute(&pool).await.unwrap();
    sqlx::query("insert into allowlist_rules (tenant_id, kind, value) values ('acme','host','github.com'),('acme','org','acme-corp')")
        .execute(&pool).await.unwrap();

    post_event(&pool, "s1", "acme-corp", 1.0).await;
    post_event(&pool, "s2", "acme-corp", 0.5).await;
    post_event(&pool, "s3", "dev-personal", 0.25).await;

    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/orgs/acme/summary")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let work = v.as_array().unwrap().iter().find(|x| x["classification"] == "work").unwrap();
    let personal = v.as_array().unwrap().iter().find(|x| x["classification"] == "personal").unwrap();
    assert_eq!(work["cost_usd"], 1.5);
    assert_eq!(work["events"], 2);
    assert_eq!(personal["cost_usd"], 0.25);
}
