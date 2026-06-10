use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts, Path, State};
use axum::http::request::Parts;
use axum::response::{Html, IntoResponse, Redirect};
use axum::Form;
use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::{Duration, Utc};
use maud::{html, Markup, DOCTYPE};
use std::collections::HashMap;
use serde::Deserialize;
use sqlx::{PgPool, Row};

use crate::passwords::verify_password;
use crate::tokens::{generate_token, hash_token};

/// Cookie-session counterpart to `AuthedUser`. Reads the `ccg_session` cookie,
/// resolves it to the logged-in user, or redirects to `/login`.
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
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .unwrap_or_default();
        let token = match jar.get("ccg_session") {
            Some(c) => c.value().to_string(),
            None => return Err(Redirect::to("/login")),
        };
        let pool = PgPool::from_ref(state);
        let hash = hash_token(&token);
        let row = sqlx::query(
            "select u.id as user_id, u.tenant_id, u.role \
             from sessions s join users u on u.id = s.user_id \
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

/// Shared page chrome: dark theme + Chart.js CDN. Wraps `body` in the standard
/// document shell and returns ready-to-serve HTML.
pub fn page(title: &str, body: Markup) -> Html<String> {
    Html(
        html! {
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
        }
        .into_string(),
    )
}

const CSS: &str = "body{font:14px/1.5 -apple-system,Segoe UI,Roboto,sans-serif;margin:0;background:#0f1115;color:#e6e6e6}\
.wrap{max-width:1100px;margin:0 auto;padding:24px}a{color:#6db3ff}h1{font-size:20px}\
table{width:100%;border-collapse:collapse;margin-top:12px}th,td{text-align:left;padding:8px 10px;border-bottom:1px solid #232733;font-size:13px}\
.badge{padding:2px 8px;border-radius:10px;font-size:12px}.work{background:#16401f;color:#7fe29a}.personal{background:#402016;color:#e2a07f}.unknown{background:#2a2f3a;color:#9aa4b2}\
.high{background:#4a1616;color:#e29a9a}.medium{background:#4a3a16;color:#e2c89a}.low{background:#2a2f3a;color:#9aa4b2}\
.finding{font-size:12px;margin:4px 0 0;color:#e2c89a}\
.ev{border-left:3px solid #2a2f3a;margin:6px 0;padding:6px 12px;border-radius:4px;background:#161922}\
.k{font-weight:600;font-size:12px;text-transform:uppercase;letter-spacing:.04em}\
.user_prompt{border-color:#6db3ff}.assistant_text{border-color:#7fe29a}.thinking{border-color:#9aa4b2;opacity:.8}.tool_call{border-color:#e2c07f}.tool_result{border-color:#7f9ae2}.file_edit{border-color:#e27fd0}.pr{border-color:#7fe2d0}\
pre{white-space:pre-wrap;word-break:break-word;margin:4px 0 0;font:12px/1.45 ui-monospace,Consolas,monospace;color:#c7ced9}\
.card{background:#161922;border:1px solid #232733;border-radius:8px;padding:16px;margin:12px 0}\
input,button{font:14px inherit;padding:8px 10px;border-radius:6px;border:1px solid #2a2f3a;background:#0f1115;color:#e6e6e6}button{background:#234;cursor:pointer}.err{color:#e27f7f}";

/// Sign-in form. `err` renders an inline error message when present.
pub fn login_page(err: Option<&str>) -> Html<String> {
    page(
        "Sign in",
        html! {
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
        },
    )
}

pub async fn login_get() -> Html<String> {
    login_page(None)
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub tenant_id: String,
    pub email: String,
    pub password: String,
}

/// Verifies credentials, creates a 30-day session, sets the `ccg_session`
/// HttpOnly cookie, and 303-redirects to `/dashboard`. Re-renders the form on
/// failure.
pub async fn login_post(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Form(f): Form<LoginForm>,
) -> axum::response::Response {
    let row = sqlx::query("select id, password_hash from users where tenant_id = $1 and email = $2")
        .bind(&f.tenant_id)
        .bind(&f.email)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten();
    if let Some(r) = row {
        let ph: String = r.get("password_hash");
        if verify_password(&f.password, &ph) {
            let uid: i64 = r.get("id");
            let (token, token_hash) = generate_token();
            let expires = Utc::now() + Duration::days(30);
            let _ = sqlx::query(
                "insert into sessions (user_id, token_hash, expires_at) values ($1,$2,$3)",
            )
            .bind(uid)
            .bind(&token_hash)
            .bind(expires)
            .execute(&pool)
            .await;
            let cookie = Cookie::build(("ccg_session", token))
                .path("/")
                .http_only(true)
                .build();
            return (jar.add(cookie), Redirect::to("/dashboard")).into_response();
        }
    }
    login_page(Some("Invalid credentials")).into_response()
}

/// Root: send signed-in browsers to the dashboard, everyone else to login.
pub async fn root(jar: CookieJar) -> Redirect {
    if jar.get("ccg_session").is_some() {
        Redirect::to("/dashboard")
    } else {
        Redirect::to("/login")
    }
}

/// Dashboard overview: work/personal/unknown donut + counts, total sessions,
/// total events, and a table of captured sessions linking to per-session detail.
pub async fn dashboard(user: WebUser, State(pool): State<PgPool>) -> Html<String> {
    let rows = sqlx::query(
        "select classification, count(*) as c, coalesce(sum(event_count),0)::bigint as ev \
         from captured_sessions where tenant_id = $1 group by classification",
    )
    .bind(&user.tenant_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();
    let mut work = 0i64;
    let mut personal = 0i64;
    let mut unknown = 0i64;
    let mut events = 0i64;
    for r in &rows {
        let c: i64 = r.get("c");
        let ev: i64 = r.get("ev");
        events += ev;
        match r.get::<String, _>("classification").as_str() {
            "work" => work = c,
            "personal" => personal = c,
            _ => unknown = c,
        }
    }
    let sessions = sqlx::query(
        "select session_id, user_email, classification, repo_org, repo_name, title, event_count, last_ts \
         from captured_sessions where tenant_id = $1 order by last_ts desc nulls last limit 100",
    )
    .bind(&user.tenant_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    // Findings counts by severity for the KPI line.
    let frows = sqlx::query(
        "select severity, count(*) as c from findings where tenant_id = $1 group by severity",
    )
    .bind(&user.tenant_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();
    let mut f_high = 0i64;
    let mut f_medium = 0i64;
    let mut f_low = 0i64;
    for r in &frows {
        let c: i64 = r.get("c");
        match r.get::<String, _>("severity").as_str() {
            "high" => f_high = c,
            "medium" => f_medium = c,
            _ => f_low = c,
        }
    }

    page(
        "Dashboard",
        html! {
            h1 { "CCGuard — captured Claude Code activity" }
            div.card style="display:flex;gap:32px;align-items:center" {
                div style="width:220px" { canvas id="donut" {} }
                div {
                    p { "Sessions captured: " b { (work + personal + unknown) } }
                    p {
                        span.badge.work { "work " (work) } " "
                        span.badge.personal { "personal " (personal) } " "
                        span.badge.unknown { "unknown " (unknown) }
                    }
                    p { "Total events: " b { (events) } }
                    p {
                        a href="/dashboard/findings" { "Findings: " }
                        span.badge.high { "high " (f_high) } " "
                        span.badge.medium { "medium " (f_medium) } " "
                        span.badge.low { "low " (f_low) }
                    }
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
                            td { a href={"/dashboard/sessions/" (sid)} { (title.clone().unwrap_or_else(|| sid.chars().take(8).collect())) } }
                            td { (email) }
                            td { (org.clone().unwrap_or_default()) "/" (name.clone().unwrap_or_default()) }
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
        },
    )
}

/// Findings list: every secret / PII detection for the tenant, newest first
/// (limit 200). Only a redacted preview is ever stored or shown. Each row links
/// back to its originating session replay. Cookie-authed via `WebUser`.
pub async fn findings(user: WebUser, State(pool): State<PgPool>) -> Html<String> {
    let rows = sqlx::query(
        "select session_id, seq, kind, rule, severity, redacted, created_at \
         from findings where tenant_id = $1 order by created_at desc, id desc limit 200",
    )
    .bind(&user.tenant_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    page(
        "Findings",
        html! {
            p { a href="/dashboard" { "← dashboard" } }
            h1 { "Findings — secrets & PII" }
            p { (rows.len()) " finding(s) (most recent 200)" }
            table {
                thead { tr { th{"Severity"} th{"Rule"} th{"Kind"} th{"Redacted"} th{"Session"} } }
                tbody {
                    @for r in &rows {
                        @let sid: String = r.get("session_id");
                        @let rule: String = r.get("rule");
                        @let kind: String = r.get("kind");
                        @let sev: String = r.get("severity");
                        @let red: String = r.get("redacted");
                        tr {
                            td { span.badge.(sev) { (sev) } }
                            td { (rule) }
                            td { (kind) }
                            td { code { (red) } }
                            td { a href={"/dashboard/sessions/" (sid)} { (sid.chars().take(8).collect::<String>()) } }
                        }
                    }
                }
            }
        },
    )
}

/// Session-replay timeline: the full ordered event record for one captured
/// session — every prompt, assistant response, thinking block, tool call
/// (+ args/target), tool result, file edit and PR — each color-coded by kind
/// with its verbatim content. Cookie-authed via `WebUser` and tenant-scoped.
///
/// SQL mirrors `handlers::timeline::timeline` verbatim so the web view and the
/// JSON API view agree. maud auto-escapes `(c)`/`(kind)` so verbatim prompt and
/// code content (incl. `<`, `>`, `&`) renders safely as text — no injection.
pub async fn session_view(
    user: WebUser,
    State(pool): State<PgPool>,
    Path(session_id): Path<String>,
) -> Html<String> {
    let meta = sqlx::query(
        "select user_email, classification, title, repo_org, repo_name \
         from captured_sessions where tenant_id = $1 and session_id = $2",
    )
    .bind(&user.tenant_id)
    .bind(&session_id)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();
    let events = sqlx::query(
        "select e.seq, e.kind, e.model, e.tool_name, e.target, b.content, \
                e.tokens_in, e.tokens_out, e.is_sidechain \
         from captured_events e \
         left join content_blobs b \
           on b.tenant_id = e.tenant_id and b.sha256 = e.content_sha \
         where e.tenant_id = $1 and e.session_id = $2 \
         order by e.seq",
    )
    .bind(&user.tenant_id)
    .bind(&session_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    // Findings for this session, grouped by event seq so each event card can
    // render an inline marker line for any secret / PII detected in its content.
    let frows = sqlx::query(
        "select seq, rule, severity from findings \
         where tenant_id = $1 and session_id = $2",
    )
    .bind(&user.tenant_id)
    .bind(&session_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();
    let mut findings_by_seq: HashMap<i64, Vec<(String, String)>> = HashMap::new();
    for r in &frows {
        let seq: i64 = r.get("seq");
        let rule: String = r.get("rule");
        let severity: String = r.get("severity");
        findings_by_seq.entry(seq).or_default().push((rule, severity));
    }

    let (email, class, title, org, name) = match &meta {
        Some(m) => (
            m.get::<String, _>("user_email"),
            m.get::<String, _>("classification"),
            m.get::<Option<String>, _>("title").unwrap_or_default(),
            m.get::<Option<String>, _>("repo_org").unwrap_or_default(),
            m.get::<Option<String>, _>("repo_name").unwrap_or_default(),
        ),
        None => (
            String::new(),
            "unknown".to_string(),
            String::new(),
            String::new(),
            String::new(),
        ),
    };
    let header = if title.is_empty() {
        session_id.clone()
    } else {
        title
    };
    let repo = if org.is_empty() && name.is_empty() {
        String::new()
    } else {
        format!("{org}/{name}")
    };

    page(
        "Session",
        html! {
            p { a href="/dashboard" { "← dashboard" } }
            h1 { (header) }
            p {
                (email) " · "
                span.badge.(class) { (class) }
                @if !repo.is_empty() { " · " (repo) }
                " · " (events.len()) " events · "
                code { (session_id) }
            }
            @for e in &events {
                @let seq: i64 = e.get("seq");
                @let kind: String = e.get("kind");
                @let tool: Option<String> = e.get("tool_name");
                @let target: Option<String> = e.get("target");
                @let content: Option<String> = e.get("content");
                @let model: Option<String> = e.get("model");
                @let tin: i64 = e.get("tokens_in");
                @let tout: i64 = e.get("tokens_out");
                @let side: bool = e.get("is_sidechain");
                div.ev.(kind) {
                    div.k {
                        (kind)
                        @if let Some(t) = &tool { " · " (t) }
                        @if let Some(tg) = &target { " · " (tg.chars().take(80).collect::<String>()) }
                        @if let Some(m) = &model { " · " (m) }
                        @if tin > 0 || tout > 0 { " · " (tin) "/" (tout) " tok" }
                        @if side { " · subagent" }
                    }
                    @if let Some(fs) = findings_by_seq.get(&seq) {
                        @for (rule, severity) in fs {
                            div.finding {
                                "⚠ " (rule) " ("
                                span.badge.(severity) { (severity) }
                                ")"
                            }
                        }
                    }
                    @if let Some(c) = &content { pre { (c) } }
                }
            }
        },
    )
}
