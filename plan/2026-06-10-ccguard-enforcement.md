# CCGuard Enforcement + Attestation + Fleet — Implementation Plan (Plan 8 of N)

> REQUIRED SUB-SKILL: superpowers:subagent-driven-development. The "teeth": generate the deployable Claude Code **managed-settings.json** that forces telemetry + the CCGuard agent + locks login to the corporate org, then **attest** each endpoint's enforcement posture and surface a **fleet** compliance view with tamper/drift detection. Built on Claude Code's REAL enterprise controls (verified 2026-06-10 against official docs).

**Grounding (real keys, do not invent):** managed-settings is the top of the precedence chain (managed > CLI > local > project > user), admin-only. Paths: Windows `C:\ProgramData\ClaudeCode\managed-settings.json` **and** `C:\Program Files\ClaudeCode\managed-settings.json` (version-variant — probe both) + registry `HKLM\SOFTWARE\Policies\ClaudeCode` value `Settings`; macOS `/Library/Application Support/ClaudeCode/managed-settings.json`; Linux `/etc/claude-code/managed-settings.json`. Enforcement keys: `forceLoginMethod` (`"claudeai"`), `forceLoginOrgUUID` (corp org UUID / array), `allowManagedHooksOnly` (`true`), `hooks` (event → `[{hooks:[{type,command|url,...}]}]`), `env` (`CLAUDE_CODE_ENABLE_TELEMETRY`,`OTEL_*`), `permissions.disableBypassPermissionsMode` (`"disable"`), `requiredMinimumVersion`, `allowedHttpHookUrls`. (See `research/total-visibility.md` §3 + the 2026-06-10 managed-settings reference.)

**What we build:** (1) a pure **policy generator** (PolicyConfig → canonical managed-settings.json + sha256). (2) Pure **attestation evaluator** (on-disk policy + active account → enforcement flags + verdict). (3) Agent **`--attest`** mode (probe OS paths, read account, POST attestation) + **`gen-policy`** CLI. (4) Server **devices/enroll/attest** + compliance (drift/stale/tamper). (5) Dashboard **Fleet** + **Policy** pages + MDM deploy artifacts.

**Guardrails:** uses Anthropic's OWN enterprise controls (non-covert, admin-legitimate). "Personal account on company tooling" detection = checking the tool's login org vs the corporate org (a config fact), consistent with `forceLoginMethod`; NOT capturing personal data. No new "never"-line crossings.

**Stack:** Rust, axum 0.7, sqlx + Postgres 17, maud, sha2, serde_json. `DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres`. Commit identity **SenGuru / senthilguru246@gmail.com**. No push.

## Roadmap position
Plans 1–7 ✅ (+5.1). **Plan 8 ← this.** Then Plan 9 on-task score + tracker connector + role profiles.

---

## Task 1 — core: policy generator (`ccguard-core::enforce`)

**Files:** Create `crates/ccguard-core/src/enforce.rs`; `pub mod enforce;` in lib.rs.

