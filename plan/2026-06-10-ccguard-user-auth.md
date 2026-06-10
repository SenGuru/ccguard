# CCGuard User Accounts + Login + Roles — Implementation Plan (Plan 3 of N)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Add dashboard user accounts (email + password + role), a login endpoint that issues session tokens, an `AuthedUser` extractor, and protect `GET /v1/orgs/:tenant/summary` so only an authenticated user of that tenant can read it.

**Architecture:** New `users` and `sessions` tables. Passwords hashed with bcrypt (`passwords.rs`). Admin-gated `POST /v1/users` creates accounts (bootstrap). `POST /v1/auth/login` verifies the password and creates a session (random token, only the hash stored, 30-day expiry), returning the plaintext session token once. An `AuthedUser` extractor resolves `Authorization: Bearer <session>` → `{user_id, tenant_id, role}`. `GET /summary` gains `AuthedUser` + a same-tenant check (cross-tenant → 403).

**Tech Stack:** Rust, axum 0.7, sqlx 0.8 (runtime queries), `bcrypt` (password hashing), reuse `tokens::generate_token`/`hash_token` for opaque session tokens, `chrono` for expiry. Postgres on `DATABASE_URL`. Commit identity: `senthilguru246@gmail.com` / `SenGuru`.

---

## Roadmap position
Plan 1 ✅ (engine). Plan 2 ✅ (tenant provisioning + ingest auth). **Plan 3 ← this (user accounts + login + roles, protect `GET /summary`).** Then: GitHub-OAuth SCM feed → dashboard UI → Rust endpoint agent → consent → Stripe.

## Design decisions (within the approved spec)
- **Sessions = opaque bearer tokens** stored hashed in a `sessions` table (same hashing as ingest tokens), 30-day expiry. (Cookie/JWT not needed yet.)
- **`POST /v1/users` is admin-gated** (`ADMIN_TOKEN` + `X-Admin-Token`) for bootstrap — owner-invites-member flows come later. Role validated against the 5 roles.
- **`GET /summary` now requires an authed user of the same tenant** (any role may view in Plan 3; per-seat drill-down role gating comes with those endpoints). Cross-tenant read → 403.
- `POST /v1/events` keeps its **ingest-token** auth (machine/agent), separate from user sessions — two distinct auth types, as designed.

## Prerequisites
- [ ] Postgres reachable; `DATABASE_URL` set. Plans 1–2 on `master`.

## File structure (this plan)
```
crates/ccguard-server/
  Cargo.toml                       # + bcrypt
  migrations/0003_users_sessions.sql  # NEW
  src/passwords.rs                 # NEW: hash_password / verify_password
  src/auth.rs                      # + AuthedUser extractor
  src/error.rs                     # + Forbidden, BadRequest variants
  src/lib.rs                       # + pub mod passwords;
  src/handlers/users.rs            # NEW: POST /v1/users
  src/handlers/sessions.rs         # NEW: POST /v1/auth/login
  src/handlers/summary.rs          # require AuthedUser + tenant check
  src/handlers/mod.rs              # + users, sessions
  src/app.rs                       # + /v1/users, /v1/auth/login routes
  tests/users.rs                   # NEW
  tests/sessions.rs                # NEW
  tests/summary.rs                 # UPDATED (now requires auth)
```

---

## Task 1: Migration, bcrypt, passwords module, error variants

**Files:** Modify `Cargo.toml`, `src/lib.rs`, `src/error.rs`; Create `migrations/0003_users_sessions.sql`, `src/passwords.rs`.

- [ ] **Step 1: Add dep** to `crates/ccguard-server/Cargo.toml` `[dependencies]`:
```toml
bcrypt = "0.15"
```

- [ ] **Step 2: Migration** `crates/ccguard-server/migrations/0003_users_sessions.sql`:
```sql
create table if not exists users (
    id            bigserial primary key,
    tenant_id     text not null references tenants(id),
    email         text not null,
    password_hash text not null,
    role          text not null check (role in ('owner','admin','manager','auditor','member')),
    created_at    timestamptz not null default now(),
    unique (tenant_id, email)
);

create table if not exists sessions (
    id          bigserial primary key,
    user_id     bigint not null references users(id),
    token_hash  text not null unique,
    created_at  timestamptz not null default now(),
    expires_at  timestamptz
);

create index if not exists sessions_hash on sessions (token_hash);
```

