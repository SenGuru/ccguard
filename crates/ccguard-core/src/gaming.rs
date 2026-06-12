//! Metadata-only gaming flags.
//!
//! The asymmetry is absolute: every flag here pushes a session toward HUMAN REVIEW,
//! never toward a `personal` label. Content is attacker-controlled, so a content
//! "gaming classifier" would both be gameable and risk falsely accusing honest devs
//! who write defensive prompts. These flags use only structural data we already
//! have, fire rarely (especially for SMBs without structural signals), but are
//! high-value when they do. Pure; no I/O.

/// The AI judged the session WORK, but the deterministic structural cascade
/// independently says it's confirmed PERSONAL — a contradiction worth a human look
/// (and the only realistic deliberate-gaming tell when a structural signal exists).
pub const LABEL_STRUCTURE_CONFLICT: &str = "label_structure_conflict";

/// True when the AI's `work` label contradicts a confirmed structural `personal`.
pub fn label_structure_conflict(ai_label: &str, provenance_class: Option<&str>) -> bool {
    ai_label.eq_ignore_ascii_case("work") && provenance_class == Some("personal")
}

/// The set of gaming flags that apply to a session (review-only; never alters the
/// label or the meter).
pub fn flags(ai_label: &str, provenance_class: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    if label_structure_conflict(ai_label, provenance_class) {
        out.push(LABEL_STRUCTURE_CONFLICT.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflict_only_when_work_vs_confirmed_personal() {
        assert!(label_structure_conflict("work", Some("personal")));
        assert!(label_structure_conflict("WORK", Some("personal")));
        assert!(!label_structure_conflict("personal", Some("personal")));
        assert!(!label_structure_conflict("work", Some("work")));
        assert!(!label_structure_conflict("work", None));
    }

    #[test]
    fn flags_lists_the_conflict() {
        assert_eq!(flags("work", Some("personal")), vec![LABEL_STRUCTURE_CONFLICT]);
        assert!(flags("work", Some("work")).is_empty());
        assert!(flags("unsure", Some("personal")).is_empty());
    }
}
