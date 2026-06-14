# ccg.json Configuration File + Logging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move `ccguard-server`'s database URL and logging settings out of env vars and into a discoverable `configuration/ccg.json` file with a `|ConfigPath|` path-substitution token.

**Architecture:** A new `config` module discovers `configuration/ccg.json` by walking up from cwd, substitutes `|ConfigPath|` with the config folder's absolute path, and deserializes a typed `AppConfig`. A new `logging` module initializes `tracing` (stdout + size-rotating file) from the config. `main.rs` loads config → inits logging → connects the pool from `config.database.url` → migrates → serves. Postgres stays the only backend.

**Tech Stack:** Rust, axum, sqlx (Postgres), serde_json, tracing + tracing-subscriber, rolling-file, tracing-appender.

---

## Toolchain note (read first)

The local `cargo` is 1.84.0 and **cannot build this repo** (a transitive dep needs Rust ≥1.85). Pick one before running any `cargo` step below:

- **Preferred:** `rustup update stable` (then `cargo --version` shows ≥1.85), or
- **Docker wrapper** (no host toolchain change) — run cargo inside the current Rust image:
  ```bash
  docker run --rm -v "$(pwd):/work" -w /work \
    -v ccguard-cargo-registry:/usr/local/cargo/registry \
    rust:alpine sh -c "apk add --no-cache build-base >/dev/null && <CARGO COMMAND>"
  ```

Wherever a step says `Run: cargo ...`, either run it directly (updated toolchain) or substitute it into `<CARGO COMMAND>` in the Docker wrapper. Config unit tests (Task 2) need **no** database; they run with a plain `cargo test`.

---

## File Structure

- `crates/ccguard-server/Cargo.toml` — add tracing/logging deps (modify).
- `crates/ccguard-server/src/lib.rs` — register the two new modules (modify).
- `crates/ccguard-server/src/config.rs` — config discovery, `|ConfigPath|` substitution, typed schema (create).
- `crates/ccguard-server/src/logging.rs` — `tracing` setup from the logging block (create).
- `crates/ccguard-server/src/main.rs` — wire config + logging into startup (modify).
- `configuration/ccg.json` — local-dev config, committed, no secret (create).
- `deploy/qa-ccg.json` — QA config with real Postgres URL, gitignored (create).
- `do.py` — push ccg.json on deploy, read DB url from it, drop DATABASE_URL from qa.env, mkdir remote dirs (modify).
- `.gitignore` — ignore `/data/` and `deploy/qa-ccg.json` (modify).

---

## Task 1: Add dependencies and register modules

**Files:**
- Modify: `crates/ccguard-server/Cargo.toml`
- Modify: `crates/ccguard-server/src/lib.rs`

- [ ] **Step 1: Add the logging/config crates to `[dependencies]`**

In `crates/ccguard-server/Cargo.toml`, under `[dependencies]` (after the `reqwest` line), add:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
rolling-file = "0.2"
```

And under `[dev-dependencies]` add:

```toml
tempfile = "3"
```

(`serde_json`, `serde`, `chrono`, `anyhow` are already present.)

- [ ] **Step 2: Register the new modules**

In `crates/ccguard-server/src/lib.rs`, add these two lines alongside the other `pub mod` declarations:

```rust
pub mod config;
pub mod logging;
```

- [ ] **Step 3: Create empty module stubs so the crate still compiles**

Create `crates/ccguard-server/src/config.rs` with `// implemented in Task 2` and `crates/ccguard-server/src/logging.rs` with `// implemented in Task 3` (single-line placeholder comments).

- [ ] **Step 4: Verify it compiles**

Run: `cargo build -p ccguard-server`
Expected: builds (pulls the new crates). Warnings about unused are fine.

- [ ] **Step 5: Commit**

```bash
git add crates/ccguard-server/Cargo.toml crates/ccguard-server/src/lib.rs crates/ccguard-server/src/config.rs crates/ccguard-server/src/logging.rs Cargo.lock
git commit -m "deps: add tracing + rolling-file for config/logging"
```

---

## Task 2: Config discovery, substitution, and schema (TDD)

**Files:**
- Modify: `crates/ccguard-server/src/config.rs`

