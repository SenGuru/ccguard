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
