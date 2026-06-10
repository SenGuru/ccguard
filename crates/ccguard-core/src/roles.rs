//! Pure job-role modeling + activity-vs-role anomaly detection.
//!
//! Stateless: given a [`JobRole`] and an [`Activity`] summary, return indicators
//! when the observed activity contradicts what the role would predict. No DB,
//! no I/O.

use serde::{Deserialize, Serialize};

/// A coarse job role for a seat/user. Serialized snake_case.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobRole {
    Engineer,
    Marketer,
    Designer,
    Pm,
    Ops,
    Sales,
    Other,
}

impl JobRole {
    /// Parse a snake_case role string. Unknown strings map to [`JobRole::Other`].
    ///
    /// Named `from_str` for call-site clarity; it is infallible (it never errors,
    /// defaulting to `Other`), so it intentionally does not implement `FromStr`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> JobRole {
        match s {
            "engineer" => JobRole::Engineer,
            "marketer" => JobRole::Marketer,
            "designer" => JobRole::Designer,
            "pm" => JobRole::Pm,
            "ops" => JobRole::Ops,
            "sales" => JobRole::Sales,
            _ => JobRole::Other,
        }
    }

    /// Stable snake_case string for storage/binding. Matches the serde repr.
    pub fn as_str(&self) -> &'static str {
        match self {
            JobRole::Engineer => "engineer",
            JobRole::Marketer => "marketer",
            JobRole::Designer => "designer",
            JobRole::Pm => "pm",
            JobRole::Ops => "ops",
            JobRole::Sales => "sales",
            JobRole::Other => "other",
        }
    }

    /// Whether this role is expected to produce code in the normal course of work.
    pub fn expects_code(&self) -> bool {
        matches!(self, JobRole::Engineer | JobRole::Ops)
    }
}

/// Pre-aggregated activity counts for a seat over some window.
pub struct Activity {
    pub code_events: i64,
    pub total_events: i64,
}

/// One indicator that observed activity contradicts the modeled role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleIndicator {
    pub kind: String,
    pub detail: String,
}

/// Indicators when activity contradicts the role. Empty if consistent.
pub fn role_anomalies(role: JobRole, a: &Activity) -> Vec<RoleIndicator> {
    let mut out = vec![];
    if !role.expects_code() && a.code_events >= 5 {
        out.push(RoleIndicator {
            kind: "non_engineer_coding".into(),
            detail: format!(
                "{} code-events from a {} role",
                a.code_events,
                role.as_str()
            ),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_as_str_round_trip_all_variants() {
        let all = [
            JobRole::Engineer,
            JobRole::Marketer,
            JobRole::Designer,
            JobRole::Pm,
            JobRole::Ops,
            JobRole::Sales,
            JobRole::Other,
        ];
        for role in all {
            assert_eq!(JobRole::from_str(role.as_str()), role);
        }
    }

    #[test]
    fn from_str_unknown_is_other() {
        assert_eq!(JobRole::from_str("ceo"), JobRole::Other);
        assert_eq!(JobRole::from_str(""), JobRole::Other);
        assert_eq!(JobRole::from_str("Engineer"), JobRole::Other); // case-sensitive
    }

    #[test]
    fn pm_serde_string_is_pm() {
        assert_eq!(JobRole::Pm.as_str(), "pm");
        assert_eq!(serde_json::to_string(&JobRole::Pm).unwrap(), "\"pm\"");
    }

    #[test]
    fn expects_code_only_engineer_and_ops() {
        assert!(JobRole::Engineer.expects_code());
        assert!(JobRole::Ops.expects_code());
        assert!(!JobRole::Marketer.expects_code());
        assert!(!JobRole::Designer.expects_code());
        assert!(!JobRole::Pm.expects_code());
        assert!(!JobRole::Sales.expects_code());
        assert!(!JobRole::Other.expects_code());
    }

    #[test]
    fn marketer_coding_above_threshold_flags() {
        let out = role_anomalies(
            JobRole::Marketer,
            &Activity {
                code_events: 8,
                total_events: 20,
            },
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, "non_engineer_coding");
        assert!(out[0].detail.contains("8 code-events"));
        assert!(out[0].detail.contains("marketer"));
    }

    #[test]
    fn engineer_coding_is_not_flagged() {
        let out = role_anomalies(
            JobRole::Engineer,
            &Activity {
                code_events: 8,
                total_events: 20,
            },
        );
        assert!(out.is_empty());
    }

    #[test]
    fn marketer_below_threshold_is_not_flagged() {
        let out = role_anomalies(
            JobRole::Marketer,
            &Activity {
                code_events: 2,
                total_events: 20,
            },
        );
        assert!(out.is_empty());
    }
}