- [ ] **Types + generator:**
```rust
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub server_url: String,    // CCGuard server base, e.g. https://ccguard.acme.com
    pub org_uuid: String,      // corporate Claude org UUID (forceLoginOrgUUID)
    pub otel_endpoint: String, // company OTel collector, e.g. https://otel.acme.com:4318
    pub min_version: String,   // requiredMinimumVersion, e.g. "2.1.38"
    pub token_env: String,     // env var name holding the ingest token, e.g. "CCGUARD_TOKEN"
}

/// Build the canonical managed-settings.json Value (stable key order via BTreeMap-backed serde_json).
pub fn managed_settings(p: &PolicyConfig) -> Value {
    let base = p.server_url.trim_end_matches('/');
    json!({
        "allowManagedHooksOnly": true,
        "allowedHttpHookUrls": [format!("{base}/*")],
        "env": {
            "CLAUDE_CODE_ENABLE_TELEMETRY": "1",
            "OTEL_METRICS_EXPORTER": "otlp",
            "OTEL_LOGS_EXPORTER": "otlp",
            "OTEL_EXPORTER_OTLP_PROTOCOL": "http/protobuf",
            "OTEL_EXPORTER_OTLP_ENDPOINT": p.otel_endpoint,
            "OTEL_LOG_TOOL_DETAILS": "1"
        },
        "forceLoginMethod": "claudeai",
        "forceLoginOrgUUID": p.org_uuid,
        "hooks": {
            "SessionEnd": [ { "hooks": [ {
                "type": "command",
                "command": format!("ccguard-agent --server {base} --token ${} --capture", p.token_env),
                "timeout": 600
            } ] } ]
        },
        "permissions": { "disableBypassPermissionsMode": "disable" },
        "requiredMinimumVersion": p.min_version
    })
}

/// Canonical pretty JSON string (serde_json sorts object keys deterministically when built from json! macro? NO —
/// use a canonicalizer): serialize via a function that recursively sorts object keys, so the hash is stable.
pub fn canonical_json(v: &Value) -> String { /* recursively sort keys, serialize compact */ }

pub fn policy_hash(p: &PolicyConfig) -> String {
    hex::encode(Sha256::digest(canonical_json(&managed_settings(p)).as_bytes()))
}

pub fn managed_settings_pretty(p: &PolicyConfig) -> String {
    serde_json::to_string_pretty(&managed_settings(p)).unwrap()
}
```
IMPORTANT: `serde_json::Value` from `json!` preserves insertion order only if the `preserve_order` feature is on; by default `serde_json::Map` is a `BTreeMap` (sorted) UNLESS `preserve_order` is enabled. CONFIRM which: if `preserve_order` is NOT a feature in this workspace, `Value` objects are already sorted and `to_string` is canonical — then `canonical_json` can just be `serde_json::to_string(v)`. If `preserve_order` IS enabled anywhere, implement a recursive key-sorter. Write a test that proves the hash is stable across two builds of the same PolicyConfig regardless.
- [ ] **Tests:** `managed_settings` contains the real keys (`forceLoginOrgUUID` == org_uuid, `allowManagedHooksOnly` true, telemetry env "1", the SessionEnd command hook contains `ccguard-agent` + the token env, `disableBypassPermissionsMode` "disable", `requiredMinimumVersion`); `policy_hash` is deterministic (same config → same hash; different org_uuid → different hash); `managed_settings_pretty` is valid JSON that round-trips.
- [ ] `cargo test -p ccguard-core` green.

---

## Task 2 — core: attestation evaluator (`ccguard-core::attest`)

**Files:** Create `crates/ccguard-core/src/attest.rs`; `pub mod attest;` in lib.rs.

- [ ] **Types + evaluator (pure):**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attestation {
    pub policy_present: bool,    // a managed-settings file was found
    pub policy_hash: Option<String>, // canonical hash of the on-disk policy
    pub policy_match: bool,      // on-disk hash == expected
    pub telemetry_on: bool,      // env.CLAUDE_CODE_ENABLE_TELEMETRY == "1"
    pub hook_present: bool,      // a hook command references ccguard-agent
    pub login_locked: bool,      // forceLoginMethod set AND forceLoginOrgUUID == expected org
    pub bypass_disabled: bool,   // permissions.disableBypassPermissionsMode == "disable"
    pub active_account: Option<String>,  // logged-in email (from ~/.claude.json)
    pub active_org: Option<String>,      // logged-in org uuid if available
    pub personal_account: bool,  // active_org present AND != expected org (or telemetry shows non-corp)
}

/// Evaluate enforcement posture from the on-disk policy JSON (None if absent) against the expected config.
pub fn evaluate(on_disk: Option<&str>, expected: &PolicyConfig,
                active_account: Option<&str>, active_org: Option<&str>) -> Attestation { ... }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Compliance { Compliant, Drifted, Tampered, NoncompliantAccount }

