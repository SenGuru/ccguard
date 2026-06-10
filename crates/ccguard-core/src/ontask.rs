//! Pure on-task scoring + ticket-reference extraction.
//!
//! Stateless: feed it pre-aggregated signals about a session, get back a score,
//! a label, and human-readable reasons. No DB, no I/O. The scoring is fully
//! deterministic — the same [`OnTaskSignals`] always produces the same output.

use crate::event::Classification;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

// --- Compiled patterns (compiled once, lazily) ---------------------------------

/// JIRA-style ticket key: e.g. `PROJ-42`, `ABC1-7`. Upper-case leading letter,
/// 1..=9 more upper-case alphanumerics, a hyphen, then digits.
static JIRA_KEY: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b[A-Z][A-Z0-9]{1,9}-\d+\b").unwrap());

/// GitHub-style issue/PR reference: `#123`. Must be preceded by start-of-string
/// or whitespace so we don't match e.g. `no-12`. The number is captured.
static GH_REF: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:^|\s)#(\d{1,7})\b").unwrap());

/// Extract ticket references from `content`.
///
/// Recognizes JIRA keys (`KEY-123`) and GitHub refs (`#123`, stored as `#<n>`).
/// De-duped across both kinds, preserving first-seen order.
pub fn extract_ticket_refs(content: &str) -> Vec<String> {
    // Collect (byte_offset, value) for every match, then sort by offset so the
    // final order is true first-seen order across both pattern kinds.
    let mut hits: Vec<(usize, String)> = Vec::new();

    for m in JIRA_KEY.find_iter(content) {
        hits.push((m.start(), m.as_str().to_string()));
    }
    for caps in GH_REF.captures_iter(content) {
        let num = caps.get(1).unwrap();
        // Use the number's start so ordering is stable and the leading
        // whitespace (if any) doesn't skew it.
        hits.push((num.start(), format!("#{}", num.as_str())));
    }

    hits.sort_by_key(|(offset, _)| *offset);

    let mut out: Vec<String> = Vec::new();
    for (_, value) in hits {
        if !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

/// Pre-aggregated, per-session signals fed into [`score`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnTaskSignals {
    pub classification: Classification,
    pub committed: bool,
    pub pr_opened: bool,
    pub ticket_referenced: bool,
    pub total_events: i64,
    pub assistant_events: i64,
}

/// Coarse on-task label derived from the numeric score.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnTaskLabel {
    OnTask,
    Review,
    OffTask,
}

impl OnTaskLabel {
    /// Stable snake_case string for storage/binding. Matches the serde repr.
    pub fn as_str(&self) -> &'static str {
        match self {
            OnTaskLabel::OnTask => "on_task",
            OnTaskLabel::Review => "review",
            OnTaskLabel::OffTask => "off_task",
        }
    }
}

