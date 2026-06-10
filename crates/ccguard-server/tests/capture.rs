use axum::body::Body;
use axum::http::{Request, StatusCode};
use sqlx::{PgPool, Row};
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::tokens::generate_token;

async fn seed(pool: &PgPool) -> String {
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
    let (t, h) = generate_token();
    sqlx::query("insert into api_tokens (tenant_id, token_hash) values ('acme',$1)")
        .bind(&h)
        .execute(pool)
        .await
        .unwrap();
    t
}

#[sqlx::test(migrations = "./migrations")]
async fn stores_session_events_and_dedupes_content(pool: PgPool) {
    let token = seed(&pool).await;
    let body = serde_json::json!({
        "session_id":"s1","user_email":"dev@acme.com",
        "repo":{"host":"github.com","org":"acme-corp","name":"r","path":"C:\\w"},
        "title":"build","cwd":"C:\\w",
        "events":[
          {"seq":0,"ts":"2026-06-10T10:00:00Z","kind":"user_prompt","content":"do X"},
          {"seq":1,"ts":"2026-06-10T10:00:01Z","kind":"tool_call","tool_name":"Bash","target":"git status","content":"do X"},
          {"seq":2,"ts":"2026-06-10T10:00:02Z","kind":"assistant_text","model":"claude-opus-4-8","content":"done","tokens_in":100,"tokens_out":20}
        ]
    })
    .to_string();
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/capture")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let sess = sqlx::query(
        "select classification, event_count from captured_sessions \
         where tenant_id='acme' and session_id='s1'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(sess.get::<String, _>("classification"), "work");
    assert_eq!(sess.get::<i32, _>("event_count"), 3);
    let ev = sqlx::query("select count(*) c from captured_events where session_id='s1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(ev.get::<i64, _>("c"), 3);
    // "do X" content appears twice but is one blob (deduped):
    let blobs =
        sqlx::query("select count(*) c from content_blobs where tenant_id='acme'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(blobs.get::<i64, _>("c"), 2); // "do X" and "done"
}

#[sqlx::test(migrations = "./migrations")]
async fn capture_requires_token(pool: PgPool) {
    seed(&pool).await;
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/capture")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"session_id":"s","user_email":"x","repo":{},"events":[]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
