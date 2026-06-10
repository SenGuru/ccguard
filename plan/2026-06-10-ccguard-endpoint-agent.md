# CCGuard Endpoint Agent — Implementation Plan (Plan 4 of N)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** A `ccguard-agent` binary that reads the local Claude Code transcripts (`~/.claude/projects/*/*.jsonl`), extracts per-interaction token usage + model, attributes each to its git repo, and POSTs `CcEvent`s to a CCGuard server with an ingest token — so CCGuard monitors *real* Claude Code usage. Metadata-only (tokens/model/repo/timing — **no prompt content**). Visible/non-covert.

**Architecture:** New `ccguard-agent` crate that depends on `ccguard-core` (reusing `CcEvent`/`Repo`/`parse_remote_url`). Pure, unit-tested modules: `paths` (find transcripts), `parse` (JSONL → `Interaction`), `pricing` (token→cost estimate), `repo` (remote URL → `Repo`), `event` (`Interaction`→`CcEvent`), `state` (per-file byte offsets for incrementality). Thin I/O glue: `poster` (reqwest blocking POST) and `main` (CLI). End-to-end is proven by a live smoke test (controller-run), mirroring prior plans.

**Tech Stack:** Rust, `ccguard-core`, serde/serde_json, chrono, anyhow, `reqwest` 0.12 (blocking), `clap` 4, `dirs` 5. Commit identity `senthilguru246@gmail.com` / `SenGuru`.

---

## Roadmap position
Plans 1–3 ✅ (engine, tenant/ingest auth, user accounts). **Plan 4 ← this (the endpoint agent — first real data source).** Then: allowlist-management API, dashboard UI, more collectors (OTel/SCM/network), consent, Stripe.

## Design decisions
- **Metadata tier only** for v1: send model, input/output tokens, an estimated cost, repo (host/org/name/path from git remote), timestamp, session id. **No prompt/content text.**
- **One `CcEvent` per assistant interaction** that has a `usage` block.
- **Repo attribution** = run `git -C <cwd> config --get remote.origin.url` then `ccguard_core::remote::parse_remote_url`. The cwd comes from the transcript lines (authoritative). No remote → repo with `path` only (server classifies unknown/by path).
- **Incremental**: per-file byte offsets persisted in `<claude_dir>/ccguard-agent-state.json`; only new bytes are processed each run.
- **Server is authoritative** on tenant (from token) and classification (from allowlist) — the agent sends `tenant_id: ""`.
- v1 runs **once per invocation** (a scheduler/loop is a later concern).

## Prerequisites
- [ ] `git` on PATH. Plans 1–3 on `master`. (Live smoke test needs the running server + Postgres.)

## File structure (this plan)
```
Cargo.toml                                  # workspace: + ccguard-agent member
crates/ccguard-agent/
  Cargo.toml
  src/main.rs        # CLI + glue (read_since, read_claude_email)
  src/paths.rs       # list_transcripts
  src/state.rs       # offset persistence
  src/parse.rs       # transcript JSONL -> Interaction
  src/pricing.rs     # estimate_cost
  src/repo.rs        # repo_from_remote / repo_for_cwd
  src/event.rs       # interaction_to_event
  src/poster.rs      # HTTP POST
```

---

## Task 1: Crate scaffold + `paths` + `state`

**Files:** Modify `Cargo.toml` (root); Create `crates/ccguard-agent/Cargo.toml`, `src/main.rs` (temporary minimal), `src/paths.rs`, `src/state.rs`.

- [ ] **Step 1: Add the workspace member.** In root `Cargo.toml`, set:
```toml
members = ["crates/ccguard-core", "crates/ccguard-server", "crates/ccguard-agent"]
```

- [ ] **Step 2: Crate manifest** `crates/ccguard-agent/Cargo.toml`:
```toml
[package]
name = "ccguard-agent"
edition.workspace = true
version.workspace = true

[dependencies]
ccguard-core = { path = "../ccguard-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
reqwest = { version = "0.12", default-features = false, features = ["json", "blocking", "rustls-tls"] }
clap = { version = "4", features = ["derive"] }
dirs = "5"
```

