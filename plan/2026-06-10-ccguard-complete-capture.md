# CCGuard Complete-Capture Pipeline — Implementation Plan (Plan 5 of N)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use `- [ ]`.

**Goal:** Turn CCGuard from "token donut" into "captures the complete record of what an employee did in Claude Code." The agent parses the FULL transcript (prompts, responses, thinking, tool calls + results, file edits, subagents, PRs, identity), posts it; the server stores it as a Session→Event→ContentBlob model; a retrieval endpoint returns the full replayable session timeline.

**Architecture:** New `ccguard-core::capture` types (`CapturedSession`, `CapturedEvent`, `EventKind`). New `ccguard-agent::transcript` full parser + `.claude.json` identity reader + `--capture` mode posting `CapturedSession`s to a new `POST /v1/capture` (ingest-token auth → tenant; repo reclassified; content sha256-deduped into `content_blobs`). Retrieval: `GET /v1/orgs/:tenant/sessions` (list) and `GET /v1/sessions/:session_id/timeline` (full ordered events + content), both `AuthedUser` + same-tenant. The existing `/v1/events` donut path is untouched.

**Tech Stack:** Rust (existing workspace), sqlx (Postgres), sha2 (already a server dep from Plan 2), serde, chrono. `DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres`. Commit identity `senthilguru246@gmail.com` / `SenGuru`.

**Reference for the parser:** the exact Claude Code transcript JSONL schema is in `research/total-visibility.md` §1 and `research/tracking-surface.md` §2; there is also a REAL `~/.claude/projects/**/*.jsonl` on this machine to inspect for exact field names. Read those before writing the parser.

---

## Roadmap position
Plans 1–4 ✅ (engine, auth, user accounts, basic agent). **Plan 5 ← this (complete capture + retrieval).** Then: Plan 6 session-replay UI · Plan 7 search/eDiscovery/findings · Plan 8 managed-settings enforcement · Plan 9 on-task score.

## Prerequisites
- [ ] Postgres reachable; `DATABASE_URL` set. Plans 1–4 on `master`.

---

## Task 1: `ccguard-core::capture` types

**Files:** Create `crates/ccguard-core/src/capture.rs`; Modify `crates/ccguard-core/src/lib.rs` (add `pub mod capture;`).

- [ ] **Step 1: Create `crates/ccguard-core/src/capture.rs`:**
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::event::Repo;

/// The typed kind of one activity atom in a Claude Code session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    UserPrompt,
    AssistantText,
    Thinking,
    ToolCall,
    ToolResult,
    FileEdit,
    BashCommand,
    Pr,
}

impl EventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventKind::UserPrompt => "user_prompt",
            EventKind::AssistantText => "assistant_text",
            EventKind::Thinking => "thinking",
            EventKind::ToolCall => "tool_call",
            EventKind::ToolResult => "tool_result",
            EventKind::FileEdit => "file_edit",
            EventKind::BashCommand => "bash_command",
            EventKind::Pr => "pr",
        }
    }
}

/// One captured activity atom. `content` is the verbatim text/diff/command/output (becomes a deduped blob).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapturedEvent {
    pub seq: i64,
    pub ts: DateTime<Utc>,
    pub kind: EventKind,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tokens_in: i64,
    #[serde(default)]
    pub tokens_out: i64,
    #[serde(default)]
    pub is_sidechain: bool,
}