/// Verdict from an attestation (staleness is added server-side from last_seen).
pub fn verdict(a: &Attestation) -> (Compliance, Vec<String>) {
    let mut reasons = vec![];
    if !a.policy_present { return (Compliance::Tampered, vec!["managed-settings missing".into()]); }
    if a.personal_account { reasons.push("personal/non-corp account in use".into()); }
    if !a.policy_match { reasons.push("policy hash drift".into()); }
    if !a.telemetry_on { reasons.push("telemetry disabled".into()); }
    if !a.hook_present { reasons.push("ccguard hook missing".into()); }
    if !a.login_locked { reasons.push("login not locked to corp org".into()); }
    if a.personal_account { return (Compliance::NoncompliantAccount, reasons); }
    if reasons.is_empty() { (Compliance::Compliant, reasons) } else { (Compliance::Drifted, reasons) }
}
```
- [ ] `evaluate` parses `on_disk` as JSON (tolerate malformed → treat as present-but-non-matching with flags false), computes its canonical hash via `enforce::canonical_json`, and sets each flag by inspecting the parsed keys. `hook_present` = any value under `hooks.*[].hooks[].command` containing `"ccguard-agent"`. `login_locked` = `forceLoginMethod` present AND `forceLoginOrgUUID` equals expected `org_uuid` (handle both string and array forms).
- [ ] **Tests:** a policy generated by `enforce::managed_settings` (serialized) → `evaluate` returns all-true, `policy_match` true, verdict Compliant. Mutations each flip the right flag + verdict: telemetry env removed → telemetry_on false + Drifted; hook command stripped → hook_present false; org uuid changed → login_locked false + policy_match false; `on_disk = None` → Tampered; `active_org = Some("other")` → personal_account true + NoncompliantAccount. Malformed JSON → present but flags false, Drifted/Tampered (assert it doesn't panic).
- [ ] `cargo test -p ccguard-core` green.

---

## Task 3 — server: devices + enroll + attest + compliance API

**Files:** Create `crates/ccguard-server/migrations/0008_devices.sql`, `crates/ccguard-server/src/handlers/fleet.rs`; modify `handlers/mod.rs`, `app.rs`; tests in a new `tests/fleet.rs`.

- [ ] **Migration `0008_devices.sql`:**
```sql
create table if not exists devices (
    id            bigserial primary key,
    tenant_id     text not null,
    device_id     text not null,         -- stable per-machine id (hostname + machine guid hash)
    hostname      text,
    os            text,
    agent_version text,
    user_email    text,
    -- latest attestation snapshot:
    policy_present  boolean not null default false,
    policy_match    boolean not null default false,
    telemetry_on    boolean not null default false,
    hook_present    boolean not null default false,
    login_locked    boolean not null default false,
    personal_account boolean not null default false,
    compliance     text not null default 'unknown',  -- compliant|drifted|tampered|noncompliant_account|stale|unknown
    reasons        text,                              -- comma-joined drift reasons
    last_seen      timestamptz,
    created_at     timestamptz not null default now(),
    unique (tenant_id, device_id)
);
create index if not exists devices_tenant_idx on devices (tenant_id, compliance);
```
- [ ] **`POST /v1/enroll`** (AuthedTenant / ingest token): body `{device_id, hostname, os, agent_version, user_email}`; upsert the device row (create or update metadata); response `{ policy_hash, managed_settings }` for the tenant's policy (see policy source below). 202/200.
- [ ] **`POST /v1/attest`** (AuthedTenant): body = the `Attestation` (serde from core) + `{device_id, agent_version}`. Server computes `verdict(&attestation)`; upsert the device row's snapshot columns + `compliance` + `reasons` + `last_seen = now()`. 202.
- [ ] **Policy source:** a tenant's `PolicyConfig` — store it in a small `tenant_policy` table (tenant_id PK, server_url, org_uuid, otel_endpoint, min_version, token_env) seeded via an admin endpoint `POST /v1/tenants/:t/policy` (admin-token gated) OR, to keep Task 3 scoped, derive a default PolicyConfig from env/columns. SIMPLEST for this task: add a `tenant_policy` table + `POST /v1/orgs/:t/policy` (AuthedUser owner/admin) to set it, and `enroll` reads it (404/409 if unset). Use `ccguard_core::enforce::{policy_hash, managed_settings_pretty}`.
- [ ] **`GET /v1/orgs/:tenant/fleet`** (AuthedUser, same-tenant): list devices with all snapshot columns + computed **staleness** (if `last_seen` older than 15 min, override `compliance` to `stale` in the response). JSON.
- [ ] **Compliance recompute on read** for staleness: a device that was `compliant` but hasn't checked in for >15 min shows `stale`. Do this in the query (`case when last_seen < now() - interval '15 minutes' then 'stale' else compliance end`) or in Rust.
- [ ] **Tests (`tests/fleet.rs`):** set a tenant policy; enroll a device (assert response carries a non-empty `policy_hash` + managed_settings containing `forceLoginOrgUUID`); POST a fully-compliant attestation → device row `compliance = compliant`; POST an attestation with telemetry_on=false → `compliance = drifted` + reasons mentions telemetry; POST with personal_account=true → `noncompliant_account`; GET fleet returns the devices; a device with an old `last_seen` (insert/update directly) shows `stale`.
- [ ] `cargo test -p ccguard-server` green.

---

## Task 4 — agent: `--attest` mode + `gen-policy` CLI

**Files:** Create `crates/ccguard-agent/src/enforce_paths.rs`; modify `crates/ccguard-agent/src/main.rs`, `poster.rs`.

- [ ] **`enforce_paths.rs`:** `fn managed_settings_candidates() -> Vec<PathBuf>` returning the OS-appropriate candidate paths (cfg!(target_os): windows → ProgramData + Program Files + (registry read is optional, note it); macos → /Library/Application Support/ClaudeCode/...; linux → /etc/claude-code/...). `fn find_managed_settings() -> Option<(PathBuf, String)>` returns the first existing path + its contents. `fn device_id() -> String` = stable id (hash of hostname + an OS machine id; fall back to hostname). Unit-test the candidate list per `cfg!` and that `device_id` is non-empty/stable across two calls.
- [ ] **`read_active_account()`** — reuse/extend the existing `read_claude_email` (from `~/.claude.json oauthAccount.emailAddress`); also pull `oauthAccount.organizationUuid`/org id if present (return `(email, org)`).
- [ ] **`main.rs`:** add `--attest` flag and a `gen-policy` path:
  - `--attest`: find the managed-settings file, read the active account+org, call `ccguard_core::attest::evaluate(on_disk, &policy, email, org)` — BUT the agent needs the expected `PolicyConfig`. Flow: on `--attest`, first `POST /v1/enroll` (sending device_id/hostname/os/agent_version/email) → response gives `{policy_hash, managed_settings}`; the agent derives the expected by... actually evaluate needs the PolicyConfig. SIMPLER: have `/v1/enroll` return the full `PolicyConfig` (the server has it) as `expected`. The agent then calls `evaluate(on_disk, &expected, email, org)` and `POST /v1/attest` with the resulting Attestation + device_id + agent_version. Print a human summary (compliant / reasons).
  - `gen-policy --org-uuid .. --otel .. [--min-version ..] [--token-env ..]`: print `managed_settings_pretty(&PolicyConfig{ server_url: args.server, ... })` to stdout (so admins can pipe it to the managed-settings path). Pure local, no network.
  - Keep `--capture` and default modes unchanged.
- [ ] **`poster.rs`:** add `post_enroll(&EnrollReq) -> Result<EnrollResp>` (EnrollResp carries `policy_hash` + `expected: PolicyConfig`) and `post_attest(&AttestReq) -> Result<u16>`.
- [ ] **Tests:** `enforce_paths` candidate-list + device_id tests; a `gen-policy` smoke (call the generator, assert output parses + contains `forceLoginOrgUUID`).
- [ ] `cargo build -p ccguard-agent` clean; `cargo test -p ccguard-agent` green.

---

## Task 5 — dashboard Fleet + Policy pages + MDM deploy artifacts

**Files:** modify `crates/ccguard-server/src/web.rs`, `app.rs`; create `deploy/` templates; tests in `tests/web.rs`.

- [ ] **`GET /dashboard/fleet`** (WebUser): table of devices — hostname, user, OS, agent version, last seen (relative), and a **compliance badge** (compliant=green / drifted=amber / stale=gray / tampered=red / noncompliant_account=red) with the drift `reasons` shown inline. Add severity-style CSS classes (`.compliant/.drifted/.stale/.tampered`). Empty state: "no devices enrolled — deploy the policy".
- [ ] **`GET /dashboard/policy`** (WebUser owner/admin): if the tenant policy is set, render the generated `managed-settings.json` in a `<pre>` (maud-escaped) + a **per-OS deploy block** (the exact file path + a copy of the install command for Windows/macOS/Linux) + a link to download the raw JSON (`/dashboard/policy/managed-settings.json` → attachment). If unset, a small form (`POST /dashboard/policy`) to set org_uuid/otel_endpoint/min_version (owner/admin only) → stores tenant_policy.
- [ ] **Dashboard KPI:** add a **Fleet** line — `N devices · M non-compliant` linking to `/dashboard/fleet`. Add `fleet` + `policy` to the top nav.
- [ ] **Deploy artifacts in `deploy/`** (real, parameterized templates the admin uses):
  - `deploy/README.md` — how managed settings work + precedence + the verify step (`/status` shows "Enterprise managed settings").
  - `deploy/windows-install.ps1` — writes the generated JSON to `C:\ProgramData\ClaudeCode\managed-settings.json` (create dir), sets it read-only/system-owned, registers a SYSTEM Scheduled Task running `ccguard-agent --attest` hourly + `--capture` on logon; notes the HKLM `Settings` registry alternative.
  - `deploy/macos-install.sh` — writes to `/Library/Application Support/ClaudeCode/managed-settings.json` (root:wheel, 644) + a launchd plist for hourly `--attest`.
  - `deploy/linux-install.sh` — writes to `/etc/claude-code/managed-settings.json` (root, 644) + a systemd timer for `--attest`.
  - Each script takes the JSON path as an arg (produced by `ccguard-agent gen-policy`). Keep them concise but correct (idempotent, mkdir -p, perms).
- [ ] **Tests (`tests/web.rs`):** set a policy (POST /dashboard/policy with cookie) → GET /dashboard/policy shows `forceLoginOrgUUID` + the Windows path; enroll+attest a device via the API helpers, then GET /dashboard/fleet with cookie → 200 + shows the hostname + a compliance badge; no-cookie /dashboard/fleet → 303 /login.
- [ ] `cargo test -p ccguard-server` green.

---

## Task 6 — full verify + commit

- [ ] `DATABASE_URL` set; whole-workspace `cargo test` ALL green; `cargo build` clean.
- [ ] Commit (identity MUST be SenGuru):
```
git -C "C:\Users\gsent\Desktop\2027-q1-projects\CCGuard" add -A
git -C "C:\Users\gsent\Desktop\2027-q1-projects\CCGuard" -c user.name=SenGuru -c user.email=senthilguru246@gmail.com commit -m "feat(enforce): managed-settings generator + endpoint attestation + fleet compliance + MDM deploy"
```
(Per-task commits also fine.) Do NOT push.

## Self-review
**Real, not invented:** every key (`forceLoginOrgUUID`, `allowManagedHooksOnly`, `hooks`, `CLAUDE_CODE_ENABLE_TELEMETRY`, `disableBypassPermissionsMode`, `requiredMinimumVersion`, OS paths) is from official Claude Code docs (2026-06-10). **The teeth:** managed-settings forces telemetry + the CCGuard agent (SessionEnd command hook) + locks login to the corp org + blocks personal accounts + blocks bypass mode + `allowManagedHooksOnly` stops users adding/removing hooks. **Detection:** attestation reports policy presence/hash/telemetry/hook/login-lock + personal-account; server adds staleness → compliant/drifted/tampered/stale/noncompliant_account with reasons. **Pure core:** generator + evaluator are pure + heavily tested (deterministic hash, every drift flag). **Scope guard:** capture/search/findings/web untouched except additive routes; Windows registry read is noted-but-optional (file probe covers it). **Guardrail:** enforces company-provided tooling via Anthropic's own controls; personal-account detection is a config check, not personal-data capture.

## Execution
Build **subagent-driven** in order 1→2→3→4→5. After green, controller: sets a demo tenant policy, runs `ccguard-agent gen-policy` to print a real managed-settings.json, writes it to a temp "managed" path, runs `ccguard-agent --attest` against it (compliant), then deletes a key and re-attests (drifted), and confirms the `/dashboard/fleet` page flips the device's badge in a browser.
