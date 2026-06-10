use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::passwords::hash_password;
use ccguard_server::tokens::generate_token;

async fn seed_user(pool: &PgPool) {
    sqlx::query("insert into tenants (id,name) values ('acme','Acme')")
        .execute(pool)
        .await
        .unwrap();
    let ph = hash_password("pw12345");
    sqlx::query("insert into users (tenant_id,email,password_hash,role) values ('acme','boss@acme.com',$1,'owner')")
        .bind(&ph)
        .execute(pool)
        .await
        .unwrap();
}

/// Allowlist 'acme-corp' as a work org + mint an ingest token for /v1/capture.
async fn seed_ingest(pool: &PgPool) -> String {
    sqlx::query(
        "insert into allowlist_rules (tenant_id,kind,value) values \
         ('acme','host','github.com'),('acme','org','acme-corp')",
    )
    .execute(pool)
    .await
    .unwrap();
    let (token, hash) = generate_token();
    sqlx::query("insert into api_tokens (tenant_id, token_hash) values ('acme',$1)")
        .bind(&hash)
        .execute(pool)
        .await
        .unwrap();
    token
}

/// Log in via the web form and return the `ccg_session` cookie value.
async fn login_cookie(pool: &PgPool) -> String {
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/web/login")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "tenant_id=acme&email=boss@acme.com&password=pw12345",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    let set_cookie = resp
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    // "ccg_session=<token>; Path=/; HttpOnly" -> extract the token
    set_cookie
        .split(';')
        .next()
        .unwrap()
        .trim_start_matches("ccg_session=")
        .to_string()
}

#[sqlx::test(migrations = "./migrations")]
async fn login_sets_cookie_and_redirects(pool: PgPool) {
    seed_user(&pool).await;
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/web/login")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "tenant_id=acme&email=boss@acme.com&password=pw12345",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/dashboard");
    assert!(resp
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("ccg_session="));
}

#[sqlx::test(migrations = "./migrations")]
async fn bad_login_rerenders_form(pool: PgPool) {
    seed_user(&pool).await;
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/web/login")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "tenant_id=acme&email=boss@acme.com&password=WRONG",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn dashboard_without_cookie_redirects_to_login(pool: PgPool) {
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .uri("/dashboard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/login");
}