- [ ] **Step 3: `paths.rs`** `crates/ccguard-agent/src/paths.rs`:
```rust
use std::path::{Path, PathBuf};

/// List transcript files: `<claude_dir>/projects/<encoded-cwd>/<session>.jsonl`.
/// (Subagent transcripts under `<session>/subagents/` are skipped in v1.)
pub fn list_transcripts(claude_dir: &Path) -> Vec<PathBuf> {
    let projects = claude_dir.join("projects");
    let mut out = Vec::new();
    if let Ok(dirs) = std::fs::read_dir(&projects) {
        for d in dirs.flatten() {
            let p = d.path();
            if p.is_dir() {
                if let Ok(files) = std::fs::read_dir(&p) {
                    for f in files.flatten() {
                        let fp = f.path();
                        if fp.extension().map(|e| e == "jsonl").unwrap_or(false) {
                            out.push(fp);
                        }
                    }
                }
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_jsonl_transcripts_only() {
        let tmp = std::env::temp_dir().join(format!("ccg_paths_{}", std::process::id()));
        let proj = tmp.join("projects").join("C--Users-x-repo");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join("s1.jsonl"), "{}").unwrap();
        std::fs::write(proj.join("notes.txt"), "ignore").unwrap();

        let found = list_transcripts(&tmp);
        std::fs::remove_dir_all(&tmp).ok();

        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("s1.jsonl"));
    }
}
```

- [ ] **Step 4: `state.rs`** `crates/ccguard-agent/src/state.rs`:
```rust
use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Per-file byte offsets already processed (so re-runs only handle new bytes).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub offsets: HashMap<String, u64>,
}

impl State {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, serde_json::to_string_pretty(self).unwrap_or_default())
    }

    pub fn offset(&self, file: &str) -> u64 {
        *self.offsets.get(file).unwrap_or(&0)
    }

    pub fn set(&mut self, file: &str, off: u64) {
        self.offsets.insert(file.to_string(), off);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_and_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("ccg_state_{}.json", std::process::id()));
        let mut s = State::default();
        assert_eq!(s.offset("a"), 0);
        s.set("a", 42);
        s.save(&tmp).unwrap();

        let loaded = State::load(&tmp);
        std::fs::remove_file(&tmp).ok();
        assert_eq!(loaded.offset("a"), 42);
        assert_eq!(loaded.offset("missing"), 0);
    }
}
```

- [ ] **Step 5: Temporary `main.rs`** so the crate builds now (replaced in Task 4) `crates/ccguard-agent/src/main.rs`:
```rust
mod paths;
mod state;

fn main() {
    println!("ccguard-agent (scaffold)");
}
```

- [ ] **Step 6: Verify + commit**

Run: `cargo test -p ccguard-agent` → expect 2 passed (paths + state). Then `cargo build -p ccguard-agent`.
```
git add crates/ccguard-agent Cargo.toml
git commit -m "feat(agent): crate scaffold + transcript discovery + offset state"
```

---

## Task 2: `parse` — transcript JSONL → interactions

**Files:** Create `crates/ccguard-agent/src/parse.rs`; Modify `src/main.rs` (add `mod parse;`).

