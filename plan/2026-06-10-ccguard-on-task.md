# CCGuard On-Task Score + Role Profiles + Review Queue — Implementation Plan (Plan 9 of N)

> REQUIRED SUB-SKILL: superpowers:subagent-driven-development. The second axis: beyond *work-vs-personal repo*, did the session actually produce **on-task work**, and does each employee's activity match their **assigned role**? Admin defines work **per-repo with context** (handles "looks unrelated but is related"), assigns **job roles**, and works a **review queue** of indicators. Senthil's early ask: "make sure they are on task no matter what" — indicators, NOT auto-verdicts.

**Design (two-axis, metadata-only):** repo-attribution (Plans 1/5, work/personal/unknown) × **output-landing + task-alignment**. On-task signals from captured data: repo class · did the session produce a commit/PR (detected from Bash `git commit` / `pr` events) · does it reference a tracked ticket (JIRA `KEY-123` / GH `#123`) · abandoned session. Score → `on_task` / `review` / `off_task` with reasons. **Role profiles:** admin assigns each employee a job role (engineer/marketer/designer/pm/ops/sales/other); activity that contradicts the role (e.g. a marketer producing heavy code) raises an indicator. **Per-repo work-definition:** admin override (work/personal/unknown + context note) consulted BEFORE the org allowlist. Indicators land in a **review queue** (open→reviewed). Live Jira/Linear status/assignee resolution needs a PAT → noted as a follow-on; ticket-*reference* detection works fully offline and is the on-task signal here.

**Stack:** Rust, axum 0.7, sqlx + Postgres 17, maud, regex. `DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres`. Commit identity **SenGuru / senthilguru246@gmail.com**. No push.

## Roadmap position
Plans 1–8 ✅. **Plan 9 ← this (final in the locked sequence).** Remaining-after: SCM webhook (commit/PR *survival*), live tracker sync, network/cloud collectors, OTel receiver, allowlist-UI, consent, Stripe.

---

## Task 1 — core: on-task signals + scoring + ticket extraction (`ccguard-core::ontask`)

**Files:** Create `crates/ccguard-core/src/ontask.rs`; `pub mod ontask;` in lib.rs.

- [ ] **Ticket extraction (pure, regex):**
```rust
/// Extract ticket references: JIRA-style KEY-123 and GitHub-style #123. De-duped, order-preserving.
pub fn extract_ticket_refs(content: &str) -> Vec<String> { ... }
```
Patterns: `\b[A-Z][A-Z0-9]{1,9}-\d+\b` (JIRA) and `(?:^|\s)#(\d{1,7})\b` (GH issue, store as `#123`). Avoid matching obvious non-tickets where easy (the JIRA regex requiring an uppercase prefix already filters most prose). De-dupe preserving first-seen order.

- [ ] **Signals + score:**
```rust
use serde::{Deserialize, Serialize};
use crate::event::Classification;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnTaskSignals {
    pub classification: Classification, // work | personal | unknown
    pub committed: bool,        // session produced a git commit (Bash) or PR event
    pub pr_opened: bool,        // a `pr` event present
    pub ticket_referenced: bool,
    pub total_events: i64,
    pub assistant_events: i64,  // assistant_text count (for abandoned heuristic)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnTaskLabel { OnTask, Review, OffTask }

/// 0..100 score + label + human reasons. Deterministic, documented weights.
pub fn score(s: &OnTaskSignals) -> (i32, OnTaskLabel, Vec<String>) {
    let mut score = 50i32;
    let mut reasons = vec![];
    match s.classification {
        Classification::Work => { score += 25; }
        Classification::Unknown => { score -= 10; reasons.push("unclassified repo".into()); }
        Classification::Personal => { score -= 40; reasons.push("personal repo".into()); }
    }
    if s.committed { score += 15; reasons.push("produced a commit".into()); } else { reasons.push("no commit landed".into()); }
    if s.pr_opened { score += 10; reasons.push("opened a PR".into()); }
    if s.ticket_referenced { score += 10; reasons.push("references a tracked ticket".into()); }
    // abandoned: had events but essentially no assistant output
    let abandoned = s.total_events >= 2 && s.assistant_events == 0;
    if abandoned { score -= 20; reasons.push("abandoned session (no output)".into()); }
    let score = score.clamp(0, 100);
    let label = if score >= 70 { OnTaskLabel::OnTask } else if score >= 40 { OnTaskLabel::Review } else { OnTaskLabel::OffTask };
    (score, label, reasons)
}
```
- [ ] **Tests:** a work session with commit + ticket → high score, `OnTask`. A personal-repo session, no commit → low score, `OffTask`, reason "personal repo". An unknown-repo session with no commit → mid, `Review`. Abandoned (total_events 5, assistant 0) subtracts. `extract_ticket_refs("fix PROJ-42 and #17 and lowercase no-12")` → `["PROJ-42","#17"]` (not `no-12`). `score` is deterministic + clamped 0..100. ~10 assertions.
- [ ] `cargo test -p ccguard-core` green.