/// End-to-end through the real /v1/capture ingest path + cookie auth:
/// capture a session with a sentinel user_prompt, log in via the web form,
/// then GET the session-replay page with the cookie and assert the verbatim
/// content renders.
#[sqlx::test(migrations = "./migrations")]
async fn session_view_renders_verbatim_content_for_logged_in_user(pool: PgPool) {
    seed_user(&pool).await;
    let ingest = seed_ingest(&pool).await;

    // POST a captured session whose first event is the sentinel prompt.
    let body = serde_json::json!({
        "session_id": "sess-xyz",
        "user_email": "dev@acme.com",
        "repo": {"host": "github.com", "org": "acme-corp", "name": "billing", "path": "C:\\w"},
        "title": "Replay me",
        "cwd": "C:\\w",
        "events": [
            {"seq": 0, "ts": "2026-06-10T10:00:00Z", "kind": "user_prompt", "content": "hello world from the test"},
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
                .header("authorization", format!("Bearer {ingest}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    // Log in via the web form, grab the ccg_session cookie token.
    let token = login_cookie(&pool).await;

    // GET the session-replay page WITH the cookie.
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/sessions/sess-xyz")
                .header("cookie", format!("ccg_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    // verbatim prompt content rendered
    assert!(
        html.contains("hello world from the test"),
        "sentinel prompt content missing from page"
    );
    // session id rendered
    assert!(html.contains("sess-xyz"), "session id missing from page");
    // tool call detail rendered
    assert!(html.contains("Bash"), "tool name missing from page");
}

/// Capture a session containing a secret via /v1/capture, log in, then GET
/// /dashboard/findings with the cookie and assert the findings table surfaces
/// the detected rule.
#[sqlx::test(migrations = "./migrations")]
async fn findings_page_lists_detected_secret_for_logged_in_user(pool: PgPool) {
    seed_user(&pool).await;
    let ingest = seed_ingest(&pool).await;

    let body = serde_json::json!({
        "session_id": "sess-leak",
        "user_email": "dev@acme.com",
        "repo": {"host": "github.com", "org": "acme-corp", "name": "billing", "path": "C:\\w"},
        "title": "Leaky",
        "cwd": "C:\\w",
        "events": [
            {"seq": 0, "ts": "2026-06-10T10:00:00Z", "kind": "user_prompt",
             "content": "deploy with AKIAIOSFODNN7EXAMPLE now"}
        ]
    })
    .to_string();
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/capture")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {ingest}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let token = login_cookie(&pool).await;

    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/findings")
                .header("cookie", format!("ccg_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(
        html.contains("aws_access_key"),
        "findings page must list the detected aws_access_key rule"
    );
    // The raw secret must never render on the findings page.
    assert!(
        !html.contains("AKIAIOSFODNN7EXAMPLE"),
        "raw secret must not appear on the findings page"
    );
}

/// Capture a session whose event content contains a distinctive marker token,
/// then full-text-search for it on the dashboard search page and assert the hit
/// (session link / snippet) renders. Also assert a no-match query says
/// "no results", an absent `q` just shows the form, and no cookie -> /login.
///
/// Marker is plain alphabetic (`zphybvqxmarkerterm`) so it survives english FTS
/// lexing as a single lexeme; `websearch_to_tsquery` matches it exactly.
#[sqlx::test(migrations = "./migrations")]
async fn dashboard_search_finds_captured_content(pool: PgPool) {
    seed_user(&pool).await;
    let ingest = seed_ingest(&pool).await;

    let body = serde_json::json!({
        "session_id": "sess-search",
        "user_email": "dev@acme.com",
        "repo": {"host": "github.com", "org": "acme-corp", "name": "billing", "path": "C:\\w"},
        "title": "Searchable",
        "cwd": "C:\\w",
        "events": [
            {"seq": 0, "ts": "2026-06-10T10:00:00Z", "kind": "user_prompt",
             "content": "please refactor the zphybvqxmarkerterm helper in billing"}
        ]
    })
    .to_string();
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/capture")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {ingest}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let token = login_cookie(&pool).await;

    // Matching query: 200 + the session link to the captured session.
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/search?q=zphybvqxmarkerterm")
                .header("cookie", format!("ccg_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(
        html.contains("/dashboard/sessions/sess-search"),
        "search results must link to the matching session"
    );
    assert!(
        html.contains("zphybvqxmarkerterm"),
        "search results must surface the marker in the snippet"
    );

    // No-match query: 200 + "no results".
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/search?q=nonexistentxyzzy")
                .header("cookie", format!("ccg_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(
        html.contains("no results"),
        "a non-matching query must say 'no results'"
    );

    // No query at all: 200, just the form.
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/search")
                .header("cookie", format!("ccg_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

/// The search page is gated by the WebUser cookie — no cookie -> /login.
#[sqlx::test(migrations = "./migrations")]
async fn search_page_without_cookie_redirects_to_login(pool: PgPool) {
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .uri("/dashboard/search")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/login");
}

/// The findings page is gated by the WebUser cookie — no cookie -> /login.
#[sqlx::test(migrations = "./migrations")]
async fn findings_page_without_cookie_redirects_to_login(pool: PgPool) {
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .uri("/dashboard/findings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/login");
}

/// eDiscovery NDJSON export: capture a session whose first event carries a
/// distinctive marker, log in, then GET the per-session export with the cookie.
/// Assert 200, the attachment `.ndjson` content-disposition, a session record
/// on line 1, and the marker payload on a later (event) line.
#[sqlx::test(migrations = "./migrations")]
async fn export_returns_ndjson_attachment_with_session_and_events(pool: PgPool) {
    seed_user(&pool).await;
    let ingest = seed_ingest(&pool).await;

    let body = serde_json::json!({
        "session_id": "sess-export",
        "user_email": "dev@acme.com",
        "repo": {"host": "github.com", "org": "acme-corp", "name": "billing", "path": "C:\\w"},
        "title": "Export me",
        "cwd": "C:\\w",
        "events": [
            {"seq": 0, "ts": "2026-06-10T10:00:00Z", "kind": "user_prompt",
             "content": "export_marker_payload"}
        ]
    })
    .to_string();
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/capture")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {ingest}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let token = login_cookie(&pool).await;

    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/sessions/sess-export/export")
                .header("cookie", format!("ccg_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let cd = resp
        .headers()
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(cd.contains("attachment"), "expected attachment disposition");
    assert!(cd.contains(".ndjson"), "expected .ndjson filename");

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let ndjson = String::from_utf8(bytes.to_vec()).unwrap();
    let mut lines = ndjson.lines();
    let first = lines.next().unwrap();
    assert!(
        first.contains("\"type\":\"session\""),
        "first line must be the session record, got: {first}"
    );
    assert!(
        ndjson.contains("export_marker_payload"),
        "event content marker must appear on a later line"
    );
}

/// Legal hold toggle: POST the hold endpoint -> 303, on_hold flips to true and
/// the session view surfaces the hold marker; POST again -> on_hold flips back
/// to false.
#[sqlx::test(migrations = "./migrations")]
async fn hold_toggles_on_hold_and_surfaces_on_session_view(pool: PgPool) {
    seed_user(&pool).await;
    let ingest = seed_ingest(&pool).await;

    let body = serde_json::json!({
        "session_id": "sess-hold",
        "user_email": "dev@acme.com",
        "repo": {"host": "github.com", "org": "acme-corp", "name": "billing", "path": "C:\\w"},
        "title": "Hold me",
        "cwd": "C:\\w",
        "events": [
            {"seq": 0, "ts": "2026-06-10T10:00:00Z", "kind": "user_prompt", "content": "hi"}
        ]
    })
    .to_string();
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/capture")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {ingest}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let token = login_cookie(&pool).await;

    // Place the hold.
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/dashboard/sessions/sess-hold/hold")
                .header("cookie", format!("ccg_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);

    let on_hold: bool =
        sqlx::query_scalar("select on_hold from captured_sessions where session_id = 'sess-hold'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(on_hold, "on_hold must be true after first toggle");

    // Session view should now surface the hold marker.
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/sessions/sess-hold")
                .header("cookie", format!("ccg_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(
        html.contains("ON LEGAL HOLD") || html.contains("🔒"),
        "session view must show the legal hold marker"
    );

    // Toggle again -> back to false.
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/dashboard/sessions/sess-hold/hold")
                .header("cookie", format!("ccg_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);

    let on_hold: bool =
        sqlx::query_scalar("select on_hold from captured_sessions where session_id = 'sess-hold'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!on_hold, "on_hold must be false after second toggle");
}

/// The export endpoint is gated by the WebUser cookie — no cookie -> /login.
#[sqlx::test(migrations = "./migrations")]
async fn export_without_cookie_redirects_to_login(pool: PgPool) {
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .uri("/dashboard/sessions/sess-xyz/export")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/login");
}

/// The session-replay page is gated by the WebUser cookie — no cookie -> /login.
#[sqlx::test(migrations = "./migrations")]
async fn session_view_without_cookie_redirects_to_login(pool: PgPool) {
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .uri("/dashboard/sessions/sess-xyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/login");
}
