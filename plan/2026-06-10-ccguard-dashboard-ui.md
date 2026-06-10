# CCGuard Dashboard UI — Implementation Plan (Plan 6 of N)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use `- [ ]`.

**Goal:** A browser dashboard that renders the captured data: cookie login → org overview (work/personal donut + captured-session list) → **session-replay timeline** (the full prompt/response/tool-call/diff record of one session). Server-rendered Rust — no separate JS toolchain.

**Architecture:** New `ccguard-server::web` module. HTML built with **maud** (compile-time, returned as `axum::response::Html<String>` so no maud-axum-version coupling). Cookie session via **axum-extra** `CookieJar` — a `WebUser` extractor reads the `ccg_session` cookie → resolves the session token to a user (same query as `AuthedUser`), redirecting to `/login` if absent/invalid. Chart.js via CDN for the donut. Pages: `/login`, `POST /web/login`, `/dashboard`, `/dashboard/sessions/:id`. All additive; `/v1/*` API untouched.

**Tech Stack:** Rust, axum 0.7, `maud = "0.26"`, `axum-extra = { version="0.9", features=["cookie"] }`, sqlx, chrono, existing `tokens`/`passwords`. `DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres`. Commit identity `senthilguru246@gmail.com` / `SenGuru`.

---

## Roadmap position
Plans 1–5 ✅ (engine, auth, accounts, agent, complete-capture). **Plan 6 ← this (dashboard UI / session replay).** Then: Plan 7 search/eDiscovery/findings · Plan 8 managed-settings enforcement · Plan 9 on-task.

## Prerequisites
- [ ] Postgres reachable; `DATABASE_URL` set. Plans 1–5 on `master`.

---

## Task 1: web auth (cookie) + login page

**Files:** Modify `crates/ccguard-server/Cargo.toml`, `crates/ccguard-server/src/lib.rs`, `crates/ccguard-server/src/app.rs`; Create `crates/ccguard-server/src/web.rs`, `crates/ccguard-server/tests/web.rs`.

- [ ] **Step 1: Deps** — add to `crates/ccguard-server/Cargo.toml` `[dependencies]`:
```toml
maud = "0.26"
axum-extra = { version = "0.9", features = ["cookie"] }
```