- [ ] **Step 3: Password module** `crates/ccguard-server/src/passwords.rs`:
```rust
use bcrypt::{hash, verify, DEFAULT_COST};

/// Hash a password with bcrypt (random per-hash salt).
pub fn hash_password(password: &str) -> String {
    hash(password, DEFAULT_COST).expect("bcrypt hash")
}

/// Verify a password against a bcrypt hash. Returns false on any mismatch or parse error.
pub fn verify_password(password: &str, hashed: &str) -> bool {
    verify(password, hashed).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrip() {
        let h = hash_password("hunter2");
        assert!(verify_password("hunter2", &h));
        assert!(!verify_password("wrong", &h));
    }

    #[test]
    fn hashes_are_salted_and_differ() {
        assert_ne!(hash_password("same"), hash_password("same"));
    }
}
```

- [ ] **Step 4: Register module** — add `pub mod passwords;` to `crates/ccguard-server/src/lib.rs` (keep existing lines; final set: `app, auth, error, handlers, passwords, tokens`).

- [ ] **Step 5: Error variants** — in `crates/ccguard-server/src/error.rs`, add two variants and their responses. The enum becomes:
```rust
#[derive(Debug)]
pub enum AppError {
    Db(sqlx::Error),
    Unauthorized(&'static str),
    Forbidden(&'static str),
    BadRequest(&'static str),
}
```
And in `into_response`, add these arms alongside the existing ones:
```rust
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg).into_response(),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
```

- [ ] **Step 6: Verify + commit**

Run: `cargo test -p ccguard-server --lib passwords::` (DATABASE_URL not needed for these unit tests) → expect 2 passed. Then `cargo build -p ccguard-server`.
```
git add crates/ccguard-server
git commit -m "feat(server): users/sessions migration + bcrypt passwords + error variants"
```

---

## Task 2: `POST /v1/users` (admin-gated)

**Files:** Create `src/handlers/users.rs`, `tests/users.rs`; Modify `src/handlers/mod.rs`, `src/app.rs`.

- [ ] **Step 1: Handler** `crates/ccguard-server/src/handlers/users.rs`:
```rust
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use sqlx::PgPool;

use crate::error::AppError;
use crate::passwords::hash_password;

const ROLES: [&str; 5] = ["owner", "admin", "manager", "auditor", "member"];

#[derive(Deserialize)]
pub struct NewUser {
    pub tenant_id: String,
    pub email: String,
    pub password: String,
    pub role: String,
}

/// Admin-gated user creation (bootstrap). Requires `X-Admin-Token` == `ADMIN_TOKEN`.
pub async fn create_user(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(body): Json<NewUser>,
) -> Result<StatusCode, AppError> {
    let admin = std::env::var("ADMIN_TOKEN").unwrap_or_default();
    let provided = headers
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if admin.is_empty() || provided != admin {
        return Err(AppError::Unauthorized("admin token required"));
    }
    if !ROLES.contains(&body.role.as_str()) {
        return Err(AppError::BadRequest("invalid role"));
    }

    let password_hash = hash_password(&body.password);
    sqlx::query("insert into users (tenant_id, email, password_hash, role) values ($1,$2,$3,$4)")
        .bind(&body.tenant_id)
        .bind(&body.email)
        .bind(&password_hash)
        .bind(&body.role)
        .execute(&pool)
        .await?;

    Ok(StatusCode::CREATED)
}
```

- [ ] **Step 2: Register module + route.** Add `pub mod users;` to `crates/ccguard-server/src/handlers/mod.rs`. In `crates/ccguard-server/src/app.rs`, add the import `users` and the route (place after the `/v1/tenants` route):
```rust
        .route("/v1/users", axum::routing::post(handlers_users_create))
```
Concretely, update `app.rs` to its final form for this plan:
```rust
use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;

use crate::handlers::{ingest, sessions, summary, tenants, users};

pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/v1/tenants", post(tenants::create_tenant))
        .route("/v1/users", post(users::create_user))
        .route("/v1/auth/login", post(sessions::login))
        .route("/v1/events", post(ingest::ingest))
        .route("/v1/orgs/:tenant/summary", get(summary::summary))
        .with_state(pool)
}
```
(`sessions::login` is created in Task 3 — if you compile after Task 2 before Task 3, temporarily comment the `/v1/auth/login` route and the `sessions` import, then restore in Task 3. Each commit must compile.)

