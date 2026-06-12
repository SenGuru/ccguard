//! Integration tests for on-task scoring, role anomalies, per-repo overrides,
//! and the indicator review-queue APIs.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::{PgPool, Row};
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::passwords::hash_password;
use ccguard_server::tokens::generate_token;

/// Seed tenant 'acme' + allowlist (github.com / acme-corp) + ingest token + an
/// owner user. Returns (ingest_token,).
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
    // Provenance policy: the personal denylist + signed personal email domain.
    // A session is only PERSONAL with an affirmative personal signal confirmed by
    // a second independent one (per the provenance cascade) — never merely because
    // its remote isn't allowlisted (that is now UNCLASSIFIED, not personal).
    sqlx::query(
        "insert into provenance_policy (tenant_id, personal_orgs, personal_email_domains) \
         values ('acme', 'my-side-project, random-org', 'gmail.com')",
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
    ingest_token
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

/// AI-primary: structural-personal now lands at 'pending'; the AI verdict is what
/// classifies it personal (and re-runs scoring/indicators). Simulate that verdict.
async fn post_verdict(pool: &PgPool, ingest: &str, session_id: &str, label: &str) -> StatusCode {
    let body = serde_json::json!({
        "session_id": session_id, "label": label, "confidence": 0.9, "reason": "test"
    })
    .to_string();
    post_json(pool, "/v1/triage/verdict", ingest, body).await
}

async fn post_json(pool: &PgPool, uri: &str, token: &str, body: String) -> StatusCode {
    app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

async fn get_json(pool: &PgPool, uri: &str, token: &str) -> (StatusCode, serde_json::Value) {
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, v)
}

async fn score_row(pool: &PgPool, session_id: &str) -> (i32, String) {
    let r = sqlx::query(
        "select score, label from session_scores where tenant_id='acme' and session_id=$1",
    )
    .bind(session_id)
    .fetch_one(pool)
    .await
    .unwrap();
    (r.get::<i32, _>("score"), r.get::<String, _>("label"))
}

async fn indicator_count(pool: &PgPool, session_id: &str, kind: &str) -> i64 {
    sqlx::query(
        "select count(*) c from indicators \
         where tenant_id='acme' and session_id=$1 and kind=$2",
    )
    .bind(session_id)
    .bind(kind)
    .fetch_one(pool)
    .await
    .unwrap()
    .get::<i64, _>("c")
}

// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn work_with_commit_and_ticket_scores_on_task(pool: PgPool) {
    let ingest = seed(&pool).await;
    // A real push to the allowlisted corp org → W-PUSH (Tier-G) → free 'work'
    // shortcut. References PROJ-7, a `git commit`, and an assistant_text.
    let body = serde_json::json!({
        "session_id":"s_work","user_email":"dev@acme.com",
        "repo":{"host":"github.com","org":"acme-corp","name":"billing","path":"C:\\w"},
        "title":"ship it","cwd":"C:\\w",
        "signals":{"pushed":true},
        "events":[
          {"seq":0,"ts":"2026-06-10T10:00:00Z","kind":"user_prompt","content":"work on PROJ-7 please"},
          {"seq":1,"ts":"2026-06-10T10:00:01Z","kind":"tool_call","tool_name":"Bash","target":"git commit -m x","content":"{\"command\":\"git commit -m x\"}"},
          {"seq":2,"ts":"2026-06-10T10:00:02Z","kind":"assistant_text","content":"done"}
        ]
    })
    .to_string();
    assert_eq!(post_capture(&pool, &ingest, body).await, StatusCode::ACCEPTED);

    let (score, label) = score_row(&pool, "s_work").await;
    assert_eq!(label, "on_task", "score was {score}");
    assert!(score >= 70, "expected high score, got {score}");

    let ticket = sqlx::query(
        "select count(*) c from session_tickets \
         where tenant_id='acme' and session_id='s_work' and ticket='PROJ-7'",
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .get::<i64, _>("c");
    assert_eq!(ticket, 1, "PROJ-7 must be recorded in session_tickets");
}

#[sqlx::test(migrations = "./migrations")]
async fn personal_no_commit_scores_off_task_and_raises_indicators(pool: PgPool) {
    let ingest = seed(&pool).await;
    // Confirmed personal: a personal-denylist org remote (P-REMOTE) AND a signed
    // commit by a personal email (P-EMAIL-SIGNED) — two independent signals. No commit work.
    let body = serde_json::json!({
        "session_id":"s_personal","user_email":"dev@acme.com",
        "repo":{"host":"github.com","org":"my-side-project","name":"toy","path":"C:\\side"},
        "title":"hobby","cwd":"C:\\side",
        "signals":{"committer_email":"me@gmail.com","commit_signed":true},
        "events":[
          {"seq":0,"ts":"2026-06-10T10:00:00Z","kind":"user_prompt","content":"build my game"},
          {"seq":1,"ts":"2026-06-10T10:00:01Z","kind":"assistant_text","content":"ok"}
        ]
    })
    .to_string();
    assert_eq!(post_capture(&pool, &ingest, body).await, StatusCode::ACCEPTED);
    // AI-primary: capture leaves it 'pending'; the AI verdict classifies it personal
    // and re-runs scoring → off_task + personal_repo indicators.
    assert_eq!(post_verdict(&pool, &ingest, "s_personal", "personal").await, StatusCode::OK);

    let (_score, label) = score_row(&pool, "s_personal").await;
    assert_eq!(label, "off_task");
    assert_eq!(indicator_count(&pool, "s_personal", "personal_repo").await, 1);
    assert_eq!(indicator_count(&pool, "s_personal", "off_task").await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn repo_override_beats_allowlist(pool: PgPool) {
    let ingest = seed(&pool).await;
    let session = login(&pool, "acme", "boss@acme.com", "pw12345").await;

    // Define an override marking an un-allowlisted repo as work.
    let ov = serde_json::json!({
        "repo_host":"github.com","repo_org":"contractor-co","repo_name":"internal-fork",
        "classification":"work","note":"internal fork"
    })
    .to_string();
    assert_eq!(
        post_json(&pool, "/v1/orgs/acme/repo-overrides", &session, ov).await,
        StatusCode::OK
    );

    // Capture a session on that repo (org not allowlisted -> would be personal).
    let body = serde_json::json!({
        "session_id":"s_override","user_email":"dev@acme.com",
        "repo":{"host":"github.com","org":"contractor-co","name":"internal-fork","path":"C:\\f"},
        "title":"fork work","cwd":"C:\\f",
        "events":[
          {"seq":0,"ts":"2026-06-10T10:00:00Z","kind":"user_prompt","content":"do the fork"},
          {"seq":1,"ts":"2026-06-10T10:00:01Z","kind":"assistant_text","content":"done"}
        ]
    })
    .to_string();
    assert_eq!(post_capture(&pool, &ingest, body).await, StatusCode::ACCEPTED);

    let stored = sqlx::query(
        "select classification from captured_sessions \
         where tenant_id='acme' and session_id='s_override'",
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .get::<String, _>("classification");
    assert_eq!(stored, "work", "override must beat the allowlist");
}

#[sqlx::test(migrations = "./migrations")]
async fn role_anomaly_raises_non_engineer_coding(pool: PgPool) {
    let ingest = seed(&pool).await;
    let session = login(&pool, "acme", "boss@acme.com", "pw12345").await;

    // Assign sam the marketer role.
    let role = serde_json::json!({
        "user_email":"sam@acme","job_role":"marketer","note":"growth"
    })
    .to_string();
    assert_eq!(
        post_json(&pool, "/v1/orgs/acme/roles", &session, role).await,
        StatusCode::OK
    );

    // Sam produces >=5 code events (file_edit / Edit / Write).
    let body = serde_json::json!({
        "session_id":"s_role","user_email":"sam@acme",
        "repo":{"host":"github.com","org":"acme-corp","name":"site","path":"C:\\w"},
        "title":"coding","cwd":"C:\\w",
        "events":[
          {"seq":0,"ts":"2026-06-10T10:00:00Z","kind":"file_edit","target":"a.rs","content":"x"},
          {"seq":1,"ts":"2026-06-10T10:00:01Z","kind":"tool_call","tool_name":"Edit","target":"b.rs","content":"y"},
          {"seq":2,"ts":"2026-06-10T10:00:02Z","kind":"tool_call","tool_name":"Write","target":"c.rs","content":"z"},
          {"seq":3,"ts":"2026-06-10T10:00:03Z","kind":"file_edit","target":"d.rs","content":"w"},
          {"seq":4,"ts":"2026-06-10T10:00:04Z","kind":"tool_call","tool_name":"Edit","target":"e.rs","content":"v"},
          {"seq":5,"ts":"2026-06-10T10:00:05Z","kind":"assistant_text","content":"done"}
        ]
    })
    .to_string();
    assert_eq!(post_capture(&pool, &ingest, body).await, StatusCode::ACCEPTED);

    assert_eq!(
        indicator_count(&pool, "s_role", "non_engineer_coding").await,
        1,
        "a marketer producing heavy code must raise non_engineer_coding"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn indicator_status_flow(pool: PgPool) {
    let ingest = seed(&pool).await;
    let session = login(&pool, "acme", "boss@acme.com", "pw12345").await;

    // Raise indicators via a confirmed-personal capture (P-REMOTE + P-EMAIL-SIGNED).
    let body = serde_json::json!({
        "session_id":"s_flow","user_email":"dev@acme.com",
        "repo":{"host":"github.com","org":"random-org","name":"toy","path":"C:\\side"},
        "title":"hobby","cwd":"C:\\side",
        "signals":{"committer_email":"me@gmail.com","commit_signed":true},
        "events":[
          {"seq":0,"ts":"2026-06-10T10:00:00Z","kind":"user_prompt","content":"hi"},
          {"seq":1,"ts":"2026-06-10T10:00:01Z","kind":"assistant_text","content":"ok"}
        ]
    })
    .to_string();
    assert_eq!(post_capture(&pool, &ingest, body).await, StatusCode::ACCEPTED);
    assert_eq!(post_verdict(&pool, &ingest, "s_flow", "personal").await, StatusCode::OK);

    // GET open indicators -> includes our personal_repo / off_task indicators.
    let (status, v) = get_json(&pool, "/v1/orgs/acme/indicators?status=open", &session).await;
    assert_eq!(status, StatusCode::OK);
    let rows = v.as_array().unwrap();
    assert!(rows.iter().any(|r| r["session_id"] == "s_flow"));
    // Pick a specific indicator id to flip.
    let target = rows
        .iter()
        .find(|r| r["session_id"] == "s_flow")
        .expect("indicator for s_flow");
    let id = target["id"].as_i64().unwrap();

    // Flip it to reviewed.
    let st = serde_json::json!({"status":"reviewed"}).to_string();
    assert_eq!(
        post_json(&pool, &format!("/v1/indicators/{id}/status"), &session, st).await,
        StatusCode::OK
    );

    // Re-GET open -> that id is gone.
    let (status, v) = get_json(&pool, "/v1/orgs/acme/indicators?status=open", &session).await;
    assert_eq!(status, StatusCode::OK);
    let rows = v.as_array().unwrap();
    assert!(
        !rows.iter().any(|r| r["id"].as_i64() == Some(id)),
        "reviewed indicator must leave the open queue"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn indicators_are_idempotent_on_recapture(pool: PgPool) {
    let ingest = seed(&pool).await;
    let body = serde_json::json!({
        "session_id":"s_idem","user_email":"dev@acme.com",
        "repo":{"host":"github.com","org":"random-org","name":"toy","path":"C:\\side"},
        "title":"hobby","cwd":"C:\\side",
        "signals":{"committer_email":"me@gmail.com","commit_signed":true},
        "events":[
          {"seq":0,"ts":"2026-06-10T10:00:00Z","kind":"user_prompt","content":"hi"},
          {"seq":1,"ts":"2026-06-10T10:00:01Z","kind":"assistant_text","content":"ok"}
        ]
    })
    .to_string();
    // Post the SAME session twice.
    assert_eq!(
        post_capture(&pool, &ingest, body.clone()).await,
        StatusCode::ACCEPTED
    );
    assert_eq!(post_capture(&pool, &ingest, body).await, StatusCode::ACCEPTED);
    // The AI verdict (personal) raises the indicators; posting it again must stay idempotent.
    assert_eq!(post_verdict(&pool, &ingest, "s_idem", "personal").await, StatusCode::OK);
    assert_eq!(post_verdict(&pool, &ingest, "s_idem", "personal").await, StatusCode::OK);

    // Each auto-indicator kind appears exactly once.
    assert_eq!(indicator_count(&pool, "s_idem", "personal_repo").await, 1);
    assert_eq!(indicator_count(&pool, "s_idem", "off_task").await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn rollup_aggregates_per_employee(pool: PgPool) {
    let ingest = seed(&pool).await;
    let session = login(&pool, "acme", "boss@acme.com", "pw12345").await;

    let body = serde_json::json!({
        "session_id":"s_roll","user_email":"dev@acme.com",
        "repo":{"host":"github.com","org":"acme-corp","name":"billing","path":"C:\\w"},
        "title":"work","cwd":"C:\\w",
        "signals":{"pushed":true},
        "events":[
          {"seq":0,"ts":"2026-06-10T10:00:00Z","kind":"user_prompt","content":"PROJ-1"},
          {"seq":1,"ts":"2026-06-10T10:00:01Z","kind":"tool_call","tool_name":"Bash","target":"git commit -m x","content":"git commit -m x"},
          {"seq":2,"ts":"2026-06-10T10:00:02Z","kind":"assistant_text","content":"done"}
        ]
    })
    .to_string();
    assert_eq!(post_capture(&pool, &ingest, body).await, StatusCode::ACCEPTED);

    let (status, v) = get_json(&pool, "/v1/orgs/acme/ontask", &session).await;
    assert_eq!(status, StatusCode::OK);
    let rows = v.as_array().unwrap();
    let dev = rows
        .iter()
        .find(|r| r["user_email"] == "dev@acme.com")
        .expect("dev rollup row");
    assert_eq!(dev["total"].as_i64().unwrap(), 1);
    assert_eq!(dev["on_task"].as_i64().unwrap(), 1);
    assert!(dev["avg_score"].as_i64().unwrap() >= 70);
}