---

## Task 2 — core: job roles + role-anomaly indicators (`ccguard-core::roles`)

**Files:** Create `crates/ccguard-core/src/roles.rs`; `pub mod roles;` in lib.rs.

- [ ] **Types + logic (pure):**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobRole { Engineer, Marketer, Designer, Pm, Ops, Sales, Other }

impl JobRole {
    pub fn from_str(s: &str) -> JobRole { /* parse, default Other */ }
    pub fn as_str(&self) -> &'static str { /* snake_case */ }
    /// Whether producing code is expected for this role.
    pub fn expects_code(&self) -> bool { matches!(self, JobRole::Engineer | JobRole::Ops) }
}

/// Observed activity for one session (or rollup): how much "code" work happened.
pub struct Activity { pub code_events: i64, pub total_events: i64 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleIndicator { pub kind: String, pub detail: String } // kind e.g. "non_engineer_coding"

/// Indicators when activity contradicts the role. Empty if consistent.
pub fn role_anomalies(role: JobRole, a: &Activity) -> Vec<RoleIndicator> {
    let mut out = vec![];
    // A non-coding role producing significant code = shadow engineering, surface it.
    if !role.expects_code() && a.code_events >= 5 {
        out.push(RoleIndicator { kind: "non_engineer_coding".into(),
            detail: format!("{} code-events from a {} role", a.code_events, role.as_str()) });
    }
    out
}
```
(`code_events` = count of `file_edit` + `tool_call` with a code tool — the server computes it; core just scores the counts.)
- [ ] **Tests:** `JobRole::from_str`/`as_str` round-trip + default Other; `expects_code` true for engineer/ops, false for marketer/designer/pm/sales/other; `role_anomalies(Marketer, {code_events:8})` → one `non_engineer_coding`; `role_anomalies(Engineer, {code_events:8})` → empty; `role_anomalies(Marketer, {code_events:2})` → empty (below threshold).
- [ ] `cargo test -p ccguard-core` green.

---

## Task 3 — server: schema + per-repo override + score-at-capture + indicators + APIs

**Files:** Create `crates/ccguard-server/migrations/0009_ontask.sql`, `crates/ccguard-server/src/handlers/ontask.rs`; modify `handlers/capture.rs`, `handlers/mod.rs`, `app.rs`; tests in a new `tests/ontask.rs`.

- [ ] **Migration `0009_ontask.sql`:**
```sql
create table if not exists repo_overrides (
    id bigserial primary key, tenant_id text not null,
    repo_host text not null, repo_org text not null, repo_name text not null,
    classification text not null,      -- work | personal | unknown
    note text, updated_at timestamptz not null default now(),
    unique (tenant_id, repo_host, repo_org, repo_name)
);
create table if not exists employee_roles (
    tenant_id text not null, user_email text not null,
    job_role text not null, note text, updated_at timestamptz not null default now(),
    primary key (tenant_id, user_email)
);
create table if not exists session_scores (
    tenant_id text not null, session_id text not null,
    score int not null, label text not null, reasons text,
    updated_at timestamptz not null default now(),
    primary key (tenant_id, session_id)
);
create table if not exists indicators (
    id bigserial primary key, tenant_id text not null,
    user_email text, session_id text, kind text not null, detail text,
    status text not null default 'open',  -- open | reviewed | dismissed
    created_at timestamptz not null default now()
);
create index if not exists indicators_tenant_idx on indicators (tenant_id, status);
create table if not exists session_tickets (
    tenant_id text not null, session_id text not null, ticket text not null,
    primary key (tenant_id, session_id, ticket)
);
```
- [ ] **Per-repo override in classification** (`handlers/capture.rs`): BEFORE calling `ccguard_core::classify::classify`, look up `repo_overrides` for `(tenant, s.repo.host, s.repo.org, s.repo.name)`; if found, use its `classification` (parse to `Classification`) and remember the `note`. Else use the allowlist classify result. Store the resulting classification on the session as today. (If host/org/name are null, skip the override lookup.)
- [ ] **Score-at-capture** (`handlers/capture.rs`, after events inserted + classification known):
  - Compute signals from the session's events: `committed` = any event where (kind `pr`) OR (kind `tool_call` AND tool_name in (Bash) AND target/content contains `git commit` OR `git push`); `pr_opened` = any `pr` event; `ticket_referenced` = any event content yields `ontask::extract_ticket_refs(..)` non-empty (also INSERT each ref into `session_tickets`); `total_events`, `assistant_events` from counts.
  - `let (sc, label, reasons) = ontask::score(&signals);` upsert `session_scores`.
  - **Raise indicators** (idempotent — clear+reinsert this session's auto-indicators, or insert-on-conflict-do-nothing keyed by (tenant,session,kind)): if `label == OffTask` → indicator kind `off_task`; if classification personal → kind `personal_repo`; role anomaly: look up `employee_roles` for `s.user_email`, compute `code_events` (count file_edit + tool_call with code tools Edit/Write/Bash), call `roles::role_anomalies(role, ..)`, insert each. Indicators carry user_email + session_id + detail. Use a unique key `(tenant_id, session_id, kind)` to stay idempotent on re-capture — add that unique index OR `on conflict do nothing` with a partial unique. (Add `unique (tenant_id, session_id, kind)` to indicators for auto-raised ones — simplest: include it in the migration and `on conflict do nothing`.)
  - Keep this defensive: scoring failures must not fail the capture (log + continue), but in tests it should work.
- [ ] **APIs** (`handlers/ontask.rs`, register in mod.rs):
  - `POST /v1/orgs/:tenant/repo-overrides` (AuthedUser owner/admin): `{repo_host,repo_org,repo_name,classification,note}` upsert. 200.
  - `POST /v1/orgs/:tenant/roles` (AuthedUser owner/admin): `{user_email, job_role, note}` upsert employee_roles. 200.
  - `GET /v1/orgs/:tenant/indicators?status=open` (AuthedUser): list indicators. JSON.
  - `POST /v1/indicators/:id/status` (AuthedUser, tenant-scoped via the row's tenant matching the user): `{status}` (reviewed|dismissed). 200.
  - `GET /v1/orgs/:tenant/ontask` (AuthedUser): per-employee rollup — avg score, session counts by label, open-indicator count (group session_scores join sessions by user_email). JSON.
- [ ] **Routes** in `app.rs` (keep existing).
- [ ] **Tests (`tests/ontask.rs`):**
  - Capture a WORK session (repo allowlisted) whose events include a Bash `git commit` + content referencing `PROJ-7` → `session_scores` row has high score + label `on_task`; `session_tickets` has `PROJ-7`.
  - Capture a PERSONAL-repo session, no commit → score low, label `off_task`, and an `indicators` row kind `personal_repo` + an `off_task` indicator exist.
  - Set a repo override (an unknown repo → `work` with a note) via the API, then capture a session on that repo → classification stored is `work` (override beat the allowlist).
  - Assign role `marketer` to `sam@acme`, capture a session by sam with >=5 Edit/Write events → an `indicators` row kind `non_engineer_coding`.
  - `GET /v1/orgs/:t/indicators?status=open` lists them; `POST /v1/indicators/:id/status {reviewed}` flips it; re-GET open no longer includes it.
  - Re-capturing the same session does NOT duplicate indicators (idempotent).
- [ ] `cargo test -p ccguard-server` green.

---

## Task 4 — dashboard: review queue + roles/overrides admin + on-task on session view + KPI

**Files:** modify `crates/ccguard-server/src/web.rs`, `app.rs`; tests in `tests/web.rs`.

- [ ] **CSS:** add label badge classes `.on_task{green} .review{amber} .off_task{red}` (reuse existing palette).
- [ ] **`GET /dashboard/review`** (WebUser): the indicators review queue — table (created, user, kind badge, detail, session link, status) for `status='open'` by default, with a small filter; each row has **Reviewed** / **Dismiss** buttons (`POST /dashboard/indicators/:id/status` form → redirect back). Empty state "queue clear".
- [ ] **`GET /dashboard/roles`** + **`POST /dashboard/roles`** (WebUser owner/admin): assign job role to an employee (email + role select + note) → upsert; list current employee_roles. On the same page (or `/dashboard/repos`), a **per-repo work-definition** form: host/org/name + classification select + context note → upsert `repo_overrides`; list current overrides. (One combined "Policy/Definitions" page is fine.)
- [ ] **Session view** (`session_view`): show the session's on-task **score + label badge** + reasons (from `session_scores`), and any **indicators** for the session, near the header.
- [ ] **Dashboard KPI:** add an **On-task** line — `% on_task sessions` (from session_scores) + **open indicators** count linking to `/dashboard/review`. Extend the top nav with `review · roles`.
- [ ] **`POST /dashboard/indicators/:id/status`** (WebUser): set status, redirect to `/dashboard/review`.
- [ ] **Tests (`tests/web.rs`):** capture a personal-repo session (raises indicators), login, GET `/dashboard/review` → 200 shows the indicator kind + session; POST the reviewed-status form → 303, indicator leaves the open queue. GET `/dashboard/roles` → 200 shows the form; POST assign a role → 303 + listed. No-cookie `/dashboard/review` → 303 /login.
- [ ] `cargo test -p ccguard-server` green.

---

## Task 5 — full verify + commit

- [ ] `DATABASE_URL` set; whole-workspace `cargo test` ALL green; `cargo build` clean.
- [ ] Commit (identity MUST be SenGuru):
```
git -C "C:\Users\gsent\Desktop\2027-q1-projects\CCGuard" add -A
git -C "C:\Users\gsent\Desktop\2027-q1-projects\CCGuard" -c user.name=SenGuru -c user.email=senthilguru246@gmail.com commit -m "feat(on-task): on-task scoring + role anomalies + per-repo overrides + review queue"
```
(Per-task commits also fine.) Do NOT push.

## Self-review
**Covers Senthil's asks:** per-repo work-definition + context note (overrides the allowlist, handles "looks unrelated but is related") ✅; admin-assigned job roles + role-anomaly indicators ("ensure on-task no matter what") ✅; on-task score from metadata (repo × commit × PR × ticket × abandoned) ✅; review queue of **indicators, not verdicts** ✅. **Pure core:** scoring + role logic + ticket extraction are pure + tested. **Idempotent:** session_scores upsert + indicators unique (tenant,session,kind) `on conflict do nothing` → re-capture doesn't duplicate. **Honest scope:** commit/PR *survival* (merged-PR) needs an SCM webhook → future; live Jira/Linear status/assignee needs a PAT → future; ticket *reference* detection is fully built + is the alignment signal now. **Guardrail:** indicators feed a human review queue (no automated punishment); consistent with monitoring company-provided tooling.

## Execution
Build **subagent-driven** (1→2 core, then 3 server, then 4 UI). After green, controller captures a work-with-commit-and-ticket session (→ on_task), a personal-no-commit session (→ off_task + indicators), and a marketer-coding session (→ role anomaly), then confirms `/dashboard/review` + the session-view on-task badge in a browser.
