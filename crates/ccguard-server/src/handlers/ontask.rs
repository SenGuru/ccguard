//! On-task admin + review-queue endpoints.
//!
//! - Repo overrides + role assignment are admin actions (owner/admin, same-tenant).
//! - Indicators list / status flips + the per-employee on-task rollup are
//!   readable by any authed user in the tenant.
//!
//! Indicators are a *review queue* (open -> reviewed/dismissed), not automated
//! verdicts — consistent with monitoring company-provided tooling.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

use crate::auth::AuthedUser;
use crate::error::AppError;

fn require_admin(user: &AuthedUser) -> Result<(), AppError> {
    if user.role != "owner" && user.role != "admin" {
        return Err(AppError::Forbidden("owner or admin role required"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// POST /v1/orgs/:tenant/repo-overrides  (AuthedUser owner/admin, same-tenant)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct SetRepoOverride {
    pub repo_host: String,
    pub repo_org: String,
    pub repo_name: String,
    pub classification: String, // work | personal | unknown
    #[serde(default)]
    pub note: Option<String>,
}

/// Upsert a per-repo work-definition override. Owner/admin only.
pub async fn set_repo_override(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(tenant): Path<String>,
    Json(body): Json<SetRepoOverride>,
) -> Result<StatusCode, AppError> {
    if user.tenant_id != tenant {
        return Err(AppError::Forbidden("cross-tenant access denied"));
    }
    require_admin(&user)?;
    sqlx::query(
        "insert into repo_overrides \
         (tenant_id, repo_host, repo_org, repo_name, classification, note, updated_at) \
         values ($1,$2,$3,$4,$5,$6, now()) \
         on conflict (tenant_id, repo_host, repo_org, repo_name) do update set \
           classification = excluded.classification, \
           note = excluded.note, \
           updated_at = now()",
    )
    .bind(&tenant)
    .bind(&body.repo_host)
    .bind(&body.repo_org)
    .bind(&body.repo_name)
    .bind(&body.classification)
    .bind(&body.note)
    .execute(&pool)
    .await?;
    Ok(StatusCode::OK)
}

// ---------------------------------------------------------------------------
// POST /v1/orgs/:tenant/roles  (AuthedUser owner/admin, same-tenant)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct SetRole {
    pub user_email: String,
    pub job_role: String, // engineer | marketer | designer | pm | ops | sales | other
    #[serde(default)]
    pub note: Option<String>,
}

/// Upsert an employee's assigned job role. Owner/admin only.
pub async fn set_role(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(tenant): Path<String>,
    Json(body): Json<SetRole>,
) -> Result<StatusCode, AppError> {
    if user.tenant_id != tenant {
        return Err(AppError::Forbidden("cross-tenant access denied"));
    }
    require_admin(&user)?;
    sqlx::query(
        "insert into employee_roles (tenant_id, user_email, job_role, note, updated_at) \
         values ($1,$2,$3,$4, now()) \
         on conflict (tenant_id, user_email) do update set \
           job_role = excluded.job_role, \
           note = excluded.note, \
           updated_at = now()",
    )
    .bind(&tenant)
    .bind(&body.user_email)
    .bind(&body.job_role)
    .bind(&body.note)
    .execute(&pool)
    .await?;
    Ok(StatusCode::OK)
}

// ---------------------------------------------------------------------------
// GET /v1/orgs/:tenant/indicators?status=open  (AuthedUser, same-tenant)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct IndicatorQuery {
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Serialize)]
pub struct IndicatorRow {
    pub id: i64,
    pub user_email: Option<String>,
    pub session_id: Option<String>,
    pub kind: String,
    pub detail: Option<String>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// List indicators for the tenant, newest-first. Defaults to `status=open`.
pub async fn list_indicators(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(tenant): Path<String>,
    Query(q): Query<IndicatorQuery>,
) -> Result<Json<Vec<IndicatorRow>>, AppError> {
    if user.tenant_id != tenant {
        return Err(AppError::Forbidden("cross-tenant access denied"));
    }
    let status = q.status.unwrap_or_else(|| "open".to_string());
    let rows = sqlx::query(
        "select id, user_email, session_id, kind, detail, status, created_at \
         from indicators where tenant_id=$1 and status=$2 \
         order by created_at desc, id desc limit 500",
    )
    .bind(&tenant)
    .bind(&status)
    .fetch_all(&pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| IndicatorRow {
                id: r.get("id"),
                user_email: r.get("user_email"),
                session_id: r.get("session_id"),
                kind: r.get("kind"),
                detail: r.get("detail"),
                status: r.get("status"),
                created_at: r.get("created_at"),
            })
            .collect(),
    ))
}

// ---------------------------------------------------------------------------
// POST /v1/indicators/:id/status  (AuthedUser, tenant-scoped via the row)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct SetStatus {
    pub status: String, // reviewed | dismissed
}

/// Flip an indicator's status. Tenant-scoped: the update only matches a row
/// belonging to the caller's tenant; if none matches -> 404.
pub async fn set_indicator_status(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(body): Json<SetStatus>,
) -> Result<StatusCode, AppError> {
    if body.status != "reviewed" && body.status != "dismissed" {
        return Err(AppError::BadRequest("status must be reviewed or dismissed"));
    }
    let res = sqlx::query("update indicators set status=$1 where id=$2 and tenant_id=$3")
        .bind(&body.status)
        .bind(id)
        .bind(&user.tenant_id)
        .execute(&pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("indicator not found"));
    }
    Ok(StatusCode::OK)
}

// ---------------------------------------------------------------------------
// GET /v1/orgs/:tenant/ontask  (AuthedUser, same-tenant)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct OnTaskRollupRow {
    pub user_email: String,
    pub avg_score: i32,
    pub total: i64,
    pub on_task: i64,
}

/// Per-employee on-task rollup: average score, session count, on-task count.
pub async fn rollup(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(tenant): Path<String>,
) -> Result<Json<Vec<OnTaskRollupRow>>, AppError> {
    if user.tenant_id != tenant {
        return Err(AppError::Forbidden("cross-tenant access denied"));
    }
    let rows = sqlx::query(
        "select s.user_email, avg(sc.score)::int avg_score, count(*) total, \
                count(*) filter (where sc.label='on_task') on_task \
         from session_scores sc \
         join captured_sessions s \
           on s.tenant_id=sc.tenant_id and s.session_id=sc.session_id \
         where sc.tenant_id=$1 \
         group by s.user_email \
         order by avg_score desc",
    )
    .bind(&tenant)
    .fetch_all(&pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| OnTaskRollupRow {
                user_email: r.get("user_email"),
                avg_score: r.get("avg_score"),
                total: r.get("total"),
                on_task: r.get("on_task"),
            })
            .collect(),
    ))
}
