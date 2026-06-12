//! Enforcement arming: the build-time precision GO/NO-GO gate, the conformal
//! selective-threshold for the Tier-A judge, and the control-plane gate inputs the
//! off-device proxy consumes. Independent ground truth comes from human-reviewed
//! triage rows (confirm = agree, relabel = disagree) — never a model self-vote.

use axum::extract::{Query, State};
use axum::Json;
use ccguard_core::conformal::{self, Calibration, CalibrationPoint};
use ccguard_core::precision_gate::{self, GateDecision, GateReport, LabeledOutcome};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

use crate::auth::AuthedTenant;
use crate::error::AppError;

/// Minimum stratified human labels before the precision gate may read GO.
pub const MIN_LABELS: usize = 200;
/// The agreed false-personal floor the Wilson upper bound must clear.
pub const MAX_FALSE_PERSONAL: f32 = 0.05;
/// Personal-stratum floor: the holdout must contain at least this many predicted-
/// personal labels before its false-personal bound is trusted for GO.
pub const MIN_PERSONAL_PREDICTIONS: usize = 40;
/// Target accepted-error for the conformal judge calibration.
pub const CONFORMAL_ALPHA: f32 = 0.10;
/// Minimum labels before the conformal calibration is usable.
pub const CONFORMAL_MIN_N: usize = 50;

/// Human-reviewed triage rows: (model label, human label, confidence).
async fn human_labels(
    pool: &PgPool,
    tenant_id: &str,
) -> Result<Vec<(String, String, f32)>, sqlx::Error> {
    let rows = sqlx::query(
        "select label, human_label, confidence from session_triage \
         where tenant_id = $1 and human_reviewed = true and human_label is not null",
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            (
                r.get::<String, _>("label"),
                r.get::<String, _>("human_label"),
                r.get::<f32, _>("confidence"),
            )
        })
        .collect())
}

/// Fit the conformal selective threshold from human-reviewed verdicts.
pub async fn load_calibration(pool: &PgPool, tenant_id: &str) -> Result<Calibration, sqlx::Error> {
    let labels = human_labels(pool, tenant_id).await?;
    let points: Vec<CalibrationPoint> = labels
        .iter()
        .map(|(model, human, conf)| CalibrationPoint {
            confidence: *conf,
            correct: model == human,
        })
        .collect();
    Ok(conformal::calibrate(&points, CONFORMAL_ALPHA, CONFORMAL_MIN_N))
}

/// Build the precision-gate report from human-reviewed verdicts.
pub async fn load_report(pool: &PgPool, tenant_id: &str) -> Result<GateReport, sqlx::Error> {
    let labels = human_labels(pool, tenant_id).await?;
    let outcomes: Vec<LabeledOutcome> = labels
        .iter()
        .map(|(model, human, _)| LabeledOutcome {
            predicted_personal: model == "personal",
            actual_personal: human == "personal",
        })
        .collect();
    Ok(precision_gate::evaluate(
        &outcomes,
        MIN_LABELS,
        MAX_FALSE_PERSONAL,
        MIN_PERSONAL_PREDICTIONS,
    ))
}

/// Recompute the precision gate + conformal calibration and persist them to
/// `enforcement_arming`. Does NOT arm — arming is a separate, GO-gated human action.
pub async fn recompute_and_store(
    pool: &PgPool,
    tenant_id: &str,
) -> Result<(GateReport, Calibration), sqlx::Error> {
    let report = load_report(pool, tenant_id).await?;
    let calib = load_calibration(pool, tenant_id).await?;
    let go = report.decision == GateDecision::Go;

    sqlx::query(
        "insert into enforcement_arming \
         (tenant_id, armed, precision_go, n_labels, false_personal_rate, \
          false_personal_upper, conformal_threshold, fail_closed_state, decided_at) \
         values ($1, false, $2, $3, $4, $5, $6, 'observation_only', now()) \
         on conflict (tenant_id) do update set \
           precision_go = excluded.precision_go, n_labels = excluded.n_labels, \
           false_personal_rate = excluded.false_personal_rate, \
           false_personal_upper = excluded.false_personal_upper, \
           conformal_threshold = excluded.conformal_threshold, \
           decided_at = now(), \
           -- a NO-GO immediately disarms (cannot stay armed on a failing gate)
           armed = enforcement_arming.armed and excluded.precision_go",
    )
    .bind(tenant_id)
    .bind(go)
    .bind(report.n as i32)
    .bind(report.false_personal_rate)
    .bind(report.false_personal_upper_ci)
    .bind(calib.threshold)
    .execute(pool)
    .await?;

    Ok((report, calib))
}

/// The persisted arming record.
#[derive(Debug, Clone, Serialize)]
pub struct ArmingRow {
    pub armed: bool,
    pub precision_go: bool,
    pub n_labels: i32,
    pub false_personal_upper: f32,
    pub conformal_threshold: f32,
}

