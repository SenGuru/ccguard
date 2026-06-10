# CCGuard Tenant Provisioning + API-Token Auth — Implementation Plan (Plan 2 of N)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Turn the open ingest pipeline into a secure multi-tenant one: provision tenants over an API, issue per-tenant ingest tokens, and require a valid bearer token on `POST /v1/events` (using the token's tenant, not a body field).

**Architecture:** Add an `api_tokens` table (store only SHA-256 hashes). A `tokens` module generates/hashes tokens. An `AuthedTenant` axum extractor (`FromRequestParts`) resolves `Authorization: Bearer <token>` → tenant_id or 401. A new admin-gated `POST /v1/tenants` provisions a tenant + its first ingest token. `POST /v1/events` gains the `AuthedTenant` extractor and stores under the authenticated tenant (fixes review item C1: unknown/forged tenant_id can no longer reach the FK).

**Tech Stack:** Rust, axum 0.7 (`FromRequestParts`, `async_trait`), sqlx 0.8 (runtime queries), `sha2`, `rand`, `hex`. Postgres on `DATABASE_URL` (local: `postgres://postgres:postgres@localhost:5432/postgres`). Commit identity: `user.email senthilguru246@gmail.com`, `user.name SenGuru`.

---

## Roadmap position

Plan 1 ✅ (core engine: event→classify→store→donut). **Plan 2 ← this (tenant provisioning + ingest auth).** Plan 3 = dashboard user accounts + login + roles (protect `GET /summary`). Then SCM feed, dashboard UI, agent, consent, Stripe.

