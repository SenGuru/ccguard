//! Conformal selective calibration for the Tier-A judge.
//!
//! Raw LLM confidence is systematically overconfident and unusable as a gate, and
//! on a self-host / Bedrock / Vertex endpoint we have no logit access for
//! temperature scaling. This is a black-box selective-risk calibration: from past
//! verdicts whose truth we later confirmed (a structural provenance signal or a
//! human ruling), learn the confidence threshold above which the judge's
//! *accepted* error is provably ≤ α (finite-sample Wilson bound). Below it the
//! judge must **abstain** (defer to review) rather than label-force. With too few
//! labels — or a confidently-wrong model — it abstains on everything.
//!
//! Pure; no I/O.

/// One historical judged session whose correctness was later confirmed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalibrationPoint {
    /// The judge's reported confidence in [0,1].
    pub confidence: f32,
    /// Whether the judge's label turned out correct.
    pub correct: bool,
}

/// A fitted calibration: the confidence threshold to accept (act on) a verdict.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Calibration {
    /// Accept a verdict only when `confidence >= threshold`. A value > 1.0 means
    /// "no threshold controls the risk — abstain on everything."
    pub threshold: f32,
    /// Calibration-set size.
    pub n: usize,
    /// Target accepted-error rate (e.g. 0.1).
    pub alpha: f32,
    /// False until enough labels exist (abstains on all while false).
    pub usable: bool,
}

/// Accept = act on the verdict; Abstain = defer to the review queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectiveDecision {
    Accept,
    Abstain,
}

/// Which operating regime the calibration is in — drives the dashboard banner so
/// the admin understands what the judge is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationRegime {
    /// Too few labels yet — the judge applies its label for visibility only and
    /// never abstains (it can't vouch, but it must still produce verdicts to review).
    Cold,
    /// Enough labels and a usable threshold — abstains below it.
    Calibrated,
    /// Enough labels but NO cutoff controls the error (the judge is confidently
    /// wrong on this tenant's data) — abstains on everything → all to review. This
    /// almost always means the work definition is the problem.
    Degenerate,
}

impl Calibration {
    /// The current operating regime.
    pub fn regime(&self) -> CalibrationRegime {
        if !self.usable {
            CalibrationRegime::Cold
        } else if self.threshold > 1.0 {
            CalibrationRegime::Degenerate
        } else {
            CalibrationRegime::Calibrated
        }
    }
}

/// 95% Wilson upper bound for `x` errors in `m` accepted predictions.
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

/// Fit the selective threshold: the **lowest** confidence cutoff whose accepted
/// set has a Wilson-upper-bounded error ≤ `alpha` (so it accepts as much as it
/// safely can). Fewer than `min_n` labels → not usable. No cutoff controls the
/// risk (a confidently-wrong model) → `threshold > 1.0` so everything abstains.
pub fn calibrate(points: &[CalibrationPoint], alpha: f32, min_n: usize) -> Calibration {
    let n = points.len();
    let alpha = alpha.clamp(0.0, 1.0);
    if n < min_n.max(1) {
        return Calibration { threshold: 1.01, n, alpha, usable: false };
    }

    // Candidate cutoffs = the distinct confidences, ascending.
    let mut confs: Vec<f32> = points.iter().map(|p| p.confidence.clamp(0.0, 1.0)).collect();
    confs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    confs.dedup();

    for &tau in &confs {
        let accepted: Vec<&CalibrationPoint> =
            points.iter().filter(|p| p.confidence.clamp(0.0, 1.0) >= tau).collect();
        if accepted.is_empty() {
            continue;
        }
        let errors = accepted.iter().filter(|p| !p.correct).count();
        if wilson_upper(errors, accepted.len()) <= alpha {
            return Calibration { threshold: tau, n, alpha, usable: true };
        }
    }

    // No cutoff controls the risk → usable (we had the labels) but abstain on all.
    Calibration { threshold: 1.01, n, alpha, usable: true }
}

/// Apply the calibration to a fresh verdict's confidence.
pub fn decide(confidence: f32, cal: &Calibration) -> SelectiveDecision {
    if cal.usable && confidence.clamp(0.0, 1.0) >= cal.threshold {
        SelectiveDecision::Accept
    } else {
        SelectiveDecision::Abstain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(confidence: f32, correct: bool) -> CalibrationPoint {
        CalibrationPoint { confidence, correct }
    }

    #[test]
    fn too_few_labels_abstains_on_everything() {
        let cal = calibrate(&[pt(0.99, true)], 0.1, 50);
        assert!(!cal.usable);
        assert_eq!(decide(0.99, &cal), SelectiveDecision::Abstain);
    }

    #[test]
    fn errors_concentrated_at_low_confidence_set_a_mid_threshold() {
        // High-confidence band is clean; the low-confidence band carries the errors.
        let mut points: Vec<_> = (0..40).map(|_| pt(0.95, true)).collect();
        points.extend((0..10).map(|_| pt(0.60, true)));
        points.extend((0..10).map(|_| pt(0.55, false))); // errors live at 0.55
        let cal = calibrate(&points, 0.10, 50);
        assert!(cal.usable);
        // Accept the confident region, abstain on the error-prone low-confidence one.
        assert_eq!(decide(0.9, &cal), SelectiveDecision::Accept);
        assert_eq!(decide(0.55, &cal), SelectiveDecision::Abstain);
        assert!(cal.threshold > 0.55 && cal.threshold <= 0.95);
    }

    #[test]
    fn confidently_wrong_model_abstains_on_everything() {
        // 50 confident-and-WRONG + 10 confident-and-right at the same 0.9 → no cutoff
        // can control the error → abstain on all, even a 0.9 verdict.
        let mut points: Vec<_> = (0..50).map(|_| pt(0.9, false)).collect();
        points.extend((0..10).map(|_| pt(0.9, true)));
        let cal = calibrate(&points, 0.10, 50);
        assert!(cal.usable);
        assert!(cal.threshold > 1.0, "threshold {} should be the abstain-all sentinel", cal.threshold);
        assert_eq!(decide(0.9, &cal), SelectiveDecision::Abstain);
    }

    #[test]
    fn stricter_alpha_demands_at_least_as_high_a_threshold() {
        let mut points: Vec<_> = (0..30).map(|_| pt(0.70, true)).collect();
        points.extend((0..6).map(|_| pt(0.70, false))); // some error in the 0.70 band
        points.extend((0..30).map(|_| pt(0.95, true))); // clean high band
        let strict = calibrate(&points, 0.02, 50);
        let loose = calibrate(&points, 0.30, 50);
        assert!(strict.threshold >= loose.threshold);
    }

    #[test]
    fn threshold_is_bounded() {
        let points: Vec<_> = (0..60).map(|i| pt((i as f32) / 60.0, i % 3 != 0)).collect();
        let cal = calibrate(&points, 0.1, 50);
        assert!(cal.threshold >= 0.0 && cal.threshold <= 1.01);
    }

    #[test]
    fn clean_model_accepts_broadly() {
        // Spread of confidences, all correct → lowest cutoff controls → accept broadly.
        let points: Vec<_> = (0..60).map(|i| pt(0.5 + (i as f32) / 200.0, true)).collect();
        let cal = calibrate(&points, 0.1, 50);
        assert!(cal.usable);
        assert_eq!(decide(0.6, &cal), SelectiveDecision::Accept);
    }
}
