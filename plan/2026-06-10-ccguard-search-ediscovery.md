# CCGuard Search + eDiscovery + Findings — Implementation Plan (Plan 7 of N)

> REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Extends the total-visibility spec (`spec/2026-06-10-total-visibility-addendum.md`). Turns the captured record into an investigable corpus: full-text search, first-class **Findings** (secret/PII/credential detection), and eDiscovery **export + legal hold**.

**Goal:** Make the captured record searchable and defensible. (1) Full-text **search** across all captured content with snippets. (2) **Findings** — every event's content scanned for secrets/credentials/PII at capture time, stored as first-class filterable rows, surfaced on the dashboard + session timeline. (3) **eDiscovery export** of a full session as NDJSON + a **legal hold** flag.

**Architecture:** Findings detection lives in `ccguard-core::findings` (pure, regex + Luhn, heavily unit-tested) so it's deterministic and testable. The server runs the scanner at capture time and stores rows in a new `findings` table; full-text search uses a Postgres `tsvector` generated column + GIN index on `content_blobs`. New read APIs + maud dashboard pages (search, findings) reuse the Plan 6 `WebUser` cookie auth + `page()` chrome. Export is a streaming NDJSON download; hold is a boolean on `captured_sessions`.

**Stack:** Rust, axum 0.7, sqlx 0.8 + Postgres 17, `regex` (new dep in core), maud. `DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres`. Commit identity **SenGuru / senthilguru246@gmail.com**. No push.

## Roadmap position
Plans 1–6 ✅ + Plan 5.1 (413 fix) ✅. **Plan 7 ← this.** Then Plan 8 managed-settings enforcement+MDM · Plan 9 on-task score+tracker connector.

---

## Task 1 — Findings core: secret/PII/credential scanner (pure)

**Files:** `crates/ccguard-core/Cargo.toml` (add `regex = "1"` and `once_cell = "1"`); Create `crates/ccguard-core/src/findings.rs`; register `pub mod findings;` in `crates/ccguard-core/src/lib.rs`.

