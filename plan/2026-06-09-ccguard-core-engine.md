# CCGuard Core Engine — Implementation Plan (Plan 1 of N)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the CCGuard core engine — accept a normalized AI-coding event, classify its repo as work/personal/unknown against a per-tenant allowlist, store it, and aggregate spend by classification (the data behind the work-vs-personal donut).

**Architecture:** A Cargo workspace. `ccguard-core` is a pure-logic crate (event types, git-remote URL parsing, classifier, aggregation) with no I/O, fully unit-tested. `ccguard-server` is an axum HTTP service over Postgres (sqlx) exposing `POST /v1/events` (ingest → classify → store) and `GET /v1/orgs/{tenant}/summary` (aggregate), integration-tested against a real Postgres via `#[sqlx::test]`.

**Tech Stack:** Rust (edition 2021), axum 0.7, tokio 1, sqlx 0.8 (Postgres, runtime queries — no `DATABASE_URL` needed at compile time), serde, chrono, Docker (local Postgres). Git identity for commits: `user.email = senthilguru246@gmail.com`, `user.name = SenGuru`.

---

## Plan roadmap (where this fits)

This is **Plan 1 of N**. Each plan ships working, testable software.

1. **Core engine** ← *this plan* — event → classify → store → aggregate.
2. **Auth, tenants & roles** — signup, org creation, owner/admin/manager/auditor/member, tenant isolation middleware.
3. **SCM feed (GitHub OAuth)** — connect GitHub → auto-build the allowlist from org repos.
4. **Minimal dashboard** — axum + askama templates + Chart.js donut: org overview + repo view + unknown-repo triage.
5. **Rust endpoint agent** — harvest `~/.claude/projects/*.jsonl` + `.claude.json`, process monitor, git-on-disk, visible/non-covert, capture-tier aware → posts CCGuard events.
6. **Consent skeleton** — capture-tier toggles, notice/acknowledgment records, retention config.
7. **Stripe skeleton** — seat-based subscription, tier gating.

Plans 2–7 are written after Plan 1 lands.

---

## Prerequisites (one-time, before Task 1)

- [ ] **Rust toolchain** installed: `rustc --version` (expect 1.79+). If missing: install from https://rustup.rs.
- [ ] **Docker Desktop** running: `docker --version`.
- [ ] Working directory is `C:\Users\gsent\Desktop\2027-q1-projects\CCGuard`.

---

## File structure (created by this plan)

```
CCGuard/
  Cargo.toml                              # workspace manifest
  .gitignore
  docker-compose.yml                      # local Postgres
  crates/
    ccguard-core/
      Cargo.toml
      src/lib.rs                          # re-exports
      src/event.rs                        # normalized CcEvent + sub-types
      src/remote.rs                       # git remote URL → host/org/name
      src/classify.rs                     # Allowlist + classify()
      src/aggregate.rs                    # totals_by_classification()
    ccguard-server/
      Cargo.toml
      src/main.rs                         # bootstrap (bind + serve)
      src/app.rs                          # Router builder (app(pool))
      src/error.rs                        # AppError -> IntoResponse
      src/handlers/mod.rs
      src/handlers/ingest.rs              # POST /v1/events
      src/handlers/summary.rs             # GET /v1/orgs/:tenant/summary
      migrations/0001_init.sql            # tenants, allowlist_rules, events
      tests/ingest.rs                     # integration: ingest classifies+stores
      tests/summary.rs                    # integration: aggregation
```

---

## Task 1: Workspace scaffold + git

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `docker-compose.yml`

- [ ] **Step 1: Initialize git and set commit identity**

Run (PowerShell, in the CCGuard folder):
```powershell
git init
git config user.email "senthilguru246@gmail.com"
git config user.name "SenGuru"
```
Expected: `Initialized empty Git repository …`

- [ ] **Step 2: Create the workspace manifest**

Create `Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = ["crates/ccguard-core", "crates/ccguard-server"]

[workspace.package]
edition = "2021"
version = "0.1.0"
```

- [ ] **Step 3: Create `.gitignore`**

Create `.gitignore`:
```gitignore
/target
**/*.rs.bk
.env
*.db
```

- [ ] **Step 4: Create local Postgres compose file**