/// A full session capture: metadata + ordered events. Posted as one batch to /v1/capture.
/// `tenant_id` is set by the server from the ingest token (not the body).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapturedSession {
    pub session_id: String,
    pub user_email: String,
    pub repo: Repo,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    pub events: Vec<CapturedEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_and_kind_snake_case() {
        let json = r#"{
            "session_id":"s1","user_email":"dev@acme.com",
            "repo":{"host":"github.com","org":"acme-corp","name":"r"},
            "title":"Build the thing","cwd":"C:\\w\\r",
            "events":[
              {"seq":0,"ts":"2026-06-10T10:00:00Z","kind":"user_prompt","content":"do X"},
              {"seq":1,"ts":"2026-06-10T10:00:01Z","kind":"tool_call","tool_name":"Bash","target":"git status","content":"{\"command\":\"git status\"}"}
            ]
        }"#;
        let s: CapturedSession = serde_json::from_str(json).unwrap();
        assert_eq!(s.session_id, "s1");
        assert_eq!(s.events.len(), 2);
        assert_eq!(s.events[0].kind, EventKind::UserPrompt);
        assert_eq!(s.events[1].tool_name.as_deref(), Some("Bash"));
        assert_eq!(EventKind::ToolResult.as_str(), "tool_result");
        // serde uses snake_case:
        assert!(serde_json::to_string(&EventKind::UserPrompt).unwrap().contains("user_prompt"));
    }
}
```

- [ ] **Step 2:** add `pub mod capture;` to `crates/ccguard-core/src/lib.rs`.

- [ ] **Step 3: Run + commit.** `cargo test -p ccguard-core capture::` → 1 passed.
```
git add crates/ccguard-core
git commit -m "feat(core): CapturedSession/CapturedEvent capture types"
```

---

## Task 2: capture storage + `POST /v1/capture`

**Files:** Create `crates/ccguard-server/migrations/0004_capture.sql`, `crates/ccguard-server/src/handlers/capture.rs`, `crates/ccguard-server/tests/capture.rs`; Modify `handlers/mod.rs`, `app.rs`.

- [ ] **Step 1: Migration** `crates/ccguard-server/migrations/0004_capture.sql`:
```sql
create table if not exists captured_sessions (
    id             bigserial primary key,
    tenant_id      text not null references tenants(id),
    session_id     text not null,
    user_email     text not null,
    repo_host      text, repo_org text, repo_name text, repo_path text,
    classification text not null,
    title          text,
    cwd            text,
    first_ts       timestamptz,
    last_ts        timestamptz,
    event_count    integer not null default 0,
    created_at     timestamptz not null default now(),
    unique (tenant_id, session_id)
);

create table if not exists content_blobs (
    id         bigserial primary key,
    tenant_id  text not null references tenants(id),
    sha256     text not null,
    content    text not null,
    bytes      integer not null,
    created_at timestamptz not null default now(),
    unique (tenant_id, sha256)
);

create table if not exists captured_events (
    id           bigserial primary key,
    tenant_id    text not null references tenants(id),
    session_id   text not null,
    seq          bigint not null,
    ts           timestamptz not null,
    kind         text not null,
    model        text,
    tool_name    text,
    target       text,
    content_sha  text,
    tokens_in    bigint not null default 0,
    tokens_out   bigint not null default 0,
    is_sidechain boolean not null default false,
    unique (tenant_id, session_id, seq)
);
create index if not exists captured_events_session on captured_events (tenant_id, session_id, seq);
create index if not exists captured_sessions_tenant_ts on captured_sessions (tenant_id, last_ts);
```

- [ ] **Step 2: Handler** `crates/ccguard-server/src/handlers/capture.rs`:
```rust
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use ccguard_core::capture::CapturedSession;
use ccguard_core::classify::{classify, Allowlist};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

use crate::auth::AuthedTenant;
use crate::error::AppError;

async fn load_allowlist(pool: &PgPool, tenant_id: &str) -> Result<Allowlist, sqlx::Error> {
    let rows = sqlx::query("select kind, value from allowlist_rules where tenant_id = $1")
        .bind(tenant_id).fetch_all(pool).await?;
    let mut a = Allowlist::default();
    for r in rows {
        let kind: String = r.get("kind");
        let value: String = r.get("value");
        match kind.as_str() { "host" => a.hosts.push(value), "org" => a.orgs.push(value), "path_root" => a.path_roots.push(value), _ => {} }
    }
    Ok(a)
}

