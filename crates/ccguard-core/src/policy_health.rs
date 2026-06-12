//! Policy health — plain-language signals that tell an admin whether their business
//! description is doing its job, computed from verdicts we already store. Surfaced
//! in the test-before-publish dry-run and on the dashboard. Pure; no I/O.

/// One verdict row for the health computation.
#[derive(Debug, Clone)]
pub struct HealthRow {
    /// The judge's label: "work" | "personal" | "unsure".
    pub label: String,
    /// A reviewer's independent label, if any.
    pub human_label: Option<String>,
    /// Whether the verdict cited a clause of the company definition.
    pub matched_clause: bool,
}

/// The three plain-language health metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Health {
    pub n: usize,
    /// Decisiveness: share of verdicts that came back `unsure`. High → "the AI can't
    /// tell what your work looks like; add concrete examples." The #1 vague-description symptom.
    pub unsure_rate: f32,
    /// Among NON-unsure verdicts, share with no matched clause — a softer vagueness signal.
    pub no_clause_rate: f32,
    /// Among human-reviewed rows, share the draft now agrees with (1.0 if none reviewed).
    pub agreement_rate: f32,
    pub n_human: usize,
}

pub fn health(rows: &[HealthRow]) -> Health {
    let n = rows.len();
    if n == 0 {
        return Health { n: 0, unsure_rate: 0.0, no_clause_rate: 0.0, agreement_rate: 1.0, n_human: 0 };
    }
    let unsure = rows.iter().filter(|r| r.label.eq_ignore_ascii_case("unsure")).count();
    let decided: Vec<&HealthRow> = rows.iter().filter(|r| !r.label.eq_ignore_ascii_case("unsure")).collect();
    let no_clause = decided.iter().filter(|r| !r.matched_clause).count();
    let human: Vec<&HealthRow> = rows.iter().filter(|r| r.human_label.is_some()).collect();
    let agree = human
        .iter()
        .filter(|r| r.human_label.as_deref().map(|h| h.eq_ignore_ascii_case(&r.label)).unwrap_or(false))
        .count();

    Health {
        n,
        unsure_rate: unsure as f32 / n as f32,
        no_clause_rate: if decided.is_empty() { 0.0 } else { no_clause as f32 / decided.len() as f32 },
        agreement_rate: if human.is_empty() { 1.0 } else { agree as f32 / human.len() as f32 },
        n_human: human.len(),
    }
}

/// One labelled session (for flip detection).
#[derive(Debug, Clone)]
pub struct LabeledSession {
    pub session_id: String,
    pub label: String,
}

/// Sessions that would flip **work → personal** between `prev` and `next` — the
/// dangerous direction (a false accusation). Used to BLOCK publishing a draft that
/// would flip a session the admin previously confirmed as work.
pub fn work_to_personal_flips(prev: &[LabeledSession], next: &[LabeledSession]) -> Vec<String> {
    let mut out = Vec::new();
    for n in next {
        if n.label.eq_ignore_ascii_case("personal") {
            if let Some(p) = prev.iter().find(|p| p.session_id == n.session_id) {
                if p.label.eq_ignore_ascii_case("work") {
                    out.push(n.session_id.clone());
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(label: &str, human: Option<&str>, matched: bool) -> HealthRow {
        HealthRow { label: label.into(), human_label: human.map(str::to_string), matched_clause: matched }
    }

    #[test]
    fn empty_is_neutral() {
        let h = health(&[]);
        assert_eq!(h.n, 0);
        assert_eq!(h.agreement_rate, 1.0);
    }

    #[test]
    fn unsure_rate_and_no_clause_rate() {
        let rows = vec![
            row("work", None, true),
            row("work", None, false),
            row("unsure", None, false),
            row("personal", None, false),
        ];
        let h = health(&rows);
        assert!((h.unsure_rate - 0.25).abs() < 1e-6); // 1 of 4
        // decided = 3 (work,work,personal); no_clause among decided = 2/3
        assert!((h.no_clause_rate - (2.0 / 3.0)).abs() < 1e-6);
    }

    #[test]
    fn agreement_only_over_human_rows() {
        let rows = vec![
            row("work", Some("work"), true),     // agree
            row("personal", Some("work"), false), // disagree
            row("work", None, true),              // not human-reviewed
        ];
        let h = health(&rows);
        assert_eq!(h.n_human, 2);
        assert!((h.agreement_rate - 0.5).abs() < 1e-6);
    }

    #[test]
    fn flips_detects_work_to_personal_only() {
        let prev = vec![
            LabeledSession { session_id: "a".into(), label: "work".into() },
            LabeledSession { session_id: "b".into(), label: "personal".into() },
            LabeledSession { session_id: "c".into(), label: "work".into() },
        ];
        let next = vec![
            LabeledSession { session_id: "a".into(), label: "personal".into() }, // flip!
            LabeledSession { session_id: "b".into(), label: "work".into() },      // personal→work (safe)
            LabeledSession { session_id: "c".into(), label: "work".into() },      // unchanged
        ];
        assert_eq!(work_to_personal_flips(&prev, &next), vec!["a"]);
    }
}