Create `docker-compose.yml`:
```yaml
services:
  db:
    image: postgres:16
    environment:
      POSTGRES_USER: ccguard
      POSTGRES_PASSWORD: ccguard
      POSTGRES_DB: ccguard
    ports:
      - "5432:5432"
```

- [ ] **Step 5: Start Postgres and verify**

Run:
```powershell
docker compose up -d
docker compose ps
```
Expected: the `db` service shows `running` and port `5432` mapped.

- [ ] **Step 6: Commit**

```powershell
git add Cargo.toml .gitignore docker-compose.yml
git commit -m "chore: workspace scaffold + local postgres"
```

---

## Task 2: Normalized event types (`ccguard-core`)

**Files:**
- Create: `crates/ccguard-core/Cargo.toml`, `crates/ccguard-core/src/lib.rs`, `crates/ccguard-core/src/event.rs`

- [ ] **Step 1: Create the core crate manifest**

Create `crates/ccguard-core/Cargo.toml`:
```toml
[package]
name = "ccguard-core"
edition.workspace = true
version.workspace = true

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 2: Create the crate root**

Create `crates/ccguard-core/src/lib.rs`:
```rust
pub mod aggregate;
pub mod classify;
pub mod event;
pub mod remote;
```
(The `aggregate`, `classify`, `remote` modules are added in later tasks; create empty files now so it compiles.)

Create empty placeholder files so the crate compiles:
- `crates/ccguard-core/src/remote.rs` → `// filled in Task 3`
- `crates/ccguard-core/src/classify.rs` → `// filled in Task 4`
- `crates/ccguard-core/src/aggregate.rs` → `// filled in Task 5`

- [ ] **Step 3: Write the failing test for event (de)serialization**

