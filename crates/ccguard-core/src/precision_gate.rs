//! Build-time GO/NO-GO precision gate for arming enforcement.
//!
//! Enforcement (the off-device proxy) may only be armed after PERSONAL-class
//! precision clears a contractually-agreed floor on a labeled, stratified holdout
//! — because the expensive mistake is throttling a developer whose session was
//! actually work. We measure the **false-personal rate** (personal predictions
//! that were really work) with a Wilson upper confidence bound and refuse to arm
//! (NO-GO) until both the label count and that bound clear the floor.
//!
//! Pure; no I/O. The server feeds confirmed labels (provenance ground truth vs the
//! predicted label) and surfaces the report on the arming page.

/// One labeled outcome: what we predicted vs the confirmed truth (personal y/n).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabeledOutcome {
    pub predicted_personal: bool,
    pub actual_personal: bool,
}

/// GO if enforcement may be armed; NO-GO otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDecision {
    Go,
    NoGo,
}

/// The measured precision report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GateReport {
    pub n: usize,
    /// TP / (TP + FP) among personal predictions; 1.0 when none were predicted.
    pub personal_precision: f32,
    /// FP / predicted-personal — a personal call that was actually work.
    pub false_personal_rate: f32,
    /// 95% Wilson upper bound on `false_personal_rate` (the number the gate uses).
    pub false_personal_upper_ci: f32,
    /// FN / actual-personal — a real personal session we called work (missed).
    pub missed_personal_rate: f32,
    pub min_labels_met: bool,
    pub floor_met: bool,
    pub decision: GateDecision,
}

/// 95% Wilson score-interval upper bound for `x` successes in `m` trials.
fn wilson_upper(x: usize, m: usize) -> f32 {
    if m == 0 {
        return 1.0;
    }
    let z = 1.96f64;
    let n = m as f64;
    let p = x as f64 / n;
    let z2 = z * z;
    let denom = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denom;
    let margin = (z * ((p * (1.0 - p) / n) + z2 / (4.0 * n * n)).sqrt()) / denom;
    ((center + margin).min(1.0)) as f32
}

/// Evaluate the holdout and decide GO/NO-GO.
///
/// `min_labels` — minimum stratified labels required (e.g. 200).
/// `max_false_personal` — the agreed floor the Wilson upper bound must not exceed.
pub fn evaluate(
    labels: &[LabeledOutcome],
    min_labels: usize,
    max_false_personal: f32,
) -> GateReport {
    let n = labels.len();
    let predicted_personal = labels.iter().filter(|l| l.predicted_personal).count();
    let tp = labels
        .iter()
        .filter(|l| l.predicted_personal && l.actual_personal)
        .count();
    let fp = predicted_personal - tp;
    let actual_personal = labels.iter().filter(|l| l.actual_personal).count();
    let fn_ = labels
        .iter()
        .filter(|l| l.actual_personal && !l.predicted_personal)
        .count();

    let personal_precision = if predicted_personal == 0 {
        1.0
    } else {
        tp as f32 / predicted_personal as f32
    };
    let false_personal_rate = if predicted_personal == 0 {
        0.0
    } else {
        fp as f32 / predicted_personal as f32
    };
    let false_personal_upper_ci = wilson_upper(fp, predicted_personal);
    let missed_personal_rate = if actual_personal == 0 {
        0.0
    } else {
        fn_ as f32 / actual_personal as f32
    };

    let min_labels_met = n >= min_labels;
    // The bound is only meaningful once some personal predictions exist; if the
    // model never predicts personal there is nothing to over-throttle (floor met).
    let floor_met = if predicted_personal == 0 {
        true
    } else {
        false_personal_upper_ci <= max_false_personal
    };
    let decision = if min_labels_met && floor_met {
        GateDecision::Go
    } else {
        GateDecision::NoGo
    };

    GateReport {
        n,
        personal_precision,
        false_personal_rate,
        false_personal_upper_ci,
        missed_personal_rate,
        min_labels_met,
        floor_met,
        decision,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lo(pred: bool, actual: bool) -> LabeledOutcome {
        LabeledOutcome { predicted_personal: pred, actual_personal: actual }
    }

    #[test]
    fn too_few_labels_is_nogo_even_if_perfect() {
        let labels: Vec<_> = (0..50).map(|_| lo(true, true)).collect();
        let r = evaluate(&labels, 200, 0.05);
        assert!(!r.min_labels_met);
        assert_eq!(r.decision, GateDecision::NoGo);
    }

    #[test]
    fn clean_separation_at_scale_is_go() {
        // 300 labels: 100 correct personal, 200 correct work, no false personal.
        let mut labels: Vec<_> = (0..100).map(|_| lo(true, true)).collect();
        labels.extend((0..200).map(|_| lo(false, false)));
        let r = evaluate(&labels, 200, 0.05);
        assert!(r.min_labels_met);
        assert_eq!(r.personal_precision, 1.0);
        assert!(r.false_personal_upper_ci <= 0.05, "upper ci {}", r.false_personal_upper_ci);
        assert_eq!(r.decision, GateDecision::Go);
    }

    #[test]
    fn high_false_personal_is_nogo() {
        // 300 labels but 30% of personal calls are actually work.
        let mut labels: Vec<_> = (0..70).map(|_| lo(true, true)).collect();
        labels.extend((0..30).map(|_| lo(true, false))); // false personal
        labels.extend((0..200).map(|_| lo(false, false)));
        let r = evaluate(&labels, 200, 0.05);
        assert!(r.false_personal_rate > 0.2);
        assert!(!r.floor_met);
        assert_eq!(r.decision, GateDecision::NoGo);
    }

    #[test]
    fn never_predicting_personal_meets_floor() {
        let labels: Vec<_> = (0..250).map(|_| lo(false, false)).collect();
        let r = evaluate(&labels, 200, 0.05);
        assert_eq!(r.personal_precision, 1.0);
        assert!(r.floor_met);
        assert_eq!(r.decision, GateDecision::Go);
    }

    #[test]
    fn wilson_upper_exceeds_point_estimate_for_small_n() {
        // 0 FP out of 10 → point estimate 0 but the upper bound is well above 0.
        let labels: Vec<_> = (0..10).map(|_| lo(true, true)).collect();
        let r = evaluate(&labels, 5, 0.05);
        assert_eq!(r.false_personal_rate, 0.0);
        assert!(r.false_personal_upper_ci > 0.05, "tiny-n upper ci should be loose: {}", r.false_personal_upper_ci);
        // min_labels met (5) but the loose CI fails the floor → NoGo.
        assert_eq!(r.decision, GateDecision::NoGo);
    }
}