/// Deterministically score a session's on-task signals.
///
/// Returns the clamped score (0..=100), the derived [`OnTaskLabel`], and an
/// ordered list of human-readable reasons explaining the contributions.
pub fn score(s: &OnTaskSignals) -> (i32, OnTaskLabel, Vec<String>) {
    let mut score: i32 = 50;
    let mut reasons: Vec<String> = Vec::new();

    match s.classification {
        Classification::Work => {
            score += 25;
        }
        Classification::Unknown => {
            score -= 10;
            reasons.push("unclassified repo".to_string());
        }
        Classification::Personal => {
            score -= 40;
            reasons.push("personal repo".to_string());
        }
    }

    if s.committed {
        score += 15;
        reasons.push("produced a commit".to_string());
    } else {
        reasons.push("no commit landed".to_string());
    }

    if s.pr_opened {
        score += 10;
        reasons.push("opened a PR".to_string());
    }

    if s.ticket_referenced {
        score += 10;
        reasons.push("references a tracked ticket".to_string());
    }

    // Abandoned: there was activity but the assistant never produced output.
    if s.total_events >= 2 && s.assistant_events == 0 {
        score -= 20;
        reasons.push("abandoned session (no output)".to_string());
    }

    let score = score.clamp(0, 100);

    let label = if score >= 70 {
        OnTaskLabel::OnTask
    } else if score >= 40 {
        OnTaskLabel::Review
    } else {
        OnTaskLabel::OffTask
    };

    (score, label, reasons)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signals(
        classification: Classification,
        committed: bool,
        pr_opened: bool,
        ticket_referenced: bool,
        total_events: i64,
        assistant_events: i64,
    ) -> OnTaskSignals {
        OnTaskSignals {
            classification,
            committed,
            pr_opened,
            ticket_referenced,
            total_events,
            assistant_events,
        }
    }

    #[test]
    fn extracts_jira_and_github_refs_deduped_in_order() {
        let got = extract_ticket_refs("fix PROJ-42 and #17 and lowercase no-12 and PROJ-42 again");
        assert_eq!(got, vec!["PROJ-42".to_string(), "#17".to_string()]);
    }

    #[test]
    fn ticket_extraction_handles_leading_ref_and_empty() {
        // `#5` at the very start (preceded by start-of-string) must match.
        assert_eq!(extract_ticket_refs("#5 first"), vec!["#5".to_string()]);
        // No refs at all.
        assert!(extract_ticket_refs("just prose, nothing tracked").is_empty());
        // `no-12` must NOT be treated as a GitHub ref (no leading whitespace/start
        // before the `#`) and `12` alone is not a ref.
        assert!(extract_ticket_refs("filename no-12.txt").is_empty());
    }

    #[test]
    fn work_commit_ticket_is_high_and_on_task() {
        let (score, label, _reasons) =
            score(&signals(Classification::Work, true, true, true, 10, 6));
        // 50 +25 +15 +10 +10 = 110 -> clamped 100.
        assert_eq!(score, 100);
        assert_eq!(label, OnTaskLabel::OnTask);
    }

    #[test]
    fn personal_no_commit_is_low_off_task_with_reason() {
        let (score, label, reasons) =
            score(&signals(Classification::Personal, false, false, false, 4, 2));
        // 50 -40 = 10.
        assert_eq!(score, 10);
        assert_eq!(label, OnTaskLabel::OffTask);
        assert!(reasons.contains(&"personal repo".to_string()));
        assert!(reasons.contains(&"no commit landed".to_string()));
    }

    #[test]
    fn unknown_no_commit_is_review() {
        let (score, label, reasons) =
            score(&signals(Classification::Unknown, false, false, false, 3, 1));
        // 50 -10 = 40 -> Review band starts at 40.
        assert_eq!(score, 40);
        assert_eq!(label, OnTaskLabel::Review);
        assert!(reasons.contains(&"unclassified repo".to_string()));
    }

    #[test]
    fn abandoned_session_subtracts() {
        let with = score(&signals(Classification::Work, false, false, false, 5, 0));
        let without = score(&signals(Classification::Work, false, false, false, 5, 3));
        // 50 +25 -20 = 55 (abandoned) vs 50 +25 = 75 (not abandoned).
        assert_eq!(with.0, 55);
        assert_eq!(without.0, 75);
        assert!(with.0 < without.0);
        assert!(with
            .2
            .contains(&"abandoned session (no output)".to_string()));
        // total_events < 2 means it is NOT abandoned even with zero assistant output.
        let single = score(&signals(Classification::Work, false, false, false, 1, 0));
        assert_eq!(single.0, 75);
        assert!(!single
            .2
            .contains(&"abandoned session (no output)".to_string()));
    }

    #[test]
    fn score_is_clamped_to_0_100() {
        // Maximal positive: clamped at 100.
        let high = score(&signals(Classification::Work, true, true, true, 10, 8));
        assert_eq!(high.0, 100);
        // Maximal negative: personal + abandoned + no commit.
        // 50 -40 -20 = -10 -> clamped 0.
        let low = score(&signals(Classification::Personal, false, false, false, 5, 0));
        assert_eq!(low.0, 0);
        assert_eq!(low.1, OnTaskLabel::OffTask);
    }

    #[test]
    fn scoring_is_deterministic() {
        let s = signals(Classification::Work, true, false, true, 7, 4);
        let a = score(&s);
        let b = score(&s);
        assert_eq!(a, b);
    }

    #[test]
    fn label_band_boundaries() {
        // Exactly 70 -> OnTask; exactly 69 -> Review; exactly 40 -> Review;
        // exactly 39 -> OffTask. Build scores via known signal combos.
        // Work + commit = 90 (OnTask).
        assert_eq!(
            score(&signals(Classification::Work, true, false, false, 5, 3)).1,
            OnTaskLabel::OnTask
        );
        // Unknown + commit = 50 +(-10) +15 = 55 (Review).
        assert_eq!(
            score(&signals(Classification::Unknown, true, false, false, 5, 3)).1,
            OnTaskLabel::Review
        );
        // Personal + commit = 50 -40 +15 = 25 (OffTask).
        assert_eq!(
            score(&signals(Classification::Personal, true, false, false, 5, 3)).1,
            OnTaskLabel::OffTask
        );
    }

    #[test]
    fn label_as_str_matches_serde_repr() {
        assert_eq!(OnTaskLabel::OnTask.as_str(), "on_task");
        assert_eq!(OnTaskLabel::Review.as_str(), "review");
        assert_eq!(OnTaskLabel::OffTask.as_str(), "off_task");
        // Confirm serde produces the same snake_case strings.
        assert_eq!(
            serde_json::to_string(&OnTaskLabel::OnTask).unwrap(),
            "\"on_task\""
        );
        assert_eq!(
            serde_json::to_string(&OnTaskLabel::OffTask).unwrap(),
            "\"off_task\""
        );
    }
}