This module needs no database — its tests run with a plain `cargo test`.

- [ ] **Step 1: Write the failing tests**

Replace the contents of `crates/ccguard-server/src/config.rs` test section by appending this module (the implementation above it comes in Step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn substitutes_token_in_nested_strings() {
        let mut v = serde_json::json!({
            "a": "|ConfigPath|../data/x",
            "b": { "c": ["|ConfigPath|y", 1, true] }
        });
        substitute(&mut v, "/root/configuration/");
        assert_eq!(v["a"], "/root/configuration/../data/x");
        assert_eq!(v["b"]["c"][0], "/root/configuration/y");
        assert_eq!(v["b"]["c"][1], 1);
    }

    #[test]
    fn finds_config_dir_in_ancestor() {
        let tmp = tempfile::tempdir().unwrap();
        let cfgdir = tmp.path().join("configuration");
        fs::create_dir_all(&cfgdir).unwrap();
        fs::write(cfgdir.join(CONFIG_FILENAME), "{}").unwrap();
        let deep = tmp.path().join("a").join("b").join("c");
        fs::create_dir_all(&deep).unwrap();

        let found = find_config_dir(Some(deep)).unwrap();
        assert_eq!(found, cfgdir.canonicalize().unwrap());
    }

    #[test]
    fn missing_config_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let err = find_config_dir(Some(tmp.path().to_path_buf()));
        assert!(err.is_err());
    }

    #[test]
    fn parses_and_substitutes_full_config() {
        let tmp = tempfile::tempdir().unwrap();
        let cfgdir = tmp.path().join("configuration");
        fs::create_dir_all(&cfgdir).unwrap();
        fs::write(
            cfgdir.join(CONFIG_FILENAME),
            r#"{ "database": { "url": "postgres://x" },
                "logging": { "level": "DEBUG", "path": "|ConfigPath|../data/logs/a.log" } }"#,
        )
        .unwrap();

        let (cfg, dir) = load_from(&cfgdir).unwrap();
        assert_eq!(cfg.database.url, "postgres://x");
        assert_eq!(cfg.logging.level.as_deref(), Some("DEBUG"));
        let want = format!("{}/../data/logs/a.log", dir.to_string_lossy().replace('\\', "/"));
        assert_eq!(cfg.logging.path.as_deref(), Some(want.as_str()));
        assert!(cfg.logging.to_stdout); // defaults to true
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p ccguard-server --lib config::tests`
Expected: FAIL to compile — `substitute`, `find_config_dir`, `load_from`, `CONFIG_FILENAME`, `AppConfig` not defined.

- [ ] **Step 3: Write the implementation**

Put this **above** the `#[cfg(test)]` module in `crates/ccguard-server/src/config.rs`:

