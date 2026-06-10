use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

use crate::auth::AuthedUser;
use crate::error::AppError;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

#[derive(Serialize)]
pub struct SearchHit {
    pub session_id: String,
    pub seq: i64,
    pub kind: String,
    pub title: Option<String>,
    pub repo_org: Option<String>,
    pub repo_name: Option<String>,
    pub snippet: String,
}

/// Shared full-text query used by both the JSON API and the dashboard page so
/// the two views stay in lockstep. Tenant-scoped. The snippet is PLAIN TEXT
/// with `«match»` markers (StartSel/StopSel), never HTML — XSS-safe by
/// construction. Callers MUST only invoke this with a non-blank `q`.
pub async fn run_search(
    pool: &PgPool,
    tenant: &str,
    q: &str,
) -> Result<Vec<SearchHit>, sqlx::Error> {
    let rows = sqlx::query(
        "select e.session_id, e.seq, e.kind, s.title, s.repo_org, s.repo_name, \
                ts_headline('english', left(b.content,800000), websearch_to_tsquery('english',$2), \
                            'StartSel=«, StopSel=», MaxFragments=2, MinWords=3, MaxWords=12') as snippet \
         from content_blobs b \
         join captured_events e on e.tenant_id=b.tenant_id and e.content_sha=b.sha256 \
         join captured_sessions s on s.tenant_id=e.tenant_id and s.session_id=e.session_id \
         where b.tenant_id=$1 and b.content_tsv @@ websearch_to_tsquery('english',$2) \
         order by e.session_id, e.seq limit 100",
    )
    .bind(tenant)
    .bind(q)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| SearchHit {
            session_id: r.get("session_id"),
            seq: r.get("seq"),
            kind: r.get("kind"),
            title: r.get("title"),
            repo_org: r.get("repo_org"),
            repo_name: r.get("repo_name"),
            snippet: r.get("snippet"),
        })
        .collect())
}

/// `GET /v1/orgs/:tenant/search?q=...` — full-text search over captured content
/// for the caller's tenant. Blank/absent `q` returns an empty array without
/// touching the database. Tenant-scoped: a caller may only search their own
/// tenant.
pub async fn search(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(tenant): Path<String>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<SearchHit>>, AppError> {
    if user.tenant_id != tenant {
        return Err(AppError::Forbidden("cross-tenant access denied"));
    }
    let q = params.q.unwrap_or_default();
    if q.trim().is_empty() {
        return Ok(Json(Vec::new()));
    }
    let hits = run_search(&pool, &tenant, &q).await?;
    Ok(Json(hits))
}
