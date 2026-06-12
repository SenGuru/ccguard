use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts, Path, Query, State};
use axum::http::request::Parts;
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::{Duration, Utc};
use maud::{html, Markup, DOCTYPE};
use std::collections::HashMap;
use serde::Deserialize;
use sqlx::{PgPool, Row};

use ccguard_core::enforce::{managed_settings_pretty, policy_hash, PolicyConfig};

use crate::error::AppError;
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
                    title { "Claresso — " (title) }
                    link rel="preconnect" href="https://fonts.googleapis.com";
                    link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Sora:wght@400;500;600;700&family=Inter:wght@400;450;500;600&family=JetBrains+Mono:wght@400;500;600&display=swap";
                    script src="https://cdn.jsdelivr.net/npm/chart.js" {}
                    style { (maud::PreEscaped(CSS)) }
                }
                body {
                    div.wrap { (body) }
                    script { (maud::PreEscaped("document.querySelectorAll('.navlinks a').forEach(function(a){if(a.getAttribute('href')===location.pathname)a.classList.add('active')});")) }
                }
            }
        }
        .into_string(),
    )
}

/// Shared top navigation, rendered consistently on every dashboard page.
pub fn nav() -> Markup {
    html! {
        div.topnav {
            a.brand href="/dashboard" { (maud::PreEscaped(LOGO_SVG)) "Claresso" }
            div.navlinks {
                a href="/dashboard" { "Dashboard" }
                a href="/dashboard/people" { "People" }
                a href="/dashboard/search" { "Search" }
                a href="/dashboard/findings" { "Findings" }
                a href="/dashboard/fleet" { "Fleet" }
                a href="/dashboard/policy" { "Policy" }
                a href="/dashboard/review" { "Review" }
                a href="/dashboard/triage" { "Triage" }
                a href="/dashboard/signals" { "Signals" }
                a href="/dashboard/usage" { "Usage" }
                a href="/dashboard/enforcement" { "Enforce" }
                a href="/dashboard/roles" { "Roles" }
            }
        }
    }
}

const LOGO_SVG: &str = "<svg class=\"logo-mk\" viewBox=\"0 0 64 64\" fill=\"none\"><path d=\"M48.38 20.53 A20 20 0 1 0 48.38 43.47\" stroke=\"currentColor\" stroke-width=\"8\" stroke-linecap=\"round\"/></svg>";

const CSS: &str = ":root{--bg:#F7F8FB;--panel:#FFFFFF;--panel-2:#F4F6FA;--panel-3:#EDF1F7;\
--ink:#0F1422;--ink-2:#5A6478;--ink-3:#9099AB;--line:#E7EAF0;--line-2:#EEF1F6;\
--accent:#2F6BFF;--accent-2:#2F6BFF;--accent-ink:#1E47C8;--accent-wash:#EAF0FF;\
--ok:#0E9F6E;--ok-bg:#E6F6EE;--warn:#A96A00;--warn-bg:#FEF3DF;\
--bad:#B42318;--bad-bg:#FDE8E8;--violet:#7A4FB5;--violet-bg:#F1E9FB}\
*{box-sizing:border-box}\
body{font:14px/1.55 Inter,system-ui,sans-serif;margin:0;background:var(--bg);color:var(--ink);-webkit-font-smoothing:antialiased}\
.wrap{max-width:1180px;margin:0 auto;padding:0 28px 72px}.wrap:has(.topnav){margin:0 0 0 234px;max-width:1180px;padding:26px 44px 84px}\
h1,h2,h3{font-family:Sora,Inter,sans-serif;font-weight:600;letter-spacing:-.025em;color:var(--ink)}h1{font-size:24px;margin:16px 0 18px}h2{font-size:19px}h3{font-size:16px}\
a{color:var(--accent-ink);text-decoration:none}a:hover{text-decoration:underline}\
small{color:var(--ink-3)}\
.topnav{position:fixed;left:0;top:0;bottom:0;z-index:30;width:234px;display:flex;flex-direction:column;gap:3px;padding:22px 16px;border-right:1px solid var(--line);background:var(--panel-2);overflow-y:auto}\
.brand{display:flex;align-items:center;gap:9px;font-family:Sora;font-weight:600;font-size:18px;color:var(--ink);letter-spacing:-.03em;text-decoration:none;padding:2px 10px 18px}.brand:hover{text-decoration:none}\
.logo-mk{width:22px;height:22px;color:var(--accent-2);flex:none}\
.navlinks{display:flex;flex-direction:column;gap:2px;border-top:1px solid var(--line);padding-top:10px;margin-top:2px}.navlinks a{display:flex;align-items:center;gap:10px;color:var(--ink-2);font-size:13.5px;font-weight:500;text-decoration:none;padding:8px 11px;border-radius:8px}.navlinks a:hover{color:var(--ink);background:rgba(15,20,34,.045);text-decoration:none}.navlinks a.active{color:var(--accent-ink);background:var(--accent-wash);box-shadow:inset 2px 0 0 var(--accent-2)}\
table{width:100%;border-collapse:collapse;margin-top:14px;background:var(--panel);border:1px solid var(--line);border-radius:11px;overflow:hidden}\
th,td{text-align:left;padding:11px 14px;border-bottom:1px solid var(--line-2);font-size:13px}thead th{font-family:'JetBrains Mono',monospace;font-size:10.5px;color:var(--ink-3);text-transform:uppercase;letter-spacing:.06em;font-weight:600;background:var(--panel-2)}tr:last-child td{border-bottom:0}tbody tr:hover{background:rgba(91,135,255,.045)}td a{color:var(--accent-2)}\
.badge{display:inline-block;padding:3px 9px;border-radius:7px;font-size:11.5px;font-weight:600;font-family:'JetBrains Mono',monospace}\
.work,.compliant,.on_task{background:var(--ok-bg);color:var(--ok)}.personal,.personal_repo{background:var(--bad-bg);color:var(--bad)}.unknown,.stale,.low{background:rgba(140,151,180,.12);color:var(--ink-2)}.pending{background:var(--accent-wash);color:var(--accent-2)}\
.high,.tampered,.noncompliant_account,.off_task{background:var(--bad-bg);color:var(--bad)}.medium,.drifted,.review,.off_assignment,.label_drifted{background:var(--warn-bg);color:var(--warn)}.non_engineer_coding{background:var(--violet-bg);color:var(--violet)}\
.finding{font-size:12px;margin:4px 0 0;color:var(--warn)}\
.ev{border:1px solid var(--line);border-left:3px solid var(--line);margin:8px 0;padding:11px 15px;border-radius:9px;background:var(--panel)}\
.k{font-weight:600;font-size:10.5px;text-transform:uppercase;letter-spacing:.06em;color:var(--ink-3);font-family:'JetBrains Mono',monospace}\
.user_prompt{border-left-color:#2F6BFF}.assistant_text{border-left-color:#3FD99A}.thinking{border-left-color:#5C6A8C}.tool_call,.bash_command{border-left-color:#E7B84D}.tool_result{border-left-color:#5B87FF}.file_edit{border-left-color:#D05FB0}.pr{border-left-color:#0DB8A0}\
pre{white-space:pre-wrap;word-break:break-word;margin:6px 0 0;font:12px/1.55 'JetBrains Mono',ui-monospace,Consolas,monospace;color:var(--ink-2)}\
.card{background:var(--panel);border:1px solid var(--line);border-radius:13px;padding:20px;margin:14px 0;box-shadow:0 1px 2px rgba(16,24,52,.05)}.card h3{margin-top:0}\
input,select,textarea{font:14px Inter;padding:9px 12px;border-radius:8px;border:1px solid var(--line);background:var(--panel-2);color:var(--ink)}input::placeholder{color:var(--ink-3)}input:focus,select:focus{outline:none;border-color:var(--accent)}\
button{font:14px Inter;font-weight:550;padding:9px 16px;border-radius:8px;border:1px solid transparent;background:var(--accent);color:#fff;cursor:pointer}button:hover{background:var(--accent-2)}\
.err{color:var(--bad)}\
.kpis{display:grid;grid-template-columns:repeat(auto-fit,minmax(132px,1fr));gap:12px;margin:16px 0}.kpi{background:var(--panel);border:1px solid var(--line);border-radius:11px;padding:14px 16px;box-shadow:0 1px 2px rgba(16,24,52,.05)}.kpi .lab{font-size:10.5px;text-transform:uppercase;letter-spacing:.05em;color:var(--ink-3);font-family:'JetBrains Mono',monospace}.kpi .big{font-family:Sora;font-weight:700;font-size:24px;margin-top:5px;color:var(--ink)}.kpi .big.az{color:var(--accent-ink)}.kpi .big.gd{color:var(--ok)}.kpi .big.bd{color:var(--bad)}\
.pagehead{display:flex;align-items:baseline;justify-content:space-between;flex-wrap:wrap;gap:8px}.muted{color:var(--ink-2)}\
@media(max-width:820px){.topnav{position:static;width:auto;flex-direction:row;align-items:center;gap:12px;overflow-x:auto;padding:10px 16px;border-right:0;border-bottom:1px solid var(--line)}.brand{padding:0 8px 0 0}.navlinks{flex-direction:row;flex-wrap:nowrap}.navlinks a{padding:6px 9px}.wrap:has(.topnav){margin:0 auto;padding:18px 18px 60px}}";

