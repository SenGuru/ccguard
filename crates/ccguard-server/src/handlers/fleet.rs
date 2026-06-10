//! Fleet enforcement endpoints: tenant policy, device enrollment, attestation
//! ingest, and the compliance fleet view.
//!
//! - Agent endpoints (`/v1/enroll`, `/v1/attest`) authenticate with the tenant's
//!   ingest token (`AuthedTenant`).
//! - Read/admin endpoints (`/v1/orgs/:tenant/policy`, `/v1/orgs/:tenant/fleet`)
//!   authenticate with a user session (`AuthedUser`) and are same-tenant guarded.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use ccguard_core::attest::{verdict, Attestation, Compliance};
use ccguard_core::enforce::{managed_settings_pretty, policy_hash, PolicyConfig};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

use crate::auth::{AuthedTenant, AuthedUser};
use crate::error::AppError;

/// Stable snake_case string for a `Compliance` verdict (matches its serde repr),
/// suitable for storing in the `devices.compliance` column.
fn compliance_str(c: Compliance) -> &'static str {
    match c {
        Compliance::Compliant => "compliant",
        Compliance::Drifted => "drifted",
        Compliance::Tampered => "tampered",
        Compliance::NoncompliantAccount => "noncompliant_account",
    }
}

/// Build a `PolicyConfig` from a `tenant_policy` row.
fn cfg_from_row(r: &sqlx::postgres::PgRow) -> PolicyConfig {
    PolicyConfig {
        server_url: r.get("server_url"),
        org_uuid: r.get("org_uuid"),
        otel_endpoint: r.get("otel_endpoint"),
        min_version: r.get("min_version"),
        token_env: r.get("token_env"),
    }
}

// ---------------------------------------------------------------------------
// POST /v1/orgs/:tenant/policy  (AuthedUser, owner/admin, same-tenant)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct SetPolicy {
    pub server_url: String,
    pub org_uuid: String,
    pub otel_endpoint: String,
    pub min_version: String,
    #[serde(default)]
    pub token_env: Option<String>,
}

/// Set (upsert) the tenant's enforcement policy. Owner/admin only.
pub async fn set_policy(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(tenant): Path<String>,
    Json(body): Json<SetPolicy>,
) -> Result<StatusCode, AppError> {
    if user.tenant_id != tenant {
        return Err(AppError::Forbidden("cross-tenant access denied"));
    }
    if user.role != "owner" && user.role != "admin" {
        return Err(AppError::Forbidden("owner or admin role required"));
    }
    let token_env = body.token_env.unwrap_or_else(|| "CCGUARD_TOKEN".to_string());
    sqlx::query(
        "insert into tenant_policy \
         (tenant_id, server_url, org_uuid, otel_endpoint, min_version, token_env, updated_at) \
         values ($1,$2,$3,$4,$5,$6, now()) \
         on conflict (tenant_id) do update set \
           server_url = excluded.server_url, \
           org_uuid = excluded.org_uuid, \
           otel_endpoint = excluded.otel_endpoint, \
           min_version = excluded.min_version, \
           token_env = excluded.token_env, \
           updated_at = now()",
    )
    .bind(&tenant)
    .bind(&body.server_url)
    .bind(&body.org_uuid)
    .bind(&body.otel_endpoint)
    .bind(&body.min_version)
    .bind(&token_env)
    .execute(&pool)
    .await?;
    Ok(StatusCode::OK)
}

// ---------------------------------------------------------------------------
// POST /v1/enroll  (AuthedTenant / ingest token)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct EnrollReq {
    pub device_id: String,
    pub hostname: Option<String>,
    pub os: Option<String>,
    pub agent_version: Option<String>,
    pub user_email: Option<String>,
}

#[derive(Serialize)]
pub struct EnrollResp {
    pub policy_hash: String,
    pub managed_settings: String,
    /// The expected PolicyConfig the agent evaluates the on-disk policy against.
    pub expected: PolicyConfig,
}

/// Enroll (or update the metadata of) a device, and return the tenant's expected
/// policy so the agent can attest locally. 409 if no policy is set for the tenant.
pub async fn enroll(
    AuthedTenant(tenant_id): AuthedTenant,
    State(pool): State<PgPool>,
    Json(body): Json<EnrollReq>,
) -> Result<Json<EnrollResp>, AppError> {
    // Upsert metadata only — never clobber the attestation snapshot columns.
    sqlx::query(
        "insert into devices (tenant_id, device_id, hostname, os, agent_version, user_email) \
         values ($1,$2,$3,$4,$5,$6) \
         on conflict (tenant_id, device_id) do update set \
           hostname = excluded.hostname, \
           os = excluded.os, \
           agent_version = excluded.agent_version, \
           user_email = excluded.user_email",
    )
    .bind(&tenant_id)
    .bind(&body.device_id)
    .bind(&body.hostname)
    .bind(&body.os)
    .bind(&body.agent_version)
    .bind(&body.user_email)
    .execute(&pool)
    .await?;

    let row = sqlx::query(
        "select server_url, org_uuid, otel_endpoint, min_version, token_env \
         from tenant_policy where tenant_id = $1",
    )
    .bind(&tenant_id)
    .fetch_optional(&pool)
    .await?;

    let Some(row) = row else {
        return Err(AppError::Conflict("tenant policy not set"));
    };
    let cfg = cfg_from_row(&row);

    Ok(Json(EnrollResp {
        policy_hash: policy_hash(&cfg),
        managed_settings: managed_settings_pretty(&cfg),
        expected: cfg,
    }))
}