- [ ] **Step 3: Test** `crates/ccguard-server/tests/users.rs`:
```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use sqlx::{PgPool, Row};
use tower::ServiceExt;

use ccguard_server::app::app;

#[sqlx::test(migrations = "./migrations")]
async fn admin_creates_user(pool: PgPool) {
    std::env::set_var("ADMIN_TOKEN", "secret-admin");
    sqlx::query("insert into tenants (id,name) values ('acme','Acme')")
        .execute(&pool).await.unwrap();

    let body = r#"{"tenant_id":"acme","email":"a@acme.com","password":"pw12345","role":"owner"}"#;
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("POST").uri("/v1/users")
            .header("content-type", "application/json")
            .header("x-admin-token", "secret-admin")
            .body(Body::from(body)).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let row = sqlx::query("select count(*) as c from users where tenant_id='acme'")
        .fetch_one(&pool).await.unwrap();
    let c: i64 = row.get("c");
    assert_eq!(c, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn create_user_rejects_bad_admin_token(pool: PgPool) {
    std::env::set_var("ADMIN_TOKEN", "secret-admin");
    sqlx::query("insert into tenants (id,name) values ('acme','Acme')")
        .execute(&pool).await.unwrap();

    let body = r#"{"tenant_id":"acme","email":"a@acme.com","password":"pw12345","role":"owner"}"#;
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("POST").uri("/v1/users")
            .header("content-type", "application/json")
            .header("x-admin-token", "WRONG")
            .body(Body::from(body)).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
```

- [ ] **Step 4: Run + commit**

Run (DATABASE_URL set): `cargo test -p ccguard-server --test users` → expect 2 passed.
```
git add crates/ccguard-server
git commit -m "feat(server): admin-gated POST /v1/users (bcrypt) (+integration tests)"
```

---

## Task 3: `POST /v1/auth/login` + `AuthedUser` extractor

**Files:** Create `src/handlers/sessions.rs`, `tests/sessions.rs`; Modify `src/handlers/mod.rs`, `src/auth.rs`.

- [ ] **Step 1: Login handler** `crates/ccguard-server/src/handlers/sessions.rs`:
```rust
use axum::extract::State;
use axum::Json;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

use crate::error::AppError;
use crate::passwords::verify_password;
use crate::tokens::generate_token;

#[derive(Deserialize)]
pub struct LoginReq {
    pub tenant_id: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResp {
    pub session_token: String,
    pub role: String,
}

pub async fn login(
    State(pool): State<PgPool>,
    Json(body): Json<LoginReq>,
) -> Result<Json<LoginResp>, AppError> {
    let row = sqlx::query(
        "select id, password_hash, role from users where tenant_id = $1 and email = $2",
    )
    .bind(&body.tenant_id)
    .bind(&body.email)
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::Unauthorized("invalid credentials"))?;

    let user_id: i64 = row.get("id");
    let password_hash: String = row.get("password_hash");
    let role: String = row.get("role");

    if !verify_password(&body.password, &password_hash) {
        return Err(AppError::Unauthorized("invalid credentials"));
    }

    let (token, token_hash) = generate_token();
    let expires_at = Utc::now() + Duration::days(30);
    sqlx::query("insert into sessions (user_id, token_hash, expires_at) values ($1,$2,$3)")
        .bind(user_id)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&pool)
        .await?;

    Ok(Json(LoginResp {
        session_token: token,
        role,
    }))
}
```

- [ ] **Step 2: Register module** — add `pub mod sessions;` to `crates/ccguard-server/src/handlers/mod.rs`. (Restore the `/v1/auth/login` route + `sessions` import in `app.rs` if you commented them in Task 2.)

- [ ] **Step 3: `AuthedUser` extractor** — append to `crates/ccguard-server/src/auth.rs` (keep the existing `AuthedTenant`; this reuses the same imports — `async_trait`, `FromRef`, `FromRequestParts`, `Parts`, `PgPool`, `Row`, `AppError`, `hash_token`):
```rust
/// Resolves `Authorization: Bearer <session-token>` to the logged-in user.
pub struct AuthedUser {
    pub user_id: i64,
    pub tenant_id: String,
    pub role: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthedUser
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
            "select u.id as user_id, u.tenant_id, u.role \
             from sessions s join users u on u.id = s.user_id \
             where s.token_hash = $1 and (s.expires_at is null or s.expires_at > now())",
        )
        .bind(&hash)
        .fetch_optional(&pool)
        .await?;

        match row {
            Some(r) => Ok(AuthedUser {
                user_id: r.get("user_id"),
                tenant_id: r.get("tenant_id"),
                role: r.get("role"),
            }),
            None => Err(AppError::Unauthorized("invalid session")),
        }
    }
}
```

