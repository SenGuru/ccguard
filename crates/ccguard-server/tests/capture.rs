use axum::body::Body;
use axum::http::{Request, StatusCode};
use sqlx::{PgPool, Row};
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::tokens::generate_token;

/// POST a CapturedSession body to /v1/capture with the given ingest token; returns the status.
async fn post_capture(pool: &PgPool, token: &str, body: String) -> StatusCode {
    app(pool.clone())
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
        .unwrap()
        .status()
}

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
    // AI-primary: a corp remote alone is now 'pending' (the AI owns the label). To
    // get a deterministic 'work' shortcut we need a Tier-G ground-truth signal — a
    // real push to the corp org. `signals.pushed` + the allowlisted remote → W-PUSH.
    let body = serde_json::json!({
        "session_id":"s1","user_email":"dev@acme.com",
        "repo":{"host":"github.com","org":"acme-corp","name":"r","path":"C:\\w"},
        "title":"build","cwd":"C:\\w",
        "signals":{"pushed":true},
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

#[sqlx::test(migrations = "./migrations")]
async fn chunked_session_accumulates_events_and_recomputes_count(pool: PgPool) {
    let token = seed(&pool).await;

    // First chunk: seqs 0-1.
    let chunk_a = serde_json::json!({
        "session_id":"chunked","user_email":"dev@acme.com",
        "repo":{"host":"github.com","org":"acme-corp","name":"r","path":"C:\\w"},
        "title":"big session","cwd":"C:\\w",
        "events":[
          {"seq":0,"ts":"2026-06-10T10:00:00Z","kind":"user_prompt","content":"a0"},
          {"seq":1,"ts":"2026-06-10T10:00:01Z","kind":"assistant_text","content":"a1"}
        ]
    })
    .to_string();
    assert_eq!(
        post_capture(&pool, &token, chunk_a).await,
        StatusCode::ACCEPTED
    );

    // After the first chunk, event_count reflects only the 2 stored rows.
    let cnt1 = sqlx::query("select event_count from captured_sessions where session_id='chunked'")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get::<i32, _>("event_count");
    assert_eq!(cnt1, 2);

    // Second chunk: SAME session_id, DISJOINT seqs 2-3.
    let chunk_b = serde_json::json!({
        "session_id":"chunked","user_email":"dev@acme.com",
        "repo":{"host":"github.com","org":"acme-corp","name":"r","path":"C:\\w"},
        "title":"big session","cwd":"C:\\w",
        "events":[
          {"seq":2,"ts":"2026-06-10T10:00:02Z","kind":"tool_call","tool_name":"Bash","content":"a2"},
          {"seq":3,"ts":"2026-06-10T10:00:03Z","kind":"assistant_text","content":"a3"}
        ]
    })
    .to_string();
    assert_eq!(
        post_capture(&pool, &token, chunk_b).await,
        StatusCode::ACCEPTED
    );

    // event_count must be the CUMULATIVE total (4), not the last batch (2) — proves recompute.
    let cnt2 = sqlx::query("select event_count from captured_sessions where session_id='chunked'")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get::<i32, _>("event_count");
    assert_eq!(cnt2, 4, "event_count must be cumulative (4), not last-batch (2)");

    // All 4 events stored in seq order (the timeline query orders by seq the same way).
    let rows = sqlx::query(
        "select seq from captured_events where tenant_id='acme' and session_id='chunked' order by seq",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let seqs: Vec<i64> = rows.iter().map(|r| r.get::<i64, _>("seq")).collect();
    assert_eq!(seqs, vec![0, 1, 2, 3], "all 4 events present in seq order");
}

#[sqlx::test(migrations = "./migrations")]
async fn capture_scans_and_stores_findings_redacted(pool: PgPool) {
    let token = seed(&pool).await;
    // Event content carries an AWS access key (secret/high) and an email (pii/medium).
    let body = serde_json::json!({
        "session_id":"leaky","user_email":"dev@acme.com",
        "repo":{"host":"github.com","org":"acme-corp","name":"r","path":"C:\\w"},
        "title":"leak","cwd":"C:\\w",
        "events":[
          {"seq":0,"ts":"2026-06-10T10:00:00Z","kind":"user_prompt",
           "content":"key AKIAIOSFODNN7EXAMPLE and contact leak@corp.com please"}
        ]
    })
    .to_string();
    assert_eq!(
        post_capture(&pool, &token, body).await,
        StatusCode::ACCEPTED
    );

    // The AWS key finding exists, is high severity, and is NOT stored verbatim.
    let aws = sqlx::query(
        "select kind, severity, redacted from findings \
         where tenant_id='acme' and session_id='leaky' and rule='aws_access_key'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(aws.get::<String, _>("kind"), "secret");
    assert_eq!(aws.get::<String, _>("severity"), "high");
    assert_ne!(
        aws.get::<String, _>("redacted"),
        "AKIAIOSFODNN7EXAMPLE",
        "raw secret must be masked, never stored verbatim"
    );

    // The email finding exists and is medium severity.
    let email = sqlx::query(
        "select severity from findings \
         where tenant_id='acme' and session_id='leaky' and rule='email'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(email.get::<String, _>("severity"), "medium");

    // Belt-and-suspenders: the raw secret appears in NO redacted value at all.
    let raw_leaks = sqlx::query(
        "select count(*) c from findings \
         where tenant_id='acme' and redacted = 'AKIAIOSFODNN7EXAMPLE'",
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .get::<i64, _>("c");
    assert_eq!(raw_leaks, 0, "no finding may store the raw secret");
}

#[sqlx::test(migrations = "./migrations")]
async fn large_body_is_accepted_not_413(pool: PgPool) {
    let token = seed(&pool).await;
    // One event with ~3 MB of content — would exceed axum's default 2 MB body limit (413)
    // before Task 1 raised the /v1/capture limit to 64 MiB.
    let big = "x".repeat(3_000_000);
    let body = serde_json::json!({
        "session_id":"huge","user_email":"dev@acme.com",
        "repo":{"host":"github.com","org":"acme-corp","name":"r","path":"C:\\w"},
        "title":"huge session","cwd":"C:\\w",
        "events":[
          {"seq":0,"ts":"2026-06-10T10:00:00Z","kind":"assistant_text","content": big}
        ]
    })
    .to_string();
    assert!(body.len() > 2 * 1024 * 1024, "test body must exceed 2 MB");

    let status = post_capture(&pool, &token, body).await;
    assert_eq!(status, StatusCode::ACCEPTED, "expected 202, got {status}");

    // Retrievable: the row stored, content blob present, event_count == 1.
    let cnt = sqlx::query("select event_count from captured_sessions where session_id='huge'")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get::<i32, _>("event_count");
    assert_eq!(cnt, 1);
    let bytes = sqlx::query("select bytes from content_blobs where tenant_id='acme'")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get::<i32, _>("bytes");
    assert_eq!(bytes, 3_000_000);
}