pub async fn load_arming(pool: &PgPool, tenant_id: &str) -> Result<ArmingRow, sqlx::Error> {
    let row = sqlx::query(
        "select armed, precision_go, n_labels, false_personal_upper, conformal_threshold \
         from enforcement_arming where tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?;
    Ok(match row {
        Some(r) => ArmingRow {
            armed: r.get("armed"),
            precision_go: r.get("precision_go"),
            n_labels: r.get("n_labels"),
            false_personal_upper: r.get("false_personal_upper"),
            conformal_threshold: r.get("conformal_threshold"),
        },
        None => ArmingRow {
            armed: false,
            precision_go: false,
            n_labels: 0,
            false_personal_upper: 1.0,
            conformal_threshold: 1.01,
        },
    })
}

/// Arm/disarm enforcement. Arming is refused unless the latest gate reads GO
/// (returns `false` when the arm was refused).
pub async fn set_armed(pool: &PgPool, tenant_id: &str, armed: bool) -> Result<bool, sqlx::Error> {
    if armed {
        // Refresh the gate first, then only arm if GO.
        let (report, _) = recompute_and_store(pool, tenant_id).await?;
        if report.decision != GateDecision::Go {
            return Ok(false);
        }
    }
    sqlx::query("update enforcement_arming set armed = $2, decided_at = now() where tenant_id = $1")
        .bind(tenant_id)
        .bind(armed)
        .execute(pool)
        .await?;
    Ok(true)
}

/// The partial gate inputs the control plane computes for one session; the proxy
/// combines them with its own version-pin / self-test / reachability and calls
/// `enforce_gate::decide`.
#[derive(Debug, Clone, Serialize)]
pub struct GateInputsDto {
    pub armed: bool,
    pub precision_go: bool,
    /// work | work_provisional | unclassified | personal_confirmed | personal_soft
    pub class: String,
    pub seat_over_allowance: bool,
}

#[derive(Deserialize)]
pub struct DecisionQuery {
    pub session: String,
    pub seat: String,
}

/// Control-plane endpoint the off-device proxy calls: `GET /v1/enforcement/decision
/// ?session=&seat=` (ingest-token auth). Returns the partial gate inputs; the proxy
/// adds its own version-pin / self-test / reachability and runs `enforce_gate::decide`.
pub async fn decision_endpoint(
    AuthedTenant(tenant): AuthedTenant,
    State(pool): State<PgPool>,
    Query(q): Query<DecisionQuery>,
) -> Result<Json<GateInputsDto>, AppError> {
    let inputs = gate_inputs(&pool, &tenant, &q.session, &q.seat).await?;
    Ok(Json(inputs))
}

/// Compute the gate inputs for a session+seat (the control-plane decision body).
pub async fn gate_inputs(
    pool: &PgPool,
    tenant_id: &str,
    session_id: &str,
    seat_email: &str,
) -> Result<GateInputsDto, sqlx::Error> {
    let arming = load_arming(pool, tenant_id).await?;
    let class = session_class(pool, tenant_id, session_id).await?;
    let over = seat_over_allowance(pool, tenant_id, seat_email).await?;
    Ok(GateInputsDto {
        armed: arming.armed,
        precision_go: arming.precision_go,
        class,
        seat_over_allowance: over,
    })
}

/// Map a session's provenance + triage state to the enforce-gate class string.
async fn session_class(
    pool: &PgPool,
    tenant_id: &str,
    session_id: &str,
) -> Result<String, sqlx::Error> {
    let row = sqlx::query(
        "select sp.class as pclass, sp.resolved_by, st.label as tlabel, st.enforceable, \
                cs.classification \
         from captured_sessions cs \
         left join session_provenance sp on sp.tenant_id=cs.tenant_id and sp.session_id=cs.session_id \
         left join session_triage st on st.tenant_id=cs.tenant_id and st.session_id=cs.session_id \
         where cs.tenant_id=$1 and cs.session_id=$2",
    )
    .bind(tenant_id)
    .bind(session_id)
    .fetch_optional(pool)
    .await?;
    let Some(r) = row else {
        return Ok("unclassified".into());
    };
    let pclass: Option<String> = r.get("pclass");
    let tlabel: Option<String> = r.get("tlabel");
    let enforceable: Option<bool> = r.get("enforceable");
    let classification: String = r.get("classification");

    // Structurally-confirmed personal (provenance) or a human-confirmed verdict.
    if pclass.as_deref() == Some("personal")
        || (enforceable == Some(true) && tlabel.as_deref() == Some("personal"))
    {
        return Ok("personal_confirmed".into());
    }
    match pclass.as_deref() {
        Some("work") => Ok("work".into()),
        Some("work_provisional") => Ok("work_provisional".into()),
        _ => {
            if classification == "personal" {
                Ok("personal_soft".into()) // LLM-only personal — never enforceable
            } else if classification == "work" {
                Ok("work_provisional".into())
            } else {
                Ok("unclassified".into())
            }
        }
    }
}

/// Is this seat over its personal allowance over the rolling 7-day window?
/// (Confirmed personal only; UNCLASSIFIED excluded — same denominator as Usage.)
async fn seat_over_allowance(
    pool: &PgPool,
    tenant_id: &str,
    seat_email: &str,
) -> Result<bool, sqlx::Error> {
    let allowance: i32 = sqlx::query_scalar(
        "select personal_allowance_pct from tenant_limit_config where tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(20);

    let cp = "(sp.class='personal' or (st.enforceable and st.label='personal'))";
    let sql = format!(
        "select \
           count(*) filter (where cs.classification='work') as work, \
           count(*) filter (where cs.classification='personal' and {cp}) as personal_confirmed \
         from captured_sessions cs \
         left join session_provenance sp on sp.tenant_id=cs.tenant_id and sp.session_id=cs.session_id \
         left join session_triage st on st.tenant_id=cs.tenant_id and st.session_id=cs.session_id \
         where cs.tenant_id=$1 and cs.user_email=$2 \
           and coalesce(cs.last_ts, cs.created_at) >= now() - interval '7 days'",
        cp = cp
    );
    let row = sqlx::query(&sql)
        .bind(tenant_id)
        .bind(seat_email)
        .fetch_one(pool)
        .await?;
    let work = row.get::<i64, _>("work") as u32;
    let personal = row.get::<i64, _>("personal_confirmed") as u32;
    let split = ccguard_core::ledger::split(
        &ccguard_core::ledger::UsageCounts {
            work,
            personal_confirmed: personal,
            unclassified: 0,
        },
        allowance.max(0) as u32,
    );
    Ok(split.over_allowance)
}