- [ ] **Step 4: Test** `crates/ccguard-server/tests/sessions.rs`:
```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::passwords::hash_password;

#[sqlx::test(migrations = "./migrations")]
async fn login_succeeds_and_rejects_bad_password(pool: PgPool) {
    sqlx::query("insert into tenants (id,name) values ('acme','Acme')")
        .execute(&pool).await.unwrap();
    let ph = hash_password("pw12345");
    sqlx::query("insert into users (tenant_id,email,password_hash,role) values ('acme','a@acme.com',$1,'admin')")
        .bind(&ph).execute(&pool).await.unwrap();

    // correct password -> 200 + token + role
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("POST").uri("/v1/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"tenant_id":"acme","email":"a@acme.com","password":"pw12345"}"#)).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v["session_token"].as_str().unwrap().starts_with("ccg_"));
    assert_eq!(v["role"], "admin");

    // wrong password -> 401
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("POST").uri("/v1/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"tenant_id":"acme","email":"a@acme.com","password":"WRONG"}"#)).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
```

- [ ] **Step 5: Run + commit**

Run: `cargo test -p ccguard-server --test sessions` → expect 1 passed. (Also confirm `cargo build` is green with the login route restored.)
```
git add crates/ccguard-server
git commit -m "feat(server): POST /v1/auth/login + AuthedUser session extractor (+integration test)"
```

---

## Task 4: Protect `GET /v1/orgs/:tenant/summary`

**Files:** Modify `src/handlers/summary.rs`, `tests/summary.rs`.

- [ ] **Step 1: Require `AuthedUser` + same-tenant check.** In `crates/ccguard-server/src/handlers/summary.rs`, add the import and change the `summary` signature to take `AuthedUser` first and reject cross-tenant reads. Add near the top:
```rust
use crate::auth::AuthedUser;
```
Replace the `summary` function signature + first lines (down to the `let rows = sqlx::query(` call) with:
```rust
pub async fn summary(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(tenant): Path<String>,
) -> Result<Json<Vec<ClassTotals>>, AppError> {
    if user.tenant_id != tenant {
        return Err(AppError::Forbidden("cross-tenant access denied"));
    }

    let rows = sqlx::query(
```
(The rest of the function — the SQL and the mapping — stays exactly as-is.)