/// Sign-in form. `err` renders an inline error message when present.
pub fn login_page(err: Option<&str>) -> Html<String> {
    page(
        "Sign in",
        html! {
            div.card style="max-width:380px;margin:64px auto" {
                div.brand style="font-size:22px;margin:2px 0 18px" { (maud::PreEscaped(LOGO_SVG)) "Claresso" }
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

    // Fleet KPI: total enrolled devices and how many are non-compliant or stale.
    let fleet_row = sqlx::query(
        "select count(*) as total, \
                count(*) filter (where compliance <> 'compliant' \
                                   or last_seen < now() - interval '15 minutes' \
                                   or last_seen is null) as noncompliant \
         from devices where tenant_id = $1",
    )
    .bind(&user.tenant_id)
    .fetch_one(&pool)
    .await
    .ok();
    let (fleet_total, fleet_noncompliant) = match &fleet_row {
        Some(r) => (r.get::<i64, _>("total"), r.get::<i64, _>("noncompliant")),
        None => (0i64, 0i64),
    };

    // On-task KPI: share of scored sessions labelled on_task + open-indicator count.
    let ontask_row = sqlx::query(
        "select count(*) total, count(*) filter (where label='on_task') ontask \
         from session_scores where tenant_id = $1",
    )
    .bind(&user.tenant_id)
    .fetch_one(&pool)
    .await
    .ok();
    let (ot_total, ot_on) = match &ontask_row {
        Some(r) => (r.get::<i64, _>("total"), r.get::<i64, _>("ontask")),
        None => (0i64, 0i64),
    };
    let ot_pct = if ot_total == 0 { 0 } else { ot_on * 100 / ot_total };
    let open_indicators: i64 = sqlx::query_scalar(
        "select count(*) from indicators where tenant_id = $1 and status = 'open'",
    )
    .bind(&user.tenant_id)
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    page(
        "Dashboard",
        html! {
            (nav())
            h1 { "Claresso — captured Claude Code activity" }
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
                    p {
                        a href="/dashboard/fleet" { "Fleet: " }
                        b { (fleet_total) } " devices · "
                        span.badge.tampered { (fleet_noncompliant) " non-compliant" }
                    }
                    p {
                        a href="/dashboard/review" { "On-task: " }
                        b { (ot_pct) "%" } " of " (ot_total) " sessions · "
                        a href="/dashboard/review" { span.badge.off_task { (open_indicators) " open indicators" } }
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
                 backgroundColor:['#16A34A','#D97706','#CBD2E0'],borderColor:'#FFFFFF',borderWidth:2}}]}},\
                 options:{{cutout:'66%',plugins:{{legend:{{labels:{{color:'#5A6478',font:{{family:'Inter'}}}}}}}}}}}});"))) }
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
            (nav())
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
        "select user_email, classification, title, repo_org, repo_name, on_hold \
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

    // On-task score + label + reasons for this session (if scored).
    let score_row = sqlx::query(
        "select score, label, reasons from session_scores \
         where tenant_id = $1 and session_id = $2",
    )
    .bind(&user.tenant_id)
    .bind(&session_id)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    // Indicators raised for this session (any status), newest-first.
    let indicator_rows = sqlx::query(
        "select kind, detail, status from indicators \
         where tenant_id = $1 and session_id = $2 \
         order by created_at desc, id desc",
    )
    .bind(&user.tenant_id)
    .bind(&session_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    // LLM triage verdict for this session (if the judge has classified it).
    let triage_row = sqlx::query(
        "select label, confidence, reason, enforceable, resolved_by, model \
         from session_triage where tenant_id = $1 and session_id = $2",
    )
    .bind(&user.tenant_id)
    .bind(&session_id)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    // Deterministic provenance verdict (the primary classifier's cascade output).
    let prov_row = sqlx::query(
        "select class, confidence, provisional, resolved_by, reasons \
         from session_provenance where tenant_id = $1 and session_id = $2",
    )
    .bind(&user.tenant_id)
    .bind(&session_id)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    let (email, class, title, org, name, on_hold) = match &meta {
        Some(m) => (
            m.get::<String, _>("user_email"),
            m.get::<String, _>("classification"),
            m.get::<Option<String>, _>("title").unwrap_or_default(),
            m.get::<Option<String>, _>("repo_org").unwrap_or_default(),
            m.get::<Option<String>, _>("repo_name").unwrap_or_default(),
            m.get::<bool, _>("on_hold"),
        ),
        None => (
            String::new(),
            "unknown".to_string(),
            String::new(),
            String::new(),
            String::new(),
            false,
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
            (nav())
            h1 {
                (header)
                @if on_hold { " " span.badge.high title="ON LEGAL HOLD" { "🔒 ON LEGAL HOLD" } }
            }
            p {
                (email) " · "
                span.badge.(class) { (class) }
                @if !repo.is_empty() { " · " (repo) }
                " · " (events.len()) " events · "
                code { (session_id) }
            }
            @if let Some(sr) = &score_row {
                @let score: i32 = sr.get("score");
                @let label: String = sr.get("label");
                @let reasons: Option<String> = sr.get("reasons");
                p {
                    "On-task: " b { (score) } " "
                    span.badge.(label) { (label) }
                    @if let Some(rs) = &reasons {
                        @if !rs.is_empty() { " — " (rs) }
                    }
                }
            }
            @if let Some(pr) = &prov_row {
                @let pclass: String = pr.get("class");
                @let pconf: f32 = pr.get("confidence");
                @let pprov: bool = pr.get("provisional");
                @let pby: String = pr.get("resolved_by");
                @let preasons: String = pr.get("reasons");
                p {
                    "Provenance: "
                    span.badge.(provenance_class_badge(&pclass)) { (pclass.replace('_', " ")) }
                    " " (format!("{:.0}%", pconf * 100.0))
                    @if pprov { " · " span.badge.medium { "provisional" } }
                    " · " small style="color:var(--ink-3)" {
                        (pby)
                        @if !preasons.is_empty() { " · signals: " (preasons) }
                    }
                }
            }
            @if let Some(tr) = &triage_row {
                @let tlabel: String = tr.get("label");
                @let tconf: f32 = tr.get("confidence");
                @let treason: String = tr.get("reason");
                @let tenf: bool = tr.get("enforceable");
                @let tby: String = tr.get("resolved_by");
                @let tmodel: String = tr.get("model");
                p {
                    "LLM triage: "
                    span.badge.(triage_label_class(&tlabel)) { (tlabel) }
                    " " (format!("{:.0}%", tconf * 100.0)) " — " (treason)
                    " · " small style="color:var(--ink-3)" {
                        (tby) "/" (tmodel) " · "
                        @if tenf { "enforceable" } @else { "visibility only" }
                    }
                }
            }
            @if !indicator_rows.is_empty() {
                ul style="margin:4px 0 0;padding-left:18px" {
                    @for ir in &indicator_rows {
                        @let kind: String = ir.get("kind");
                        @let detail: Option<String> = ir.get("detail");
                        @let status: String = ir.get("status");
                        li.finding {
                            span.badge.(kind) { (kind) }
                            @if let Some(d) = &detail { " " (d) }
                            " (" (status) ")"
                        }
                    }
                }
            }
            p {
                a href={"/dashboard/sessions/" (session_id) "/export"} { "Export NDJSON" }
                " · "
                form method="post" action={"/dashboard/sessions/" (session_id) "/hold"} style="display:inline" {
                    button type="submit" {
                        @if on_hold { "Release hold" } @else { "Place legal hold" }
                    }
                }
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

/// eDiscovery export: stream one captured session as newline-delimited JSON
/// (NDJSON). Line 1 is the session record; each subsequent line is one event in
/// `seq` order with its verbatim content joined from `content_blobs` (null when
/// no blob). Tenant-scoped via `WebUser`; a session not owned by this tenant
/// returns 404 (never another tenant's data). Served as an `attachment` so the
/// browser downloads `<session_id>.ndjson`.
pub async fn export(
    user: WebUser,
    State(pool): State<PgPool>,
    Path(session_id): Path<String>,
) -> Result<Response, AppError> {
    let meta = sqlx::query(
        "select session_id, user_email, classification, title, repo_org, repo_name, on_hold \
         from captured_sessions where tenant_id = $1 and session_id = $2",
    )
    .bind(&user.tenant_id)
    .bind(&session_id)
    .fetch_optional(&pool)
    .await?;

    let meta = match meta {
        Some(m) => m,
        None => return Ok((StatusCode::NOT_FOUND, "session not found").into_response()),
    };

    let events = sqlx::query(
        "select e.seq, e.kind, e.tool_name, e.target, b.content, \
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
    .await?;

    let mut lines: Vec<String> = Vec::with_capacity(events.len() + 1);
    let session_line = serde_json::json!({
        "type": "session",
        "session_id": meta.get::<String, _>("session_id"),
        "user_email": meta.get::<String, _>("user_email"),
        "classification": meta.get::<String, _>("classification"),
        "title": meta.get::<Option<String>, _>("title"),
        "repo_org": meta.get::<Option<String>, _>("repo_org"),
        "repo_name": meta.get::<Option<String>, _>("repo_name"),
        "on_hold": meta.get::<bool, _>("on_hold"),
    });
    lines.push(serde_json::to_string(&session_line).unwrap_or_default());

    for e in &events {
        let event_line = serde_json::json!({
            "type": "event",
            "seq": e.get::<i64, _>("seq"),
            "kind": e.get::<String, _>("kind"),
            "tool_name": e.get::<Option<String>, _>("tool_name"),
            "target": e.get::<Option<String>, _>("target"),
            "content": e.get::<Option<String>, _>("content"),
            "tokens_in": e.get::<i64, _>("tokens_in"),
            "tokens_out": e.get::<i64, _>("tokens_out"),
            "is_sidechain": e.get::<bool, _>("is_sidechain"),
        });
        lines.push(serde_json::to_string(&event_line).unwrap_or_default());
    }

    let body = lines.join("\n");
    let disposition = format!("attachment; filename=\"{session_id}.ndjson\"");
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/x-ndjson".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        body,
    )
        .into_response())
}

/// Legal hold toggle: flip `on_hold` for one captured session, then 303-redirect
/// back to its replay page. Tenant-scoped via `WebUser` — the update only ever
/// touches a row owned by this tenant.
pub async fn hold(
    user: WebUser,
    State(pool): State<PgPool>,
    Path(session_id): Path<String>,
) -> Result<Redirect, AppError> {
    sqlx::query(
        "update captured_sessions set on_hold = not on_hold \
         where tenant_id = $1 and session_id = $2",
    )
    .bind(&user.tenant_id)
    .bind(&session_id)
    .execute(&pool)
    .await?;
    Ok(Redirect::to(&format!("/dashboard/sessions/{session_id}")))
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
}

/// Full-text search page over captured content. GET form prefilled with the
/// current query; on a non-blank `q` it runs the same tenant-scoped query as the
/// JSON API (`handlers::search::run_search`) and renders the matching events.
///
/// The snippet from `ts_headline` is PLAIN TEXT with `«match»` markers — it is
/// rendered maud-escaped via `(hit.snippet)` (NOT `PreEscaped`), so any `<`,
/// `>`, `&` in captured content is escaped: XSS-safe. Cookie-authed via
/// `WebUser` and tenant-scoped.
pub async fn search(
    user: WebUser,
    State(pool): State<PgPool>,
    Query(params): Query<SearchParams>,
) -> Html<String> {
    let q = params.q.unwrap_or_default();
    let trimmed = q.trim();
    let hits = if trimmed.is_empty() {
        Vec::new()
    } else {
        crate::handlers::search::run_search(&pool, &user.tenant_id, &q)
            .await
            .unwrap_or_default()
    };
    let has_query = !trimmed.is_empty();

    page(
        "Search",
        html! {
            (nav())
            h1 { "Search captured content" }
            form method="get" action="/dashboard/search" {
                input type="text" name="q" value=(q) placeholder="Search prompts, code, tool output…" style="width:70%";
                " " button type="submit" { "Search" }
            }
            @if has_query {
                p { (hits.len()) " result(s)" }
                @if hits.is_empty() {
                    p { "no results" }
                } @else {
                    @for hit in &hits {
                        div.ev.(hit.kind) {
                            div.k {
                                a href={"/dashboard/sessions/" (hit.session_id)} {
                                    (hit.title.clone().unwrap_or_else(|| hit.session_id.chars().take(8).collect()))
                                }
                                " · " (hit.kind)
                                @let repo = format!(
                                    "{}/{}",
                                    hit.repo_org.clone().unwrap_or_default(),
                                    hit.repo_name.clone().unwrap_or_default()
                                );
                                @if repo != "/" { " · " (repo) }
                            }
                            pre { (hit.snippet) }
                        }
                    }
                }
            }
        },
    )
}

// ---------------------------------------------------------------------------
// Fleet compliance page — GET /dashboard/fleet  (WebUser)
// ---------------------------------------------------------------------------

/// Per-OS managed-settings install path. The same path the deploy scripts write
/// to and the same precedence note shown on the policy page.
const MANAGED_SETTINGS_PATHS: &[(&str, &str)] = &[
    ("Windows", r"C:\ProgramData\ClaudeCode\managed-settings.json"),
    (
        "macOS",
        "/Library/Application Support/ClaudeCode/managed-settings.json",
    ),
    ("Linux", "/etc/claude-code/managed-settings.json"),
];

/// Fleet compliance view: every enrolled device for the tenant with its latest
/// attestation verdict. Reuses the exact staleness override from `fleet::list`
/// (`last_seen` null or older than 15 minutes reads `stale`). The compliance
/// string doubles as the badge CSS class. Cookie-authed + tenant-scoped.
pub async fn fleet(user: WebUser, State(pool): State<PgPool>) -> Html<String> {
    let rows = sqlx::query(
        "select device_id, hostname, os, agent_version, user_email, reasons, last_seen, \
                case when last_seen is null or last_seen < now() - interval '15 minutes' \
                     then 'stale' else compliance end as compliance \
         from devices where tenant_id = $1 \
         order by last_seen desc nulls last limit 500",
    )
    .bind(&user.tenant_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    page(
        "Fleet",
        html! {
            (nav())
            h1 { "Fleet compliance" }
            p { (rows.len()) " device(s) enrolled" }
            @if rows.is_empty() {
                div.card {
                    p { "No devices enrolled yet — deploy the policy from the " a href="/dashboard/policy" { "Policy page" } "." }
                }
            } @else {
                table {
                    thead { tr { th{"Host"} th{"User"} th{"OS"} th{"Agent ver"} th{"Last seen"} th{"Compliance"} th{"Reasons"} } }
                    tbody {
                        @for r in &rows {
                            @let device_id: String = r.get("device_id");
                            @let hostname: Option<String> = r.get("hostname");
                            @let os: Option<String> = r.get("os");
                            @let ver: Option<String> = r.get("agent_version");
                            @let email: Option<String> = r.get("user_email");
                            @let compliance: String = r.get("compliance");
                            @let reasons: Option<String> = r.get("reasons");
                            @let last_seen: Option<chrono::DateTime<chrono::Utc>> = r.get("last_seen");
                            tr {
                                td { (hostname.clone().unwrap_or_else(|| device_id.clone())) }
                                td { (email.clone().unwrap_or_default()) }
                                td { (os.clone().unwrap_or_default()) }
                                td { (ver.clone().unwrap_or_default()) }
                                td { (last_seen.map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string()).unwrap_or_else(|| "never".into())) }
                                td { span.badge.(compliance) { (compliance) } }
                                td { (reasons.clone().unwrap_or_default()) }
                            }
                        }
                    }
                }
            }
        },
    )
}

// ---------------------------------------------------------------------------
// Policy generator page — GET/POST /dashboard/policy  (WebUser)
// ---------------------------------------------------------------------------

/// Load the tenant's `tenant_policy` row as a `PolicyConfig`, if set.
async fn load_policy(pool: &PgPool, tenant_id: &str) -> Option<PolicyConfig> {
    let row = sqlx::query(
        "select server_url, org_uuid, otel_endpoint, min_version, token_env \
         from tenant_policy where tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;
    Some(PolicyConfig {
        server_url: row.get("server_url"),
        org_uuid: row.get("org_uuid"),
        otel_endpoint: row.get("otel_endpoint"),
        min_version: row.get("min_version"),
        token_env: row.get("token_env"),
    })
}

/// Render the per-OS managed-settings deploy block (install paths + precedence).
fn deploy_block(hash: &str) -> Markup {
    html! {
        div.card {
            h1 style="font-size:16px" { "Deploy" }
            p { "Write the generated " code { "managed-settings.json" } " to the OS-specific path below. This file is " b { "highest-precedence" } " — a user cannot override it." }
            table {
                thead { tr { th{"OS"} th{"Path"} } }
                tbody {
                    @for (os, path) in MANAGED_SETTINGS_PATHS {
                        tr { td { (os) } td { code { (path) } } }
                    }
                }
            }
            p { "Policy hash: " code { (hash) } }
            p { a href="/dashboard/policy/managed-settings.json" { "Download managed-settings.json" } }
        }
    }
}

/// Policy page (GET). If a policy is set, render the generated managed-settings
/// JSON (maud-escaped in a `pre`), the per-OS deploy block, the policy hash, and
/// a download link. If unset, render the configure form to owner/admin; everyone
/// else sees an "ask an owner" hint. Cookie-authed + tenant-scoped.
pub async fn policy_get(user: WebUser, State(pool): State<PgPool>) -> Html<String> {
    let cfg = load_policy(&pool, &user.tenant_id).await;
    let can_edit = user.role == "owner" || user.role == "admin";

    page(
        "Policy",
        html! {
            (nav())
            h1 { "Enforcement policy" }
            @match &cfg {
                Some(c) => {
                    p { "This is the Claude Code enterprise " b { "managed-settings.json" } " Claresso generates for your org. It forces telemetry on, pins the corp login org, and wires the capture hook to your server." }
                    div.card {
                        pre { (managed_settings_pretty(c)) }
                    }
                    (deploy_block(&policy_hash(c)))
                    @if can_edit {
                        (policy_form(c))
                    }
                }
                None => {
                    @if can_edit {
                        p { "No policy configured yet. Set one below to generate the managed-settings.json for your fleet." }
                        (policy_form(&default_policy()))
                    } @else {
                        div.card { p { "No policy configured yet — ask an owner to configure the policy." } }
                    }
                }
            }
        },
    )
}

/// Default seed values for the configure form when no policy exists yet.
fn default_policy() -> PolicyConfig {
    PolicyConfig {
        server_url: String::new(),
        org_uuid: String::new(),
        otel_endpoint: String::new(),
        min_version: "2.1.38".to_string(),
        token_env: "CCGUARD_TOKEN".to_string(),
    }
}

/// The owner/admin policy configure form, prefilled from `c`.
fn policy_form(c: &PolicyConfig) -> Markup {
    html! {
        div.card {
            h1 style="font-size:16px" { "Configure policy" }
            form method="post" action="/dashboard/policy" {
                p { "Corp Claude org UUID " br;
                    input type="text" name="org_uuid" value=(c.org_uuid) placeholder="org-…" style="width:100%"; }
                p { "OTEL collector endpoint " br;
                    input type="text" name="otel_endpoint" value=(c.otel_endpoint) placeholder="https://otel.corp:4318" style="width:100%"; }
                p { "Claresso server URL " br;
                    input type="text" name="server_url" value=(c.server_url) placeholder="https://claresso.acme.com" style="width:100%"; }
                p { "Minimum Claude Code version " br;
                    input type="text" name="min_version" value=(c.min_version) style="width:100%"; }
                p { "Token env var " br;
                    input type="text" name="token_env" value=(c.token_env) style="width:100%"; }
                p { button type="submit" { "Save policy" } }
            }
        }
    }
}

#[derive(Deserialize)]
pub struct PolicyForm {
    pub org_uuid: String,
    pub otel_endpoint: String,
    #[serde(default)]
    pub server_url: String,
    #[serde(default)]
    pub min_version: String,
    #[serde(default)]
    pub token_env: String,
}

/// Policy page (POST): upsert the tenant policy. Owner/admin only (else 403).
/// Empty `min_version`/`token_env` fall back to the documented defaults; an empty
/// `server_url` falls back to the request's own base. 303-redirect on success.
pub async fn policy_set(
    user: WebUser,
    State(pool): State<PgPool>,
    Form(f): Form<PolicyForm>,
) -> Result<Response, AppError> {
    if user.role != "owner" && user.role != "admin" {
        return Err(AppError::Forbidden("owner or admin role required"));
    }
    let min_version = if f.min_version.trim().is_empty() {
        "2.1.38".to_string()
    } else {
        f.min_version.trim().to_string()
    };
    let token_env = if f.token_env.trim().is_empty() {
        "CCGUARD_TOKEN".to_string()
    } else {
        f.token_env.trim().to_string()
    };
    let server_url = f.server_url.trim().to_string();

    sqlx::query(
        "insert into tenant_policy \
         (tenant_id, server_url, org_uuid, otel_endpoint, min_version, token_env, updated_at) \
         values ($1,$2,$3,$4,$5,$6, now()) \
         on conflict (tenant_id) do update set \
           server_url = excluded.server_url, \
           org_uuid = excluded.org_uuid, \
           otel_endpoint = excluded.otel_endpoint, \
           min_version = excluded.min_version, \
           token_env = excluded.token_env, \
           updated_at = now()",
    )
    .bind(&user.tenant_id)
    .bind(&server_url)
    .bind(&f.org_uuid)
    .bind(&f.otel_endpoint)
    .bind(&min_version)
    .bind(&token_env)
    .execute(&pool)
    .await?;

    Ok(Redirect::to("/dashboard/policy").into_response())
}

/// Download the generated managed-settings.json for the tenant as a JSON
/// attachment. 404 when no policy is set. Cookie-authed + tenant-scoped.
pub async fn policy_download(
    user: WebUser,
    State(pool): State<PgPool>,
) -> Result<Response, AppError> {
    let cfg = match load_policy(&pool, &user.tenant_id).await {
        Some(c) => c,
        None => return Ok((StatusCode::NOT_FOUND, "policy not set").into_response()),
    };
    let body = managed_settings_pretty(&cfg);
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"managed-settings.json\"".to_string(),
            ),
        ],
        body,
    )
        .into_response())
}