/// Ingest one captured session (metadata + ordered events). Tenant comes from the ingest token.
/// Idempotent: re-posting the same session/seq is a no-op; content is sha256-deduped.
pub async fn capture(
    AuthedTenant(tenant_id): AuthedTenant,
    State(pool): State<PgPool>,
    Json(s): Json<CapturedSession>,
) -> Result<StatusCode, AppError> {
    let allow = load_allowlist(&pool, &tenant_id).await?;
    let (class, _conf) = classify(s.repo.host.as_deref(), s.repo.org.as_deref(), s.repo.path.as_deref(), &allow);

    let first_ts = s.events.iter().map(|e| e.ts).min();
    let last_ts = s.events.iter().map(|e| e.ts).max();

    sqlx::query(
        "insert into captured_sessions (tenant_id, session_id, user_email, repo_host, repo_org, repo_name, repo_path, classification, title, cwd, first_ts, last_ts, event_count) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) \
         on conflict (tenant_id, session_id) do update set last_ts = excluded.last_ts, event_count = excluded.event_count, title = coalesce(excluded.title, captured_sessions.title)")
        .bind(&tenant_id).bind(&s.session_id).bind(&s.user_email)
        .bind(&s.repo.host).bind(&s.repo.org).bind(&s.repo.name).bind(&s.repo.path)
        .bind(class.as_str()).bind(&s.title).bind(&s.cwd)
        .bind(first_ts).bind(last_ts).bind(s.events.len() as i32)
        .execute(&pool).await?;

    for e in &s.events {
        let content_sha = match &e.content {
            Some(c) => {
                let sha = hex::encode(Sha256::digest(c.as_bytes()));
                sqlx::query("insert into content_blobs (tenant_id, sha256, content, bytes) values ($1,$2,$3,$4) on conflict (tenant_id, sha256) do nothing")
                    .bind(&tenant_id).bind(&sha).bind(c).bind(c.len() as i32).execute(&pool).await?;
                Some(sha)
            }
            None => None,
        };
        sqlx::query(
            "insert into captured_events (tenant_id, session_id, seq, ts, kind, model, tool_name, target, content_sha, tokens_in, tokens_out, is_sidechain) \
             values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) on conflict (tenant_id, session_id, seq) do nothing")
            .bind(&tenant_id).bind(&s.session_id).bind(e.seq).bind(e.ts).bind(e.kind.as_str())
            .bind(&e.model).bind(&e.tool_name).bind(&e.target).bind(&content_sha)
            .bind(e.tokens_in).bind(e.tokens_out).bind(e.is_sidechain)
            .execute(&pool).await?;
    }
    Ok(StatusCode::ACCEPTED)
}
```
(Add `hex = "0.4"` to `crates/ccguard-server/Cargo.toml` `[dependencies]` if not present.)

- [ ] **Step 3: Register** — add `pub mod capture;` to `handlers/mod.rs`; add route to `app.rs`: `.route("/v1/capture", post(capture::capture))`.

- [ ] **Step 4: Test** `crates/ccguard-server/tests/capture.rs`:
```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use sqlx::{PgPool, Row};
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::tokens::generate_token;

async fn seed(pool: &PgPool) -> String {
    sqlx::query("insert into tenants (id,name) values ('acme','Acme')").execute(pool).await.unwrap();
    sqlx::query("insert into allowlist_rules (tenant_id,kind,value) values ('acme','host','github.com'),('acme','org','acme-corp')").execute(pool).await.unwrap();
    let (t, h) = generate_token();
    sqlx::query("insert into api_tokens (tenant_id, token_hash) values ('acme',$1)").bind(&h).execute(pool).await.unwrap();
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
    }).to_string();
    let resp = app(pool.clone()).oneshot(Request::builder().method("POST").uri("/v1/capture")
        .header("content-type","application/json").header("authorization", format!("Bearer {token}"))
        .body(Body::from(body)).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let sess = sqlx::query("select classification, event_count from captured_sessions where tenant_id='acme' and session_id='s1'").fetch_one(&pool).await.unwrap();
    assert_eq!(sess.get::<String,_>("classification"), "work");
    assert_eq!(sess.get::<i32,_>("event_count"), 3);
    let ev = sqlx::query("select count(*) c from captured_events where session_id='s1'").fetch_one(&pool).await.unwrap();
    assert_eq!(ev.get::<i64,_>("c"), 3);
    // "do X" content appears twice but is one blob (deduped):
    let blobs = sqlx::query("select count(*) c from content_blobs where tenant_id='acme'").fetch_one(&pool).await.unwrap();
    assert_eq!(blobs.get::<i64,_>("c"), 2); // "do X" and "done"
}

