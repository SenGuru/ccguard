use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use sqlx::{PgPool, Row};

use crate::auth::AuthedUser;
use crate::error::AppError;

#[derive(Serialize)]
pub struct ClassTotals {
    pub classification: String,
    pub cost_usd: f64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub events: i64,
}

pub async fn summary(
    user: AuthedUser,
    State(pool): State<PgPool>,
    Path(tenant): Path<String>,
) -> Result<Json<Vec<ClassTotals>>, AppError> {
    if user.tenant_id != tenant {
        return Err(AppError::Forbidden("cross-tenant access denied"));
    }

    let rows = sqlx::query(
        "select classification, \
                coalesce(sum(cost_usd),0)::double precision as cost_usd, \
                coalesce(sum(tokens_in),0)::bigint  as tokens_in, \
                coalesce(sum(tokens_out),0)::bigint as tokens_out, \
                count(*)                    as events \
         from events where tenant_id = $1 group by classification \
         order by classification",
    )
    .bind(&tenant)
    .fetch_all(&pool)
    .await?;

    let out = rows
        .into_iter()
        .map(|r| ClassTotals {
            classification: r.get("classification"),
            cost_usd: r.get("cost_usd"),
            tokens_in: r.get("tokens_in"),
            tokens_out: r.get("tokens_out"),
            events: r.get("events"),
        })
        .collect();
    Ok(Json(out))
}