- [ ] **Step 2: `web.rs`** — `crates/ccguard-server/src/web.rs`:
```rust
use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts, State};
use axum::http::request::Parts;
use axum::response::{Html, IntoResponse, Redirect};
use axum::Form;
use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::{Duration, Utc};
use maud::{html, Markup, DOCTYPE};
use serde::Deserialize;
use sqlx::{PgPool, Row};

use crate::passwords::verify_password;
use crate::tokens::{generate_token, hash_token};

/// Cookie-session-authenticated dashboard user. Redirects to /login if absent/invalid.
pub struct WebUser {
    pub user_id: i64,
    pub tenant_id: String,
    pub role: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for WebUser
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Redirect;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Redirect> {
        let jar = CookieJar::from_request_parts(parts, state).await.unwrap();
        let token = match jar.get("ccg_session") {
            Some(c) => c.value().to_string(),
            None => return Err(Redirect::to("/login")),
        };
        let pool = PgPool::from_ref(state);
        let hash = hash_token(&token);
        let row = sqlx::query(
            "select u.id as user_id, u.tenant_id, u.role from sessions s \
             join users u on u.id = s.user_id \
             where s.token_hash = $1 and (s.expires_at is null or s.expires_at > now())",
        )
        .bind(&hash)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten();
        match row {
            Some(r) => Ok(WebUser {
                user_id: r.get("user_id"),
                tenant_id: r.get("tenant_id"),
                role: r.get("role"),
            }),
            None => Err(Redirect::to("/login")),
        }
    }
}

/// Shared page chrome.
pub fn page(title: &str, body: Markup) -> Html<String> {
    Html(html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "CCGuard — " (title) }
                script src="https://cdn.jsdelivr.net/npm/chart.js" {}
                style { (maud::PreEscaped(CSS)) }
            }
            body { div.wrap { (body) } }
        }
    }.into_string())
}

const CSS: &str = "body{font:14px/1.5 -apple-system,Segoe UI,Roboto,sans-serif;margin:0;background:#0f1115;color:#e6e6e6}\
.wrap{max-width:1100px;margin:0 auto;padding:24px}a{color:#6db3ff}\
h1{font-size:20px}table{width:100%;border-collapse:collapse;margin-top:12px}\
th,td{text-align:left;padding:8px 10px;border-bottom:1px solid #232733;font-size:13px}\
.badge{padding:2px 8px;border-radius:10px;font-size:12px}.work{background:#16401f;color:#7fe29a}\
.personal{background:#402016;color:#e2a07f}.unknown{background:#2a2f3a;color:#9aa4b2}\
.ev{border-left:3px solid #2a2f3a;margin:6px 0;padding:6px 12px;border-radius:4px;background:#161922}\
.k{font-weight:600;font-size:12px;text-transform:uppercase;letter-spacing:.04em}\
.user_prompt{border-color:#6db3ff}.assistant_text{border-color:#7fe29a}.thinking{border-color:#9aa4b2;opacity:.8}\
.tool_call{border-color:#e2c07f}.tool_result{border-color:#7f9ae2}.file_edit{border-color:#e27fd0}.pr{border-color:#7fe2d0}\
pre{white-space:pre-wrap;word-break:break-word;margin:4px 0 0;font:12px/1.45 ui-monospace,Consolas,monospace;color:#c7ced9}\
.card{background:#161922;border:1px solid #232733;border-radius:8px;padding:16px;margin:12px 0}\
input,button{font:14px inherit;padding:8px 10px;border-radius:6px;border:1px solid #2a2f3a;background:#0f1115;color:#e6e6e6}\
button{background:#234; cursor:pointer}.err{color:#e27f7f}";

pub fn login_page(err: Option<&str>) -> Html<String> {
    page("Sign in", html! {
        div.card style="max-width:380px;margin:48px auto" {
            h1 { "CCGuard" }
            @if let Some(e) = err { p.err { (e) } }
            form method="post" action="/web/login" {
                p { input type="text" name="tenant_id" placeholder="Tenant ID (t_…)" style="width:100%"; }
                p { input type="email" name="email" placeholder="Email" style="width:100%"; }
                p { input type="password" name="password" placeholder="Password" style="width:100%"; }
                p { button type="submit" { "Sign in" } }
            }
        }
    })
}

pub async fn login_get() -> Html<String> { login_page(None) }

#[derive(Deserialize)]
pub struct LoginForm { pub tenant_id: String, pub email: String, pub password: String }

pub async fn login_post(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Form(f): Form<LoginForm>,
) -> axum::response::Response {
    let row = sqlx::query("select password_hash from users where tenant_id = $1 and email = $2")
        .bind(&f.tenant_id).bind(&f.email).fetch_optional(&pool).await.ok().flatten();
    if let Some(r) = row {
        let ph: String = r.get("password_hash");
        if verify_password(&f.password, &ph) {
            // find user id again for the session row
            let uid: i64 = sqlx::query("select id from users where tenant_id=$1 and email=$2")
                .bind(&f.tenant_id).bind(&f.email).fetch_one(&pool).await.unwrap().get("id");
            let (token, token_hash) = generate_token();
            let expires = Utc::now() + Duration::days(30);
            let _ = sqlx::query("insert into sessions (user_id, token_hash, expires_at) values ($1,$2,$3)")
                .bind(uid).bind(&token_hash).bind(expires).execute(&pool).await;
            let cookie = Cookie::build(("ccg_session", token)).path("/").http_only(true).build();
            return (jar.add(cookie), Redirect::to("/dashboard")).into_response();
        }
    }
    login_page(Some("Invalid credentials")).into_response()
}

pub async fn root(jar: CookieJar) -> Redirect {
    if jar.get("ccg_session").is_some() { Redirect::to("/dashboard") } else { Redirect::to("/login") }
}
```

- [ ] **Step 3: Register** — add `pub mod web;` to `crates/ccguard-server/src/lib.rs`. In `app.rs`, add routes (keep all existing):
```rust
        .route("/", get(web::root))
        .route("/login", get(web::login_get))
        .route("/web/login", post(web::login_post))
```
(Dashboard routes added in Tasks 2–3.)