## Design decisions (within the approved spec)
- **Ingest auth = per-tenant API tokens** (the endpoint agent/collectors authenticate with these). Format `ccg_<48 hex>`; only the SHA-256 hash is stored; the plaintext is shown once at creation.
- **Provisioning is admin-gated** for now via an `ADMIN_TOKEN` env var + `X-Admin-Token` header. (Self-serve signup with user accounts is Plan 3.)
- **`GET /summary` stays open in Plan 2** (it's a dashboard-user concern → protected in Plan 3). Documented, not forgotten.
- Existing Plan-1 ingest tests are updated to authenticate (they currently POST with no token).

## Prerequisites
- [ ] Postgres reachable; `DATABASE_URL` set. Plan 1 merged on `master`.

## File structure (this plan)
```
crates/ccguard-server/
  Cargo.toml                         # + sha2, rand, hex
  migrations/0002_api_tokens.sql     # NEW
  src/tokens.rs                      # NEW: generate_token(), hash_token()
  src/auth.rs                        # NEW: AuthedTenant extractor
  src/error.rs                       # + Unauthorized variant
  src/lib.rs                         # + pub mod tokens; pub mod auth;
  src/handlers/tenants.rs            # NEW: POST /v1/tenants
  src/handlers/mod.rs                # + pub mod tenants;
  src/handlers/ingest.rs            # require AuthedTenant
  src/app.rs                         # + /v1/tenants route
  tests/tenants.rs                   # NEW
  tests/auth.rs                      # NEW
  tests/ingest.rs                    # UPDATED to authenticate
```

---

## Task 1: Deps, migration, token module, AppError::Unauthorized

**Files:**
- Modify: `crates/ccguard-server/Cargo.toml`
- Create: `crates/ccguard-server/migrations/0002_api_tokens.sql`, `crates/ccguard-server/src/tokens.rs`
- Modify: `crates/ccguard-server/src/lib.rs`, `crates/ccguard-server/src/error.rs`

- [ ] **Step 1: Add dependencies** to `crates/ccguard-server/Cargo.toml` `[dependencies]` (keep existing ones):
```toml
sha2 = "0.10"
rand = "0.8"
hex = "0.4"
```

- [ ] **Step 2: Create the migration** `crates/ccguard-server/migrations/0002_api_tokens.sql`:
```sql
create table if not exists api_tokens (
    id          bigserial primary key,
    tenant_id   text not null references tenants(id),
    token_hash  text not null unique,
    name        text not null default 'ingest',
    created_at  timestamptz not null default now(),
    revoked_at  timestamptz
);

create index if not exists api_tokens_hash on api_tokens (token_hash);
```

- [ ] **Step 3: Create the token module** `crates/ccguard-server/src/tokens.rs`:
```rust
use rand::RngCore;
use sha2::{Digest, Sha256};

/// SHA-256 hex of a token string. Deterministic; what we store and look up by.
pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Generate a new ingest token. Returns (plaintext, hash). The plaintext is shown
/// to the caller exactly once; only the hash is persisted.
pub fn generate_token() -> (String, String) {
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token = format!("ccg_{}", hex::encode(bytes));
    let hash = hash_token(&token);
    (token, hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        assert_eq!(hash_token("ccg_abc"), hash_token("ccg_abc"));
        assert_ne!(hash_token("ccg_abc"), hash_token("ccg_xyz"));
        assert_eq!(hash_token("ccg_abc").len(), 64); // sha256 hex
    }

    #[test]
    fn generate_returns_prefixed_token_with_matching_hash() {
        let (token, hash) = generate_token();
        assert!(token.starts_with("ccg_"));
        assert_eq!(token.len(), 4 + 48); // "ccg_" + 24 bytes hex
        assert_eq!(hash, hash_token(&token));
    }

    #[test]
    fn two_tokens_differ() {
        assert_ne!(generate_token().0, generate_token().0);
    }
}
```

- [ ] **Step 4: Register the module** — add to `crates/ccguard-server/src/lib.rs`:
```rust
pub mod app;
pub mod auth;
pub mod error;
pub mod handlers;
pub mod tokens;
```
(Create an empty `crates/ccguard-server/src/auth.rs` with `// filled in Task 2` so it compiles now.)

- [ ] **Step 5: Add the Unauthorized error variant** — replace `crates/ccguard-server/src/error.rs` with:
```rust
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug)]
pub enum AppError {
    Db(sqlx::Error),
    Unauthorized(&'static str),
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
                // Log details server-side; return a generic message (don't leak DB internals).
                eprintln!("db error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
            }
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg).into_response(),
        }
    }
}
```

- [ ] **Step 6: Verify + commit**

Run: `cargo test -p ccguard-server --lib tokens::` → expect 3 passed. Then `cargo build -p ccguard-server` (should compile; `auth.rs` is an empty placeholder).
```
git add crates/ccguard-server
git commit -m "feat(server): api_tokens migration + token gen/hash + AppError::Unauthorized"
```

---

## Task 2: `AuthedTenant` extractor

**Files:**
- Modify: `crates/ccguard-server/src/auth.rs`
- Create: `crates/ccguard-server/tests/auth.rs`

- [ ] **Step 1: Implement the extractor** — replace `crates/ccguard-server/src/auth.rs` with:
```rust
use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use sqlx::{PgPool, Row};

use crate::error::AppError;
use crate::tokens::hash_token;

/// Resolves `Authorization: Bearer <token>` to the owning tenant id, or 401.
pub struct AuthedTenant(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for AuthedTenant
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = PgPool::from_ref(state);
        let token = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized("missing bearer token"))?;

        let hash = hash_token(token);
        let row = sqlx::query(
            "select tenant_id from api_tokens where token_hash = $1 and revoked_at is null",
        )
        .bind(&hash)
        .fetch_optional(&pool)
        .await?;

        match row {
            Some(r) => Ok(AuthedTenant(r.get("tenant_id"))),
            None => Err(AppError::Unauthorized("invalid token")),
        }
    }
}
```

- [ ] **Step 2: Write the integration test** `crates/ccguard-server/tests/auth.rs`:
```rust
use axum::extract::FromRequestParts;
use axum::http::Request;
use ccguard_server::auth::AuthedTenant;
use ccguard_server::tokens::generate_token;
use sqlx::PgPool;

async fn seed_token(pool: &PgPool) -> String {
    sqlx::query("insert into tenants (id, name) values ('acme','Acme')")
        .execute(pool).await.unwrap();
    let (token, hash) = generate_token();
    sqlx::query("insert into api_tokens (tenant_id, token_hash) values ('acme', $1)")
        .bind(&hash).execute(pool).await.unwrap();
    token
}

async fn extract(pool: PgPool, header: Option<&str>) -> Result<AuthedTenant, ccguard_server::error::AppError> {
    let mut builder = Request::builder().uri("/");
    if let Some(h) = header {
        builder = builder.header("authorization", h);
    }
    let req = builder.body(()).unwrap();
    let (mut parts, _) = req.into_parts();
    AuthedTenant::from_request_parts(&mut parts, &pool).await
}

#[sqlx::test(migrations = "./migrations")]
async fn valid_token_resolves_tenant(pool: PgPool) {
    let token = seed_token(&pool).await;
    let res = extract(pool, Some(&format!("Bearer {token}"))).await;
    assert!(matches!(res, Ok(AuthedTenant(ref t)) if t == "acme"));
}

#[sqlx::test(migrations = "./migrations")]
async fn invalid_token_is_rejected(pool: PgPool) {
    seed_token(&pool).await;
    let res = extract(pool, Some("Bearer ccg_deadbeef")).await;
    assert!(matches!(res, Err(ccguard_server::error::AppError::Unauthorized(_))));
}

#[sqlx::test(migrations = "./migrations")]
async fn missing_header_is_rejected(pool: PgPool) {
    seed_token(&pool).await;
    let res = extract(pool, None).await;
    assert!(matches!(res, Err(ccguard_server::error::AppError::Unauthorized(_))));
}
```

- [ ] **Step 3: Run + commit**

Run (DB up, `DATABASE_URL` set): `cargo test -p ccguard-server --test auth` → expect 3 passed.
```
git add crates/ccguard-server
git commit -m "feat(server): AuthedTenant bearer-token extractor (+integration tests)"
```

---

## Task 3: Tenant provisioning endpoint `POST /v1/tenants`

**Files:**
- Create: `crates/ccguard-server/src/handlers/tenants.rs`, `crates/ccguard-server/tests/tenants.rs`
- Modify: `crates/ccguard-server/src/handlers/mod.rs`, `crates/ccguard-server/src/app.rs`

- [ ] **Step 1: Implement the handler** `crates/ccguard-server/src/handlers/tenants.rs`:
```rust
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::AppError;
use crate::tokens::generate_token;

#[derive(Deserialize)]
pub struct NewTenant {
    pub name: String,
}

#[derive(Serialize)]
pub struct TenantCreated {
    pub tenant_id: String,
    pub ingest_token: String,
}

fn random_tenant_id() -> String {
    let mut bytes = [0u8; 6];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("t_{}", hex::encode(bytes))
}

/// Admin-gated tenant provisioning. Requires `X-Admin-Token` matching the
/// `ADMIN_TOKEN` env var. Creates a tenant and its first ingest token; the token
/// plaintext is returned exactly once.
pub async fn create_tenant(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(body): Json<NewTenant>,
) -> Result<Json<TenantCreated>, AppError> {
    let admin = std::env::var("ADMIN_TOKEN").unwrap_or_default();
    let provided = headers
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if admin.is_empty() || provided != admin {
        return Err(AppError::Unauthorized("admin token required"));
    }

    let tenant_id = random_tenant_id();
    sqlx::query("insert into tenants (id, name) values ($1, $2)")
        .bind(&tenant_id)
        .bind(&body.name)
        .execute(&pool)
        .await?;

    let (token, hash) = generate_token();
    sqlx::query("insert into api_tokens (tenant_id, token_hash) values ($1, $2)")
        .bind(&tenant_id)
        .bind(&hash)
        .execute(&pool)
        .await?;

    Ok(Json(TenantCreated {
        tenant_id,
        ingest_token: token,
    }))
}
```

- [ ] **Step 2: Register module + route**

Add to `crates/ccguard-server/src/handlers/mod.rs`:
```rust
pub mod ingest;
pub mod summary;
pub mod tenants;
```
Replace `crates/ccguard-server/src/app.rs` with:
```rust
use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;

use crate::handlers::{ingest, summary, tenants};

pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/v1/tenants", post(tenants::create_tenant))
        .route("/v1/events", post(ingest::ingest))
        .route("/v1/orgs/:tenant/summary", get(summary::summary))
        .with_state(pool)
}
```

- [ ] **Step 3: Write the integration test** `crates/ccguard-server/tests/tenants.rs`:
```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::{PgPool, Row};
use tower::ServiceExt;

use ccguard_server::app::app;

#[sqlx::test(migrations = "./migrations")]
async fn provisions_tenant_with_token(pool: PgPool) {
    std::env::set_var("ADMIN_TOKEN", "secret-admin");

    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/tenants")
                .header("content-type", "application/json")
                .header("x-admin-token", "secret-admin")
                .body(Body::from(r#"{"name":"Acme Inc"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let tenant_id = v["tenant_id"].as_str().unwrap();
    let token = v["ingest_token"].as_str().unwrap();
    assert!(token.starts_with("ccg_"));

    // tenant + token row persisted
    let row = sqlx::query("select name from tenants where id = $1")
        .bind(tenant_id).fetch_one(&pool).await.unwrap();
    let name: String = row.get("name");
    assert_eq!(name, "Acme Inc");

    let cnt = sqlx::query("select count(*) as c from api_tokens where tenant_id = $1")
        .bind(tenant_id).fetch_one(&pool).await.unwrap();
    let c: i64 = cnt.get("c");
    assert_eq!(c, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn rejects_wrong_admin_token(pool: PgPool) {
    std::env::set_var("ADMIN_TOKEN", "secret-admin");

    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/tenants")
                .header("content-type", "application/json")
                .header("x-admin-token", "WRONG")
                .body(Body::from(r#"{"name":"Acme Inc"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
```

- [ ] **Step 4: Run + commit**

Run: `cargo test -p ccguard-server --test tenants` → expect 2 passed.
```
git add crates/ccguard-server
git commit -m "feat(server): admin-gated POST /v1/tenants provisioning (+integration tests)"
```

---

## Task 4: Require auth on `POST /v1/events`

**Files:**
- Modify: `crates/ccguard-server/src/handlers/ingest.rs`, `crates/ccguard-server/tests/ingest.rs`

- [ ] **Step 1: Add the `AuthedTenant` extractor to the handler.** In `crates/ccguard-server/src/handlers/ingest.rs`, change imports and the `ingest` signature/body so it uses the authenticated tenant instead of the body's `tenant_id`.

Add to the imports at the top:
```rust
use crate::auth::AuthedTenant;
```
Replace the `ingest` function signature and its first lines (the doc comment + `pub async fn ingest(...) {` down to the `classify(...)` call) with:
```rust
/// Ingest a CcEvent. The tenant is taken from the authenticated API token, NOT the
/// request body; any `tenant_id`/`repo.classification`/`repo.confidence` in the
/// payload is ignored and recomputed. (Extractors that read the body must come last,
/// so `AuthedTenant` precedes `Json`.)
pub async fn ingest(
    AuthedTenant(tenant_id): AuthedTenant,
    State(pool): State<PgPool>,
    Json(ev): Json<CcEvent>,
) -> Result<StatusCode, AppError> {
    let allow = load_allowlist(&pool, &tenant_id).await?;
    let (class, confidence) = classify(
        ev.repo.host.as_deref(),
        ev.repo.org.as_deref(),
        ev.repo.path.as_deref(),
        &allow,
    );
```
Then in the `INSERT` bindings, change the first bind from the body's tenant to the authenticated tenant: replace `.bind(&ev.tenant_id)` with:
```rust
    .bind(&tenant_id)
```
(Everything else in the INSERT stays the same.)

- [ ] **Step 2: Update the existing ingest tests to authenticate.** Replace `crates/ccguard-server/tests/ingest.rs` with:
```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::{PgPool, Row};
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::tokens::generate_token;

/// Seed tenant + allowlist + an ingest token; return the plaintext token.
async fn seed(pool: &PgPool) -> String {
    sqlx::query("insert into tenants (id, name) values ('acme', 'Acme')")
        .execute(pool).await.unwrap();
    sqlx::query("insert into allowlist_rules (tenant_id, kind, value) values ('acme','host','github.com'),('acme','org','acme-corp')")
        .execute(pool).await.unwrap();
    let (token, hash) = generate_token();
    sqlx::query("insert into api_tokens (tenant_id, token_hash) values ('acme', $1)")
        .bind(&hash).execute(pool).await.unwrap();
    token
}

fn ev_body(session: &str, org: &str, cost: f64) -> String {
    serde_json::json!({
        "tenant_id": "ignored-by-server",
        "user": { "email": "dev@acme.com" },
        "tool": "claude-code",
        "session_id": session,
        "ts": "2026-06-10T10:00:00Z",
        "repo": { "host": "github.com", "org": org, "name": "r" },
        "source_layer": "endpoint_agent",
        "activity": { "type": "api_request", "cost_usd": cost }
    }).to_string()
}

#[sqlx::test(migrations = "./migrations")]
async fn ingest_classifies_company_repo_as_work(pool: PgPool) {
    let token = seed(&pool).await;
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/events")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(ev_body("s1", "acme-corp", 0.5)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let row = sqlx::query("select tenant_id, classification from events where session_id = 's1'")
        .fetch_one(&pool).await.unwrap();
    let tenant: String = row.get("tenant_id");
    let class: String = row.get("classification");
    assert_eq!(tenant, "acme"); // from the token, NOT the body's "ignored-by-server"
    assert_eq!(class, "work");
}

#[sqlx::test(migrations = "./migrations")]
async fn ingest_classifies_outside_repo_as_personal(pool: PgPool) {
    let token = seed(&pool).await;
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/events")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(ev_body("s2", "dev-personal", 0.3)))
                .unwrap(),
        )
        .await
        .unwrap();
    let _ = resp.collect().await;

    let row = sqlx::query("select classification from events where session_id = 's2'")
        .fetch_one(&pool).await.unwrap();
    let class: String = row.get("classification");
    assert_eq!(class, "personal");
}

#[sqlx::test(migrations = "./migrations")]
async fn ingest_without_token_is_unauthorized(pool: PgPool) {
    seed(&pool).await;
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/events")
                .header("content-type", "application/json")
                .body(Body::from(ev_body("s3", "acme-corp", 0.5)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // nothing stored
    let cnt = sqlx::query("select count(*) as c from events")
        .fetch_one(&pool).await.unwrap();
    let c: i64 = cnt.get("c");
    assert_eq!(c, 0);
}
```

- [ ] **Step 3: Full suite + commit**

Run: `cargo test` (whole workspace, `DATABASE_URL` set). Expect: core 15 + tokens 3 + auth 3 + tenants 2 + ingest 3 + summary 1 = **27 passing**.
```
git add crates/ccguard-server
git commit -m "feat(server): require API-token auth on POST /v1/events; tenant from token (fixes C1)"
```

---

## Self-Review (done while writing this plan)

**Spec coverage:** tenant provisioning ✅ (Task 3) · per-tenant API tokens (hashed) ✅ (Task 1) · authenticated ingest using the token's tenant ✅ (Task 4, fixes review C1) · `Unauthorized`→401 path ✅ (Task 1). Dashboard user accounts/login + protecting `GET /summary` are **deferred to Plan 3** (documented, not gaps).

**Placeholder scan:** none — all code complete, all commands have expected counts.

**Type consistency:** `hash_token`/`generate_token` (Task 1) used by the extractor (Task 2), provisioning (Task 3), and tests (Tasks 2–4). `AuthedTenant(pub String)` (Task 2) destructured in `ingest` (Task 4). `AppError::Unauthorized(&'static str)` (Task 1) returned by the extractor and provisioning, rendered as 401. The extractor's `Rejection = AppError`, so handler error handling stays uniform. `api_tokens.token_hash` is `unique` and queried with `revoked_at is null`.

**Known sharp edges for the implementer:**
- Extractor order in `ingest`: `AuthedTenant` (FromRequestParts) MUST precede `Json` (consumes body). The plan’s signature already has the right order.
- `tests/tenants.rs` sets `ADMIN_TOKEN` via `std::env::set_var`; keep one provisioning concern per test (env is process-global). Each integration test file is its own process, so no cross-file bleed.
- `auth.rs` must be created as an empty placeholder in Task 1 Step 4 so `lib.rs` compiles before Task 2 fills it.

---

## Execution handoff
Plan saved to `CCGuard\plan\2026-06-10-ccguard-tenant-auth.md`. Continue **subagent-driven** (same as Plan 1): fresh subagent per task, verify against Postgres between tasks.