- [ ] **Step 2: Rewrite the summary test to authenticate.** Replace `crates/ccguard-server/tests/summary.rs` with:
```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::passwords::hash_password;
use ccguard_server::tokens::generate_token;

/// Seed tenant 'acme' + allowlist + an ingest token + an owner user. Returns the ingest token.
async fn seed(pool: &PgPool) -> String {
    sqlx::query("insert into tenants (id,name) values ('acme','Acme')")
        .execute(pool).await.unwrap();
    sqlx::query("insert into allowlist_rules (tenant_id,kind,value) values ('acme','host','github.com'),('acme','org','acme-corp')")
        .execute(pool).await.unwrap();
    let (token, hash) = generate_token();
    sqlx::query("insert into api_tokens (tenant_id, token_hash) values ('acme',$1)")
        .bind(&hash).execute(pool).await.unwrap();
    let ph = hash_password("pw12345");
    sqlx::query("insert into users (tenant_id,email,password_hash,role) values ('acme','boss@acme.com',$1,'owner')")
        .bind(&ph).execute(pool).await.unwrap();
    token
}

async fn login(pool: &PgPool, tenant: &str, email: &str, pw: &str) -> String {
    let body = serde_json::json!({"tenant_id":tenant,"email":email,"password":pw}).to_string();
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("POST").uri("/v1/auth/login")
            .header("content-type", "application/json").body(Body::from(body)).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["session_token"].as_str().unwrap().to_string()
}

async fn post_event(pool: &PgPool, ingest: &str, session: &str, org: &str, cost: f64) {
    let body = serde_json::json!({
        "tenant_id":"ignored","user":{"email":"d@acme.com"},"tool":"claude-code",
        "session_id":session,"ts":"2026-06-10T10:00:00Z",
        "repo":{"host":"github.com","org":org,"name":"r"},
        "source_layer":"endpoint_agent","activity":{"type":"api_request","cost_usd":cost}
    }).to_string();
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("POST").uri("/v1/events")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {ingest}"))
            .body(Body::from(body)).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[sqlx::test(migrations = "./migrations")]
async fn summary_requires_auth_and_groups_spend(pool: PgPool) {
    let ingest = seed(&pool).await;
    let session = login(&pool, "acme", "boss@acme.com", "pw12345").await;
    post_event(&pool, &ingest, &session, "acme-corp", 1.0).await;
    post_event(&pool, &ingest, &session, "acme-corp", 0.5).await;
    post_event(&pool, &ingest, &session, "dev-personal", 0.25).await;

    // unauthenticated -> 401
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("GET").uri("/v1/orgs/acme/summary")
            .body(Body::empty()).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // authenticated -> 200 + grouped totals
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("GET").uri("/v1/orgs/acme/summary")
            .header("authorization", format!("Bearer {session}"))
            .body(Body::empty()).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let work = v.as_array().unwrap().iter().find(|x| x["classification"] == "work").unwrap();
    assert_eq!(work["cost_usd"], 1.5);
    assert_eq!(work["events"], 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn summary_cross_tenant_is_forbidden(pool: PgPool) {
    seed(&pool).await; // tenant 'acme' + its owner
    // a second tenant + user
    sqlx::query("insert into tenants (id,name) values ('other','Other')")
        .execute(&pool).await.unwrap();
    let ph = hash_password("pw12345");
    sqlx::query("insert into users (tenant_id,email,password_hash,role) values ('other','x@other.com',$1,'owner')")
        .bind(&ph).execute(&pool).await.unwrap();

    let session = login(&pool, "other", "x@other.com", "pw12345").await;
    // 'other' user reading 'acme' summary -> 403
    let resp = app(pool.clone())
        .oneshot(Request::builder().method("GET").uri("/v1/orgs/acme/summary")
            .header("authorization", format!("Bearer {session}"))
            .body(Body::empty()).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
```

- [ ] **Step 3: Full suite + commit**

Run: `cargo test` (whole workspace, DATABASE_URL set). Expected **33 passing** = core 15 + server lib (tokens 3 + passwords 2) + auth 3 + ingest 3 + tenants 2 + users 2 + sessions 1 + summary 2.
```
git add crates/ccguard-server
git commit -m "feat(server): protect GET /summary with AuthedUser + same-tenant check"
```

---

## Self-Review (done while writing this plan)
**Spec coverage:** user accounts + roles ✅ (Task 1–2) · login/sessions ✅ (Task 3) · protected summary + tenant isolation ✅ (Task 4). Owner-invites-member authz, password reset, SSO/SCIM → later plans (documented).

**Placeholder scan:** none; all code complete; expected test counts given.

**Type consistency:** `hash_password`/`verify_password` (Task 1) used in `users` (Task 2), `sessions` login (Task 3), and tests. `AuthedUser{user_id,tenant_id,role}` (Task 3) destructured in `summary` (Task 4). `generate_token`/`hash_token` reused for sessions. New `AppError::{Forbidden,BadRequest}` (Task 1) rendered 403/400 and returned by `users`/`summary`. Session tokens carry the `ccg_` prefix (reused generator) — the login test asserts that.

**Known sharp edges for the implementer:**
- `app.rs` references `sessions::login` (Task 3). If compiling after Task 2 but before Task 3, comment the `/v1/auth/login` route + `sessions` import, restore in Task 3 — every commit must compile.
- `AuthedUser` and `AuthedTenant` both live in `auth.rs` and share imports — don't duplicate `use` lines.
- `summary` extractor order: `AuthedUser`, `State`, `Path` are all non-body extractors — order is fine; just put `AuthedUser` first for clarity.
- bcrypt `DEFAULT_COST` (12) makes hashing ~hundreds of ms; tests do a few hashes — still fast.

---

## Execution handoff
Plan saved to `CCGuard\plan\2026-06-10-ccguard-user-auth.md`. Continue **subagent-driven** against the live Postgres.
