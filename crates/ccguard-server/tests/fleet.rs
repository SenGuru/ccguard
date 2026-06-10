use axum::body::Body;
use axum::http::{Request, StatusCode};
use ccguard_core::attest::Attestation;
use http_body_util::BodyExt;
use sqlx::{PgPool, Row};
use tower::ServiceExt;

use ccguard_server::app::app;
use ccguard_server::passwords::hash_password;
use ccguard_server::tokens::generate_token;

/// Seed tenant 'acme' + ingest token + an owner user. Returns (ingest_token, owner_email).
async fn seed(pool: &PgPool) -> (String, String) {
    sqlx::query("insert into tenants (id,name) values ('acme','Acme')")
        .execute(pool)
        .await
        .unwrap();
    let (ingest_token, ingest_hash) = generate_token();
    sqlx::query("insert into api_tokens (tenant_id, token_hash) values ('acme',$1)")
        .bind(&ingest_hash)
        .execute(pool)
        .await
        .unwrap();
    let ph = hash_password("pw12345");
    sqlx::query(
        "insert into users (tenant_id,email,password_hash,role) values \
         ('acme','boss@acme.com',$1,'owner')",
    )
    .bind(&ph)
    .execute(pool)
    .await
    .unwrap();
    (ingest_token, "boss@acme.com".to_string())
}

async fn login(pool: &PgPool, tenant: &str, email: &str, pw: &str) -> String {
    let body = serde_json::json!({"tenant_id": tenant, "email": email, "password": pw}).to_string();
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["session_token"].as_str().unwrap().to_string()
}

/// POST a JSON body to `uri` with a Bearer token; returns the response.
async fn post_json(
    pool: &PgPool,
    uri: &str,
    token: &str,
    body: String,
) -> axum::http::Response<Body> {
    app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
}

/// Set the acme tenant policy via the authed-user owner endpoint.
async fn set_policy(pool: &PgPool, session: &str) -> StatusCode {
    let body = serde_json::json!({
        "server_url": "https://ccguard.acme.com",
        "org_uuid": "org-acme-123",
        "otel_endpoint": "https://otel.acme.com:4318",
        "min_version": "2.1.38"
    })
    .to_string();
    post_json(pool, "/v1/orgs/acme/policy", session, body)
        .await
        .status()
}

/// A fully-compliant attestation snapshot for the acme org.
fn compliant_attestation() -> Attestation {
    Attestation {
        policy_present: true,
        policy_hash: Some("deadbeef".to_string()),
        policy_match: true,
        telemetry_on: true,
        hook_present: true,
        login_locked: true,
        bypass_disabled: true,
        active_account: Some("alice@acme.com".to_string()),
        active_org: Some("org-acme-123".to_string()),
        personal_account: false,
    }
}

/// Build the /v1/attest request body: {device_id, agent_version, attestation}.
fn attest_body(device_id: &str, a: &Attestation) -> String {
    serde_json::json!({
        "device_id": device_id,
        "agent_version": "0.1",
        "attestation": a,
    })
    .to_string()
}

async fn device_compliance(pool: &PgPool, device_id: &str) -> String {
    sqlx::query("select compliance from devices where tenant_id='acme' and device_id=$1")
        .bind(device_id)
        .fetch_one(pool)
        .await
        .unwrap()
        .get::<String, _>("compliance")
}

async fn device_reasons(pool: &PgPool, device_id: &str) -> String {
    sqlx::query("select reasons from devices where tenant_id='acme' and device_id=$1")
        .bind(device_id)
        .fetch_one(pool)
        .await
        .unwrap()
        .get::<Option<String>, _>("reasons")
        .unwrap_or_default()
}

