use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::passwords::hash_password;
use ccguard_server::tokens::generate_token;

/// Seed tenant 'acme' + allowlist + an ingest token + an owner user. Returns the ingest token.
async fn seed(pool: &PgPool) -> String {
    sqlx::query("insert into tenants (id,name) values ('acme','Acme')")
        .execute(pool).await.unwrap();
    sqlx::query("insert into allowlist_rules (tenant_id,kind,value) values ('acme','host','github.com'),('acme','org','acme-corp')")
        .execute(pool).await.unwrap();
    let (token, hash) = generate_token();
    sqlx::query("insert into api_tokens (tenant_id, token_hash) values ('acme',$1)")
        .bind(&hash).execute(pool).await.unwrap();
    let ph = hash_password("pw12345");
    sqlx::query("insert into users (tenant_id,email,password_hash,role) values ('acme','boss@acme.com',$1,'owner')")
        .bind(&ph).execute(pool).await.unwrap();
    token
}

async fn login(pool: &PgPool, tenant: &str, email: &str, pw: &str) -> String {
    let body = serde_json::json!({"tenant_id":tenant,"email":email,"password":pw}).to_string();
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("POST").uri("/v1/auth/login")
            .header("content-type", "application/json").body(Body::from(body)).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["session_token"].as_str().unwrap().to_string()
}

async fn post_event(pool: &PgPool, ingest: &str, session: &str, org: &str, cost: f64) {
    let body = serde_json::json!({
        "tenant_id":"ignored","user":{"email":"d@acme.com"},"tool":"claude-code",
        "session_id":session,"ts":"2026-06-10T10:00:00Z",
        "repo":{"host":"github.com","org":org,"name":"r"},
        "source_layer":"endpoint_agent","activity":{"type":"api_request","cost_usd":cost}
    }).to_string();
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("POST").uri("/v1/events")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {ingest}"))
            .body(Body::from(body)).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[sqlx::test(migrations = "./migrations")]
async fn summary_requires_auth_and_groups_spend(pool: PgPool) {
    let ingest = seed(&pool).await;
    let session = login(&pool, "acme", "boss@acme.com", "pw12345").await;
    post_event(&pool, &ingest, &session, "acme-corp", 1.0).await;
    post_event(&pool, &ingest, &session, "acme-corp", 0.5).await;
    post_event(&pool, &ingest, &session, "dev-personal", 0.25).await;

    // unauthenticated -> 401
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("GET").uri("/v1/orgs/acme/summary")
            .body(Body::empty()).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // authenticated -> 200 + grouped totals
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("GET").uri("/v1/orgs/acme/summary")
            .header("authorization", format!("Bearer {session}"))
            .body(Body::empty()).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let work = v.as_array().unwrap().iter().find(|x| x["classification"] == "work").unwrap();
    assert_eq!(work["cost_usd"], 1.5);
    assert_eq!(work["events"], 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn summary_cross_tenant_is_forbidden(pool: PgPool) {
    seed(&pool).await; // tenant 'acme' + its owner
    // a second tenant + user
    sqlx::query("insert into tenants (id,name) values ('other','Other')")
        .execute(&pool).await.unwrap();
    let ph = hash_password("pw12345");
    sqlx::query("insert into users (tenant_id,email,password_hash,role) values ('other','x@other.com',$1,'owner')")
        .bind(&ph).execute(&pool).await.unwrap();

    let session = login(&pool, "other", "x@other.com", "pw12345").await;
    // 'other' user reading 'acme' summary -> 403
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("GET").uri("/v1/orgs/acme/summary")
            .header("authorization", format!("Bearer {session}"))
            .body(Body::empty()).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