```rust
//! JSON application config with a discoverable `configuration` folder.
//!
//! On startup the app looks for `configuration/ccg.json`, starting at the current
//! working directory and walking up each ancestor to the filesystem root. If none
//! is found the app errors and dies — intentional, so a misdeploy fails loudly.
//!
//! Any string value may contain the token `|ConfigPath|`, replaced with the
//! absolute path of the discovered `configuration` folder **with a trailing
//! slash**, e.g. `"|ConfigPath|../data/logs/ccguard.log"`.

use std::path::{Path, PathBuf};

use serde::Deserialize;

const CONFIG_DIRNAME: &str = "configuration";
pub const CONFIG_FILENAME: &str = "ccg.json";
const CONFIG_TOKEN: &str = "|ConfigPath|";
const OVERRIDE_ENV: &str = "CCGUARD_CONFIG_DIR";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not locate {CONFIG_DIRNAME}/{CONFIG_FILENAME} from {0} up to the filesystem root")]
    NotFound(String),
    #[error("{OVERRIDE_ENV}={0:?} does not contain {CONFIG_FILENAME}")]
    OverrideMissing(String),
    #[error("reading {0}: {1}")]
    Io(String, std::io::Error),
    #[error("parsing {0}: {1}")]
    Parse(String, serde_json::Error),
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    pub level: Option<String>,
    pub path: Option<String>,
    pub max_bytes: Option<u64>,
    pub backup_count: Option<usize>,
    #[serde(default = "default_true")]
    pub to_stdout: bool,
    #[serde(default)]
    pub quiet_targets: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            level: None,
            path: None,
            max_bytes: None,
            backup_count: None,
            to_stdout: true,
            quiet_targets: Vec::new(),
        }
    }
}

/// Locate the `configuration` folder: `CCGUARD_CONFIG_DIR` override, else cwd then
/// each ancestor. `start` defaults to the current working directory.
pub fn find_config_dir(start: Option<PathBuf>) -> Result<PathBuf, ConfigError> {
    if let Ok(override_dir) = std::env::var(OVERRIDE_ENV) {
        let d = PathBuf::from(&override_dir);
        if d.join(CONFIG_FILENAME).is_file() {
            return Ok(d);
        }
        if d.join(CONFIG_DIRNAME).join(CONFIG_FILENAME).is_file() {
            return Ok(d.join(CONFIG_DIRNAME));
        }
        return Err(ConfigError::OverrideMissing(override_dir));
    }

    let start = match start {
        Some(s) => s,
        None => std::env::current_dir().map_err(|e| ConfigError::Io(".".into(), e))?,
    };
    let start = start.canonicalize().unwrap_or(start);
    for dir in std::iter::once(start.as_path()).chain(start.ancestors()) {
        let candidate = dir.join(CONFIG_DIRNAME);
        if candidate.join(CONFIG_FILENAME).is_file() {
            return Ok(candidate);
        }
    }
    Err(ConfigError::NotFound(start.to_string_lossy().into_owned()))
}

/// Recursively replace `|ConfigPath|` in every string within `value`.
fn substitute(value: &mut serde_json::Value, replacement: &str) {
    match value {
        serde_json::Value::String(s) => {
            if s.contains(CONFIG_TOKEN) {
                *s = s.replace(CONFIG_TOKEN, replacement);
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(|v| substitute(v, replacement)),
        serde_json::Value::Object(map) => {
            map.values_mut().for_each(|v| substitute(v, replacement))
        }
        _ => {}
    }
}

/// Read + `|ConfigPath|`-resolve + deserialize the config in an explicit folder.
pub fn load_from(config_dir: &Path) -> Result<(AppConfig, PathBuf), ConfigError> {
    let file = config_dir.join(CONFIG_FILENAME);
    let text = std::fs::read_to_string(&file)
        .map_err(|e| ConfigError::Io(file.to_string_lossy().into_owned(), e))?;
    let mut value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| ConfigError::Parse(file.to_string_lossy().into_owned(), e))?;
    // posix path + trailing slash, so "|ConfigPath|../data" -> "<dir>/../data".
    let mut replacement = config_dir.to_string_lossy().replace('\\', "/");
    if !replacement.ends_with('/') {
        replacement.push('/');
    }
    substitute(&mut value, &replacement);
    let cfg: AppConfig = serde_json::from_value(value)
        .map_err(|e| ConfigError::Parse(file.to_string_lossy().into_owned(), e))?;
    Ok((cfg, config_dir.to_path_buf()))
}

/// Discover and load the config from cwd/ancestors (the startup entry point).
pub fn load() -> Result<(AppConfig, PathBuf), ConfigError> {
    let dir = find_config_dir(None)?;
    load_from(&dir)
}
```