// ---------------------------------------------------------------------------
// POST /v1/attest  (AuthedTenant / ingest token)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct AttestReq {
    pub device_id: String,
    pub agent_version: Option<String>,
    pub attestation: Attestation,
}

/// Ingest a device attestation: compute the compliance verdict and persist the
/// latest snapshot. The agent passes the full `Attestation` (core serde shape).
pub async fn attest(
    AuthedTenant(tenant_id): AuthedTenant,
    State(pool): State<PgPool>,
    Json(body): Json<AttestReq>,
) -> Result<StatusCode, AppError> {
    let a = body.attestation;
    let (compliance, reasons) = verdict(&a);
    let reasons_joined = reasons.join(", ");

    sqlx::query(
        "insert into devices \
         (tenant_id, device_id, agent_version, user_email, \
          policy_present, policy_match, telemetry_on, hook_present, login_locked, \
          personal_account, compliance, reasons, last_seen) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12, now()) \
         on conflict (tenant_id, device_id) do update set \
           agent_version    = excluded.agent_version, \
           user_email       = excluded.user_email, \
           policy_present   = excluded.policy_present, \
           policy_match     = excluded.policy_match, \
           telemetry_on     = excluded.telemetry_on, \
           hook_present     = excluded.hook_present, \
           login_locked     = excluded.login_locked, \
           personal_account = excluded.personal_account, \
           compliance       = excluded.compliance, \
           reasons          = excluded.reasons, \
           last_seen        = now()",
    )
    .bind(&tenant_id)
    .bind(&body.device_id)
    .bind(&body.agent_version)
    .bind(&a.active_account)
    .bind(a.policy_present)
    .bind(a.policy_match)
    .bind(a.telemetry_on)
    .bind(a.hook_present)
    .bind(a.login_locked)
    .bind(a.personal_account)
    .bind(compliance_str(compliance))
    .bind(&reasons_joined)
    .execute(&pool)
    .await?;

    Ok(StatusCode::ACCEPTED)
}

// ---------------------------------------------------------------------------
// GET /v1/orgs/:tenant/fleet  (AuthedUser, same-tenant)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct FleetRow {
    pub device_id: String,
    pub hostname: Option<String>,
    pub os: Option<String>,
    pub agent_version: Option<String>,
    pub user_email: Option<String>,
    pub policy_present: bool,
    pub policy_match: bool,
    pub telemetry_on: bool,
    pub hook_present: bool,
    pub login_locked: bool,
    pub personal_account: bool,
    pub compliance: String,
    pub reasons: Option<String>,
    pub last_seen: Option<chrono::DateTime<chrono::Utc>>,
}

/// List the tenant's devices, newest-checkin first, with staleness applied: a
/// device whose `last_seen` is null or older than 15 minutes is reported `stale`.
pub async fn list(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(tenant): Path<String>,
) -> Result<Json<Vec<FleetRow>>, AppError> {
    if user.tenant_id != tenant {
        return Err(AppError::Forbidden("cross-tenant access denied"));
    }
    let rows = sqlx::query(
        "select device_id, hostname, os, agent_version, user_email, \
                policy_present, policy_match, telemetry_on, hook_present, login_locked, \
                personal_account, reasons, last_seen, \
                case when last_seen is null or last_seen < now() - interval '15 minutes' \
                     then 'stale' else compliance end as compliance \
         from devices where tenant_id = $1 \
         order by last_seen desc nulls last limit 500",
    )
    .bind(&tenant)
    .fetch_all(&pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| FleetRow {
                device_id: r.get("device_id"),
                hostname: r.get("hostname"),
                os: r.get("os"),
                agent_version: r.get("agent_version"),
                user_email: r.get("user_email"),
                policy_present: r.get("policy_present"),
                policy_match: r.get("policy_match"),
                telemetry_on: r.get("telemetry_on"),
                hook_present: r.get("hook_present"),
                login_locked: r.get("login_locked"),
                personal_account: r.get("personal_account"),
                compliance: r.get("compliance"),
                reasons: r.get("reasons"),
                last_seen: r.get("last_seen"),
            })
            .collect(),
    ))
}