- [ ] **`Finding` type + scanner:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    pub kind: FindingKind,      // secret | pii
    pub rule: String,           // e.g. "aws_access_key", "github_token", "email", "credit_card"
    pub severity: Severity,     // high | medium | low
    pub redacted: String,       // safe preview: first/last few chars, middle masked
    pub start: usize,           // byte offset in the scanned content
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind { Secret, Pii }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity { High, Medium, Low }
```
- [ ] **`pub fn scan(content: &str) -> Vec<Finding>`** that applies a fixed rule set. Use `once_cell::sync::Lazy` for compiled `Regex`es. Rules (high-precision, provider-prefixed where possible to avoid false-positive noise):
  - **Secrets (severity High):**
    - `aws_access_key`: `AKIA[0-9A-Z]{16}`
    - `github_token`: `gh[posru]_[A-Za-z0-9]{36,}`
    - `anthropic_key`: `sk-ant-[A-Za-z0-9_-]{20,}`
    - `openai_key`: `sk-[A-Za-z0-9]{20,}` (run AFTER anthropic so `sk-ant-` matches the more specific rule; or require not preceded by `ant-` — simplest: check anthropic first and skip openai matches that start at the same spot)
    - `slack_token`: `xox[baprs]-[A-Za-z0-9-]{10,}`
    - `stripe_secret`: `(sk|rk)_live_[A-Za-z0-9]{16,}`
    - `google_api_key`: `AIza[0-9A-Za-z_-]{35}`
    - `jwt`: `eyJ[A-Za-z0-9_-]{8,}\.eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}`
    - `private_key`: `-----BEGIN (RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY-----`
  - **PII (severity Medium):**
    - `email`: `[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}`
    - `us_ssn`: `\b\d{3}-\d{2}-\d{4}\b`
    - `credit_card`: candidate `\b(?:\d[ -]?){13,16}\b`, then strip non-digits and **validate with Luhn in code** (a pure `fn luhn_valid(digits: &str) -> bool`); only emit if Luhn passes (drops most false positives).
  - **`redacted`**: helper `fn redact(m: &str) -> String` showing first 4 + `…` + last 2 chars (or all-masked if ≤6 chars). NEVER store the full secret in `redacted` (raw stays only in `content_blobs`).
  - De-dupe: if multiple rules match the exact same span, keep the most specific (first in a priority order). Returning a couple of overlapping findings is acceptable; avoid emitting both openai+anthropic for one `sk-ant-` token.
- [ ] **Unit tests (thorough):** each rule matches a positive example and ignores a near-miss; `sk-ant-...` yields exactly one `anthropic_key` (not also `openai_key`); a Luhn-valid test card (`4242 4242 4242 4242`) is found, a Luhn-invalid 16-digit string is NOT; `redact` never returns the full input for inputs >6 chars; `scan` on benign prose returns empty; an email is found. Aim for ~12+ assertions.
- [ ] `cargo test -p ccguard-core` green.

---

## Task 2 — Server: findings table + scan at capture + findings API + dashboard surface

**Files:** Create `crates/ccguard-server/migrations/0005_findings.sql`; modify `crates/ccguard-server/src/handlers/capture.rs`, `crates/ccguard-server/src/web.rs`, `crates/ccguard-server/src/app.rs`; add tests to `crates/ccguard-server/tests/capture.rs` and `tests/web.rs`.

- [ ] **Migration `0005_findings.sql`:**
```sql
create table if not exists findings (
    id         bigserial primary key,
    tenant_id  text not null,
    session_id text not null,
    seq        bigint not null,
    kind       text not null,        -- secret | pii
    rule       text not null,
    severity   text not null,        -- high | medium | low
    redacted   text not null,
    created_at timestamptz not null default now(),
    unique (tenant_id, session_id, seq, rule, redacted)
);
create index if not exists findings_tenant_idx on findings (tenant_id, severity);
create index if not exists findings_session_idx on findings (tenant_id, session_id);
```
- [ ] **Scan at capture time** — in `handlers/capture.rs`, inside the event loop, when an event has content, call `ccguard_core::findings::scan(content)` and insert each finding (idempotent `on conflict do nothing` on the unique key). Keep it in the same request flow; it's cheap. (Scan the event's content string you already have.)
- [ ] **APIs** (AuthedUser, tenant-scoped, mirror `timeline.rs`):
  - `GET /v1/orgs/:tenant/findings` → JSON list (newest first, limit 200) with session_id/seq/kind/rule/severity/redacted.
- [ ] **Dashboard surface (maud, in `web.rs`):**
  - On `/dashboard`: add a **Findings** KPI line — counts by severity (e.g. `high N · medium N`) with a link to `/dashboard/findings`.
  - New `GET /dashboard/findings` (WebUser): a table of findings (severity badge, rule, kind, redacted snippet, session link `/dashboard/sessions/<id>` anchored to the seq). Reuse `page()` + badge CSS (add `.high{...}.medium{...}.low{...}` severity colors to the CSS const).
  - On the **session timeline** (`session_view`): for each event that has findings, render a small inline `⚠ rule (severity)` marker above/!in the event card. (Query findings for the session once, group by seq, render on matching events.)
- [ ] **Routes** in `app.rs`: `.route("/v1/orgs/:tenant/findings", get(timeline::findings))` (or put the handler in a new `handlers/findings.rs`) and `.route("/dashboard/findings", get(web::findings))`.
- [ ] **Tests:**
  - `tests/capture.rs`: POST a session whose event content contains `AKIA` + a test secret and an email → assert rows land in `findings` (query the table) with the right rules/severity, and that `redacted` is NOT the raw secret.
  - `tests/web.rs`: login + GET `/dashboard/findings` with cookie → 200 + body contains the rule name; no-cookie → 303 → /login.
- [ ] `cargo test -p ccguard-server` green.

---

## Task 3 — Full-text search

**Files:** Create `crates/ccguard-server/migrations/0006_fts.sql`; modify `crates/ccguard-server/src/web.rs`, `crates/ccguard-server/src/app.rs`, and a search handler (in `handlers/`); tests in `tests/web.rs`.

- [ ] **Migration `0006_fts.sql`** — generated tsvector + GIN index (truncate input to stay under Postgres's ~1 MB tsvector limit):
```sql
alter table content_blobs
  add column if not exists content_tsv tsvector
  generated always as (to_tsvector('english', left(content, 800000))) stored;
create index if not exists content_blobs_tsv_idx on content_blobs using gin (content_tsv);
```
- [ ] **Search API** `GET /v1/orgs/:tenant/search?q=...` (AuthedUser): join `content_blobs` (tsv match) → `captured_events` (same tenant + content_sha) → `captured_sessions`, return up to 100 rows: session_id, seq, kind, session title/repo, and a `ts_headline('english', left(content,800000), websearch_to_tsquery('english',$q))` snippet. Guard empty `q` → empty result. Use `websearch_to_tsquery` (handles user query syntax safely).
```sql
select e.session_id, e.seq, e.kind, s.title, s.repo_org, s.repo_name,
       ts_headline('english', left(b.content,800000), websearch_to_tsquery('english',$2),
                   'MaxFragments=2, MinWords=3, MaxWords=12') as snippet
from content_blobs b
join captured_events e on e.tenant_id=b.tenant_id and e.content_sha=b.sha256
join captured_sessions s on s.tenant_id=e.tenant_id and s.session_id=e.session_id
where b.tenant_id=$1 and b.content_tsv @@ websearch_to_tsquery('english',$2)
order by e.session_id, e.seq limit 100
```
- [ ] **Dashboard `GET /dashboard/search?q=...`** (WebUser): a search box (GET form) + results list — each result shows the snippet (rendered, but `ts_headline` `<b>` tags must be allowed: build the highlighted snippet with `maud::PreEscaped` ONLY around the server-generated headline, and HTML-escape the surrounding text; simplest safe approach: strip `ts_headline`'s default `<b>`/`</b>`, or set `StartSel`/`StopSel` to a sentinel and render as escaped text with the matched term wrapped — to avoid XSS from content, DO NOT PreEscape raw content). Recommended: set `ts_headline(... , 'StartSel=«, StopSel=»')` so the snippet is plain text with `«match»` markers, render it maud-escaped (safe), and style is optional. Link each result to `/dashboard/sessions/<id>`.
- [ ] **Routes** in `app.rs`. Add a nav link to `/dashboard/search` from the dashboard.
- [ ] **Test** (`tests/web.rs`): capture a session with content containing a distinctive token (e.g. `zphybvqx_marker`), login, GET `/dashboard/search?q=zphybvqx_marker` with cookie → 200 + body contains the session link / snippet; a query with no hits → 200 + "no results".
- [ ] `cargo test -p ccguard-server` green.

---

## Task 4 — eDiscovery export (NDJSON) + legal hold

**Files:** Create `crates/ccguard-server/migrations/0007_hold.sql`; modify `crates/ccguard-server/src/web.rs`, `crates/ccguard-server/src/app.rs`; tests in `tests/web.rs`.

- [ ] **Migration `0007_hold.sql`:** `alter table captured_sessions add column if not exists on_hold boolean not null default false;`
- [ ] **Export** `GET /dashboard/sessions/:id/export` (WebUser, tenant-scoped) → `Content-Type: application/x-ndjson`, `Content-Disposition: attachment; filename="<session>.ndjson"`. Body = one JSON object per line: first a `{"type":"session", ...meta...}` line, then one `{"type":"event","seq":..,"kind":..,"tool_name":..,"target":..,"content":..,"tokens_in":..,"tokens_out":..,"is_sidechain":..}` line per event in seq order (content joined from `content_blobs`). Build the string server-side and return as `(headers, String)`. This is the defensible "hand the full record to legal/IR" artifact.
- [ ] **Legal hold** `POST /dashboard/sessions/:id/hold` (WebUser) → toggles `on_hold`, redirect back to the session view. On `session_view`, show the hold state + a toggle button ("Place legal hold" / "Release hold"), and show a 🔒 marker in the session header when held. (We never delete data; hold is a visible compliance flag + intent marker, self-audited.)
- [ ] **Routes** in `app.rs`. Add an "Export NDJSON" link on the session view.
- [ ] **Tests** (`tests/web.rs`): (a) capture a session, login, GET `…/export` with cookie → 200, `content-disposition` attachment, body has a `"type":"session"` line + an event line containing the event content. (b) POST `…/hold` toggles `on_hold` true (query the row), GET session view shows the held state.
- [ ] `cargo test -p ccguard-server` green.

---

## Task 5 — full workspace verify + commit

- [ ] `DATABASE_URL` set; whole-workspace `cargo test` ALL green (core findings, server capture/web/search/findings/export, agent unchanged). `cargo build` clean.
- [ ] Commit (identity MUST be SenGuru):
```
git -C "C:\Users\gsent\Desktop\2027-q1-projects\CCGuard" add -A
git -C "C:\Users\gsent\Desktop\2027-q1-projects\CCGuard" -c user.name=SenGuru -c user.email=senthilguru246@gmail.com commit -m "feat(ediscovery): full-text search + secret/PII findings + NDJSON export + legal hold"
```
Do NOT push. (One commit per task is also fine — implementer's choice — as long as each is green.)

## Self-review
**Coverage:** search ✅(T3) · findings/secret-PII ✅(T1+T2) · export+hold ✅(T4). **Reuse:** WebUser/page/badge from Plan 6; AuthedUser/timeline query shapes from Plan 5; classify untouched. **Security:** findings store REDACTED previews (raw only in the already-captured blob); search snippets use a sentinel (`«»`) + maud escaping so captured content can't inject HTML (no PreEscape of raw content — only of server-controlled markup). **Scale:** tsvector truncates to 800 KB to dodge Postgres's tsvector size limit (note it; large blobs are still fully stored + exportable, just FTS-indexed on the prefix). **Idempotency:** findings unique key → re-capture/chunked posts don't duplicate. **Guardrails:** findings detect secrets/PII for compliance — this is the "is sensitive data leaking into prompts" lens, on company-provided tooling, consistent with the locked direction; no new "never"-line crossings.

## Execution
Build **subagent-driven**, one implementer per task (or batch T1→T2 since T2 depends on T1). After green, controller runs the server, captures a session containing a planted secret + a searchable marker, and confirms in a browser: `/dashboard/findings` lists the secret (redacted), `/dashboard/search?q=...` finds it, and the NDJSON export downloads the full record.