- [ ] **Step 4: Test** `crates/ccguard-server/tests/web.rs`:
```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use sqlx::PgPool;
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::passwords::hash_password;

async fn seed_user(pool: &PgPool) {
    sqlx::query("insert into tenants (id,name) values ('acme','Acme')").execute(pool).await.unwrap();
    let ph = hash_password("pw12345");
    sqlx::query("insert into users (tenant_id,email,password_hash,role) values ('acme','boss@acme.com',$1,'owner')")
        .bind(&ph).execute(pool).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn login_sets_cookie_and_redirects(pool: PgPool) {
    seed_user(&pool).await;
    let resp = app(pool.clone()).oneshot(Request::builder().method("POST").uri("/web/login")
        .header("content-type","application/x-www-form-urlencoded")
        .body(Body::from("tenant_id=acme&email=boss@acme.com&password=pw12345")).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/dashboard");
    assert!(resp.headers().get("set-cookie").unwrap().to_str().unwrap().contains("ccg_session="));
}

#[sqlx::test(migrations = "./migrations")]
async fn bad_login_rerenders_form(pool: PgPool) {
    seed_user(&pool).await;
    let resp = app(pool.clone()).oneshot(Request::builder().method("POST").uri("/web/login")
        .header("content-type","application/x-www-form-urlencoded")
        .body(Body::from("tenant_id=acme&email=boss@acme.com&password=WRONG")).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK); // re-renders the form (200), no redirect
}

#[sqlx::test(migrations = "./migrations")]
async fn dashboard_without_cookie_redirects_to_login(pool: PgPool) {
    // /dashboard route is added in Task 2; if not yet present this asserts 404 — re-run after Task 2.
    let resp = app(pool.clone()).oneshot(Request::builder().uri("/dashboard").body(Body::empty()).unwrap()).await.unwrap();
    assert!(resp.status() == StatusCode::SEE_OTHER || resp.status() == StatusCode::NOT_FOUND);
}
```

- [ ] **Step 5: Run + commit.** `cargo test -p ccguard-server --test web` (first two pass; third passes fully after Task 2).
```
git add crates/ccguard-server Cargo.toml
git commit -m "feat(web): maud chrome + cookie session (WebUser) + login page/post"
```

---

## Task 2: dashboard overview (donut + session list)

**Files:** Modify `crates/ccguard-server/src/web.rs`, `crates/ccguard-server/src/app.rs`.

- [ ] **Step 1: Handler** — add to `web.rs`:
```rust
pub async fn dashboard(
    user: WebUser,
    State(pool): State<PgPool>,
) -> Html<String> {
    // session counts by classification (captured data)
    let rows = sqlx::query(
        "select classification, count(*) as c, coalesce(sum(event_count),0) as ev \
         from captured_sessions where tenant_id = $1 group by classification")
        .bind(&user.tenant_id).fetch_all(&pool).await.unwrap_or_default();
    let mut work = 0i64; let mut personal = 0i64; let mut unknown = 0i64; let mut events = 0i64;
    for r in &rows {
        let c: i64 = r.get("c"); let ev: i64 = r.get("ev"); events += ev;
        match r.get::<String,_>("classification").as_str() {
            "work" => work = c, "personal" => personal = c, _ => unknown = c,
        }
    }
    let sessions = sqlx::query(
        "select session_id, user_email, classification, repo_org, repo_name, title, event_count, last_ts \
         from captured_sessions where tenant_id = $1 order by last_ts desc nulls last limit 100")
        .bind(&user.tenant_id).fetch_all(&pool).await.unwrap_or_default();

    page("Dashboard", html! {
        h1 { "CCGuard — captured Claude Code activity" }
        div.card style="display:flex;gap:32px;align-items:center" {
            div style="width:220px" { canvas id="donut" {} }
            div {
                p { "Sessions captured: " b { (work + personal + unknown) } }
                p { span.badge.work { "work " (work) } " "
                    span.badge.personal { "personal " (personal) } " "
                    span.badge.unknown { "unknown " (unknown) } }
                p { "Total events: " b { (events) } }
            }
        }
        table {
            thead { tr { th{"Session"} th{"User"} th{"Repo"} th{"Class"} th{"Events"} } }
            tbody {
                @for s in &sessions {
                    @let sid: String = s.get("session_id");
                    @let class: String = s.get("classification");
                    @let org: Option<String> = s.get("repo_org");
                    @let name: Option<String> = s.get("repo_name");
                    @let title: Option<String> = s.get("title");
                    @let ec: i32 = s.get("event_count");
                    @let email: String = s.get("user_email");
                    tr {
                        td { a href={"/dashboard/sessions/" (sid)} { (title.unwrap_or_else(|| sid.chars().take(8).collect())) } }
                        td { (email) }
                        td { (org.unwrap_or_default()) "/" (name.unwrap_or_default()) }
                        td { span.badge.(class) { (class) } }
                        td { (ec) }
                    }
                }
            }
        }
        script { (maud::PreEscaped(format!(
            "new Chart(document.getElementById('donut'),{{type:'doughnut',\
             data:{{labels:['work','personal','unknown'],datasets:[{{data:[{work},{personal},{unknown}],\
             backgroundColor:['#16a34a','#d97706','#475569']}}]}},\
             options:{{plugins:{{legend:{{labels:{{color:'#e6e6e6'}}}}}}}}}});"))) }
    })
}
```

- [ ] **Step 2: Route** — in `app.rs` add `.route("/dashboard", get(web::dashboard))`.