#[sqlx::test(migrations = "./migrations")]
async fn capture_requires_token(pool: PgPool) {
    seed(&pool).await;
    let resp = app(pool.clone()).oneshot(Request::builder().method("POST").uri("/v1/capture")
        .header("content-type","application/json")
        .body(Body::from(r#"{"session_id":"s","user_email":"x","repo":{},"events":[]}"#)).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
```

- [ ] **Step 5: Run + commit.** `cargo test -p ccguard-server --test capture` → 2 passed.
```
git add crates/ccguard-server
git commit -m "feat(server): POST /v1/capture stores Session/Event/ContentBlob (sha256-deduped)"
```

---

## Task 3: retrieval — session list + full timeline

**Files:** Create `crates/ccguard-server/src/handlers/timeline.rs`, `crates/ccguard-server/tests/timeline.rs`; Modify `handlers/mod.rs`, `app.rs`.

- [ ] **Step 1: Handler** `crates/ccguard-server/src/handlers/timeline.rs`:
```rust
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use sqlx::{PgPool, Row};

use crate::auth::AuthedUser;
use crate::error::AppError;

#[derive(Serialize)]
pub struct SessionRow {
    pub session_id: String,
    pub user_email: String,
    pub classification: String,
    pub repo_org: Option<String>,
    pub repo_name: Option<String>,
    pub title: Option<String>,
    pub event_count: i32,
}

/// List captured sessions for the caller's tenant.
pub async fn list_sessions(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(tenant): Path<String>,
) -> Result<Json<Vec<SessionRow>>, AppError> {
    if user.tenant_id != tenant {
        return Err(AppError::Forbidden("cross-tenant access denied"));
    }
    let rows = sqlx::query(
        "select session_id, user_email, classification, repo_org, repo_name, title, event_count \
         from captured_sessions where tenant_id = $1 order by last_ts desc nulls last limit 500")
        .bind(&tenant).fetch_all(&pool).await?;
    Ok(Json(rows.into_iter().map(|r| SessionRow {
        session_id: r.get("session_id"), user_email: r.get("user_email"),
        classification: r.get("classification"), repo_org: r.get("repo_org"),
        repo_name: r.get("repo_name"), title: r.get("title"), event_count: r.get("event_count"),
    }).collect()))
}

#[derive(Serialize)]
pub struct TimelineEvent {
    pub seq: i64,
    pub ts: chrono::DateTime<chrono::Utc>,
    pub kind: String,
    pub model: Option<String>,
    pub tool_name: Option<String>,
    pub target: Option<String>,
    pub content: Option<String>,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub is_sidechain: bool,
}

/// Full ordered timeline (with verbatim content) for one session, scoped to the caller's tenant.
pub async fn timeline(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<TimelineEvent>>, AppError> {
    let rows = sqlx::query(
        "select e.seq, e.ts, e.kind, e.model, e.tool_name, e.target, b.content, e.tokens_in, e.tokens_out, e.is_sidechain \
         from captured_events e left join content_blobs b on b.tenant_id = e.tenant_id and b.sha256 = e.content_sha \
         where e.tenant_id = $1 and e.session_id = $2 order by e.seq")
        .bind(&user.tenant_id).bind(&session_id).fetch_all(&pool).await?;
    Ok(Json(rows.into_iter().map(|r| TimelineEvent {
        seq: r.get("seq"), ts: r.get("ts"), kind: r.get("kind"), model: r.get("model"),
        tool_name: r.get("tool_name"), target: r.get("target"), content: r.get("content"),
        tokens_in: r.get("tokens_in"), tokens_out: r.get("tokens_out"), is_sidechain: r.get("is_sidechain"),
    }).collect()))
}
```

- [ ] **Step 2: Register** — `pub mod timeline;` in `handlers/mod.rs`; routes in `app.rs`:
```rust
        .route("/v1/orgs/:tenant/sessions", get(timeline::list_sessions))
        .route("/v1/sessions/:session_id/timeline", get(timeline::timeline))
```

- [ ] **Step 3: Test** `crates/ccguard-server/tests/timeline.rs` — seed tenant+allowlist+ingest-token+user, POST a capture (reuse the Task-2 body shape), login, then GET `/v1/orgs/acme/sessions` (expect 1 session, classification work) and GET `/v1/sessions/s1/timeline` (expect 3 events in seq order, content present, the bash tool_call has tool_name "Bash"). Also assert unauthenticated timeline GET → 401, and a cross-tenant user → empty/forbidden. (Mirror the auth helpers from `tests/summary.rs`.)

- [ ] **Step 4: Run + commit.** `cargo test -p ccguard-server --test timeline` passes.
```
git add crates/ccguard-server
git commit -m "feat(server): GET sessions list + full session timeline retrieval"
```

---

## Task 4: agent full-transcript parser + `--capture` mode

**Files:** Create `crates/ccguard-agent/src/transcript.rs`; Modify `crates/ccguard-agent/src/poster.rs`, `crates/ccguard-agent/src/main.rs`.

**Read first:** `research/total-visibility.md` §1 (exact JSONL line types/fields) and inspect a real `~/.claude/projects/**/*.jsonl` to confirm field names.

- [ ] **Step 1: `transcript.rs`** — `pub fn parse_session(content: &str, fallback_cwd: Option<&str>) -> ccguard_core::capture::CapturedSession`. Parse newline JSON; produce ordered `CapturedEvent`s (seq = running index):
  - `user` line with text `message.content` → `UserPrompt` (content = verbatim text). Track `cwd`/`gitBranch` across lines.
  - `assistant` line: per content block — `text` → `AssistantText` (attach `message.model`, and `message.usage.input_tokens/output_tokens` on the first text/assistant event of the turn); `thinking` → `Thinking` (content = thinking text); `tool_use` → `ToolCall` (`tool_name` = name, `target` = derived (Bash `command`, Read/Edit/Write `file_path`, WebFetch `url`), `content` = JSON of `input`). For Edit/Write tool_use also acceptable to mark kind `FileEdit`.
  - tool result (`user` line carrying `toolUseResult`/`tool_result` block) → `ToolResult` (`tool_name` if resolvable, `content` = stdout/result text; for big results, deref `<session>/tool-results/<toolu_id>.txt` if present).
  - `ai-title` line → set `session.title`.
  - `pr-link` line → `Pr` event (`target` = prUrl, content = repo/number).
  - session-level: `session_id` from line `sessionId` (or fallback), `cwd`.
  Skip unparseable lines. Include a unit test with a crafted multi-line transcript asserting the event sequence/kinds/content (prompt → assistant text w/ tokens → tool_call Bash → tool_result; plus an ai-title sets the title).
  Stretch (do if time permits, else note): also parse `<session>/subagents/agent-*.jsonl` with `is_sidechain=true`.

- [ ] **Step 2: identity** — reuse `read_claude_email` from `main.rs` (already reads `.claude.json oauthAccount.emailAddress`) to populate `user_email`; repo via the existing `RepoCache` on the session cwd.

- [ ] **Step 3: poster** — add to `crates/ccguard-agent/src/poster.rs`:
```rust
use ccguard_core::capture::CapturedSession;
impl Poster {
    pub fn post_capture(&self, s: &CapturedSession) -> anyhow::Result<u16> {
        let url = self.url.replace("/v1/events", "/v1/capture");
        let resp = self.client.post(&url).bearer_auth(&self.token).json(s).send()?;
        Ok(resp.status().as_u16())
    }
}
```
(Adjust if `Poster` stores base URL differently; ensure it posts to `<server>/v1/capture`.)

- [ ] **Step 4: `--capture` mode in `main.rs`** — add a `#[arg(long)] capture: bool` flag. When set, for each transcript file: read new bytes (reuse `read_since` + state), `parse_session`, set `user_email` + `repo` (RepoCache on cwd), POST via `post_capture`, count. When not set, keep the existing token-event behavior. Print "captured N session(s)."

- [ ] **Step 5: Verify + commit.** `cargo test -p ccguard-agent` (incl. the new transcript test) passes; `cargo build` whole workspace; `cargo run -p ccguard-agent -- --help` shows `--capture`.
```
git add crates/ccguard-agent
git commit -m "feat(agent): full-transcript parser + --capture mode posting CapturedSessions"
```

---

## Self-Review (done while writing this plan)
**Coverage:** capture types ✅(T1) · store Session/Event/ContentBlob + dedupe ✅(T2) · list + full timeline retrieval ✅(T3) · agent full parse + capture mode ✅(T4). UI/search/findings/enforcement/on-task → Plans 6–9 (documented). **Types:** `CapturedSession`/`CapturedEvent`/`EventKind` (T1) used by `/v1/capture` (T2) and the agent (T4); `EventKind::as_str` writes the `kind` column read back by the timeline (T3). `AuthedTenant` (ingest) gates `/v1/capture`; `AuthedUser`+tenant-check gates retrieval. sha256 dedupe via the existing server `sha2` dep + `hex`. **Sharp edges:** add `hex` to server Cargo.toml; the parser is exploratory — read the research doc + a real transcript first; keep the existing `/v1/events` path intact (capture is additive).

## Execution handoff
Build **subagent-driven** against the live Postgres; controller runs a **live capture of a small real session** afterward to prove the full timeline (prompts + tool calls + diffs) is retrievable.