Create `crates/ccguard-core/src/event.rs`:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CcEvent {
    pub tenant_id: String,
    pub user: User,
    pub tool: String,
    pub session_id: String,
    pub ts: DateTime<Utc>,
    pub repo: Repo,
    #[serde(default)]
    pub content_ref: Option<String>,
    pub source_layer: String,
    pub activity: Activity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub email: String,
    #[serde(default)]
    pub seat_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Repo {
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub org: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub classification: Option<Classification>,
    #[serde(default)]
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Classification {
    Work,
    Personal,
    Unknown,
}

impl Classification {
    pub fn as_str(&self) -> &'static str {
        match self {
            Classification::Work => "work",
            Classification::Personal => "personal",
            Classification::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Activity {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub tokens_in: i64,
    #[serde(default)]
    pub tokens_out: i64,
    #[serde(default)]
    pub cost_usd: f64,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub decision: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_minimal_event_with_defaults() {
        let json = r#"{
            "tenant_id": "acme",
            "user": { "email": "dev@acme.com" },
            "tool": "claude-code",
            "session_id": "s1",
            "ts": "2026-06-09T21:13:00Z",
            "repo": { "host": "github.com", "org": "acme-corp", "name": "billing" },
            "source_layer": "endpoint_agent",
            "activity": { "type": "api_request", "cost_usd": 0.12, "tokens_in": 100 }
        }"#;
        let ev: CcEvent = serde_json::from_str(json).unwrap();
        assert_eq!(ev.tenant_id, "acme");
        assert_eq!(ev.user.email, "dev@acme.com");
        assert_eq!(ev.user.seat_id, None);
        assert_eq!(ev.repo.org.as_deref(), Some("acme-corp"));
        assert_eq!(ev.repo.classification, None);
        assert_eq!(ev.activity.cost_usd, 0.12);
        assert_eq!(ev.activity.tokens_out, 0);
    }

    #[test]
    fn classification_serializes_lowercase() {
        let s = serde_json::to_string(&Classification::Work).unwrap();
        assert_eq!(s, "\"work\"");
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:
```powershell
cargo test -p ccguard-core event::
```
Expected: 2 passed. (If a compile error mentions missing modules, confirm the placeholder files from Step 2 exist.)

- [ ] **Step 5: Commit**

```powershell
git add crates/ccguard-core
git commit -m "feat(core): normalized CcEvent types + serde defaults"
```

---

## Task 3: Git remote URL parsing (`ccguard-core::remote`)

**Files:**
- Modify: `crates/ccguard-core/src/remote.rs`

- [ ] **Step 1: Write the failing tests**

Replace `crates/ccguard-core/src/remote.rs` with:
```rust
//! Parse a git remote URL into (host, org, name). Handles scp-like (`git@host:org/repo.git`),
//! https, and ssh forms, with or without a trailing `.git`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteIdentity {
    pub host: String,
    pub org: String,
    pub name: String,
}

pub fn parse_remote_url(url: &str) -> Option<RemoteIdentity> {
    let mut s = url.trim();
    if let Some(stripped) = s.strip_suffix(".git") {
        s = stripped;
    }

    let (host, path) = if let Some(rest) = s.strip_prefix("git@") {
        // scp-like: git@github.com:org/repo
        let (h, p) = rest.split_once(':')?;
        (h.to_string(), p.to_string())
    } else {
        // strip scheme://
        let no_scheme = match s.find("://") {
            Some(i) => &s[i + 3..],
            None => s,
        };
        // strip optional user@
        let no_user = match no_scheme.split_once('@') {
            Some((_, after)) => after,
            None => no_scheme,
        };
        let (h, p) = no_user.split_once('/')?;
        (h.to_string(), p.to_string())
    };

    let parts: Vec<&str> = path.split('/').filter(|x| !x.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }
    let org = parts[0].to_string();
    let name = parts[parts.len() - 1].to_string();
    if host.is_empty() || org.is_empty() || name.is_empty() {
        return None;
    }
    Some(RemoteIdentity {
        host: host.to_ascii_lowercase(),
        org,
        name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(host: &str, org: &str, name: &str) -> RemoteIdentity {
        RemoteIdentity {
            host: host.into(),
            org: org.into(),
            name: name.into(),
        }
    }

    #[test]
    fn parses_scp_like() {
        assert_eq!(
            parse_remote_url("git@github.com:acme-corp/billing.git"),
            Some(id("github.com", "acme-corp", "billing"))
        );
    }

    #[test]
    fn parses_https_with_and_without_git_suffix() {
        assert_eq!(
            parse_remote_url("https://github.com/acme-corp/billing.git"),
            Some(id("github.com", "acme-corp", "billing"))
        );
        assert_eq!(
            parse_remote_url("https://github.com/acme-corp/billing"),
            Some(id("github.com", "acme-corp", "billing"))
        );
    }

    #[test]
    fn parses_ssh_scheme_with_user() {
        assert_eq!(
            parse_remote_url("ssh://git@gitlab.acme.com/group/billing.git"),
            Some(id("gitlab.acme.com", "group", "billing"))
        );
    }

    #[test]
    fn subgroup_org_is_first_segment_name_is_last() {
        assert_eq!(
            parse_remote_url("https://gitlab.acme.com/group/sub/billing.git"),
            Some(id("gitlab.acme.com", "group", "billing"))
        );
    }

    #[test]
    fn host_is_lowercased() {
        assert_eq!(
            parse_remote_url("git@GitHub.com:Acme/Repo.git"),
            Some(id("github.com", "Acme", "Repo"))
        );
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_remote_url("not-a-url"), None);
        assert_eq!(parse_remote_url("https://github.com/onlyone"), None);
    }
}
```

- [ ] **Step 2: Run to verify they pass**

Run:
```powershell
cargo test -p ccguard-core remote::
```
Expected: 6 passed.

- [ ] **Step 3: Commit**

```powershell
git add crates/ccguard-core/src/remote.rs
git commit -m "feat(core): parse git remote URLs (scp/https/ssh) to host/org/name"
```

---

## Task 4: Classifier (`ccguard-core::classify`)

**Files:**
- Modify: `crates/ccguard-core/src/classify.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Replace `crates/ccguard-core/src/classify.rs` with:
```rust
use crate::event::Classification;

/// A tenant's approved-resources allowlist.
#[derive(Debug, Default, Clone)]
pub struct Allowlist {
    pub hosts: Vec<String>,      // approved git hosts, e.g. "github.com"
    pub orgs: Vec<String>,       // approved orgs/owners, e.g. "acme-corp"
    pub path_roots: Vec<String>, // approved local path roots, e.g. "c:\\work"
}

/// Classify using the strongest available signal: git remote (host+org) first, then local path.
pub fn classify(
    repo_host: Option<&str>,
    repo_org: Option<&str>,
    repo_path: Option<&str>,
    allow: &Allowlist,
) -> (Classification, f32) {
    if let (Some(host), Some(org)) = (repo_host, repo_org) {
        let host_ok = allow.hosts.iter().any(|h| h.eq_ignore_ascii_case(host));
        let org_ok = allow.orgs.iter().any(|o| o.eq_ignore_ascii_case(org));
        return if host_ok && org_ok {
            (Classification::Work, 0.9)
        } else {
            (Classification::Personal, 0.8)
        };
    }
    if let Some(path) = repo_path {
        let p = path.replace('\\', "/").to_ascii_lowercase();
        let hit = allow
            .path_roots
            .iter()
            .any(|r| p.starts_with(&r.replace('\\', "/").to_ascii_lowercase()));
        return if hit {
            (Classification::Work, 0.6)
        } else {
            (Classification::Unknown, 0.3)
        };
    }
    (Classification::Unknown, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allow() -> Allowlist {
        Allowlist {
            hosts: vec!["github.com".into()],
            orgs: vec!["acme-corp".into()],
            path_roots: vec!["c:\\work".into()],
        }
    }

    #[test]
    fn host_and_org_match_is_work() {
        let (c, conf) = classify(Some("github.com"), Some("acme-corp"), None, &allow());
        assert_eq!(c, Classification::Work);
        assert!(conf > 0.5);
    }

    #[test]
    fn org_outside_allowlist_is_personal() {
        let (c, _) = classify(Some("github.com"), Some("dev-personal"), None, &allow());
        assert_eq!(c, Classification::Personal);
    }

    #[test]
    fn matching_is_case_insensitive() {
        let (c, _) = classify(Some("GitHub.com"), Some("ACME-Corp"), None, &allow());
        assert_eq!(c, Classification::Work);
    }

    #[test]
    fn path_root_match_is_work() {
        let (c, _) = classify(None, None, Some("C:\\work\\scratch"), &allow());
        assert_eq!(c, Classification::Work);
    }

    #[test]
    fn no_signal_is_unknown() {
        let (c, conf) = classify(None, None, None, &allow());
        assert_eq!(c, Classification::Unknown);
        assert_eq!(conf, 0.0);
    }
}
```

- [ ] **Step 2: Run to verify they pass**

Run:
```powershell
cargo test -p ccguard-core classify::
```
Expected: 5 passed.

- [ ] **Step 3: Commit**

```powershell
git add crates/ccguard-core/src/classify.rs
git commit -m "feat(core): repo-allowlist classifier (work/personal/unknown)"
```

---

## Task 5: Aggregation (`ccguard-core::aggregate`)

**Files:**
- Modify: `crates/ccguard-core/src/aggregate.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Replace `crates/ccguard-core/src/aggregate.rs` with:
```rust
use std::collections::HashMap;

use crate::event::{CcEvent, Classification};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Totals {
    pub cost_usd: f64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub events: u64,
}

/// Sum spend/usage grouped by the event's repo classification.
/// Events with no classification set are counted as Unknown.
pub fn totals_by_classification(events: &[CcEvent]) -> HashMap<Classification, Totals> {
    let mut out: HashMap<Classification, Totals> = HashMap::new();
    for e in events {
        let class = e.repo.classification.unwrap_or(Classification::Unknown);
        let t = out.entry(class).or_default();
        t.cost_usd += e.activity.cost_usd;
        t.tokens_in += e.activity.tokens_in;
        t.tokens_out += e.activity.tokens_out;
        t.events += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Activity, Repo, User};
    use chrono::Utc;

    fn ev(class: Classification, cost: f64) -> CcEvent {
        CcEvent {
            tenant_id: "acme".into(),
            user: User { email: "d@acme.com".into(), seat_id: None },
            tool: "claude-code".into(),
            session_id: "s".into(),
            ts: Utc::now(),
            repo: Repo {
                host: None, org: None, name: None, path: None,
                classification: Some(class), confidence: 0.9,
            },
            content_ref: None,
            source_layer: "test".into(),
            activity: Activity {
                kind: "api_request".into(),
                tokens_in: 10, tokens_out: 5, cost_usd: cost,
                model: None, tool_name: None, decision: None,
            },
        }
    }

    #[test]
    fn sums_cost_per_classification() {
        let events = vec![
            ev(Classification::Work, 1.0),
            ev(Classification::Work, 0.5),
            ev(Classification::Personal, 0.25),
        ];
        let totals = totals_by_classification(&events);
        assert_eq!(totals[&Classification::Work].cost_usd, 1.5);
        assert_eq!(totals[&Classification::Work].events, 2);
        assert_eq!(totals[&Classification::Personal].cost_usd, 0.25);
        assert!(!totals.contains_key(&Classification::Unknown));
    }
}
```

- [ ] **Step 2: Run to verify they pass**

Run:
```powershell
cargo test -p ccguard-core aggregate::
```
Expected: 1 passed.

- [ ] **Step 3: Run the whole core crate + commit**

Run:
```powershell
cargo test -p ccguard-core
```
Expected: all core tests pass (event + remote + classify + aggregate).
```powershell
git add crates/ccguard-core/src/aggregate.rs
git commit -m "feat(core): aggregate totals by classification"
```

---

## Task 6: Server crate + DB schema + pool

**Files:**
- Create: `crates/ccguard-server/Cargo.toml`, `crates/ccguard-server/migrations/0001_init.sql`, `crates/ccguard-server/src/error.rs`, `crates/ccguard-server/src/app.rs`, `crates/ccguard-server/src/main.rs`, `crates/ccguard-server/src/handlers/mod.rs`

- [ ] **Step 1: Create the server manifest**

Create `crates/ccguard-server/Cargo.toml`:
```toml
[package]
name = "ccguard-server"
edition.workspace = true
version.workspace = true

[dependencies]
ccguard-core = { path = "../ccguard-core" }
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio", "postgres", "chrono", "macros", "migrate"] }
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
http-body-util = "0.1"
```

- [ ] **Step 2: Create the migration**

Create `crates/ccguard-server/migrations/0001_init.sql`:
```sql
create table if not exists tenants (
    id         text primary key,
    name       text not null,
    created_at timestamptz not null default now()
);

create table if not exists allowlist_rules (
    id         bigserial primary key,
    tenant_id  text not null references tenants(id),
    kind       text not null check (kind in ('host', 'org', 'path_root')),
    value      text not null,
    created_at timestamptz not null default now()
);

create table if not exists events (
    id            bigserial primary key,
    tenant_id     text not null references tenants(id),
    user_email    text not null,
    seat_id       text,
    tool          text not null,
    session_id    text not null,
    ts            timestamptz not null,
    repo_host     text,
    repo_org      text,
    repo_name     text,
    repo_path     text,
    classification text not null,
    confidence    real not null default 0,
    activity_type text not null,
    tokens_in     bigint not null default 0,
    tokens_out    bigint not null default 0,
    cost_usd      double precision not null default 0,
    model         text,
    tool_name     text,
    content_ref   text,
    source_layer  text not null,
    created_at    timestamptz not null default now()
);

create index if not exists events_tenant_ts on events (tenant_id, ts);
create index if not exists events_tenant_class on events (tenant_id, classification);
```

- [ ] **Step 3: Create the error type**

Create `crates/ccguard-server/src/error.rs`:
```rust
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug)]
pub enum AppError {
    Db(sqlx::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Db(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Db(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")).into_response()
            }
        }
    }
}
```

- [ ] **Step 4: Create the handlers module placeholder**

Create `crates/ccguard-server/src/handlers/mod.rs`:
```rust
pub mod ingest;
pub mod summary;
```
Create empty placeholders so it compiles (filled in Tasks 7–8):
- `crates/ccguard-server/src/handlers/ingest.rs` → `// filled in Task 7`
- `crates/ccguard-server/src/handlers/summary.rs` → `// filled in Task 8`

- [ ] **Step 5: Create the Router builder**

Create `crates/ccguard-server/src/app.rs`:
```rust
use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;

use crate::handlers::{ingest, summary};

pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/v1/events", post(ingest::ingest))
        .route("/v1/orgs/:tenant/summary", get(summary::summary))
        .with_state(pool)
}
```

- [ ] **Step 6: Create the binary entrypoint**

Create `crates/ccguard-server/src/main.rs`:
```rust
mod app;
mod error;
mod handlers;

use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ccguard:ccguard@localhost:5432/ccguard".into());
    let pool = PgPoolOptions::new().connect(&url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    println!("CCGuard server listening on :8080");
    axum::serve(listener, app::app(pool)).await?;
    Ok(())
}
```
(Add `anyhow = "1"` to `[dependencies]` in `crates/ccguard-server/Cargo.toml`.)

- [ ] **Step 7: Verify it compiles**

Run:
```powershell
cargo build -p ccguard-server
```
Expected: builds (handlers are empty stubs; `app.rs` references `ingest::ingest` and `summary::summary` which don't exist yet — so this will FAIL to compile until Task 7/8). **To keep Task 6 self-contained, temporarily comment out the two `.route(...)` lines in `app.rs`, confirm `cargo build -p ccguard-server` succeeds, then uncomment them.** They are implemented in the next two tasks.

- [ ] **Step 8: Commit**

```powershell
git add crates/ccguard-server Cargo.toml
git commit -m "feat(server): crate scaffold, migrations, pool, router skeleton"
```

---

## Task 7: Ingest endpoint (`POST /v1/events`)

**Files:**
- Modify: `crates/ccguard-server/src/handlers/ingest.rs`
- Create: `crates/ccguard-server/tests/ingest.rs`

- [ ] **Step 1: Implement the handler**

Replace `crates/ccguard-server/src/handlers/ingest.rs` with:
```rust
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use ccguard_core::classify::{classify, Allowlist};
use ccguard_core::event::CcEvent;
use sqlx::{PgPool, Row};

use crate::error::AppError;

async fn load_allowlist(pool: &PgPool, tenant_id: &str) -> Result<Allowlist, sqlx::Error> {
    let rows = sqlx::query("select kind, value from allowlist_rules where tenant_id = $1")
        .bind(tenant_id)
        .fetch_all(pool)
        .await?;
    let mut allow = Allowlist::default();
    for row in rows {
        let kind: String = row.get("kind");
        let value: String = row.get("value");
        match kind.as_str() {
            "host" => allow.hosts.push(value),
            "org" => allow.orgs.push(value),
            "path_root" => allow.path_roots.push(value),
            _ => {}
        }
    }
    Ok(allow)
}

pub async fn ingest(
    State(pool): State<PgPool>,
    Json(ev): Json<CcEvent>,
) -> Result<StatusCode, AppError> {
    let allow = load_allowlist(&pool, &ev.tenant_id).await?;
    let (class, confidence) = classify(
        ev.repo.host.as_deref(),
        ev.repo.org.as_deref(),
        ev.repo.path.as_deref(),
        &allow,
    );

    sqlx::query(
        "insert into events (tenant_id, user_email, seat_id, tool, session_id, ts, \
         repo_host, repo_org, repo_name, repo_path, classification, confidence, \
         activity_type, tokens_in, tokens_out, cost_usd, model, tool_name, content_ref, source_layer) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)",
    )
    .bind(&ev.tenant_id)
    .bind(&ev.user.email)
    .bind(&ev.user.seat_id)
    .bind(&ev.tool)
    .bind(&ev.session_id)
    .bind(ev.ts)
    .bind(&ev.repo.host)
    .bind(&ev.repo.org)
    .bind(&ev.repo.name)
    .bind(&ev.repo.path)
    .bind(class.as_str())
    .bind(confidence)
    .bind(&ev.activity.kind)
    .bind(ev.activity.tokens_in)
    .bind(ev.activity.tokens_out)
    .bind(ev.activity.cost_usd)
    .bind(&ev.activity.model)
    .bind(&ev.activity.tool_name)
    .bind(&ev.content_ref)
    .bind(&ev.source_layer)
    .execute(&pool)
    .await?;

    Ok(StatusCode::ACCEPTED)
}
```

- [ ] **Step 2: Write the integration test**

Create `crates/ccguard-server/tests/ingest.rs`:
```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::{PgPool, Row};
use tower::ServiceExt; // for `oneshot`

// Pull in the crate's app builder by path. Expose `app` for tests:
use ccguard_server::app::app;

async fn seed(pool: &PgPool) {
    sqlx::query("insert into tenants (id, name) values ('acme', 'Acme')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("insert into allowlist_rules (tenant_id, kind, value) values ('acme','host','github.com'),('acme','org','acme-corp')")
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn ingest_classifies_company_repo_as_work(pool: PgPool) {
    seed(&pool).await;

    let body = serde_json::json!({
        "tenant_id": "acme",
        "user": { "email": "dev@acme.com" },
        "tool": "claude-code",
        "session_id": "s1",
        "ts": "2026-06-09T21:13:00Z",
        "repo": { "host": "github.com", "org": "acme-corp", "name": "billing" },
        "source_layer": "endpoint_agent",
        "activity": { "type": "api_request", "cost_usd": 0.5, "tokens_in": 100, "tokens_out": 20 }
    });

    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/events")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let row = sqlx::query("select classification, cost_usd from events where tenant_id = 'acme'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let class: String = row.get("classification");
    let cost: f64 = row.get("cost_usd");
    assert_eq!(class, "work");
    assert_eq!(cost, 0.5);
}

#[sqlx::test(migrations = "./migrations")]
async fn ingest_classifies_outside_repo_as_personal(pool: PgPool) {
    seed(&pool).await;

    let body = serde_json::json!({
        "tenant_id": "acme",
        "user": { "email": "dev@acme.com" },
        "tool": "claude-code",
        "session_id": "s2",
        "ts": "2026-06-09T21:14:00Z",
        "repo": { "host": "github.com", "org": "dev-personal", "name": "sideproj" },
        "source_layer": "endpoint_agent",
        "activity": { "type": "api_request", "cost_usd": 0.3 }
    });

    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/events")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let _ = resp.collect().await; // drain

    let row = sqlx::query("select classification from events where session_id = 's2'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let class: String = row.get("classification");
    assert_eq!(class, "personal");
}
```

- [ ] **Step 3: Expose `app` to integration tests**

Integration tests (`tests/`) can only see the crate's public library API, but this crate is currently binary-only. Add a library target. Create `crates/ccguard-server/src/lib.rs`:
```rust
pub mod app;
pub mod error;
pub mod handlers;
```
Then trim `crates/ccguard-server/src/main.rs` to use the library:
```rust
use ccguard_server::app::app;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ccguard:ccguard@localhost:5432/ccguard".into());
    let pool = PgPoolOptions::new().connect(&url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    println!("CCGuard server listening on :8080");
    axum::serve(listener, app(pool)).await?;
    Ok(())
}
```
(The `mod` declarations move from `main.rs` into `lib.rs`; `main.rs` now depends on the library crate `ccguard_server`.)

- [ ] **Step 4: Run the ingest tests**

Ensure Postgres is up (`docker compose up -d`). `#[sqlx::test]` creates an isolated test database per test and runs the migrations automatically. It needs a base connection — set it once:
```powershell
$env:DATABASE_URL = "postgres://ccguard:ccguard@localhost:5432/ccguard"
cargo test -p ccguard-server --test ingest
```
Expected: 2 passed.

- [ ] **Step 5: Commit**

```powershell
git add crates/ccguard-server
git commit -m "feat(server): POST /v1/events ingest -> classify -> store (+integration tests)"
```

---

## Task 8: Summary endpoint (`GET /v1/orgs/:tenant/summary`)

**Files:**
- Modify: `crates/ccguard-server/src/handlers/summary.rs`
- Create: `crates/ccguard-server/tests/summary.rs`

- [ ] **Step 1: Implement the handler**

Replace `crates/ccguard-server/src/handlers/summary.rs` with:
```rust
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use sqlx::{PgPool, Row};

use crate::error::AppError;

#[derive(Serialize)]
pub struct ClassTotals {
    pub classification: String,
    pub cost_usd: f64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub events: i64,
}

pub async fn summary(
    State(pool): State<PgPool>,
    Path(tenant): Path<String>,
) -> Result<Json<Vec<ClassTotals>>, AppError> {
    let rows = sqlx::query(
        "select classification, \
                coalesce(sum(cost_usd),0)   as cost_usd, \
                coalesce(sum(tokens_in),0)  as tokens_in, \
                coalesce(sum(tokens_out),0) as tokens_out, \
                count(*)                    as events \
         from events where tenant_id = $1 group by classification \
         order by classification",
    )
    .bind(&tenant)
    .fetch_all(&pool)
    .await?;

    let out = rows
        .into_iter()
        .map(|r| ClassTotals {
            classification: r.get("classification"),
            cost_usd: r.get("cost_usd"),
            tokens_in: r.get("tokens_in"),
            tokens_out: r.get("tokens_out"),
            events: r.get("events"),
        })
        .collect();
    Ok(Json(out))
}
```

- [ ] **Step 2: Write the integration test**

Create `crates/ccguard-server/tests/summary.rs`:
```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

use ccguard_server::app::app;

async fn post_event(pool: &PgPool, session: &str, org: &str, cost: f64) {
    let body = serde_json::json!({
        "tenant_id": "acme",
        "user": { "email": "dev@acme.com" },
        "tool": "claude-code",
        "session_id": session,
        "ts": "2026-06-09T21:13:00Z",
        "repo": { "host": "github.com", "org": org, "name": "r" },
        "source_layer": "endpoint_agent",
        "activity": { "type": "api_request", "cost_usd": cost }
    });
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/events")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[sqlx::test(migrations = "./migrations")]
async fn summary_groups_spend_by_classification(pool: PgPool) {
    sqlx::query("insert into tenants (id, name) values ('acme','Acme')")
        .execute(&pool).await.unwrap();
    sqlx::query("insert into allowlist_rules (tenant_id, kind, value) values ('acme','host','github.com'),('acme','org','acme-corp')")
        .execute(&pool).await.unwrap();

    post_event(&pool, "s1", "acme-corp", 1.0).await;   // work
    post_event(&pool, "s2", "acme-corp", 0.5).await;   // work
    post_event(&pool, "s3", "dev-personal", 0.25).await; // personal

    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/orgs/acme/summary")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let work = v.as_array().unwrap().iter().find(|x| x["classification"] == "work").unwrap();
    let personal = v.as_array().unwrap().iter().find(|x| x["classification"] == "personal").unwrap();
    assert_eq!(work["cost_usd"], 1.5);
    assert_eq!(work["events"], 2);
    assert_eq!(personal["cost_usd"], 0.25);
}
```

- [ ] **Step 3: Run the summary test**

Run (Postgres up, `DATABASE_URL` set as in Task 7):
```powershell
cargo test -p ccguard-server --test summary
```
Expected: 1 passed.

- [ ] **Step 4: Run the whole workspace test suite**

Run:
```powershell
cargo test
```
Expected: all `ccguard-core` unit tests + all `ccguard-server` integration tests pass.

- [ ] **Step 5: Manual smoke test (optional but recommended)**

```powershell
cargo run -p ccguard-server
# in another terminal:
# (seed a tenant+allowlist via psql, POST an event, GET the summary)
```

- [ ] **Step 6: Commit**

```powershell
git add crates/ccguard-server
git commit -m "feat(server): GET /v1/orgs/:tenant/summary aggregation (+integration test)"
```

---

## Self-Review (done while writing this plan)

**Spec coverage (Plan 1 scope):** normalized event ✅ (Task 2) · ingest API ✅ (Task 7) · multi-tenant store ✅ (Task 6, tenant_id on every row) · repo-allowlist classifier ✅ (Task 4, wired in Task 7) · aggregation/donut data ✅ (Tasks 5 + 8). Auth/roles, dashboard UI, consent, Stripe, the agent, and SCM are **explicitly deferred to Plans 2–7** (noted in roadmap) — not gaps.

**Placeholder scan:** no TBD/TODO; every code step has complete code; every command has expected output.

**Type consistency:** `CcEvent`/`Repo`/`Activity`/`Classification` defined in Task 2 are used unchanged in Tasks 5, 7, 8. `Allowlist` defined in Task 4 is constructed in Task 7. `classify(host, org, path, &allow)` signature in Task 4 matches its call site in Task 7. `Classification::as_str()` (Task 2) is used in the ingest insert (Task 7) and compared against DB strings in tests. The classifier writes `"work"|"personal"|"unknown"`; the summary groups on the same column.

**Known sharp edges flagged for the implementer:**
- Task 6 Step 7: `app.rs` won't compile until Tasks 7–8 exist — comment the routes to verify the scaffold, then uncomment.
- Task 7 Step 3 converts the binary-only crate into lib + bin so `tests/` can import `app`. Do this before running any server integration test.
- `#[sqlx::test]` needs a reachable Postgres and `DATABASE_URL` set to the base DB; it provisions an isolated DB per test.

---

## Execution handoff

Plan saved to `CCGuard\plan\2026-06-09-ccguard-core-engine.md`. Choose how to execute:

1. **Subagent-Driven (recommended)** — a fresh subagent per task, two-stage review between tasks, fast iteration.
2. **Inline Execution** — execute the tasks in this session with checkpoints for review.