- [ ] **Step 3: Run + commit.** Re-run `cargo test -p ccguard-server --test web` (all three now pass — the no-cookie `/dashboard` returns 303 redirect). `cargo build` green.
```
git add crates/ccguard-server
git commit -m "feat(web): dashboard overview — work/personal donut + captured-session list"
```

---

## Task 3: session-replay timeline page

**Files:** Modify `crates/ccguard-server/src/web.rs`, `crates/ccguard-server/src/app.rs`; add a test to `crates/ccguard-server/tests/web.rs`.

- [ ] **Step 1: Handler** — add to `web.rs`:
```rust
use axum::extract::Path;

pub async fn session_view(
    user: WebUser,
    State(pool): State<PgPool>,
    Path(session_id): Path<String>,
) -> Html<String> {
    let meta = sqlx::query(
        "select user_email, classification, title, repo_org, repo_name from captured_sessions \
         where tenant_id = $1 and session_id = $2")
        .bind(&user.tenant_id).bind(&session_id).fetch_optional(&pool).await.ok().flatten();
    let events = sqlx::query(
        "select e.seq, e.kind, e.tool_name, e.target, b.content, e.tokens_in, e.tokens_out, e.is_sidechain \
         from captured_events e left join content_blobs b on b.tenant_id=e.tenant_id and b.sha256=e.content_sha \
         where e.tenant_id = $1 and e.session_id = $2 order by e.seq")
        .bind(&user.tenant_id).bind(&session_id).fetch_all(&pool).await.unwrap_or_default();

    let (email, class, title) = match &meta {
        Some(m) => (m.get::<String,_>("user_email"), m.get::<String,_>("classification"),
                    m.get::<Option<String>,_>("title").unwrap_or_default()),
        None => (String::new(), "unknown".into(), String::new()),
    };

    page("Session", html! {
        p { a href="/dashboard" { "← dashboard" } }
        h1 { (if title.is_empty() { session_id.clone() } else { title }) }
        p { (email) " · " span.badge.(class) { (class) } " · " (events.len()) " events" }
        @for e in &events {
            @let kind: String = e.get("kind");
            @let tool: Option<String> = e.get("tool_name");
            @let target: Option<String> = e.get("target");
            @let content: Option<String> = e.get("content");
            @let side: bool = e.get("is_sidechain");
            div.ev.(kind) {
                div.k {
                    (kind)
                    @if let Some(t) = &tool { " · " (t) }
                    @if let Some(tg) = &target { " · " (tg.chars().take(80).collect::<String>()) }
                    @if side { " · subagent" }
                }
                @if let Some(c) = &content { pre { (c) } }
            }
        }
    })
}
```
(maud auto-escapes `(c)` in `pre`, so verbatim prompt/code content renders safely.)

- [ ] **Step 2: Route** — in `app.rs` add `.route("/dashboard/sessions/:session_id", get(web::session_view))`.

- [ ] **Step 3: Test** — add to `tests/web.rs` a test that: seeds tenant+user+an ingest token, POSTs a `CapturedSession` to `/v1/capture` (with a `user_prompt` event whose content = "hello world from the test"), logs in via `/web/login` capturing the `set-cookie`, then GETs `/dashboard/sessions/<id>` **with that cookie** and asserts 200 + the body contains "hello world from the test" and the session id. (Reuse helpers; extract the cookie value from the login response's `set-cookie` header.)

- [ ] **Step 4: Full suite + commit.** `cargo test` (whole workspace, DATABASE_URL set) — all pass (42 + new web tests).
```
git add crates/ccguard-server
git commit -m "feat(web): session-replay timeline — full prompt/response/tool/diff record"
```

---

## Self-Review (done while writing this plan)
**Coverage:** cookie login ✅(T1) · overview+donut+session list ✅(T2) · session-replay timeline ✅(T3). Search/findings/export → Plan 7. **Types:** `WebUser` (cookie→user) parallels `AuthedUser`; reuses `passwords::verify_password`, `tokens::generate_token/hash_token`, the `sessions` table. Pages return `Html<String>` (maud `.into_string()`) so no maud-axum version coupling. Reads `captured_sessions`/`captured_events`/`content_blobs` from Plan 5. **Sharp edges:** maud `@let`/`@for`/`@if let` syntax — keep bindings inside the `html!` macro; `span.badge.(class)` interpolates the class name as a CSS class; `Cookie::build((k,v))` is the axum-extra 0.9 builder shape; `Form` needs the urlencoded content-type in tests; the login redirect is **303 SEE_OTHER** (axum `Redirect::to` default). Donut uses Chart.js from CDN (needs internet to render in a browser, but tests only assert server HTML).

## Execution handoff
Build **subagent-driven** against live Postgres; controller runs the server and opens `/login` → `/dashboard` → a session view in a browser to confirm the replay renders.
