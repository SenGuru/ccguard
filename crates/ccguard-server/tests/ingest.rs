use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::{PgPool, Row};
use tower::ServiceExt; // for `oneshot`

use ccguard_server::app::app;

async fn seed(pool: &PgPool) {
    sqlx::query("insert into tenants (id, name) values ('acme', 'Acme')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("insert into allowlist_rules (tenant_id, kind, value) values ('acme','host','github.com'),('acme','org','acme-corp')")
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn ingest_classifies_company_repo_as_work(pool: PgPool) {
    seed(&pool).await;

    let body = serde_json::json!({
        "tenant_id": "acme",
        "user": { "email": "dev@acme.com" },
        "tool": "claude-code",
        "session_id": "s1",
        "ts": "2026-06-09T21:13:00Z",
        "repo": { "host": "github.com", "org": "acme-corp", "name": "billing" },
        "source_layer": "endpoint_agent",
        "activity": { "type": "api_request", "cost_usd": 0.5, "tokens_in": 100, "tokens_out": 20 }
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

    let row = sqlx::query("select classification, cost_usd from events where tenant_id = 'acme'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let class: String = row.get("classification");
    let cost: f64 = row.get("cost_usd");
    assert_eq!(class, "work");
    assert_eq!(cost, 0.5);
}

#[sqlx::test(migrations = "./migrations")]
async fn ingest_classifies_outside_repo_as_personal(pool: PgPool) {
    seed(&pool).await;

    let body = serde_json::json!({
        "tenant_id": "acme",
        "user": { "email": "dev@acme.com" },
        "tool": "claude-code",
        "session_id": "s2",
        "ts": "2026-06-09T21:14:00Z",
        "repo": { "host": "github.com", "org": "dev-personal", "name": "sideproj" },
        "source_layer": "endpoint_agent",
        "activity": { "type": "api_request", "cost_usd": 0.3 }
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
    let _ = resp.collect().await;

    let row = sqlx::query("select classification from events where session_id = 's2'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let class: String = row.get("classification");
    assert_eq!(class, "personal");
}