// ---------------------------------------------------------------------------
// Indicator review queue — GET /dashboard/review  (WebUser)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ReviewParams {
    pub status: Option<String>,
}

/// Indicator review queue. Lists tenant indicators for the requested `status`
/// (default `open`), newest-first, each with a session link and per-row
/// Reviewed / Dismiss buttons. Cookie-authed + tenant-scoped.
pub async fn review(
    user: WebUser,
    State(pool): State<PgPool>,
    Query(params): Query<ReviewParams>,
) -> Html<String> {
    let status = params
        .status
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "open".to_string());
    let rows = sqlx::query(
        "select id, user_email, session_id, kind, detail, status, created_at \
         from indicators where tenant_id = $1 and status = $2 \
         order by created_at desc, id desc limit 500",
    )
    .bind(&user.tenant_id)
    .bind(&status)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    page(
        "Review queue",
        html! {
            (nav())
            h1 { "Indicator review queue" }
            p {
                "Showing " b { (status) } " indicators · "
                a href="/dashboard/review?status=open" { "open" } " · "
                a href="/dashboard/review?status=reviewed" { "reviewed" } " · "
                a href="/dashboard/review?status=dismissed" { "dismissed" }
            }
            @if rows.is_empty() {
                div.card { p { "Review queue clear." } }
            } @else {
                table {
                    thead { tr { th{"Created"} th{"User"} th{"Kind"} th{"Detail"} th{"Session"} th{"Status"} th{"Actions"} } }
                    tbody {
                        @for r in &rows {
                            @let id: i64 = r.get("id");
                            @let email: Option<String> = r.get("user_email");
                            @let session_id: Option<String> = r.get("session_id");
                            @let kind: String = r.get("kind");
                            @let detail: Option<String> = r.get("detail");
                            @let st: String = r.get("status");
                            @let created: chrono::DateTime<chrono::Utc> = r.get("created_at");
                            tr {
                                td { (created.format("%Y-%m-%d %H:%M UTC").to_string()) }
                                td { (email.clone().unwrap_or_default()) }
                                td { span.badge.(kind) { (kind) } }
                                td { (detail.clone().unwrap_or_default()) }
                                td {
                                    @if let Some(sid) = &session_id {
                                        a href={"/dashboard/sessions/" (sid)} { (sid.chars().take(8).collect::<String>()) }
                                    }
                                }
                                td { (st) }
                                td {
                                    form method="post" action={"/dashboard/indicators/" (id) "/status"} style="display:inline" {
                                        input type="hidden" name="status" value="reviewed";
                                        button type="submit" { "Reviewed" }
                                    }
                                    " "
                                    form method="post" action={"/dashboard/indicators/" (id) "/status"} style="display:inline" {
                                        input type="hidden" name="status" value="dismissed";
                                        button type="submit" { "Dismiss" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}

#[derive(Deserialize)]
pub struct IndicatorStatusForm {
    pub status: String,
}

/// Flip an indicator's status (reviewed | dismissed) from the review queue,
/// then 303-redirect back. Tenant-scoped: only a row owned by this tenant is
/// ever touched; an unowned/unknown id is silently ignored. Cookie-authed.
pub async fn indicator_status(
    user: WebUser,
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Form(f): Form<IndicatorStatusForm>,
) -> Result<Redirect, AppError> {
    let status = f.status.trim();
    if status == "reviewed" || status == "dismissed" {
        sqlx::query("update indicators set status = $1 where id = $2 and tenant_id = $3")
            .bind(status)
            .bind(id)
            .bind(&user.tenant_id)
            .execute(&pool)
            .await?;
    }
    Ok(Redirect::to("/dashboard/review"))
}

// ---------------------------------------------------------------------------
// Roles + per-repo work-definitions admin — GET/POST /dashboard/roles (WebUser)
// ---------------------------------------------------------------------------

/// The seven assignable job roles, used to render the role `<select>`.
const JOB_ROLES: &[&str] = &["engineer", "marketer", "designer", "pm", "ops", "sales", "other"];

/// Roles + per-repo work-definitions admin page (GET). Owner/admin see the two
/// edit forms; everyone sees the current lists. Cookie-authed + tenant-scoped.
pub async fn roles_get(user: WebUser, State(pool): State<PgPool>) -> Html<String> {
    let can_edit = user.role == "owner" || user.role == "admin";

    let role_rows = sqlx::query(
        "select user_email, job_role, note, assignment from employee_roles \
         where tenant_id = $1 order by user_email",
    )
    .bind(&user.tenant_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let repo_rows = sqlx::query(
        "select repo_host, repo_org, repo_name, classification, note from repo_overrides \
         where tenant_id = $1 order by repo_host, repo_org, repo_name",
    )
    .bind(&user.tenant_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    page(
        "Roles",
        html! {
            (nav())
            h1 { "Roles & work definitions" }
            @if !can_edit {
                div.card { p { "Ask an owner or admin to edit roles and work definitions." } }
            }

            div.card {
                h1 style="font-size:16px" { "People — role & assignment" }
                p style="color:var(--ink-2);font-size:13px" {
                    "Role = their function (drives anomaly flags). Assignment = what they're "
                    "working on right now (drives the off-assignment flag: company work that "
                    "isn't their lane). The assignment is read by the AI judge, in plain English."
                }
                @if can_edit {
                    form method="post" action="/dashboard/roles" {
                        input type="hidden" name="kind" value="role";
                        p { "Employee email " br;
                            input type="text" name="user_email" placeholder="dev@acme.com" style="width:100%"; }
                        p { "Job role " br;
                            select name="job_role" style="width:100%" {
                                @for r in JOB_ROLES { option value=(r) { (r) } }
                            }
                        }
                        p { "Assigned to (what they should be working on) " br;
                            input type="text" name="assignment"
                                placeholder="e.g. Grove — the screen-understanding engine" style="width:100%"; }
                        p { "Note " br;
                            input type="text" name="note" placeholder="optional" style="width:100%"; }
                        p { button type="submit" { "Save" } }
                    }
                }
                @if role_rows.is_empty() {
                    p { "No people configured yet." }
                } @else {
                    table {
                        thead { tr { th{"Email"} th{"Role"} th{"Assigned to"} th{"Note"} } }
                        tbody {
                            @for r in &role_rows {
                                @let email: String = r.get("user_email");
                                @let role: String = r.get("job_role");
                                @let note: Option<String> = r.get("note");
                                @let assignment: Option<String> = r.get("assignment");
                                tr {
                                    td { (email) }
                                    td { (role) }
                                    td { (assignment.clone().unwrap_or_default()) }
                                    td { (note.clone().unwrap_or_default()) }
                                }
                            }
                        }
                    }
                }
            }

            div.card {
                h1 style="font-size:16px" { "Per-repo work definitions" }
                @if can_edit {
                    form method="post" action="/dashboard/roles" {
                        input type="hidden" name="kind" value="repo";
                        p { "Repo host " br;
                            input type="text" name="repo_host" placeholder="github.com" style="width:100%"; }
                        p { "Repo org " br;
                            input type="text" name="repo_org" placeholder="acme-corp" style="width:100%"; }
                        p { "Repo name " br;
                            input type="text" name="repo_name" placeholder="billing" style="width:100%"; }
                        p { "Classification " br;
                            select name="classification" style="width:100%" {
                                option value="work" { "work" }
                                option value="personal" { "personal" }
                                option value="unknown" { "unknown" }
                            }
                        }
                        p { "Note " br;
                            input type="text" name="note" placeholder="optional" style="width:100%"; }
                        p { button type="submit" { "Save work definition" } }
                    }
                }
                @if repo_rows.is_empty() {
                    p { "No per-repo overrides yet." }
                } @else {
                    table {
                        thead { tr { th{"Host"} th{"Org"} th{"Name"} th{"Classification"} th{"Note"} } }
                        tbody {
                            @for r in &repo_rows {
                                @let host: String = r.get("repo_host");
                                @let org: String = r.get("repo_org");
                                @let name: String = r.get("repo_name");
                                @let class: String = r.get("classification");
                                @let note: Option<String> = r.get("note");
                                tr {
                                    td { (host) }
                                    td { (org) }
                                    td { (name) }
                                    td { span.badge.(class) { (class) } }
                                    td { (note.clone().unwrap_or_default()) }
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}

#[derive(Deserialize)]
pub struct RolesForm {
    pub kind: String, // role | repo
    #[serde(default)]
    pub user_email: String,
    #[serde(default)]
    pub job_role: String,
    #[serde(default)]
    pub repo_host: String,
    #[serde(default)]
    pub repo_org: String,
    #[serde(default)]
    pub repo_name: String,
    #[serde(default)]
    pub classification: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub assignment: String,
}

/// Roles admin (POST). Owner/admin only (else 403). Branches on `kind`:
/// `role` upserts `employee_roles`, `repo` upserts `repo_overrides`. The empty
/// note is stored as NULL. 303-redirect back to the roles page on success.
pub async fn roles_set(
    user: WebUser,
    State(pool): State<PgPool>,
    Form(f): Form<RolesForm>,
) -> Result<Response, AppError> {
    if user.role != "owner" && user.role != "admin" {
        return Err(AppError::Forbidden("owner or admin role required"));
    }
    let note: Option<String> = {
        let n = f.note.trim();
        if n.is_empty() {
            None
        } else {
            Some(n.to_string())
        }
    };
    let assignment: Option<String> = {
        let a = f.assignment.trim();
        if a.is_empty() {
            None
        } else {
            Some(a.to_string())
        }
    };
    match f.kind.as_str() {
        "role" => {
            sqlx::query(
                "insert into employee_roles (tenant_id, user_email, job_role, note, assignment, updated_at) \
                 values ($1,$2,$3,$4,$5, now()) \
                 on conflict (tenant_id, user_email) do update set \
                   job_role = excluded.job_role, \
                   note = excluded.note, \
                   assignment = excluded.assignment, \
                   updated_at = now()",
            )
            .bind(&user.tenant_id)
            .bind(f.user_email.trim())
            .bind(f.job_role.trim())
            .bind(&note)
            .bind(&assignment)
            .execute(&pool)
            .await?;
        }
        "repo" => {
            sqlx::query(
                "insert into repo_overrides \
                 (tenant_id, repo_host, repo_org, repo_name, classification, note, updated_at) \
                 values ($1,$2,$3,$4,$5,$6, now()) \
                 on conflict (tenant_id, repo_host, repo_org, repo_name) do update set \
                   classification = excluded.classification, \
                   note = excluded.note, \
                   updated_at = now()",
            )
            .bind(&user.tenant_id)
            .bind(f.repo_host.trim())
            .bind(f.repo_org.trim())
            .bind(f.repo_name.trim())
            .bind(f.classification.trim())
            .bind(&note)
            .execute(&pool)
            .await?;
        }
        _ => return Err(AppError::BadRequest("kind must be role or repo")),
    }
    Ok(Redirect::to("/dashboard/roles").into_response())
}

// ---- LLM triage --------------------------------------------------------------

/// Render the Triage page with an optional result banner. Shows API-key status,
/// the tenant's work-definition config (owner/admin editable), how many sessions
/// are still unclassified, a "run" action, and the most recent verdicts.
async fn render_triage(
    pool: &PgPool,
    user: &WebUser,
    banner: Option<Markup>,
    prefill: Option<(String, String, String)>,
) -> Html<String> {
    let mut cfg = crate::handlers::triage::load_config(pool, &user.tenant_id)
        .await
        .unwrap_or_default();
    // Prefill the form (from a starter template, or an AI draft) so the admin can
    // edit a noun or two and save.
    if let Some((bd, wa, pe)) = prefill {
        cfg.business_desc = bd;
        cfg.work_allowed = wa;
        cfg.personal_examples = pe;
    }
    let can_edit = user.role == "owner" || user.role == "admin";
    let key_present = crate::triage_client::api_key_present();

    // Sessions awaiting the AI judge ('pending' — the agent's local Claude Code, or
    // the opt-in server Run, will resolve them).
    let unclassified: i64 = sqlx::query_scalar(
        "select count(*) from captured_sessions where tenant_id=$1 and classification='pending'",
    )
    .bind(&user.tenant_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Conformal calibration regime (drives the banner).
    let calib = crate::handlers::enforcement::load_calibration(pool, &user.tenant_id)
        .await
        .unwrap_or(ccguard_core::conformal::Calibration { threshold: 1.01, n: 0, alpha: 0.1, usable: false });
    let regime = calib.regime();

    // Verdict tallies + recent verdicts (+ health inputs).
    let trows = sqlx::query(
        "select t.session_id, t.label, t.confidence, t.reason, t.enforceable, t.resolved_by, \
                t.mixed, t.matched_clause, t.gaming_flags, t.human_label, \
                s.title, s.repo_org, s.repo_name \
         from session_triage t \
         left join captured_sessions s on s.tenant_id=t.tenant_id and s.session_id=t.session_id \
         where t.tenant_id=$1 order by t.updated_at desc limit 100",
    )
    .bind(&user.tenant_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    let (mut tw, mut tp, mut tu) = (0i64, 0i64, 0i64);
    let mut health_rows = Vec::new();
    for r in &trows {
        let label: String = r.get("label");
        match label.as_str() {
            "work" => tw += 1,
            "personal" => tp += 1,
            _ => tu += 1,
        }
        health_rows.push(ccguard_core::policy_health::HealthRow {
            label,
            human_label: r.get("human_label"),
            matched_clause: r.get::<Option<String>, _>("matched_clause").is_some(),
        });
    }
    let health = ccguard_core::policy_health::health(&health_rows);

    // Random spot-check: a few work-labeled sessions not yet human-reviewed. The
    // gamer can't predict which work-labeled session gets human eyes — this is the
    // real anti-gaming pressure, and it keeps the calibration set fresh + balanced.
    let spot = sqlx::query(
        "select t.session_id, s.title, s.repo_org, s.repo_name from session_triage t \
         left join captured_sessions s on s.tenant_id=t.tenant_id and s.session_id=t.session_id \
         where t.tenant_id=$1 and t.label='work' and t.human_reviewed=false \
         order by random() limit 5",
    )
    .bind(&user.tenant_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Top reasons the AI got it wrong (from reviewer relabels) → refine the description.
    let reasons = sqlx::query(
        "select relabel_reason, count(*) c from session_triage \
         where tenant_id=$1 and relabel_reason is not null and relabel_reason <> '' \
         group by relabel_reason order by c desc limit 5",
    )
    .bind(&user.tenant_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    page(
        "Triage",
        html! {
            (nav())
            h1 { "Classification — AI judges against your business" }
            @if let Some(b) = banner { (b) }
            @match regime {
                ccguard_core::conformal::CalibrationRegime::Degenerate => {
                    div.card style="border-color:#b42318;background:#fde8e8" {
                        b { "⚠ The AI is confidently wrong on your data." }
                        " Every session is going to manual review. Your business description is "
                        "almost certainly the problem — make it more concrete (what does your work "
                        "look like in code?) and re-save."
                    }
                }
                ccguard_core::conformal::CalibrationRegime::Cold => {
                    div.card style="background:var(--accent-wash);border-color:var(--accent)" {
                        b { "The AI is still learning your judgment." }
                        " It's labeling for visibility only and holding back no one. As you Confirm/"
                        "Relabel verdicts (≈50 reviews), it starts saying " i { "unsure" } " when it isn't "
                        "confident by your standards."
                    }
                }
                ccguard_core::conformal::CalibrationRegime::Calibrated => {
                    div.card style="background:var(--ok-bg);border-color:rgba(63,217,154,.35)" {
                        b { "Calibrated to your judgment." }
                        " The AI now abstains to review below "
                        (format!("{:.0}%", calib.threshold.min(1.0) * 100.0)) " confidence (fit on "
                        (calib.n) " of your reviews)."
                    }
                }
            }
            @if health.n > 0 {
                div.card {
                    h3 { "How well is it reading your business?" }
                    p {
                        "Decisiveness: " b { (format!("{:.0}%", (1.0 - health.unsure_rate) * 100.0)) } " decided"
                        @if health.unsure_rate > 0.4 { " — " span.badge.medium { "high unsure" } " your description may be too vague; add concrete examples of your work." }
                        " · clause-matched on " (format!("{:.0}%", (1.0 - health.no_clause_rate) * 100.0)) " of decided"
                    }
                    @if health.n_human > 0 {
                        p {
                            "Agreement with your " (health.n_human) " correction(s): "
                            b { (format!("{:.0}%", health.agreement_rate * 100.0)) }
                            @if health.agreement_rate < 0.7 { " — " span.badge.high { "drifting from your calls" } }
                        }
                    }
                }
            }
            div.card {
                p {
                    "When the deterministic signal cascade can't tell whether a session is work or personal, "
                    "a Claude judge reads the repo, prompts and touched files and labels it "
                    b { "work" } ", " b { "personal" } ", or " b { "unsure" } " with a one-line reason."
                }
                p {
                    "Verdicts update the dashboard view immediately. For usage-limiting / enforcement a verdict only "
                    "counts once an independent structural signal agrees " b { "or" } " an admin confirms it — "
                    "session content is model-judged and gameable, so a wrong " i { "personal" } " must not throttle anyone on its own."
                }
                p {
                    b { "Two ways to run the judge." }
                    " Preferred: the endpoint agent's " code { "--triage" } " mode runs each employee's "
                    b { "own logged-in Claude Code" } " (" code { "claude -p" } ") — no CCGuard API key, uses the "
                    "company's existing Claude seat, and session content never leaves their machine. "
                    "Alternatively the " b { "Run" } " button below calls the Anthropic API from the server."
                }
                p {
                    "Server-side Anthropic API key: "
                    @if key_present { span.badge.work { "configured" } }
                    @else { span.badge.unknown { "not set" } " — only needed for the server-side Run button (set " code { "ANTHROPIC_API_KEY" } ", optionally " code { "ANTHROPIC_BASE_URL" } " for in-tenancy, and restart). The agent " code { "--triage" } " path needs none of this." }
                }
                p {
                    b { (unclassified) } " session(s) currently unclassified · verdicts so far: "
                    span.badge.work { "work " (tw) } " "
                    span.badge.personal { "personal " (tp) } " "
                    span.badge.unknown { "unsure " (tu) }
                }
                @if can_edit && cfg.enabled && key_present && unclassified > 0 {
                    form method="post" action="/dashboard/triage/run" style="display:inline" {
                        button type="submit" { "Run triage on next " (unclassified.min(25)) " session(s)" }
                    }
                } @else if can_edit && !cfg.enabled {
                    p.err { "Triage is disabled — enable it below to run." }
                }
            }
            @if can_edit {
                div.card {
                    h3 { "Your business — what the AI judges against" }
                    p { small style="color:var(--ink-3)" { "Plain English. This is the classifier: the AI reads each session against your description. Be concrete about what your work looks like in code." } }
                    form method="post" action="/dashboard/triage/draft" style="margin:8px 0" {
                        "Describe your business in one sentence — AI drafts the rest: " br;
                        input type="text" name="one_liner" style="width:60%" placeholder="e.g. I run a 3-person Shopify agency building client storefronts";
                        " " button type="submit" style="padding:6px 12px" { "Draft it" }
                    }
                    p { small { "…or start from a template (then edit): " }
                        @for t in ccguard_core::policy_template::all() {
                            " " a.badge.unknown href={"/dashboard/triage?template=" (t.key)} style="text-decoration:none" { (t.name) }
                        }
                    }
                    form method="post" action="/dashboard/triage/config" {
                        p {
                            label {
                                input type="checkbox" name="enabled" value="on" checked[cfg.enabled] style="width:auto;margin-right:8px";
                                "Enable AI classification for this org"
                            }
                        }
                        p { "What does your business do, and what does its real work look like in code? " br;
                            textarea name="business_desc" rows="4" style="width:100%" placeholder="e.g. We're a 3-person Shopify agency. Our work is building and maintaining client storefronts, custom theme code, and Shopify apps — often starting brand-new repos for new clients." { (cfg.business_desc) } }
                        p { "What is Claude Code allowed to be used for? " br;
                            textarea name="work_allowed" rows="3" style="width:100%" placeholder="e.g. Any client project, our internal tools and scripts, and learning/spikes for client work. NOT personal side-projects, job applications, or hobby code." { (cfg.work_allowed) } }
                        p { "(Optional) What is NOT your work? — examples only, never a hard rule " br;
                            textarea name="personal_examples" rows="2" style="width:100%" placeholder="e.g. a personal game, a portfolio site, leetcode practice" { (cfg.personal_examples) } }
                        details {
                            summary style="cursor:pointer;color:var(--ink-2)" { "Advanced — typed predicates (optional, authoritative)" }
                            p style="margin-top:8px" { b { "Structured policy (typed predicates — authoritative)" } }
                        p { "Work git / email domains " br;
                            input type="text" name="work_domains" value=(cfg.work_domains) style="width:100%" placeholder="acme.com, eng.acme.com, gitlab.acme.com"; }
                        p { "Work ticket prefixes " br;
                            input type="text" name="work_ticket_prefixes" value=(cfg.work_ticket_prefixes) style="width:100%" placeholder="ACME, BILL, PLAT"; }
                        p { "Approved work languages " br;
                            input type="text" name="approved_langs" value=(cfg.approved_langs) style="width:100%" placeholder="rust, go, typescript"; }
                            p { "Supplemental note (free text — NOT trusted for instructions) " br;
                                textarea name="work_definition" rows="3" style="width:100%;font:13px 'JetBrains Mono',monospace" placeholder="e.g. internal tooling and prototypes count as work." { (cfg.work_definition) } }
                        }
                        p { "Judge model " br;
                            input type="text" name="model" value=(cfg.model) style="width:320px"; }
                        p { button type="submit" { "Save business description" } }
                    }
                }
            }
            @if !trows.is_empty() {
                h3 { "Recent verdicts" }
                table {
                    thead { tr { th{"Session"} th{"Verdict"} th{"Conf."} th{"Enforceable"} th{"Reason"} th{} } }
                    tbody {
                        @for r in &trows {
                            @let sid: String = r.get("session_id");
                            @let label: String = r.get("label");
                            @let conf: f32 = r.get("confidence");
                            @let enf: bool = r.get("enforceable");
                            @let by: String = r.get("resolved_by");
                            @let reason: String = r.get("reason");
                            @let title: Option<String> = r.get("title");
                            @let org: Option<String> = r.get("repo_org");
                            @let name: Option<String> = r.get("repo_name");
                            @let mixed: bool = r.get("mixed");
                            @let flags: Vec<String> = r.get("gaming_flags");
                            tr {
                                td {
                                    a href={"/dashboard/sessions/" (sid)} {
                                        (title.clone().filter(|t| !t.is_empty())
                                            .unwrap_or_else(|| sid.chars().take(8).collect()))
                                    }
                                    @let repo = format!("{}/{}", org.clone().unwrap_or_default(), name.clone().unwrap_or_default());
                                    @if repo != "/" { br; small style="color:var(--ink-3)" { (repo) } }
                                }
                                td {
                                    span.badge.(triage_label_class(&label)) { (label) }
                                    @if mixed { " " span.badge.medium { "mixed" } }
                                    @if !flags.is_empty() { " " span.badge.high title="structural signal contradicts this label — review" { "⚑ contested" } }
                                }
                                td { (format!("{:.0}%", conf * 100.0)) }
                                td {
                                    @if enf { span.badge.work { "yes" } @if by == "human" { " (confirmed)" } }
                                    @else { span.badge.unknown { "no" } }
                                }
                                td { (reason) }
                                td {
                                    @if can_edit {
                                        @if !enf {
                                            form method="post" action={"/dashboard/triage/" (sid) "/confirm"} style="display:inline" {
                                                button type="submit" style="padding:4px 9px;font-size:12px" { "Confirm" }
                                            }
                                            " "
                                        }
                                        form method="post" action={"/dashboard/triage/" (sid) "/relabel"} style="display:inline" {
                                            select name="label" style="padding:3px 6px;font-size:12px" {
                                                option value="work" { "work" }
                                                option value="personal" { "personal" }
                                                option value="unsure" { "unsure" }
                                            }
                                            select name="reason" style="padding:3px 6px;font-size:12px" {
                                                option value="" { "why?" }
                                                option value="unfamiliar_name" { "unfamiliar name" }
                                                option value="internal_tooling" { "internal tooling" }
                                                option value="real_personal" { "real personal project" }
                                                option value="other" { "other" }
                                            }
                                            button type="submit" style="padding:4px 9px;font-size:12px;background:transparent;color:var(--accent-2);border:1px solid var(--line)" { "Relabel" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            @if !reasons.is_empty() {
                div.card {
                    h3 { "Top reasons the AI got it wrong" }
                    p { small style="color:var(--ink-3)" { "From your relabels — the fix is usually a sentence in your business description." } }
                    ul {
                        @for r in &reasons {
                            @let why: String = r.get("relabel_reason");
                            @let c: i64 = r.get("c");
                            li { (why.replace('_', " ")) " — " b { (c) } "×"
                                @if why == "internal_tooling" { " · try adding: " i { "\"Internal tools like our deploy scripts are company work.\"" } }
                                @if why == "unfamiliar_name" { " · try adding: " i { "\"New or unfamiliar repos/clients are still our work.\"" } }
                            }
                        }
                    }
                }
            }
            @if can_edit && !spot.is_empty() {
                div.card {
                    h3 { "Spot-check — random work-labeled sessions" }
                    p { small style="color:var(--ink-3)" { "A fresh random sample for a human look. Confirm if the AI got it right, or relabel. Keeps the AI honest (no one can predict which session you'll check)." } }
                    table {
                        thead { tr { th{"Session"} th{} } }
                        tbody {
                            @for r in &spot {
                                @let sid: String = r.get("session_id");
                                @let title: Option<String> = r.get("title");
                                @let org: Option<String> = r.get("repo_org");
                                @let name: Option<String> = r.get("repo_name");
                                tr {
                                    td {
                                        a href={"/dashboard/sessions/" (sid)} { (title.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| sid.chars().take(8).collect())) }
                                        @let repo = format!("{}/{}", org.clone().unwrap_or_default(), name.clone().unwrap_or_default());
                                        @if repo != "/" { " " small style="color:var(--ink-3)" { "(" (repo) ")" } }
                                    }
                                    td {
                                        form method="post" action={"/dashboard/triage/" (sid) "/confirm"} style="display:inline" {
                                            button type="submit" style="padding:4px 9px;font-size:12px" { "✓ correct (work)" }
                                        }
                                        " "
                                        form method="post" action={"/dashboard/triage/" (sid) "/relabel"} style="display:inline" {
                                            input type="hidden" name="label" value="personal";
                                            input type="hidden" name="reason" value="real_personal";
                                            button type="submit" style="padding:4px 9px;font-size:12px;background:transparent;color:var(--accent-2);border:1px solid var(--line)" { "actually personal" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}

/// Map a triage label to the CSS badge class (reuse the existing palette).
fn triage_label_class(label: &str) -> &'static str {
    match label {
        "work" => "work",
        "personal" => "personal",
        _ => "unknown",
    }
}

/// GET /dashboard/triage
pub async fn triage_page(
    user: WebUser,
    State(pool): State<PgPool>,
    Query(q): Query<HashMap<String, String>>,
) -> Html<String> {
    let prefill = q.get("template").and_then(|k| ccguard_core::policy_template::by_key(k)).map(|t| {
        (
            t.business_desc.to_string(),
            t.work_allowed.to_string(),
            t.personal_examples.to_string(),
        )
    });
    render_triage(&pool, &user, None, prefill).await
}

#[derive(Deserialize)]
pub struct DraftForm {
    pub one_liner: String,
}

/// POST /dashboard/triage/draft — AI-assisted drafting: expand the owner's one
/// sentence into the three policy fields (prefilled for them to edit + save). Needs
/// a server API key (the admin's browser can't run an employee's local Claude Code);
/// without one, point them at the templates. Owner/admin only.
pub async fn triage_draft(
    user: WebUser,
    State(pool): State<PgPool>,
    Form(f): Form<DraftForm>,
) -> Result<Response, AppError> {
    if user.role != "owner" && user.role != "admin" {
        return Err(AppError::Forbidden("owner or admin role required"));
    }
    if f.one_liner.trim().is_empty() {
        return Ok(Redirect::to("/dashboard/triage").into_response());
    }
    if !crate::triage_client::api_key_present() {
        let banner = html! { div.card style="border-color:#a96a00;background:#fef3df" {
            "AI drafting needs a server API key (" code { "ANTHROPIC_API_KEY" } "). Pick a starter template below instead — it's the same result in one click."
        }};
        return Ok(render_triage(&pool, &user, Some(banner), None).await.into_response());
    }
    let prompt = ccguard_core::policy_draft::draft_prompt(&f.one_liner);
    let client = reqwest::Client::new();
    let (banner, prefill) = match crate::triage_client::draft(&client, "claude-haiku-4-5", &prompt).await {
        Ok(text) => match ccguard_core::policy_draft::parse_draft(&text) {
            Some(triple) => (
                html! { div.card style="background:var(--ok-bg);border-color:rgba(63,217,154,.35)" { "Drafted from your description — review and edit below, then save." } },
                Some(triple),
            ),
            None => (html! { div.card.err { "Couldn't parse a draft — try a template." } }, None),
        },
        Err(e) => (
            html! { div.card.err { "Drafting failed: " (e.to_string()) " — try a template." } },
            None,
        ),
    };
    Ok(render_triage(&pool, &user, Some(banner), prefill).await.into_response())
}

#[derive(Deserialize)]
pub struct TriageConfigForm {
    #[serde(default)]
    pub enabled: Option<String>,
    #[serde(default)]
    pub work_definition: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub work_domains: String,
    #[serde(default)]
    pub work_ticket_prefixes: String,
    #[serde(default)]
    pub approved_langs: String,
    #[serde(default)]
    pub business_desc: String,
    #[serde(default)]
    pub work_allowed: String,
    #[serde(default)]
    pub personal_examples: String,
}

/// POST /dashboard/triage/config — owner/admin upsert of the business-description
/// policy. Bumps `policy_version` (verdicts/labels bind to the policy that produced
/// them) and re-enqueues the most recent sessions to re-drain under the new policy.
pub async fn triage_config_set(
    user: WebUser,
    State(pool): State<PgPool>,
    Form(f): Form<TriageConfigForm>,
) -> Result<Response, AppError> {
    if user.role != "owner" && user.role != "admin" {
        return Err(AppError::Forbidden("owner or admin role required"));
    }
    let enabled = f.enabled.as_deref() == Some("on");
    let model = if f.model.trim().is_empty() {
        ccguard_core::triage::DEFAULT_MODEL.to_string()
    } else {
        f.model.trim().to_string()
    };
    sqlx::query(
        "insert into tenant_triage_config \
         (tenant_id, enabled, work_definition, model, work_domains, work_ticket_prefixes, \
          approved_langs, business_desc, work_allowed, personal_examples, policy_version, updated_at) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,1, now()) \
         on conflict (tenant_id) do update set \
           enabled = excluded.enabled, work_definition = excluded.work_definition, \
           model = excluded.model, work_domains = excluded.work_domains, \
           work_ticket_prefixes = excluded.work_ticket_prefixes, \
           approved_langs = excluded.approved_langs, business_desc = excluded.business_desc, \
           work_allowed = excluded.work_allowed, personal_examples = excluded.personal_examples, \
           policy_version = tenant_triage_config.policy_version + 1, updated_at = now()",
    )
    .bind(&user.tenant_id)
    .bind(enabled)
    .bind(f.work_definition.trim())
    .bind(&model)
    .bind(f.work_domains.trim())
    .bind(f.work_ticket_prefixes.trim())
    .bind(f.approved_langs.trim())
    .bind(f.business_desc.trim())
    .bind(f.work_allowed.trim())
    .bind(f.personal_examples.trim())
    .execute(&pool)
    .await?;

    // Re-classify under the new policy: reset the most recent <=30 sessions to
    // 'pending' (bounded — never re-bills full history) so they re-drain.
    sqlx::query(
        "update captured_sessions set classification='pending' \
         where tenant_id=$1 and session_id in ( \
            select session_id from captured_sessions \
            where tenant_id=$1 and classification in ('work','personal','unknown') \
            order by last_ts desc nulls last limit 30)",
    )
    .bind(&user.tenant_id)
    .execute(&pool)
    .await?;
    sqlx::query(
        "update session_triage set next_retry_at = now() where tenant_id=$1 \
         and session_id in (select session_id from captured_sessions \
            where tenant_id=$1 and classification='pending')",
    )
    .bind(&user.tenant_id)
    .execute(&pool)
    .await?;

    Ok(Redirect::to("/dashboard/triage").into_response())
}

/// POST /dashboard/triage/run — sweep up to 25 unclassified sessions through the
/// judge, then re-render the page with a result banner. Owner/admin only.
pub async fn triage_run(user: WebUser, State(pool): State<PgPool>) -> Result<Response, AppError> {
    if user.role != "owner" && user.role != "admin" {
        return Err(AppError::Forbidden("owner or admin role required"));
    }
    let cfg = crate::handlers::triage::load_config(&pool, &user.tenant_id)
        .await
        .unwrap_or_default();
    if !cfg.enabled {
        return Err(AppError::BadRequest("triage is disabled for this org"));
    }
    let summary = crate::handlers::triage::run_unclassified(&pool, &user.tenant_id, &cfg, 25)
        .await
        .unwrap_or_default();

    let banner = html! {
        div.card style="border-color:var(--accent);background:var(--accent-wash)" {
            @if summary.attempted == 0 && summary.errors.is_empty() {
                p { "Nothing to triage — no unclassified sessions remaining." }
            } @else {
                p {
                    b { "Triaged " (summary.attempted) " session(s):" } " "
                    span.badge.work { "work " (summary.work) } " "
                    span.badge.personal { "personal " (summary.personal) } " "
                    span.badge.unknown { "unsure " (summary.unsure) } " "
                    span.badge.medium { "abstained " (summary.abstained) }
                }
            }
            @if !summary.errors.is_empty() {
                @for e in &summary.errors {
                    p.err { "⚠ " (e) }
                }
            }
        }
    };
    Ok(render_triage(&pool, &user, Some(banner), None).await.into_response())
}

/// POST /dashboard/triage/:session_id/confirm — a human confirms a verdict so it
/// counts toward enforcement / usage-limiting. Owner/admin only.
pub async fn triage_confirm(
    user: WebUser,
    State(pool): State<PgPool>,
    Path(session_id): Path<String>,
) -> Result<Response, AppError> {
    if user.role != "owner" && user.role != "admin" {
        return Err(AppError::Forbidden("owner or admin role required"));
    }
    // Confirm = the reviewer AGREES with the verdict → it counts for enforcement,
    // and is recorded as an independent human label equal to the model's label.
    sqlx::query(
        "update session_triage set enforceable = true, resolved_by = 'human', \
           human_reviewed = true, human_label = label, updated_at = now() \
         where tenant_id = $1 and session_id = $2",
    )
    .bind(&user.tenant_id)
    .bind(&session_id)
    .execute(&pool)
    .await?;
    Ok(Redirect::to("/dashboard/triage").into_response())
}

#[derive(Deserialize)]
pub struct RelabelForm {
    pub label: String, // work | personal | unsure
    #[serde(default)]
    pub reason: String, // why the AI was wrong (unfamiliar_name / internal_tooling / ...)
}

/// POST /dashboard/triage/:session_id/relabel — a reviewer DISAGREES and records
/// the correct label. This is the independent ground truth that makes the
/// conformal calibration and the precision gate non-circular. A personal relabel
/// is enforceable; anything else is visibility-only. Mirrors the human label onto
/// the session classification. Owner/admin only.
pub async fn triage_relabel(
    user: WebUser,
    State(pool): State<PgPool>,
    Path(session_id): Path<String>,
    Form(f): Form<RelabelForm>,
) -> Result<Response, AppError> {
    if user.role != "owner" && user.role != "admin" {
        return Err(AppError::Forbidden("owner or admin role required"));
    }
    let label = match f.label.as_str() {
        "work" | "personal" | "unsure" => f.label.as_str(),
        _ => return Err(AppError::BadRequest("label must be work, personal or unsure")),
    };
    // A human-confirmed personal is enforceable; otherwise not.
    let enforceable = label == "personal";
    let reason = (!f.reason.trim().is_empty()).then(|| f.reason.trim().to_string());
    sqlx::query(
        "update session_triage set human_reviewed = true, human_label = $3, \
           resolved_by = 'human', enforceable = $4, relabel_reason = $5, updated_at = now() \
         where tenant_id = $1 and session_id = $2",
    )
    .bind(&user.tenant_id)
    .bind(&session_id)
    .bind(label)
    .bind(enforceable)
    .bind(&reason)
    .execute(&pool)
    .await?;
    // Mirror the human's definite label onto the session (unsure → unclassified).
    let class = match label {
        "work" => "work",
        "personal" => "personal",
        _ => "unknown",
    };
    sqlx::query("update captured_sessions set classification = $3 where tenant_id=$1 and session_id=$2")
        .bind(&user.tenant_id)
        .bind(&session_id)
        .bind(class)
        .execute(&pool)
        .await?;
    Ok(Redirect::to("/dashboard/triage").into_response())
}

// ---- Enforcement arming (precision GO/NO-GO + conformal threshold) -----------

/// GET /dashboard/enforcement — the build-time precision gate and the conformal
/// judge threshold. Enforcement can only be armed after the gate reads GO; until
/// then it is observation-only. This page is the contestable record the dev's own
/// view reflects ("no rung armed against me").
pub async fn enforcement_page(user: WebUser, State(pool): State<PgPool>) -> Html<String> {
    let can_edit = user.role == "owner" || user.role == "admin";
    let report = crate::handlers::enforcement::load_report(&pool, &user.tenant_id)
        .await
        .unwrap_or_else(|_| ccguard_core::precision_gate::evaluate(&[], crate::handlers::enforcement::MIN_LABELS, crate::handlers::enforcement::MAX_FALSE_PERSONAL, crate::handlers::enforcement::MIN_PERSONAL_PREDICTIONS));
    let calib = crate::handlers::enforcement::load_calibration(&pool, &user.tenant_id)
        .await
        .unwrap_or(ccguard_core::conformal::Calibration { threshold: 1.01, n: 0, alpha: 0.1, usable: false });
    let arming = crate::handlers::enforcement::load_arming(&pool, &user.tenant_id)
        .await
        .unwrap_or(crate::handlers::enforcement::ArmingRow { armed: false, precision_go: false, n_labels: 0, false_personal_upper: 1.0, conformal_threshold: 1.01 });
    let go = report.decision == ccguard_core::precision_gate::GateDecision::Go;

    page(
        "Enforcement",
        html! {
            (nav())
            h1 { "Enforcement arming — precision gate" }
            div.card {
                p {
                    "Hard-block lives only in the " b { "off-device proxy" } " (on-device hooks can't reliably "
                    "block) and may be armed " b { "only" } " after PERSONAL-class precision clears a labeled "
                    "holdout — because throttling a developer whose session was actually work is the expensive "
                    "mistake. Until GO, everything is observation-only and nothing is armed."
                }
                p {
                    "The proxy then blocks only the START of a session a structural signal confirms personal "
                    "(never UNCLASSIFIED, never a single work signal, never a content-only label), only when the "
                    "seat is over allowance — with a warm, one-click-recoverable message. It "
                    b { "fails open" } " on any control-plane outage and " b { "fails closed" } " on an untested "
                    "Claude Code version."
                }
            }
            div.card {
                h3 { "Precision GO/NO-GO" }
                p {
                    "Independent human labels: " b { (report.n) }
                    " (need ≥ " (crate::handlers::enforcement::MIN_LABELS) ") · "
                    "PERSONAL precision: " b { (format!("{:.0}%", report.personal_precision * 100.0)) } " · "
                    "false-personal upper bound: " b { (format!("{:.1}%", report.false_personal_upper_ci * 100.0)) }
                    " (floor " (format!("{:.0}%", crate::handlers::enforcement::MAX_FALSE_PERSONAL * 100.0)) ")"
                }
                p {
                    "Decision: "
                    @if go { span.badge.work { "GO" } } @else { span.badge.high { "NO-GO" } }
                    @if !report.min_labels_met { " — insufficient labeled holdout (relabel verdicts on the Triage page to build it)" }
                    @else if !report.floor_met { " — false-personal rate above floor" }
                }
                h3 style="margin-top:16px" { "Conformal judge threshold" }
                p {
                    @if calib.usable {
                        "The judge abstains below " b { (format!("{:.0}%", calib.threshold.min(1.0) * 100.0)) }
                        " confidence (fit on " (calib.n) " labels)."
                        @if calib.threshold > 1.0 { " Currently abstaining on ALL verdicts (no threshold controls the error)." }
                    } @else {
                        "Not yet calibrated — need ≥ " (crate::handlers::enforcement::CONFORMAL_MIN_N)
                        " human labels; until then the judge runs uncalibrated (visibility only)."
                    }
                }
            }
            div.card style=(if arming.armed { "border-color:#b42318" } else { "" }) {
                h3 { "Arming status" }
                p {
                    "Enforcement is "
                    @if arming.armed { span.badge.high { "ARMED" } } @else { span.badge.work { "observation-only" } }
                    "."
                }
                @if can_edit {
                    form method="post" action="/dashboard/enforcement/recompute" style="display:inline" {
                        button type="submit" style="background:transparent;color:var(--accent-2);border:1px solid var(--line)" { "Recompute gate" }
                    }
                    " "
                    @if arming.armed {
                        form method="post" action="/dashboard/enforcement/disarm" style="display:inline" {
                            button type="submit" { "Disarm" }
                        }
                    } @else if go {
                        form method="post" action="/dashboard/enforcement/arm" style="display:inline" {
                            button type="submit" { "Arm enforcement (GO met)" }
                        }
                    } @else {
                        button disabled style="opacity:.5;cursor:not-allowed" { "Arm — blocked (NO-GO)" }
                    }
                }
            }
        },
    )
}

/// POST /dashboard/enforcement/recompute — refresh the gate + calibration.
pub async fn enforcement_recompute(user: WebUser, State(pool): State<PgPool>) -> Result<Response, AppError> {
    if user.role != "owner" && user.role != "admin" {
        return Err(AppError::Forbidden("owner or admin role required"));
    }
    let _ = crate::handlers::enforcement::recompute_and_store(&pool, &user.tenant_id).await?;
    Ok(Redirect::to("/dashboard/enforcement").into_response())
}

/// POST /dashboard/enforcement/arm — arm only if the gate reads GO.
pub async fn enforcement_arm(user: WebUser, State(pool): State<PgPool>) -> Result<Response, AppError> {
    if user.role != "owner" && user.role != "admin" {
        return Err(AppError::Forbidden("owner or admin role required"));
    }
    crate::handlers::enforcement::set_armed(&pool, &user.tenant_id, true).await?;
    Ok(Redirect::to("/dashboard/enforcement").into_response())
}

/// POST /dashboard/enforcement/disarm — always allowed.
pub async fn enforcement_disarm(user: WebUser, State(pool): State<PgPool>) -> Result<Response, AppError> {
    if user.role != "owner" && user.role != "admin" {
        return Err(AppError::Forbidden("owner or admin role required"));
    }
    crate::handlers::enforcement::set_armed(&pool, &user.tenant_id, false).await?;
    Ok(Redirect::to("/dashboard/enforcement").into_response())
}

/// Badge class for a provenance class string (reuse the existing palette).
fn provenance_class_badge(class: &str) -> &'static str {
    match class {
        "work" => "work",
        "work_provisional" => "medium",
        "personal" => "personal",
        _ => "unknown",
    }
}

// ---- Signals: provenance policy (what counts as corp) ------------------------

/// GET /dashboard/signals — configure the deterministic provenance cascade's
/// notion of "corp" (hosts, orgs, email domains, ticket prefixes, MDM env var,
/// registry patterns) plus the personal denylist. Owner/admin editable.
pub async fn signals_page(user: WebUser, State(pool): State<PgPool>) -> Html<String> {
    let can_edit = user.role == "owner" || user.role == "admin";

    // Corp hosts/orgs live in allowlist_rules (reused by the cascade).
    let arows = sqlx::query("select kind, value from allowlist_rules where tenant_id = $1")
        .bind(&user.tenant_id)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();
    let mut hosts: Vec<String> = Vec::new();
    let mut orgs: Vec<String> = Vec::new();
    for r in &arows {
        match r.get::<String, _>("kind").as_str() {
            "host" => hosts.push(r.get("value")),
            "org" => orgs.push(r.get("value")),
            _ => {}
        }
    }

    let p = sqlx::query(
        "select corp_email_domains, personal_orgs, personal_email_domains, ticket_prefixes, \
                corp_env_name, registry_patterns from provenance_policy where tenant_id = $1",
    )
    .bind(&user.tenant_id)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();
    let get = |col: &str| -> String { p.as_ref().map(|r| r.get::<String, _>(col)).unwrap_or_default() };
    let corp_env = {
        let v = get("corp_env_name");
        if v.is_empty() { "CCGUARD_CORP".to_string() } else { v }
    };

    // Verdict tallies across the cascade.
    let crows = sqlx::query(
        "select class, count(*) as c from session_provenance where tenant_id = $1 group by class",
    )
    .bind(&user.tenant_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    page(
        "Signals",
        html! {
            (nav())
            h1 { "Provenance signals — what counts as corp" }
            div.card {
                p {
                    "The primary classifier is a deterministic, content-free cascade. Only "
                    b { "ground-truth" } " signals (a real push to a corp org, or a cryptographically "
                    "signed commit by a corp identity) auto-resolve a session to " span.badge.work { "work" }
                    ". Dev-mutable hints (git-config email, registry fingerprints, monorepo, ticket "
                    "branch, MDM env var) are " b { "corroborators" } " → " span.badge.medium { "work provisional" }
                    ", never an auto-classify on their own. Nothing reads prompt or code content."
                }
                p {
                    "A session is only " span.badge.personal { "personal" } " with an affirmative personal "
                    "signal confirmed by two independent signals — so new / separate / remote-less work is "
                    b { "never silently flagged personal" } ". Anything undecided is "
                    span.badge.unknown { "unclassified" } " (terminal-safe) and flows to the "
                    a href="/dashboard/triage" { "LLM triage tier" } "."
                }
                @if !crows.is_empty() {
                    p {
                        "Verdicts so far: "
                        @for r in &crows {
                            @let cl: String = r.get("class");
                            @let c: i64 = r.get("c");
                            span.badge.(provenance_class_badge(&cl)) { (cl.replace('_', " ")) " " (c) } " "
                        }
                    }
                }
            }
            @if can_edit {
                div.card {
                    h3 { "Corporate definition" }
                    form method="post" action="/dashboard/signals/config" {
                        p { "Corp git hosts (one per line / comma) " br;
                            textarea name="corp_hosts" rows="2" style="width:100%;font:13px 'JetBrains Mono',monospace" placeholder="github.com&#10;gitlab.acme.com" { (hosts.join("\n")) } }
                        p { "Corp orgs / owners " br;
                            textarea name="corp_orgs" rows="2" style="width:100%;font:13px 'JetBrains Mono',monospace" placeholder="acme-corp&#10;acme-internal" { (orgs.join("\n")) } }
                        p { "Corp email domains (signed-commit identity) " br;
                            input type="text" name="corp_email_domains" value=(get("corp_email_domains")) style="width:100%" placeholder="acme.com, eng.acme.com"; }
                        p { "MDM-injected corp env var name (C-MDM-ENV) " br;
                            input type="text" name="corp_env_name" value=(corp_env) style="width:320px"; }
                        p { "Ticket key prefixes (branch / commit) " br;
                            input type="text" name="ticket_prefixes" value=(get("ticket_prefixes")) style="width:100%" placeholder="ACME, BILL, PLAT"; }
                        p { "Corp registry patterns (npm scope / host substrings) " br;
                            input type="text" name="registry_patterns" value=(get("registry_patterns")) style="width:100%" placeholder="@acme, artifactory.acme.com"; }
                        h3 style="margin-top:18px" { "Personal denylist (affirmative personal signals)" }
                        p { "Known-personal orgs / destinations " br;
                            input type="text" name="personal_orgs" value=(get("personal_orgs")) style="width:100%" placeholder="my-personal-gh, side-projects"; }
                        p { "Known-personal email domains (signed) " br;
                            input type="text" name="personal_email_domains" value=(get("personal_email_domains")) style="width:100%" placeholder="gmail.com, outlook.com"; }
                        p { button type="submit" { "Save provenance policy" } }
                    }
                }
            }
        },
    )
}

#[derive(Deserialize)]
pub struct SignalsForm {
    #[serde(default)] pub corp_hosts: String,
    #[serde(default)] pub corp_orgs: String,
    #[serde(default)] pub corp_email_domains: String,
    #[serde(default)] pub corp_env_name: String,
    #[serde(default)] pub ticket_prefixes: String,
    #[serde(default)] pub registry_patterns: String,
    #[serde(default)] pub personal_orgs: String,
    #[serde(default)] pub personal_email_domains: String,
}

/// Split a textarea/CSV field into trimmed tokens.
fn split_tokens(s: &str) -> Vec<String> {
    s.split([',', '\n', '\r', ';', ' '])
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(str::to_string)
        .collect()
}

/// POST /dashboard/signals/config — owner/admin upsert of the provenance policy.
pub async fn signals_config_set(
    user: WebUser,
    State(pool): State<PgPool>,
    Form(f): Form<SignalsForm>,
) -> Result<Response, AppError> {
    if user.role != "owner" && user.role != "admin" {
        return Err(AppError::Forbidden("owner or admin role required"));
    }

    // Replace corp host/org allowlist rows.
    sqlx::query("delete from allowlist_rules where tenant_id = $1 and kind in ('host','org')")
        .bind(&user.tenant_id)
        .execute(&pool)
        .await?;
    for h in split_tokens(&f.corp_hosts) {
        sqlx::query("insert into allowlist_rules (tenant_id, kind, value) values ($1,'host',$2)")
            .bind(&user.tenant_id)
            .bind(&h)
            .execute(&pool)
            .await?;
    }
    for o in split_tokens(&f.corp_orgs) {
        sqlx::query("insert into allowlist_rules (tenant_id, kind, value) values ($1,'org',$2)")
            .bind(&user.tenant_id)
            .bind(&o)
            .execute(&pool)
            .await?;
    }

    let corp_env = if f.corp_env_name.trim().is_empty() {
        "CCGUARD_CORP".to_string()
    } else {
        f.corp_env_name.trim().to_string()
    };
    sqlx::query(
        "insert into provenance_policy \
         (tenant_id, corp_email_domains, personal_orgs, personal_email_domains, \
          ticket_prefixes, corp_env_name, registry_patterns, updated_at) \
         values ($1,$2,$3,$4,$5,$6,$7, now()) \
         on conflict (tenant_id) do update set \
           corp_email_domains = excluded.corp_email_domains, \
           personal_orgs = excluded.personal_orgs, \
           personal_email_domains = excluded.personal_email_domains, \
           ticket_prefixes = excluded.ticket_prefixes, \
           corp_env_name = excluded.corp_env_name, \
           registry_patterns = excluded.registry_patterns, \
           updated_at = now()",
    )
    .bind(&user.tenant_id)
    .bind(f.corp_email_domains.trim())
    .bind(f.personal_orgs.trim())
    .bind(f.personal_email_domains.trim())
    .bind(f.ticket_prefixes.trim())
    .bind(&corp_env)
    .bind(f.registry_patterns.trim())
    .execute(&pool)
    .await?;

    Ok(Redirect::to("/dashboard/signals").into_response())
}

// ---- Usage: the Co-Owned Ledger (humane personal split, transparency only) ---

/// GET /dashboard/usage — the personal/work session-count split over a rolling
/// 7-day window. Cohort aggregate (reciprocal common-knowledge) + per-user RAW
/// counts (no per-individual personal-share %). Observation-only; no rung armed.
pub async fn usage_page(user: WebUser, State(pool): State<PgPool>) -> Html<String> {
    let can_edit = user.role == "owner" || user.role == "admin";

    // Config (allowance % + observation-since).
    let cfg = sqlx::query(
        "select personal_allowance_pct, armed, observation_since \
         from tenant_limit_config where tenant_id = $1",
    )
    .bind(&user.tenant_id)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();
    let allowance: i32 = cfg.as_ref().map(|r| r.get("personal_allowance_pct")).unwrap_or(20);
    let armed: bool = cfg.as_ref().map(|r| r.get("armed")).unwrap_or(false);
    // Admin's optional weekly token budget (their figure, not a scraped limit).
    let weekly_budget: i64 = sqlx::query_scalar::<_, Option<i64>>(
        "select weekly_token_budget from tenant_limit_config where tenant_id = $1",
    )
    .bind(&user.tenant_id)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten()
    .flatten()
    .unwrap_or(0);

    // Measured-token split (work/personal/unknown) over the rolling 7 days. Tokens
    // come straight from the transcripts (tokens_in+tokens_out per event); summed by
    // the session's class. A second lens beside the session-count ledger — labeled
    // "measured tokens", never dollars and never the opaque account limit.
    let tok_rows = sqlx::query(
        "select coalesce(cs.classification,'unknown') as class, \
                coalesce(sum(e.tokens_in + e.tokens_out),0)::bigint as toks \
         from captured_sessions cs \
         join captured_events e on e.tenant_id=cs.tenant_id and e.session_id=cs.session_id \
         where cs.tenant_id=$1 and coalesce(cs.last_ts, cs.created_at) >= now() - interval '7 days' \
         group by coalesce(cs.classification,'unknown')",
    )
    .bind(&user.tenant_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();
    let mut tok_work: i64 = 0;
    let mut tok_personal: i64 = 0;
    let mut tok_unknown: i64 = 0;
    for r in &tok_rows {
        let c: String = r.get("class");
        let t: i64 = r.get("toks");
        match c.as_str() {
            "work" => tok_work = t,
            "personal" => tok_personal = t,
            _ => tok_unknown += t,
        }
    }
    let tok_total = tok_work + tok_personal + tok_unknown;
    let pct = |n: i64| -> i64 {
        if tok_total > 0 {
            (n as f64 / tok_total as f64 * 100.0).round() as i64
        } else {
            0
        }
    };
    let budget_used_pct: i64 = if weekly_budget > 0 {
        (tok_total as f64 / weekly_budget as f64 * 100.0).round() as i64
    } else {
        0
    };

    // Per-user token totals (work vs personal), rolling 7 days.
    let tok_user_rows = sqlx::query(
        "select cs.user_email, \
           coalesce(sum((e.tokens_in+e.tokens_out)) filter (where cs.classification='work'),0)::bigint as work, \
           coalesce(sum((e.tokens_in+e.tokens_out)) filter (where cs.classification='personal'),0)::bigint as personal, \
           coalesce(sum((e.tokens_in+e.tokens_out)) filter (where coalesce(cs.classification,'unknown') not in ('work','personal')),0)::bigint as other \
         from captured_sessions cs \
         join captured_events e on e.tenant_id=cs.tenant_id and e.session_id=cs.session_id \
         where cs.tenant_id=$1 and coalesce(cs.last_ts, cs.created_at) >= now() - interval '7 days' \
         group by cs.user_email order by (coalesce(sum(e.tokens_in+e.tokens_out),0)) desc",
    )
    .bind(&user.tenant_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();
    let fmt_k = |n: i64| -> String {
        if n >= 1_000_000 {
            format!("{:.1}M", n as f64 / 1_000_000.0)
        } else if n >= 1_000 {
            format!("{:.0}k", n as f64 / 1_000.0)
        } else {
            n.to_string()
        }
    };
    let since = cfg
        .as_ref()
        .map(|r| r.get::<chrono::DateTime<Utc>, _>("observation_since"))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "(not yet configured)".to_string());

    // Confirmed-personal predicate: structural personal OR a human-confirmed verdict.
    // An LLM-only "personal" is NOT confirmed and is excluded from the meter.
    let confirmed_personal = "(sp.class='personal' or (st.enforceable and st.label='personal'))";
    let cohort_sql = format!(
        "select \
           count(*) filter (where cs.classification='work') as work, \
           count(*) filter (where cs.classification='personal' and {cp}) as personal_confirmed, \
           count(*) filter (where cs.classification='unknown' \
                or (cs.classification='personal' and not {cp})) as excluded \
         from captured_sessions cs \
         left join session_provenance sp on sp.tenant_id=cs.tenant_id and sp.session_id=cs.session_id \
         left join session_triage st on st.tenant_id=cs.tenant_id and st.session_id=cs.session_id \
         where cs.tenant_id=$1 and coalesce(cs.last_ts, cs.created_at) >= now() - interval '7 days'",
        cp = confirmed_personal
    );
    let row = sqlx::query(&cohort_sql)
        .bind(&user.tenant_id)
        .fetch_one(&pool)
        .await
        .ok();
    let (work, personal, excluded) = match &row {
        Some(r) => (
            r.get::<i64, _>("work") as u32,
            r.get::<i64, _>("personal_confirmed") as u32,
            r.get::<i64, _>("excluded") as u32,
        ),
        None => (0, 0, 0),
    };
    let s = ccguard_core::ledger::split(
        &ccguard_core::ledger::UsageCounts { work, personal_confirmed: personal, unclassified: excluded },
        allowance.max(0) as u32,
    );

    // Per-user RAW counts (no per-individual personal-share %), rolling 7 days.
    let per_user_sql = format!(
        "select cs.user_email, \
           count(*) filter (where cs.classification='work') as work, \
           count(*) filter (where cs.classification='personal' and {cp}) as personal_confirmed, \
           count(*) filter (where cs.classification='unknown' \
                or (cs.classification='personal' and not {cp})) as excluded \
         from captured_sessions cs \
         left join session_provenance sp on sp.tenant_id=cs.tenant_id and sp.session_id=cs.session_id \
         left join session_triage st on st.tenant_id=cs.tenant_id and st.session_id=cs.session_id \
         where cs.tenant_id=$1 and coalesce(cs.last_ts, cs.created_at) >= now() - interval '7 days' \
         group by cs.user_email order by personal_confirmed desc, work desc",
        cp = confirmed_personal
    );
    let urows = sqlx::query(&per_user_sql)
        .bind(&user.tenant_id)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    page(
        "Usage",
        html! {
            (nav())
            h1 { "Usage split — personal vs work" }
            div.card style="background:var(--accent-wash);border-color:var(--accent)" {
                p {
                    b { "Observation-only since " (since) "." } " No enforcement rung is armed against anyone. "
                    "v1 is transparency + a contestable ledger — limiting is not a v1 capability."
                    @if armed { " " span.badge.high { "ARMED" } }
                }
                p style="margin:0" {
                    small style="color:var(--ink-2)" {
                        "Split is by " b { "session count" } " over a rolling 7 days — an "
                        i { "estimated split, not billed dollars" } " (Claude Code's token logs undercount "
                        "100–174×, so a dollar meter would fire on fiction). UNCLASSIFIED sessions are "
                        b { "excluded" } " from the split entirely."
                    }
                }
            }
            div.card {
                h3 { "Cohort aggregate (reciprocal — the same number every dev sees)" }
                p {
                    span.badge.work { "work " (s.work) } " "
                    span.badge.personal { "personal (confirmed) " (s.personal) } " "
                    span.badge.unknown { "excluded " (s.unclassified_excluded) }
                }
                p {
                    "Personal share: " b { (s.personal_share_pct) "%" }
                    " of " (s.denominator) " classified sessions · allowance "
                    b { (s.allowance_pct) "%" } " · "
                    @if s.over_allowance {
                        span.badge.high { "over by " (-s.headroom_pct) "%" }
                    } @else {
                        span.badge.work { (s.headroom_pct) "% headroom" }
                    }
                }
                @if s.denominator == 0 {
                    p.finding { "No classified work/personal sessions in the last 7 days." }
                }
            }
            div.card {
                h3 { "Measured tokens — work vs personal (7d)" }
                p { small style="color:var(--ink-3)" {
                    "A second lens: actual tokens from the transcripts (input+output), summed by "
                    "the session's class. This is " i { "measured token volume" } ", not dollars and "
                    "not a % of your account's limit (that limit isn't cleanly readable, and these "
                    "tokens aren't the unit it counts in). Cache tokens aren't included."
                } }
                p {
                    span.badge.work { "work " (fmt_k(tok_work)) " (" (pct(tok_work)) "%)" } " "
                    span.badge.personal { "personal " (fmt_k(tok_personal)) " (" (pct(tok_personal)) "%)" } " "
                    span.badge.unknown { "unclassified " (fmt_k(tok_unknown)) " (" (pct(tok_unknown)) "%)" }
                }
                p { "Total measured this week: " b { (fmt_k(tok_total)) } " tokens"
                    @if weekly_budget > 0 {
                        " · against your set budget of " b { (fmt_k(weekly_budget)) } " → "
                        @if budget_used_pct >= 100 {
                            span.badge.high { (budget_used_pct) "% used" }
                        } @else {
                            span.badge.work { (budget_used_pct) "% used" }
                        }
                    }
                }
                @if tok_total == 0 {
                    p.finding { "No token-bearing events in the last 7 days." }
                }
                table {
                    thead { tr { th{"Developer"} th{"Work"} th{"Personal"} th{"Other"} } }
                    tbody {
                        @for r in &tok_user_rows {
                            @let em: String = r.get("user_email");
                            @let w: i64 = r.get("work");
                            @let p: i64 = r.get("personal");
                            @let o: i64 = r.get("other");
                            tr {
                                td { (em) }
                                td { (fmt_k(w)) }
                                td { (fmt_k(p)) }
                                td { (fmt_k(o)) }
                            }
                        }
                    }
                }
            }
            div.card {
                h3 { "Per developer (raw counts)" }
                p { small style="color:var(--ink-3)" {
                    "Managers see raw counts only — never a per-individual personal-share % — until "
                    "PERSONAL-class precision clears the contractual floor on a labeled holdout. "
                    "Unclassified is shown as excluded, never imputed."
                } }
                table {
                    thead { tr { th{"Developer"} th{"Work"} th{"Personal (confirmed)"} th{"Excluded"} } }
                    tbody {
                        @for r in &urows {
                            @let em: String = r.get("user_email");
                            @let w: i64 = r.get("work");
                            @let pc: i64 = r.get("personal_confirmed");
                            @let ex: i64 = r.get("excluded");
                            tr {
                                td { (em) }
                                td { (w) }
                                td { (pc) }
                                td { (ex) }
                            }
                        }
                    }
                }
            }
            @if can_edit {
                div.card {
                    h3 { "Settings" }
                    form method="post" action="/dashboard/usage/config" {
                        p { "Personal allowance (% of classified " b { "sessions" } ", not spend) " br;
                            input type="number" name="personal_allowance_pct" min="0" max="100" value=(allowance) style="width:120px"; }
                        p { "Weekly token budget (your plan's rough weekly allowance — 0 to hide) " br;
                            input type="number" name="weekly_token_budget" min="0" value=(weekly_budget) style="width:200px";
                            " " small style="color:var(--ink-3)" { "your figure, not a scraped limit" } }
                        p { button type="submit" { "Save" } }
                    }
                }
            }
        },
    )
}

#[derive(Deserialize)]
pub struct UsageForm {
    #[serde(default)]
    pub personal_allowance_pct: Option<i32>,
    #[serde(default)]
    pub weekly_token_budget: Option<i64>,
}

/// POST /dashboard/usage/config — owner/admin set the personal allowance % and the
/// optional weekly token budget. `armed` stays false in v1 (transparency only);
/// observation_since is preserved.
pub async fn usage_config_set(
    user: WebUser,
    State(pool): State<PgPool>,
    Form(f): Form<UsageForm>,
) -> Result<Response, AppError> {
    if user.role != "owner" && user.role != "admin" {
        return Err(AppError::Forbidden("owner or admin role required"));
    }
    let pct = f.personal_allowance_pct.unwrap_or(20).clamp(0, 100);
    let budget = f.weekly_token_budget.unwrap_or(0).max(0);
    sqlx::query(
        "insert into tenant_limit_config (tenant_id, personal_allowance_pct, weekly_token_budget, armed, observation_since, updated_at) \
         values ($1,$2,$3,false, now(), now()) \
         on conflict (tenant_id) do update set \
           personal_allowance_pct = excluded.personal_allowance_pct, \
           weekly_token_budget = excluded.weekly_token_budget, updated_at = now()",
    )
    .bind(&user.tenant_id)
    .bind(pct)
    .bind(budget)
    .execute(&pool)
    .await?;
    Ok(Redirect::to("/dashboard/usage").into_response())
}

fn yesno(b: Option<bool>) -> &'static str {
    match b {
        Some(true) => "yes",
        Some(false) => "no",
        None => "—",
    }
}

/// People roster: one row per seat (user_email) with their session split, role,
/// assignment, and open-flag count. Each links to the per-person detail.
pub async fn people(user: WebUser, State(pool): State<PgPool>) -> Html<String> {
    let rows = sqlx::query(
        "select cs.user_email as em, count(*) as sessions, \
           count(*) filter (where cs.classification='work') as work, \
           count(*) filter (where cs.classification='personal') as personal, \
           count(*) filter (where cs.classification not in ('work','personal')) as other, \
           coalesce((select count(*) from indicators i \
             where i.tenant_id=cs.tenant_id and i.user_email=cs.user_email and i.status='open'),0) as flags, \
           (select job_role from employee_roles r where r.tenant_id=cs.tenant_id and r.user_email=cs.user_email) as role, \
           (select assignment from employee_roles r where r.tenant_id=cs.tenant_id and r.user_email=cs.user_email) as assignment \
         from captured_sessions cs where cs.tenant_id=$1 \
         group by cs.user_email, cs.tenant_id order by sessions desc",
    )
    .bind(&user.tenant_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    page(
        "People",
        html! {
            (nav())
            h1 { "People — usage by person" }
            p.muted style="margin-bottom:2px" { "One seat per row. Open a person to see every session, device, finding and flag in one place." }
            table {
                thead { tr { th{"Person"} th{"Role"} th{"Assigned to"} th{"Sessions"} th{"Work"} th{"Personal"} th{"Other"} th{"Open flags"} } }
                tbody {
                    @for r in &rows {
                        @let em: String = r.get("em");
                        @let role: Option<String> = r.get("role");
                        @let asg: Option<String> = r.get("assignment");
                        @let s: i64 = r.get("sessions");
                        @let w: i64 = r.get("work");
                        @let p: i64 = r.get("personal");
                        @let o: i64 = r.get("other");
                        @let f: i64 = r.get("flags");
                        tr {
                            td { a href={"/dashboard/people/" (em)} { (em) } }
                            td { (role.unwrap_or_else(|| "—".into())) }
                            td.muted { (asg.unwrap_or_else(|| "—".into())) }
                            td { (s) }
                            td { @if w>0 { span.badge.work { (w) } } @else { "0" } }
                            td { @if p>0 { span.badge.personal { (p) } } @else { "0" } }
                            td { (o) }
                            td { @if f>0 { span.badge.off_task { (f) } } @else { "0" } }
                        }
                    }
                }
            }
        },
    )
}

/// Per-person detail: everything about one seat in one place — role + assignment,
/// enrolled device(s) + compliance, session/token/on-task KPIs, recent sessions,
/// open flags. Keyed by user_email.
pub async fn person(
    user: WebUser,
    State(pool): State<PgPool>,
    Path(email): Path<String>,
) -> Html<String> {
    let rl = sqlx::query("select job_role, assignment from employee_roles where tenant_id=$1 and user_email=$2")
        .bind(&user.tenant_id).bind(&email).fetch_optional(&pool).await.ok().flatten();
    let role = rl.as_ref().map(|r| r.get::<String, _>("job_role")).unwrap_or_default();
    let assignment = rl.as_ref().and_then(|r| r.get::<Option<String>, _>("assignment")).unwrap_or_default();

    let agg = sqlx::query(
        "select count(*) as sessions, count(*) filter (where classification='work') as work, \
           count(*) filter (where classification='personal') as personal, \
           count(*) filter (where classification not in ('work','personal')) as other, \
           coalesce(sum(event_count),0)::bigint as events \
         from captured_sessions where tenant_id=$1 and user_email=$2")
        .bind(&user.tenant_id).bind(&email).fetch_one(&pool).await.ok();
    let (sessions, work, personal, other, events) = match &agg {
        Some(r) => (r.get::<i64, _>("sessions"), r.get::<i64, _>("work"), r.get::<i64, _>("personal"), r.get::<i64, _>("other"), r.get::<i64, _>("events")),
        None => (0, 0, 0, 0, 0),
    };

    let tok = sqlx::query(
        "select coalesce(sum((e.tokens_in+e.tokens_out)) filter (where cs.classification='work'),0)::bigint as work, \
           coalesce(sum((e.tokens_in+e.tokens_out)) filter (where cs.classification='personal'),0)::bigint as personal, \
           coalesce(sum(e.tokens_in+e.tokens_out),0)::bigint as total \
         from captured_events e join captured_sessions cs on cs.tenant_id=e.tenant_id and cs.session_id=e.session_id \
         where cs.tenant_id=$1 and cs.user_email=$2")
        .bind(&user.tenant_id).bind(&email).fetch_one(&pool).await.ok();
    let (tw, tp, tt) = match &tok {
        Some(r) => (r.get::<i64, _>("work"), r.get::<i64, _>("personal"), r.get::<i64, _>("total")),
        None => (0, 0, 0),
    };

    let ot = sqlx::query(
        "select count(*) as scored, count(*) filter (where sc.label='on_task') as ontask \
         from session_scores sc join captured_sessions cs on cs.tenant_id=sc.tenant_id and cs.session_id=sc.session_id \
         where cs.tenant_id=$1 and cs.user_email=$2")
        .bind(&user.tenant_id).bind(&email).fetch_one(&pool).await.ok();
    let (scored, ontask) = match &ot { Some(r) => (r.get::<i64, _>("scored"), r.get::<i64, _>("ontask")), None => (0, 0) };
    let ontask_pct = if scored > 0 { (ontask as f64 / scored as f64 * 100.0).round() as i64 } else { 0 };

    let findings_n = sqlx::query_scalar::<_, i64>(
        "select count(*) from findings f join captured_sessions cs on cs.tenant_id=f.tenant_id and cs.session_id=f.session_id \
         where cs.tenant_id=$1 and cs.user_email=$2")
        .bind(&user.tenant_id).bind(&email).fetch_one(&pool).await.unwrap_or(0);

    let devs = sqlx::query(
        "select device_id, hostname, os, compliance, last_seen, policy_present, login_locked, personal_account \
         from devices where tenant_id=$1 and user_email=$2 order by last_seen desc nulls last")
        .bind(&user.tenant_id).bind(&email).fetch_all(&pool).await.unwrap_or_default();

    let sess = sqlx::query(
        "select cs.session_id, cs.title, cs.classification, cs.event_count, sc.label as ontask, st.off_assignment \
         from captured_sessions cs \
         left join session_scores sc on sc.tenant_id=cs.tenant_id and sc.session_id=cs.session_id \
         left join session_triage st on st.tenant_id=cs.tenant_id and st.session_id=cs.session_id \
         where cs.tenant_id=$1 and cs.user_email=$2 order by cs.last_ts desc nulls last limit 25")
        .bind(&user.tenant_id).bind(&email).fetch_all(&pool).await.unwrap_or_default();

    let inds = sqlx::query(
        "select kind, detail from indicators where tenant_id=$1 and user_email=$2 and status='open' order by created_at desc limit 30")
        .bind(&user.tenant_id).bind(&email).fetch_all(&pool).await.unwrap_or_default();

    let fmtk = |n: i64| -> String {
        if n >= 1_000_000 { format!("{:.1}M", n as f64 / 1e6) }
        else if n >= 1_000 { format!("{:.0}k", n as f64 / 1e3) }
        else { n.to_string() }
    };

    page(
        "Person",
        html! {
            (nav())
            div.pagehead {
                h1 style="margin-bottom:4px" { (email) }
                a.muted href="/dashboard/people" { "← all people" }
            }
            p.muted style="margin-bottom:8px" {
                "Role: " b { (if role.is_empty() { "—".to_string() } else { role.clone() }) }
                "  ·  Assigned to: " b { (if assignment.is_empty() { "—".to_string() } else { assignment.clone() }) }
                "  ·  " a href="/dashboard/roles" { "edit" }
            }
            div.kpis {
                div.kpi { div.lab{"Sessions"} div.big{ (sessions) } }
                div.kpi { div.lab{"Work"} div.big.gd{ (work) } }
                div.kpi { div.lab{"Personal"} div.big.bd{ (personal) } }
                div.kpi { div.lab{"Other"} div.big{ (other) } }
                div.kpi { div.lab{"On-task"} div.big.az{ (ontask_pct) "%" } }
                div.kpi { div.lab{"Findings"} div.big{ (findings_n) } }
                div.kpi { div.lab{"Events"} div.big{ (fmtk(events)) } }
            }
            div.kpis {
                div.kpi { div.lab{"Tokens · total"} div.big{ (fmtk(tt)) } }
                div.kpi { div.lab{"Tokens · work"} div.big.gd{ (fmtk(tw)) } }
                div.kpi { div.lab{"Tokens · personal"} div.big.bd{ (fmtk(tp)) } }
            }
            div.card {
                h3 { "Device(s)" }
                @if devs.is_empty() {
                    p.muted { "No enrolled device reporting for this seat yet." }
                } @else {
                    table {
                        thead { tr { th{"Hostname"} th{"OS"} th{"Compliance"} th{"Policy"} th{"Login lock"} th{"Personal acct"} th{"Last seen"} } }
                        tbody {
                            @for d in &devs {
                                @let host: Option<String> = d.get("hostname");
                                @let os: Option<String> = d.get("os");
                                @let comp: Option<String> = d.get("compliance");
                                @let pol: Option<bool> = d.get("policy_present");
                                @let lock: Option<bool> = d.get("login_locked");
                                @let pers: Option<bool> = d.get("personal_account");
                                @let ls: Option<chrono::DateTime<Utc>> = d.get("last_seen");
                                @let c = comp.clone().unwrap_or_else(|| "unknown".into());
                                tr {
                                    td { (host.unwrap_or_else(|| "—".into())) }
                                    td { (os.unwrap_or_else(|| "—".into())) }
                                    td { span.badge.(c) { (c) } }
                                    td { (yesno(pol)) }
                                    td { (yesno(lock)) }
                                    td { (yesno(pers)) }
                                    td.muted { (ls.map(|t| t.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| "—".into())) }
                                }
                            }
                        }
                    }
                }
            }
            h3 style="margin-top:22px" { "Recent sessions" }
            table {
                thead { tr { th{"Session"} th{"Class"} th{"On-task"} th{"Events"} } }
                tbody {
                    @for r in &sess {
                        @let sid: String = r.get("session_id");
                        @let title: Option<String> = r.get("title");
                        @let class: Option<String> = r.get("classification");
                        @let otl: Option<String> = r.get("ontask");
                        @let offa: Option<bool> = r.get("off_assignment");
                        @let cl = class.unwrap_or_else(|| "unknown".into());
                        tr {
                            td { a href={"/dashboard/sessions/" (sid)} { (title.unwrap_or_else(|| sid.chars().take(8).collect::<String>())) } }
                            td { span.badge.(cl.clone()) { (cl) } @if offa == Some(true) { " " span.badge.off_assignment { "off-assignment" } } }
                            td { @if let Some(l) = otl { span.badge.(l.clone()) { (l) } } @else { "—" } }
                            td { (r.get::<i32, _>("event_count")) }
                        }
                    }
                }
            }
            @if !inds.is_empty() {
                h3 style="margin-top:22px" { "Open flags" }
                table {
                    thead { tr { th{"Kind"} th{"Detail"} } }
                    tbody {
                        @for i in &inds {
                            @let k: String = i.get("kind");
                            @let det: String = i.get("detail");
                            tr { td { span.badge.(k.clone()) { (k.replace('_', " ")) } } td.muted { (det) } }
                        }
                    }
                }
            }
        },
    )
}
