# CCGuard Capture Hardening — fix the 413 silent-data-loss (Plan 5.1)

> Fixes a correctness bug found during Plan 6 live verification: `/v1/capture` rejects long sessions (HTTP 413, axum's default 2MB body limit) and the agent silently skips them. Violates the "capture everything" promise. This makes capture scale to arbitrarily large sessions.

**Root causes (all three get fixed):**
1. Server `/v1/capture` uses axum's default **2 MB** JSON body limit → large sessions (e.g. an 11 MB transcript) → **413**.
2. Agent capture loop advances the per-file byte offset **even when the POST fails** → rejected data is marked consumed and never retried = **silent loss**.
3. Server capture handler derives `event_count`/`first_ts`/`last_ts` from the **posted batch**, so splitting a session across multiple POSTs would store wrong totals.

**Fix strategy:** raise the server body limit (safety net) **+** chunk large sessions agent-side by a content-byte budget (real fix; each POST stays small) **+** recompute session aggregates from stored rows on the server (correct under chunking + idempotent re-posts) **+** only advance agent state on success (no silent loss). The server already inserts events idempotently (`on conflict (tenant_id, session_id, seq) do nothing`) and dedupes content by sha256, so chunked/repeated POSTs accumulate correctly.

**Stack:** Rust, axum 0.7, sqlx, reqwest (blocking). `DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres`. Commit identity **SenGuru / senthilguru246@gmail.com**. No push.

---

## Task 1 — server: raise capture body limit

**File:** `crates/ccguard-server/src/app.rs`.

- [ ] Import `use axum::extract::DefaultBodyLimit;`.
- [ ] Apply a generous limit to the capture route ONLY (leave other routes at the safe 2 MB default):
```rust
.route("/v1/capture", post(capture::capture).layer(DefaultBodyLimit::max(64 * 1024 * 1024)))
```
(64 MiB safety net; the agent chunks well under this.)
- [ ] Build clean. (Body-limit behavior is exercised by Task 3's large-body test.)

## Task 2 — server: recompute session aggregates from stored rows

**File:** `crates/ccguard-server/src/handlers/capture.rs`.

- [ ] Keep `classify` + the content-blob dedup + the idempotent event insert exactly as-is.
- [ ] Change the session upsert so the **per-batch** `event_count`/`first_ts`/`last_ts` are only provisional, then after inserting this batch's events, run ONE update that recomputes them from ALL stored rows for the session:
```rust
    // after the for-loop that inserts events:
    sqlx::query(
        "update captured_sessions s set \
           event_count = sub.cnt, \
           first_ts    = sub.min_ts, \
           last_ts     = sub.max_ts \
         from (select count(*) as cnt, min(ts) as min_ts, max(ts) as max_ts \
               from captured_events \
               where tenant_id = $1 and session_id = $2) sub \
         where s.tenant_id = $1 and s.session_id = $2",
    )
    .bind(&tenant_id)
    .bind(&s.session_id)
    .execute(&pool)
    .await?;
```
- [ ] The initial upsert can keep binding `s.events.len()`/batch first_ts/last_ts (they're immediately corrected by the update). Leave the `title = coalesce(...)` ON CONFLICT clause as-is so a later chunk doesn't null the title.
- [ ] `event_count` column is `integer`; `count(*)` returns `bigint` — cast: `event_count = sub.cnt::int` (or `count(*)::int as cnt`). Use whichever compiles; prefer `count(*)::int as cnt` in the subquery.

## Task 3 — server test: chunked capture + large body

**File:** `crates/ccguard-server/tests/capture.rs` (append; reuse existing helpers/ingest-token setup in that file).

- [ ] **Chunked-session test:** POST the SAME `session_id` twice with DISJOINT seq ranges (e.g. seqs 0–1 then 2–3), then GET `/v1/sessions/<id>/timeline` and assert all 4 events are present in seq order, AND GET `/v1/orgs/<tenant>/sessions` (or query the row) shows `event_count == 4` (proves aggregate recompute, not last-batch=2).
- [ ] **Large-body test:** POST one `CapturedSession` whose single event `content` is a ~3 MB string (`"x".repeat(3_000_000)`), assert `202` (would be `413` before Task 1). Then assert it's retrievable.
- [ ] `DATABASE_URL` set; `cargo test -p ccguard-server --test capture` green, plus full `cargo test -p ccguard-server` green (no regressions).

## Task 4 — agent: chunk large sessions + only advance state on success

**Files:** Create `crates/ccguard-agent/src/chunk.rs`; modify `crates/ccguard-agent/src/state.rs`, `crates/ccguard-agent/src/main.rs`, `crates/ccguard-agent/src/lib.rs`/module list as needed.

- [ ] **`chunk.rs` — pure, testable splitter.** A function that splits a `CapturedSession` into ≥1 `CapturedSession`s, each carrying the same metadata (session_id, user_email, repo, title, cwd) but a disjoint, ordered subset of events, so each chunk's total `content` bytes stays under a budget:
```rust
use ccguard_core::capture::CapturedSession;

/// Target content-bytes per POST. 3 MiB keeps each request far under the server's 64 MiB cap.
pub const CHUNK_CONTENT_BUDGET: usize = 3 * 1024 * 1024;

/// Split a session's events into byte-budgeted chunks (same metadata, disjoint ordered seqs).
/// Always emits ≥1 event per chunk (a single oversized event goes alone).
pub fn chunk_session(s: &CapturedSession, budget: usize) -> Vec<CapturedSession> {
    let mut chunks = Vec::new();
    let mut cur: Vec<_> = Vec::new();
    let mut cur_bytes = 0usize;
    for e in &s.events {
        let sz = e.content.as_ref().map(|c| c.len()).unwrap_or(0);
        if !cur.is_empty() && cur_bytes + sz > budget {
            chunks.push(meta_with(s, std::mem::take(&mut cur)));
            cur_bytes = 0;
        }
        cur.push(e.clone());
        cur_bytes += sz;
    }
    if !cur.is_empty() { chunks.push(meta_with(s, cur)); }
    if chunks.is_empty() { chunks.push(meta_with(s, Vec::new())); } // empty session → 1 metadata-only post
    chunks
}

fn meta_with(s: &CapturedSession, events: Vec<ccguard_core::capture::CapturedEvent>) -> CapturedSession {
    CapturedSession {
        session_id: s.session_id.clone(),
        user_email: s.user_email.clone(),
        repo: s.repo.clone(),
        title: s.title.clone(),
        cwd: s.cwd.clone(),
        events,
    }
}
```
(Adjust field names to the real `CapturedSession`/`CapturedEvent` structs — READ `crates/ccguard-core/src/capture.rs` first. `CapturedEvent` must derive `Clone`; if it doesn't, add `#[derive(Clone)]` to it in core. `Repo` likewise needs `Clone` — it almost certainly already derives it; if not, add it.)
- [ ] **Unit tests in `chunk.rs`:** (a) a session with several events and a tiny budget produces multiple chunks whose concatenated events equal the original in order with no dropped/duplicated seq; (b) a single event larger than the budget yields one chunk containing just that event; (c) every chunk preserves session_id + metadata.

- [ ] **`state.rs` — add a capture watermark** (max seq confirmed-sent per file), separate from the legacy byte `offsets` (which token mode still uses):
```rust
    #[serde(default)]
    pub capture_seqs: HashMap<String, i64>,
    // ...
    pub fn capture_watermark(&self, file: &str) -> i64 { *self.capture_seqs.get(file).unwrap_or(&-1) }
    pub fn set_capture_watermark(&mut self, file: &str, seq: i64) { self.capture_seqs.insert(file.to_string(), seq); }
```
Add a roundtrip test. `#[serde(default)]` so existing state files without the field still load.

- [ ] **`main.rs` capture branch — rework** (the `if args.capture { ... }` block):
  - For each transcript file: read the **whole file** (not `read_since` — capture needs the full file to recover session metadata/seqs; drop the byte-offset path for capture mode), parse via `transcript::parse_session`, populate identity/repo/session_id fallback exactly as today.
  - Let `wm = st.capture_watermark(&key)`. Filter to events with `seq > wm` (so unchanged files re-parse cheaply but re-POST nothing; a grown file posts only its new events). If none, continue.
  - Build a trimmed `CapturedSession` with just those events, then `chunk::chunk_session(&trimmed, chunk::CHUNK_CONTENT_BUDGET)`.
  - POST each chunk **in order** via `poster.post_capture`. On `Ok(202)`: advance a local `max_sent` to that chunk's max event seq. On any non-202 or `Err`: print a clear error (include the HTTP code) and **STOP processing this file** (break) — do NOT advance further, so the unsent tail retries next run.
  - After the loop for the file, if `max_sent > wm` call `st.set_capture_watermark(&key, max_sent)`. (Only the confirmed-sent high-water mark is persisted — no silent loss.)
  - Track counts: `captured` (chunks or sessions fully sent), `failed` (files with an unsent tail). Print a summary that distinguishes success from failure (e.g. `captured N session(s), M had send errors (will retry)`).
  - Keep the legacy `else` (token-event) branch unchanged.
- [ ] Register `mod chunk;` in `main.rs`.

- [ ] **Build + test:** `cargo build -p ccguard-agent` clean; `cargo test -p ccguard-agent` green (existing 17 + new chunk/state tests).

## Task 5 — full workspace verify + commit

- [ ] `DATABASE_URL` set; whole-workspace `cargo test` — ALL green (server incl. new capture tests, agent incl. chunk tests, core, web). Report the totals.
- [ ] `cargo build` workspace clean.
- [ ] Commit (identity MUST be SenGuru):
```
git -C "C:\Users\gsent\Desktop\2027-q1-projects\CCGuard" add -A
git -C "C:\Users\gsent\Desktop\2027-q1-projects\CCGuard" -c user.name=SenGuru -c user.email=senthilguru246@gmail.com commit -m "fix(capture): chunk large sessions + raise body limit + recompute aggregates (no more 413 data loss)"
```
Do NOT push.

## Self-review
**Covers all 3 defects:** body limit (T1), silent-loss-on-failure (T4 state-on-success-only), wrong-aggregates-under-chunking (T2 recompute). **Idempotency preserved:** server already `do nothing` on dup (tenant,session,seq) + sha256 content dedup, so chunked + retried POSTs converge. **Efficiency:** capture re-parses whole files each run (cheap, local) but the seq watermark means only new events are POSTed. **Edge:** a single event > 64 MiB still 413s (pathological); chunker keeps ≥1 event/chunk so it's attempted and the error is reported (not silent) — acceptable, note it. **Scope guard:** legacy `/v1/events` token path and its byte-offset state untouched.

## Execution
Build **subagent-driven** against live Postgres. After green, controller re-runs the live capture of a large (>2 MB) real transcript that previously 413'd and confirms it now lands with the correct event_count.