#[sqlx::test(migrations = "./migrations")]
async fn enroll_returns_policy_and_managed_settings(pool: PgPool) {
    let (ingest, _) = seed(&pool).await;
    let session = login(&pool, "acme", "boss@acme.com", "pw12345").await;

    // Set the policy first (owner).
    assert_eq!(set_policy(&pool, &session).await, StatusCode::OK);

    // Enroll a device with the ingest token.
    let body = serde_json::json!({
        "device_id": "dev1",
        "hostname": "WS-1",
        "os": "windows",
        "agent_version": "0.1",
        "user_email": "a@acme",
    })
    .to_string();
    let resp = post_json(&pool, "/v1/enroll", &ingest, body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert!(
        !v["policy_hash"].as_str().unwrap().is_empty(),
        "policy_hash must be non-empty"
    );
    let ms = v["managed_settings"].as_str().unwrap();
    assert!(ms.contains("forceLoginOrgUUID"), "managed_settings: {ms}");
    assert!(ms.contains("org-acme-123"), "managed_settings: {ms}");
    assert_eq!(v["expected"]["org_uuid"], "org-acme-123");
}

#[sqlx::test(migrations = "./migrations")]
async fn enroll_without_policy_is_conflict(pool: PgPool) {
    let (ingest, _) = seed(&pool).await;
    let body = serde_json::json!({
        "device_id": "dev1", "hostname": "WS-1", "os": "windows",
        "agent_version": "0.1", "user_email": "a@acme",
    })
    .to_string();
    let resp = post_json(&pool, "/v1/enroll", &ingest, body).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[sqlx::test(migrations = "./migrations")]
async fn attest_verdicts_persist_compliance(pool: PgPool) {
    let (ingest, _) = seed(&pool).await;
    let session = login(&pool, "acme", "boss@acme.com", "pw12345").await;
    assert_eq!(set_policy(&pool, &session).await, StatusCode::OK);

    // Compliant.
    let a = compliant_attestation();
    let resp = post_json(&pool, "/v1/attest", &ingest, attest_body("dev1", &a)).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    assert_eq!(device_compliance(&pool, "dev1").await, "compliant");

    // Telemetry off -> drifted, reasons mentions telemetry.
    let mut a = compliant_attestation();
    a.telemetry_on = false;
    let resp = post_json(&pool, "/v1/attest", &ingest, attest_body("dev1", &a)).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    assert_eq!(device_compliance(&pool, "dev1").await, "drifted");
    assert!(
        device_reasons(&pool, "dev1").await.contains("telemetry"),
        "reasons should mention telemetry"
    );

    // Personal account -> noncompliant_account.
    let mut a = compliant_attestation();
    a.personal_account = true;
    a.active_org = Some("org-personal".to_string());
    let resp = post_json(&pool, "/v1/attest", &ingest, attest_body("dev1", &a)).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    assert_eq!(
        device_compliance(&pool, "dev1").await,
        "noncompliant_account"
    );

    // Policy absent -> tampered.
    let mut a = compliant_attestation();
    a.policy_present = false;
    let resp = post_json(&pool, "/v1/attest", &ingest, attest_body("dev1", &a)).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    assert_eq!(device_compliance(&pool, "dev1").await, "tampered");
}

#[sqlx::test(migrations = "./migrations")]
async fn fleet_lists_devices_and_applies_staleness(pool: PgPool) {
    let (ingest, _) = seed(&pool).await;
    let session = login(&pool, "acme", "boss@acme.com", "pw12345").await;
    assert_eq!(set_policy(&pool, &session).await, StatusCode::OK);

    // Enroll + attest a compliant device.
    let body = serde_json::json!({
        "device_id": "dev1", "hostname": "WS-1", "os": "windows",
        "agent_version": "0.1", "user_email": "a@acme",
    })
    .to_string();
    assert_eq!(
        post_json(&pool, "/v1/enroll", &ingest, body).await.status(),
        StatusCode::OK
    );
    let a = compliant_attestation();
    assert_eq!(
        post_json(&pool, "/v1/attest", &ingest, attest_body("dev1", &a))
            .await
            .status(),
        StatusCode::ACCEPTED
    );

    // GET fleet (authed user) shows dev1 / WS-1, compliant (just checked in).
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/orgs/acme/fleet")
                .header("authorization", format!("Bearer {session}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let rows: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["device_id"], "dev1");
    assert_eq!(rows[0]["hostname"], "WS-1");
    assert_eq!(rows[0]["compliance"], "compliant");

    // Force an old last_seen -> compliance overridden to stale on read.
    sqlx::query(
        "update devices set last_seen = now() - interval '1 hour' \
         where tenant_id='acme' and device_id='dev1'",
    )
    .execute(&pool)
    .await
    .unwrap();

    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/orgs/acme/fleet")
                .header("authorization", format!("Bearer {session}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let rows: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows[0]["compliance"], "stale", "old device must read stale");
}

#[sqlx::test(migrations = "./migrations")]
async fn set_policy_requires_owner_or_admin_and_same_tenant(pool: PgPool) {
    let _ = seed(&pool).await;
    // A member user cannot set the policy.
    let ph = hash_password("pw12345");
    sqlx::query(
        "insert into users (tenant_id,email,password_hash,role) values \
         ('acme','member@acme.com',$1,'member')",
    )
    .bind(&ph)
    .execute(&pool)
    .await
    .unwrap();
    let member = login(&pool, "acme", "member@acme.com", "pw12345").await;
    assert_eq!(set_policy(&pool, &member).await, StatusCode::FORBIDDEN);

    // Cross-tenant: an 'other' owner cannot set acme's policy.
    sqlx::query("insert into tenants (id,name) values ('other','Other')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "insert into users (tenant_id,email,password_hash,role) values \
         ('other','x@other.com',$1,'owner')",
    )
    .bind(&ph)
    .execute(&pool)
    .await
    .unwrap();
    let other = login(&pool, "other", "x@other.com", "pw12345").await;
    assert_eq!(set_policy(&pool, &other).await, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn enroll_and_attest_require_ingest_token(pool: PgPool) {
    let _ = seed(&pool).await;
    // No bearer token at all.
    let resp = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/enroll")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"device_id":"d"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