- [ ] **Step 1: `parse.rs`**:
```rust
use serde::Deserialize;

/// One billable assistant interaction extracted from a transcript.
#[derive(Debug, Clone, PartialEq)]
pub struct Interaction {
    pub session_id: String,
    pub ts: String, // RFC3339 timestamp string from the transcript
    pub cwd: Option<String>,
    pub model: String,
    pub tokens_in: i64,
    pub tokens_out: i64,
}

#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(rename = "sessionId", default)]
    session_id: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    message: Option<Msg>,
}

#[derive(Deserialize)]
struct Msg {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize, Default)]
struct Usage {
    #[serde(default)]
    input_tokens: i64,
    #[serde(default)]
    output_tokens: i64,
}

/// Parse newline-delimited transcript JSON into billable assistant interactions.
/// `cwd` is tracked across lines (it usually appears on user lines, not assistant lines);
/// `fallback_cwd` seeds it. Non-assistant lines, lines without usage, and malformed lines are skipped.
pub fn parse_transcript(content: &str, fallback_cwd: Option<&str>) -> Vec<Interaction> {
    let mut last_cwd = fallback_cwd.map(|s| s.to_string());
    let mut out = Vec::new();
    for raw in content.lines() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let line: Line = match serde_json::from_str(raw) {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.cwd.is_some() {
            last_cwd = line.cwd.clone();
        }
        if line.kind.as_deref() != Some("assistant") {
            continue;
        }
        let msg = match line.message {
            Some(m) => m,
            None => continue,
        };
        let usage = match msg.usage {
            Some(u) => u,
            None => continue,
        };
        if usage.input_tokens == 0 && usage.output_tokens == 0 {
            continue;
        }
        out.push(Interaction {
            session_id: line.session_id.unwrap_or_default(),
            ts: line.timestamp.unwrap_or_default(),
            cwd: last_cwd.clone(),
            model: msg.model.unwrap_or_else(|| "unknown".to_string()),
            tokens_in: usage.input_tokens,
            tokens_out: usage.output_tokens,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_assistant_usage_and_carries_cwd() {
        // a user line carries cwd; the following assistant line carries usage but no cwd
        let content = r#"
{"type":"user","sessionId":"s1","cwd":"C:\\work\\repo","timestamp":"2026-06-10T10:00:00Z","message":{"role":"user","content":"hi"}}
{"type":"assistant","sessionId":"s1","timestamp":"2026-06-10T10:00:01Z","message":{"role":"assistant","model":"claude-opus-4-8","usage":{"input_tokens":1000,"output_tokens":200}}}
{"type":"file-history-snapshot"}
{"type":"assistant","sessionId":"s1","timestamp":"2026-06-10T10:00:02Z","message":{"role":"assistant","model":"claude-opus-4-8","usage":{"input_tokens":0,"output_tokens":0}}}
"#;
        let got = parse_transcript(content, None);
        assert_eq!(got.len(), 1);
        let i = &got[0];
        assert_eq!(i.session_id, "s1");
        assert_eq!(i.cwd.as_deref(), Some("C:\\work\\repo"));
        assert_eq!(i.model, "claude-opus-4-8");
        assert_eq!(i.tokens_in, 1000);
        assert_eq!(i.tokens_out, 200);
        assert_eq!(i.ts, "2026-06-10T10:00:01Z");
    }

    #[test]
    fn skips_garbage_lines() {
        let content = "not json\n{}\n{\"type\":\"assistant\"}\n";
        assert!(parse_transcript(content, None).is_empty());
    }
}
```

- [ ] **Step 2: Register module** — add `mod parse;` to `crates/ccguard-agent/src/main.rs` (keep `mod paths; mod state;`).

- [ ] **Step 3: Run + commit**

Run: `cargo test -p ccguard-agent parse::` → expect 2 passed.
```
git add crates/ccguard-agent
git commit -m "feat(agent): parse Claude Code transcripts into billable interactions"
```

---

## Task 3: `pricing` + `repo` + `event`

**Files:** Create `src/pricing.rs`, `src/repo.rs`, `src/event.rs`; Modify `src/main.rs` (add the three `mod`s).

- [ ] **Step 1: `pricing.rs`**:
```rust
/// Approximate USD per 1M tokens (input, output) by model family. Rough estimate for cost display.
pub fn price_per_mtok(model: &str) -> (f64, f64) {
    let m = model.to_ascii_lowercase();
    if m.contains("opus") {
        (15.0, 75.0)
    } else if m.contains("haiku") {
        (1.0, 5.0)
    } else {
        // sonnet and unknown default
        (3.0, 15.0)
    }
}

pub fn estimate_cost(model: &str, tokens_in: i64, tokens_out: i64) -> f64 {
    let (pin, pout) = price_per_mtok(model);
    (tokens_in as f64 / 1_000_000.0) * pin + (tokens_out as f64 / 1_000_000.0) * pout
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opus_cost_matches_table() {
        // 1M in @ $15 + 1M out @ $75 = $90
        assert!((estimate_cost("claude-opus-4-8", 1_000_000, 1_000_000) - 90.0).abs() < 1e-9);
    }

    #[test]
    fn unknown_model_defaults_to_sonnet_pricing() {
        assert_eq!(price_per_mtok("mystery"), (3.0, 15.0));
    }
}
```

