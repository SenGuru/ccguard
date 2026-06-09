use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::{PgPool, Row};
use tower::ServiceExt;

use ccguard_server::app::app;

#[sqlx::test(migrations = "./migrations")]
async fn provisions_tenant_with_token(pool: PgPool) {
    std::env::set_var("ADMIN_TOKEN", "secret-admin");

    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/tenants")
                .header("content-type", "application/json")
                .header("x-admin-token", "secret-admin")
                .body(Body::from(r#"{"name":"Acme Inc"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let tenant_id = v["tenant_id"].as_str().unwrap();
    let token = v["ingest_token"].as_str().unwrap();
    assert!(token.starts_with("ccg_"));

    // tenant + token row persisted
    let row = sqlx::query("select name from tenants where id = $1")
        .bind(tenant_id).fetch_one(&pool).await.unwrap();
    let name: String = row.get("name");
    assert_eq!(name, "Acme Inc");

    let cnt = sqlx::query("select count(*) as c from api_tokens where tenant_id = $1")
        .bind(tenant_id).fetch_one(&pool).await.unwrap();
    let c: i64 = cnt.get("c");
    assert_eq!(c, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn rejects_wrong_admin_token(pool: PgPool) {
    std::env::set_var("ADMIN_TOKEN", "secret-admin");

    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/tenants")
                .header("content-type", "application/json")
                .header("x-admin-token", "WRONG")
                .body(Body::from(r#"{"name":"Acme Inc"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
