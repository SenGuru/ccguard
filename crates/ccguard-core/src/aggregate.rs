use std::collections::HashMap;

use crate::event::{CcEvent, Classification};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Totals {
    pub cost_usd: f64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub events: u64,
}

/// Sum spend/usage grouped by the event's repo classification.
/// Events with no classification set are counted as Unknown.
pub fn totals_by_classification(events: &[CcEvent]) -> HashMap<Classification, Totals> {
    let mut out: HashMap<Classification, Totals> = HashMap::new();
    for e in events {
        let class = e.repo.classification.unwrap_or(Classification::Unknown);
        let t = out.entry(class).or_default();
        t.cost_usd += e.activity.cost_usd;
        t.tokens_in += e.activity.tokens_in;
        t.tokens_out += e.activity.tokens_out;
        t.events += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Activity, Repo, User};
    use chrono::Utc;

    fn ev(class: Classification, cost: f64) -> CcEvent {
        CcEvent {
            tenant_id: "acme".into(),
            user: User { email: "d@acme.com".into(), seat_id: None },
            tool: "claude-code".into(),
            session_id: "s".into(),
            ts: Utc::now(),
            repo: Repo {
                host: None, org: None, name: None, path: None,
                classification: Some(class), confidence: 0.9,
            },
            content_ref: None,
            source_layer: "test".into(),
            activity: Activity {
                kind: "api_request".into(),
                tokens_in: 10, tokens_out: 5, cost_usd: cost,
                model: None, tool_name: None, decision: None,
            },
        }
    }

    #[test]
    fn sums_cost_per_classification() {
        let events = vec![
            ev(Classification::Work, 1.0),
            ev(Classification::Work, 0.5),
            ev(Classification::Personal, 0.25),
        ];
        let totals = totals_by_classification(&events);
        assert_eq!(totals[&Classification::Work].cost_usd, 1.5);
        assert_eq!(totals[&Classification::Work].events, 2);
        assert_eq!(totals[&Classification::Personal].cost_usd, 0.25);
        assert!(!totals.contains_key(&Classification::Unknown));
    }
}