- [ ] **Step 2: `repo.rs`**:
```rust
use std::process::Command;

use ccguard_core::event::Repo;
use ccguard_core::remote::parse_remote_url;

/// Build a `Repo` from an optional git remote URL + the working directory.
/// Pure: no I/O. With a parseable remote → host/org/name filled; otherwise path only.
pub fn repo_from_remote(remote: Option<&str>, cwd: &str) -> Repo {
    match remote.and_then(parse_remote_url) {
        Some(id) => Repo {
            host: Some(id.host),
            org: Some(id.org),
            name: Some(id.name),
            path: Some(cwd.to_string()),
            classification: None,
            confidence: 0.0,
        },
        None => Repo {
            host: None,
            org: None,
            name: None,
            path: Some(cwd.to_string()),
            classification: None,
            confidence: 0.0,
        },
    }
}

/// Attribute a working directory to a repo by reading its git remote (`git -C <cwd> ...`).
pub fn repo_for_cwd(cwd: &str) -> Repo {
    repo_from_remote(git_remote(cwd).as_deref(), cwd)
}

fn git_remote(cwd: &str) -> Option<String> {
    let out = Command::new("git")
        .args(["-C", cwd, "config", "--get", "remote.origin.url"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_remote_to_host_org_name() {
        let r = repo_from_remote(Some("git@github.com:acme-corp/billing.git"), "C:\\work\\billing");
        assert_eq!(r.host.as_deref(), Some("github.com"));
        assert_eq!(r.org.as_deref(), Some("acme-corp"));
        assert_eq!(r.name.as_deref(), Some("billing"));
        assert_eq!(r.path.as_deref(), Some("C:\\work\\billing"));
    }

    #[test]
    fn no_remote_is_path_only() {
        let r = repo_from_remote(None, "C:\\scratch");
        assert!(r.host.is_none() && r.org.is_none());
        assert_eq!(r.path.as_deref(), Some("C:\\scratch"));
    }
}
```

- [ ] **Step 3: `event.rs`**:
```rust
use chrono::{DateTime, Utc};

use ccguard_core::event::{Activity, CcEvent, User};

use crate::parse::Interaction;
use crate::pricing::estimate_cost;
use crate::repo::repo_for_cwd;

/// Map a parsed interaction to a CcEvent ready to POST. Returns None if it has no cwd or an
/// unparseable timestamp. `tenant_id` is left empty (the server sets it from the ingest token).
pub fn interaction_to_event(i: &Interaction, user_email: &str) -> Option<CcEvent> {
    let cwd = i.cwd.clone()?;
    let ts: DateTime<Utc> = i.ts.parse().ok()?;
    let repo = repo_for_cwd(&cwd);
    let cost = estimate_cost(&i.model, i.tokens_in, i.tokens_out);

    Some(CcEvent {
        tenant_id: String::new(),
        user: User {
            email: user_email.to_string(),
            seat_id: None,
        },
        tool: "claude-code".to_string(),
        session_id: i.session_id.clone(),
        ts,
        repo,
        content_ref: None,
        source_layer: "endpoint_agent".to_string(),
        activity: Activity {
            kind: "api_request".to_string(),
            tokens_in: i.tokens_in,
            tokens_out: i.tokens_out,
            cost_usd: cost,
            model: Some(i.model.clone()),
            tool_name: None,
            decision: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_event_with_tokens_cost_and_model() {
        // cwd that is not a git repo → host/org None, path set; tokens/model/cost populated.
        let i = Interaction {
            session_id: "s1".into(),
            ts: "2026-06-10T10:00:01Z".into(),
            cwd: Some(std::env::temp_dir().to_string_lossy().to_string()),
            model: "claude-opus-4-8".into(),
            tokens_in: 1_000_000,
            tokens_out: 0,
        };
        let ev = interaction_to_event(&i, "dev@acme.com").unwrap();
        assert_eq!(ev.tool, "claude-code");
        assert_eq!(ev.source_layer, "endpoint_agent");
        assert_eq!(ev.tenant_id, "");
        assert_eq!(ev.user.email, "dev@acme.com");
        assert_eq!(ev.activity.tokens_in, 1_000_000);
        assert_eq!(ev.activity.model.as_deref(), Some("claude-opus-4-8"));
        assert!((ev.activity.cost_usd - 15.0).abs() < 1e-9); // 1M opus input @ $15
    }

    #[test]
    fn none_without_cwd_or_bad_ts() {
        let base = Interaction {
            session_id: "s".into(),
            ts: "2026-06-10T10:00:01Z".into(),
            cwd: None,
            model: "claude-sonnet-4-6".into(),
            tokens_in: 10,
            tokens_out: 5,
        };
        assert!(interaction_to_event(&base, "x").is_none()); // no cwd

        let bad_ts = Interaction { cwd: Some("/tmp".into()), ts: "not-a-date".into(), ..base };
        assert!(interaction_to_event(&bad_ts, "x").is_none());
    }
}
```

