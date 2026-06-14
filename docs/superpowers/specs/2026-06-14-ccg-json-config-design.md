# CCGuard — `ccg.json` configuration file + logging

**Status:** Approved design · **Date:** 2026-06-14 · **Owner:** Senthil

## 1. Overview

The `ccguard-server` binary currently reads its database URL from the `DATABASE_URL`
environment variable and logs with `println!`. This change moves **database and
logging settings into a discoverable JSON config file `ccg.json`**, mirroring the
pattern already used by the attend app (`configuration/appconfig.json` +
`|ConfigFolder|` token).

Locked decisions:
- **Postgres only.** The SQLite example in the request illustrates the
  `|ConfigPath|` substitution; the engine stays Postgres. No SQLite backend.
- **`ccg.json` is the sole DB-config source.** The binary stops reading
  `DATABASE_URL`. (Tests are unaffected — see §6.)
- **Full logging parity** with attend: level, size-rotating file
  (`max_bytes` + `backup_count`), stdout toggle, per-target quieting.

## 2. Config discovery & `|ConfigPath|` substitution

A new module `crates/ccguard-server/src/config.rs`:

- Searches for a directory named `configuration` containing `ccg.json`, starting
  at the current working directory, then **each ancestor up to the filesystem
  root** (covers local / parent / grandparent / great-grandparent / …).
- Honors an optional `CCGUARD_CONFIG_DIR` env override (must contain `ccg.json`,
  or be the parent of a `configuration/ccg.json`) — for containers/tests.
- If nothing is found before the root: print a clear error and **exit non-zero**.
  No silent fallback — a misdeploy fails loudly.
- After parsing, every **string** value (recursively, in nested objects/arrays)
  has the token **`|ConfigPath|`** replaced with the **absolute path of the
  discovered `configuration` folder, with a trailing slash**, in posix form.

  Example: with `configuration` at `/opt/ccguard/configuration`,
  `|ConfigPath|` → `/opt/ccguard/configuration/`, so
  `|ConfigPath|../data/logs/ccguard.log` → `/opt/ccguard/data/logs/ccguard.log`
  (a `data/` sibling of `configuration/`).

## 3. `ccg.json` schema

```json
{
  "database": { "url": "postgres://user:pass@host:5432/db?sslmode=prefer" },
  "logging": {
    "level": "INFO",
    "path": "|ConfigPath|../data/logs/ccguard.log",
    "max_bytes": 10485760,
    "backup_count": 7,
    "to_stdout": true,
    "quiet_targets": ["tower_http=warn", "sqlx=warn"]
  }
}
```

- `database.url` — required; the only DB source. Missing → startup error.
- `logging` — optional block; sensible defaults if absent (level INFO, stdout on,
  no file if `path` omitted).

## 4. Logging (`logging.rs`)

- Built on `tracing` + `tracing-subscriber`. Root level from `logging.level`.
- **Size-rotating file** honoring `max_bytes` + `backup_count` via the
  `rolling-file` crate, wrapped in `tracing-appender`'s non-blocking writer (the
  worker guard is held in `main`). Created only when `path` is set; parent dir is
  created if missing.
- **Line format** mirrors attend: `timestamp \t LEVEL \t target \t message`.
  The IP column is omitted — there is no per-request context plumbed today; it can
  be added later via a tower middleware + task-local. Backups are **not** gzipped
  (deviation from attend, by choice).
- `to_stdout` (default true) toggles a stdout layer using the same formatter.
- `quiet_targets` entries are applied as per-target directives in the
  `EnvFilter` (e.g. `tower_http=warn`) to silence noisy crates.

## 5. `main.rs` flow

`load config → init logging → connect pool (url from config) → run migrations →
bind (CCGUARD_BIND, unchanged) → serve`. Existing `println!`/`eprintln!` become
`tracing` calls.

## 6. Tests

Unaffected. Integration tests use `#[sqlx::test(migrations = "./migrations")]`,
which provisions ephemeral databases from `DATABASE_URL` at the harness level,
independent of the app's config loader. The config loader is invoked only from
`main.rs` (and a thin startup helper not used by tests).

## 7. Files & deploy impact

- **New:** `src/config.rs`, `src/logging.rs`.
- **New (committed):** `configuration/ccg.json` for local dev — points at the
  local docker Postgres (`postgres://ccguard:ccguard@localhost:5432/ccguard`,
  no secret), logs to `|ConfigPath|../data/logs/ccguard.log`.
- **New (gitignored):** `deploy/qa-ccg.json` — the real QA Postgres URL (with
  password). Pushed to `/opt/ccguard/configuration/ccg.json` on every deploy.
- **`do.py`:**
  - `deploy` sftp's `qa-ccg.json` → `<remote>/configuration/ccg.json`.
  - `qa.env` drops `DATABASE_URL` (keeps `ADMIN_TOKEN`, `CCGUARD_BIND`,
    `ANTHROPIC_*`).
  - `_db_steps` reads the DB URL from `deploy/qa-ccg.json` instead of `qa.env`.
  - `provision` creates `<remote>/configuration/` and `<remote>/data/logs/`.
  - systemd `WorkingDirectory=/opt/ccguard` so discovery finds
    `configuration/ccg.json`.
- **`.gitignore`:** add `/data/` and `deploy/qa-ccg.json`.
- **`Cargo.toml`:** add `tracing`, `tracing-subscriber` (env-filter),
  `rolling-file`, `tracing-appender`.

## 8. Non-goals

- No SQLite backend.
- No per-request IP logging / request-trace middleware (future).
- No gzip of rotated logs.
- `CCGUARD_BIND` and the admin/Anthropic secrets stay in the environment.
