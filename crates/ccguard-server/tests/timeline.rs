use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::passwords::hash_password;
use ccguard_server::tokens::generate_token;

/// Seed tenant 'acme' + allowlist + ingest token + owner user.
async fn seed(pool: &PgPool) -> (String, String) {
    sqlx::query("insert into tenants (id,name) values ('acme','Acme')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "insert into allowlist_rules (tenant_id,kind,value) values \
         ('acme','host','github.com'),('acme','org','acme-corp')",
    )
    .execute(pool)
    .await
    .unwrap();
    let (ingest_token, ingest_hash) = generate_token();
    sqlx::query("insert into api_tokens (tenant_id, token_hash) values ('acme',$1)")
        .bind(&ingest_hash)
        .execute(pool)
        .await
        .unwrap();
    let ph = hash_password("pw12345");
    sqlx::query(
        "insert into users (tenant_id,email,password_hash,role) values \
         ('acme','boss@acme.com',$1,'owner')",
    )
    .bind(&ph)
    .execute(pool)
    .await
    .unwrap();
    (ingest_token, "boss@acme.com".to_string())
}

async fn login(pool: &PgPool, tenant: &str, email: &str, pw: &str) -> String {
    let body = serde_json::json!({"tenant_id": tenant, "email": email, "password": pw}).to_string();
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["session_token"].as_str().unwrap().to_string()
}

async fn post_capture(pool: &PgPool, ingest_token: &str) {
    let body = serde_json::json!({
        "session_id": "s1",
        "user_email": "dev@acme.com",
        "repo": {"host": "github.com", "org": "acme-corp", "name": "billing", "path": "C:\\w"},
        "title": "Build billing module",
        "cwd": "C:\\w",
        "events": [
            {"seq": 0, "ts": "2026-06-10T10:00:00Z", "kind": "user_prompt", "content": "implement billing"},
            {"seq": 1, "ts": "2026-06-10T10:00:01Z", "kind": "tool_call", "tool_name": "Bash", "target": "git status", "content": "{\"command\":\"git status\"}"},
            {"seq": 2, "ts": "2026-06-10T10:00:02Z", "kind": "assistant_text", "model": "claude-opus-4-8", "content": "done", "tokens_in": 100, "tokens_out": 20}
        ]
    })
    .to_string();
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/capture")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {ingest_token}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[sqlx::test(migrations = "./migrations")]
async fn list_sessions_and_timeline(pool: PgPool) {
    let (ingest, _) = seed(&pool).await;
    post_capture(&pool, &ingest).await;
    let session_token = login(&pool, "acme", "boss@acme.com", "pw12345").await;

    // GET /v1/orgs/acme/sessions -> 1 session, classification work
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/orgs/acme/sessions")
                .header("authorization", format!("Bearer {session_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let sessions: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let sessions = sessions.as_array().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["session_id"], "s1");
    assert_eq!(sessions[0]["classification"], "work");
    assert_eq!(sessions[0]["event_count"], 3);
    assert_eq!(sessions[0]["title"], "Build billing module");

    // GET /v1/sessions/s1/timeline -> 3 events in seq order, with content, bash has tool_name
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/sessions/s1/timeline")
                .header("authorization", format!("Bearer {session_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let events: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let events = events.as_array().unwrap();
    assert_eq!(events.len(), 3);
    // seq order
    assert_eq!(events[0]["seq"], 0);
    assert_eq!(events[0]["kind"], "user_prompt");
    assert_eq!(events[0]["content"], "implement billing");
    // bash tool_call has tool_name
    assert_eq!(events[1]["seq"], 1);
    assert_eq!(events[1]["kind"], "tool_call");
    assert_eq!(events[1]["tool_name"], "Bash");
    assert_eq!(events[1]["target"], "git status");
    // assistant text has model and tokens
    assert_eq!(events[2]["seq"], 2);
    assert_eq!(events[2]["kind"], "assistant_text");
    assert_eq!(events[2]["model"], "claude-opus-4-8");
    assert_eq!(events[2]["tokens_in"], 100);
    assert_eq!(events[2]["tokens_out"], 20);
}

#[sqlx::test(migrations = "./migrations")]
async fn unauthenticated_timeline_returns_401(pool: PgPool) {
    let (ingest, _) = seed(&pool).await;
    post_capture(&pool, &ingest).await;

    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/sessions/s1/timeline")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn cross_tenant_sessions_list_is_forbidden(pool: PgPool) {
    let (ingest, _) = seed(&pool).await;
    post_capture(&pool, &ingest).await;

    // create a second tenant + user
    sqlx::query("insert into tenants (id,name) values ('other','Other')")
        .execute(&pool)
        .await
        .unwrap();
    let ph = hash_password("pw12345");
    sqlx::query(
        "insert into users (tenant_id,email,password_hash,role) values \
         ('other','x@other.com',$1,'owner')",
    )
    .bind(&ph)
    .execute(&pool)
    .await
    .unwrap();

    let other_session = login(&pool, "other", "x@other.com", "pw12345").await;
    // 'other' user reading 'acme' sessions -> 403
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/orgs/acme/sessions")
                .header("authorization", format!("Bearer {other_session}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