- [ ] **Step 4: Register modules** — add `mod pricing; mod repo; mod event;` to `crates/ccguard-agent/src/main.rs`.

- [ ] **Step 5: Run + commit**

Run: `cargo test -p ccguard-agent` → expect all agent unit tests pass (paths 1 + state 1 + parse 2 + pricing 2 + repo 2 + event 2 = 10).
```
git add crates/ccguard-agent
git commit -m "feat(agent): pricing, git-repo attribution, and interaction->CcEvent mapping"
```

---

## Task 4: `poster` + CLI wiring (`main`)

**Files:** Create `src/poster.rs`; Replace `src/main.rs`.

- [ ] **Step 1: `poster.rs`**:
```rust
use anyhow::Result;

use ccguard_core::event::CcEvent;

/// Posts CcEvents to a CCGuard server's ingest endpoint with a bearer ingest token.
pub struct Poster {
    client: reqwest::blocking::Client,
    url: String,
    token: String,
}

impl Poster {
    pub fn new(server: &str, token: &str) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            url: format!("{}/v1/events", server.trim_end_matches('/')),
            token: token.to_string(),
        }
    }

    /// POST one event; returns the HTTP status code.
    pub fn post(&self, ev: &CcEvent) -> Result<u16> {
        let resp = self
            .client
            .post(&self.url)
            .bearer_auth(&self.token)
            .json(ev)
            .send()?;
        Ok(resp.status().as_u16())
    }
}
```