Add `thiserror = "1"` to `[dependencies]` in `crates/ccguard-server/Cargo.toml` (used for `ConfigError`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p ccguard-server --lib config::tests`
Expected: PASS (4 tests). Note: `load_from` returns the dir as passed; the `finds_config_dir_in_ancestor` test compares against `canonicalize()` — `find_config_dir` canonicalizes `start`, so the returned candidate is already canonical.

- [ ] **Step 5: Commit**

```bash
git add crates/ccguard-server/src/config.rs crates/ccguard-server/Cargo.toml Cargo.lock
git commit -m "config: discover configuration/ccg.json + |ConfigPath| substitution"
```

---

## Task 3: Logging setup from config

**Files:**
- Modify: `crates/ccguard-server/src/logging.rs`

This module is verified by compiling + the manual smoke test in Task 7 (file-handler behavior is awkward to unit-test deterministically).

- [ ] **Step 1: Write the implementation**

Replace `crates/ccguard-server/src/logging.rs` with:

```rust
//! Tracing-based logging configured from the `logging` block of ccg.json.
//!
//! Renders each record as a tab-separated line: `ts \t LEVEL \t target \t msg`.
//! A size-rotating file handler honors `max_bytes` + `backup_count`; `to_stdout`
//! toggles a stdout layer; `quiet_targets` lower the level of noisy crates.

use std::fmt;

use rolling_file::{BasicRollingFileAppender, RollingConditionBasic};
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::EnvFilter;

use crate::config::LoggingConfig;

const DEFAULT_MAX_BYTES: u64 = 10 * 1024 * 1024;
const DEFAULT_BACKUPS: usize = 7;

/// Attend-style one-line formatter: `date \t LEVEL \t target \t message`.
struct TabFormatter;

impl<S, N> FormatEvent<S, N> for TabFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let meta = event.metadata();
        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        write!(writer, "{}\t{}\t{}\t", ts, meta.level(), meta.target())?;
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

fn build_filter(cfg: &LoggingConfig) -> EnvFilter {
    let level = cfg.level.clone().unwrap_or_else(|| "INFO".into());
    let mut filter = EnvFilter::new(level);
    for target in &cfg.quiet_targets {
        if let Ok(directive) = target.parse() {
            filter = filter.add_directive(directive);
        }
    }
    filter
}

/// Initialize the global tracing subscriber. The returned guard must be held for
/// the lifetime of the program (it flushes the non-blocking file writer on drop).
pub fn init(cfg: &LoggingConfig) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let filter = build_filter(cfg);

    let stdout_layer = if cfg.to_stdout {
        Some(
            tracing_subscriber::fmt::layer()
                .event_format(TabFormatter)
                .with_ansi(false)
                .with_writer(std::io::stdout),
        )
    } else {
        None
    };

    let mut guard = None;
    let file_layer = match &cfg.path {
        Some(path) => {
            if let Some(parent) = std::path::Path::new(path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let condition = RollingConditionBasic::new()
                .max_size(cfg.max_bytes.unwrap_or(DEFAULT_MAX_BYTES));
            match BasicRollingFileAppender::new(
                path,
                condition,
                cfg.backup_count.unwrap_or(DEFAULT_BACKUPS),
            ) {
                Ok(appender) => {
                    let (nb, g) = tracing_appender::non_blocking(appender);
                    guard = Some(g);
                    Some(
                        tracing_subscriber::fmt::layer()
                            .event_format(TabFormatter)
                            .with_ansi(false)
                            .with_writer(nb),
                    )
                }
                Err(e) => {
                    eprintln!("warning: could not open log file {path}: {e}");
                    None
                }
            }
        }
        None => None,
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    guard
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p ccguard-server`
Expected: builds. If `rolling-file`'s API names differ for the pinned version, adjust `RollingConditionBasic`/`BasicRollingFileAppender` per the compiler error (the crate exposes a size condition + a max-files appender).

- [ ] **Step 3: Commit**

```bash
git add crates/ccguard-server/src/logging.rs
git commit -m "logging: tracing stdout + size-rotating file from ccg.json"
```

---

## Task 4: Wire config + logging into `main.rs`

**Files:**
- Modify: `crates/ccguard-server/src/main.rs`

- [ ] **Step 1: Replace `main.rs` with the config-driven startup**

```rust
use ccguard_server::app::app;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Config is mandatory: locate configuration/ccg.json (cwd or any ancestor),
    // resolve |ConfigPath|, and die loudly if it's missing.
    let (cfg, _config_dir) = ccguard_server::config::load().unwrap_or_else(|e| {
        eprintln!("CCGuard config error: {e}");
        std::process::exit(1);
    });

    // Logging is configured before anything else logs. Guard lives until main exits.
    let _log_guard = ccguard_server::logging::init(&cfg.logging);

    let pool = PgPoolOptions::new().connect(&cfg.database.url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Bind address stays env-configurable for the reverse-proxy deploy.
    let bind = std::env::var("CCGUARD_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("CCGuard server listening on {bind}");
    axum::serve(listener, app(pool)).await?;
    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p ccguard-server`
Expected: builds. The old `DATABASE_URL` env read is gone.

- [ ] **Step 3: Commit**

```bash
git add crates/ccguard-server/src/main.rs
git commit -m "server: load db url + logging from ccg.json (drop DATABASE_URL env)"
```

---

## Task 5: Local dev config + gitignore + data dir

**Files:**
- Create: `configuration/ccg.json`
- Modify: `.gitignore`

- [ ] **Step 1: Create the local-dev config (committed, no secret)**

`configuration/ccg.json`:

```json
{
  "database": { "url": "postgres://ccguard:ccguard@localhost:5432/ccguard" },
  "logging": {
    "level": "INFO",
    "path": "|ConfigPath|../data/logs/ccguard.log",
    "max_bytes": 10485760,
    "backup_count": 7,
    "to_stdout": true,
    "quiet_targets": ["sqlx=warn"]
  }
}
```

- [ ] **Step 2: Ignore runtime data and the QA secret config**

Append to `.gitignore`:

```gitignore
# Runtime data (sqlite/logs written under a data/ sibling of configuration/).
/data/

# QA config carries the real Postgres URL/password — pushed at deploy time.
deploy/qa-ccg.json
```

- [ ] **Step 3: Smoke-test locally (optional, needs docker Postgres)**

```bash
python do.py db          # start local Postgres
cargo run -p ccguard-server
```
Expected: stdout shows a tab-separated `... INFO ccguard_server CCGuard server listening on 0.0.0.0:8080` line, and `data/logs/ccguard.log` is created with the same line.

- [ ] **Step 4: Commit**

```bash
git add configuration/ccg.json .gitignore
git commit -m "config: local-dev ccg.json + ignore /data and qa-ccg.json"
```

---

## Task 6: Deploy via ccg.json (update `do.py`)

**Files:**
- Create: `deploy/qa-ccg.json` (gitignored)
- Modify: `deploy/qa.env` (gitignored — remove DATABASE_URL)
- Modify: `do.py`

- [ ] **Step 1: Create the QA config from the existing DATABASE_URL**

Create `deploy/qa-ccg.json` using the URL currently in `deploy/qa.env`:

```json
{
  "database": { "url": "postgres://postgres:%2FD3i565%40Dmin123%23%23@159.65.153.42:5432/ccguard-qa-1?sslmode=prefer" },
  "logging": {
    "level": "INFO",
    "path": "|ConfigPath|../data/logs/ccguard.log",
    "max_bytes": 10485760,
    "backup_count": 7,
    "to_stdout": true,
    "quiet_targets": ["sqlx=warn"]
  }
}
```

Then remove the `DATABASE_URL=` line from `deploy/qa.env` (keep `CCGUARD_BIND`, `ADMIN_TOKEN`, and any `ANTHROPIC_*`).

- [ ] **Step 2: Add a config-file field to `DeployConfig` and loader**

In `do.py`, add to the `DeployConfig` dataclass (near `env_file`):

```python
    config_file: Path | None    # pushed to <remote_path>/configuration/ccg.json
```

Add a property on `DeployConfig`:

```python
    @property
    def remote_config(self) -> str:
        return f"{self.remote_path}/configuration/{self.service_name}.json".replace(
            f"{self.service_name}.json", "ccg.json")
```

(Simpler: hardcode `return f"{self.remote_path}/configuration/ccg.json"`.)

In `_load_deploy_config`, set it from config (default `deploy/<env>-ccg.json`):

```python
        config_file=_deploy_path(deploy.get("configFile", f"{env_name}-ccg.json")),
```

- [ ] **Step 3: Read the DB URL from `qa-ccg.json` in `_db_steps`**

Change the top of `_db_steps` to read the URL from the JSON config instead of the env file:

```python
def _db_steps(cfg: DeployConfig) -> list[str]:
    """Idempotent CREATE DATABASE from the DATABASE url in deploy/<env>-ccg.json."""
    if cfg.config_file is None or not cfg.config_file.is_file():
        print("warning: no ccg.json config — skipping db creation")
        return []
    raw = json.loads(cfg.config_file.read_text(encoding="utf-8"))
    url = (raw.get("database", {}) or {}).get("url", "")
    if not url:
        print("warning: no database.url in ccg.json — skipping db creation")
        return []
    parts = urlsplit(url)
    # ... rest unchanged (user/password/host/port/dbname parsing + psql steps) ...
```

- [ ] **Step 4: Push `ccg.json` during deploy**

In `cmd_deploy`, after the env-file validation, validate the config file:

```python
    if cfg.config_file is None or not cfg.config_file.is_file():
        print(f"config file {cfg.config_file} not found — create deploy/{env_name}-ccg.json")
        return 2
```

Add an sftp upload + remote install. Define a tmp path near the others:

```python
    remote_tmp_cfg = f"/tmp/{cfg.service_name}-ccg.json"
```

After `_sftp_put(client, resolved_env, remote_tmp_env)` add:

```python
        _sftp_put(client, cfg.config_file, remote_tmp_cfg)
```

And in the `steps` list, before `systemctl start`, add (after the env-file `chmod 600` step):

```python
            f"mkdir -p {cfg.remote_path}/configuration {cfg.remote_path}/data/logs",
            f"mv {remote_tmp_cfg} {cfg.remote_path}/configuration/ccg.json",
            f"chmod 600 {cfg.remote_path}/configuration/ccg.json",
```

- [ ] **Step 5: Create remote dirs during provision**

In `cmd_provision`, the `steps` list already has `mkdir -p {cfg.remote_path}/bin {cfg.remote_path}/certs`. Extend it to also create the config + data dirs:

```python
            f"mkdir -p {cfg.remote_path}/bin {cfg.remote_path}/certs "
            f"{cfg.remote_path}/configuration {cfg.remote_path}/data/logs",
```

- [ ] **Step 6: Verify the generated steps**

Run:
```bash
python - <<'PY'
import do
cfg = do._load_deploy_config('qa')
print("remote config:", cfg.remote_config)
for s in do._db_steps(cfg):
    print(s)
PY
```
Expected: prints `/opt/ccguard/configuration/ccg.json` and a `psql ... CREATE DATABASE "ccguard-qa-1"` step sourced from `qa-ccg.json`.

- [ ] **Step 7: Commit (do.py only — secrets stay local)**

```bash
git status --short   # confirm deploy/qa-ccg.json and deploy/qa.env do NOT appear
git add do.py
git commit -m "deploy: push configuration/ccg.json; read db url from it"
```

---

## Task 7: Full build + verification

**Files:** none (verification only)

- [ ] **Step 1: Build the Linux binary**

Run: `python do.py build qa`
Expected: `built target/linux-musl/release/ccguard-server (... MB, linux x86_64-musl)`.

- [ ] **Step 2: Run the config unit tests**

Run: `cargo test -p ccguard-server --lib config::tests` (via updated toolchain or the Docker wrapper)
Expected: 4 passed.

- [ ] **Step 3: Manual smoke (optional, needs local Postgres)**

Run: `python do.py db && cargo run -p ccguard-server`
Expected: tab-formatted startup line on stdout AND in `data/logs/ccguard.log`; killing it leaves the rotated log in place.

- [ ] **Step 4: Negative test — missing config dies loudly**

Run from a directory with no `configuration/` ancestor (e.g. a temp dir): `CCGUARD_CONFIG_DIR=/nonexistent <binary>`
Expected: prints `CCguard config error: ...` and exits non-zero.

---

## Self-Review

- **Spec coverage:** discovery + ancestor walk (Task 2), `|ConfigPath|` trailing-slash substitution (Task 2), schema (Task 2), Postgres-only db url from file (Task 4), full-parity logging incl. size rotation/level/stdout/quiet (Task 3), env override `CCGUARD_CONFIG_DIR` (Task 2), tests unaffected (config tests are DB-free; integration tests untouched), deploy/do.py/gitignore/Cargo deps (Tasks 1,5,6). All spec sections map to tasks.
- **Placeholders:** none — every code step contains full code.
- **Type consistency:** `AppConfig`/`DatabaseConfig`/`LoggingConfig`, `find_config_dir`/`substitute`/`load_from`/`load`, `CONFIG_FILENAME` used identically across tasks; `logging::init(&cfg.logging)` matches the `LoggingConfig` field on `AppConfig`; `_db_steps` consumes `cfg.config_file` defined in Task 6 Step 2.