- [ ] **Step 2: Replace `crates/ccguard-agent/src/main.rs`**:
```rust
mod event;
mod parse;
mod paths;
mod poster;
mod pricing;
mod repo;
mod state;

use std::path::{Path, PathBuf};

use clap::Parser;

use crate::event::interaction_to_event;
use crate::parse::parse_transcript;
use crate::poster::Poster;
use crate::state::State;

/// CCGuard endpoint agent — VISIBLE monitoring of this machine's Claude Code usage.
/// Sends metadata only (model, token counts, repo, timing) — never prompt or code content.
#[derive(Parser, Debug)]
#[command(name = "ccguard-agent")]
struct Args {
    /// CCGuard server base URL, e.g. http://localhost:8080
    #[arg(long)]
    server: String,
    /// Ingest token (ccg_...) for this tenant
    #[arg(long)]
    token: String,
    /// Claude config dir (default: ~/.claude)
    #[arg(long)]
    claude_dir: Option<String>,
    /// Override the reported user email (default: read from ~/.claude.json)
    #[arg(long)]
    email: Option<String>,
}

fn read_since(path: &Path, offset: u64) -> std::io::Result<(String, u64)> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    if offset >= len {
        return Ok((String::new(), len));
    }
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    Ok((buf, len))
}

fn read_claude_email(claude_dir: &Path) -> Option<String> {
    let json_path = claude_dir.parent()?.join(".claude.json");
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(json_path).ok()?).ok()?;
    v.get("oauthAccount")?
        .get("emailAddress")?
        .as_str()
        .map(|s| s.to_string())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let claude_dir = args
        .claude_dir
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".claude")))
        .expect("could not determine claude dir");

    let email = args
        .email
        .or_else(|| read_claude_email(&claude_dir))
        .unwrap_or_else(|| "unknown@local".to_string());

    println!(
        "CCGuard agent — VISIBLE Claude Code usage monitoring (metadata only).\n  server: {}\n  claude dir: {}\n  user: {}",
        args.server,
        claude_dir.display(),
        email
    );

    let state_path = claude_dir.join("ccguard-agent-state.json");
    let mut st = State::load(&state_path);
    let poster = Poster::new(&args.server, &args.token);

    let mut sent = 0usize;
    let mut skipped = 0usize;
    for file in paths::list_transcripts(&claude_dir) {
        let key = file.to_string_lossy().to_string();
        let (chunk, new_off) = match read_since(&file, st.offset(&key)) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if chunk.is_empty() {
            continue;
        }
        for interaction in parse_transcript(&chunk, None) {
            match interaction_to_event(&interaction, &email) {
                Some(ev) => match poster.post(&ev) {
                    Ok(202) => sent += 1,
                    Ok(code) => {
                        eprintln!("  POST returned HTTP {code}");
                        skipped += 1;
                    }
                    Err(e) => {
                        eprintln!("  POST error: {e}");
                        skipped += 1;
                    }
                },
                None => skipped += 1,
            }
        }
        st.set(&key, new_off);
    }
    st.save(&state_path)?;
    println!("CCGuard agent: sent {sent} event(s), skipped {skipped}.");
    Ok(())
}
```

- [ ] **Step 3: Verify it builds and the CLI works**

Run:
```
cargo build -p ccguard-agent
cargo run -p ccguard-agent -- --help
```
Expected: builds; `--help` prints the usage with `--server`, `--token`, `--claude-dir`, `--email`. Then run the full workspace unit tests (no DB needed for agent; server tests need DATABASE_URL):
```
cargo test -p ccguard-agent
```
Expected: 10 passed.

- [ ] **Step 4: Commit**
```
git add crates/ccguard-agent
git commit -m "feat(agent): HTTP poster + CLI that harvests transcripts and posts events"
```

---

## Self-Review (done while writing this plan)
**Spec coverage:** transcript discovery ✅ (paths) · parsing usage ✅ (parse) · repo attribution via git remote ✅ (repo, reusing core `parse_remote_url`) · CcEvent mapping + cost ✅ (event/pricing) · incremental offsets ✅ (state) · POST with ingest token ✅ (poster) · visible/metadata-only ✅ (main banner, no content fields). Scheduler/loop, content tier, cross-tool (Cursor/Copilot) → later.

**Placeholder scan:** none; complete code; expected counts given.

**Type consistency:** `Interaction` (parse) consumed by `interaction_to_event` (event). `repo_from_remote`/`repo_for_cwd` (repo) return core `Repo`. `CcEvent`/`Activity`/`User`/`Repo` are the SAME `ccguard_core` types the server deserializes — so the POST body matches the server's `Json<CcEvent>` exactly (the shared-types payoff). `State::offset/set/save/load` used by `main`. `Poster::post` returns the HTTP code; `main` treats 202 as success (matches the server's `StatusCode::ACCEPTED`).

**Known sharp edges:**
- `main.rs` grows a `mod` line each task — Task 4 replaces it with the full set; ensure all six `mod`s are present.
- The agent depends on `git` on PATH for repo attribution; absent git → path-only repos (server classifies unknown).
- `reqwest` uses `rustls-tls` (no OpenSSL needed on Windows). First build compiles many crates — expect a few minutes.

---

## Execution handoff
Plan saved to `CCGuard\plan\2026-06-10-ccguard-endpoint-agent.md`. Build **subagent-driven**; the controller runs the **live end-to-end smoke test** afterward (real server + Postgres + crafted git repos → agent → donut), as with prior plans.
