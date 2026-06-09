use chrono::{DateTime, Utc};

use ccguard_core::event::{Activity, CcEvent, User};

use crate::parse::Interaction;
use crate::pricing::estimate_cost;
use crate::repo::repo_for_cwd;

/// Map a parsed interaction to a CcEvent ready to POST. Returns None if it has no cwd or an
/// unparseable timestamp. `tenant_id` is left empty (the server sets it from the ingest token).
pub fn interaction_to_event(i: &Interaction, user_email: &str) -> Option<CcEvent> {
    let cwd = i.cwd.clone()?;
    let ts: DateTime<Utc> = i.ts.parse().ok()?;
    let repo = repo_for_cwd(&cwd);
    let cost = estimate_cost(&i.model, i.tokens_in, i.tokens_out);

    Some(CcEvent {
        tenant_id: String::new(),
        user: User {
            email: user_email.to_string(),
            seat_id: None,
        },
        tool: "claude-code".to_string(),
        session_id: i.session_id.clone(),
        ts,
        repo,
        content_ref: None,
        source_layer: "endpoint_agent".to_string(),
        activity: Activity {
            kind: "api_request".to_string(),
            tokens_in: i.tokens_in,
            tokens_out: i.tokens_out,
            cost_usd: cost,
            model: Some(i.model.clone()),
            tool_name: None,
            decision: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_event_with_tokens_cost_and_model() {
        // cwd that is not a git repo → host/org None, path set; tokens/model/cost populated.
        let i = Interaction {
            session_id: "s1".into(),
            ts: "2026-06-10T10:00:01Z".into(),
            cwd: Some(std::env::temp_dir().to_string_lossy().to_string()),
            model: "claude-opus-4-8".into(),
            tokens_in: 1_000_000,
            tokens_out: 0,
        };
        let ev = interaction_to_event(&i, "dev@acme.com").unwrap();
        assert_eq!(ev.tool, "claude-code");
        assert_eq!(ev.source_layer, "endpoint_agent");
        assert_eq!(ev.tenant_id, "");
        assert_eq!(ev.user.email, "dev@acme.com");
        assert_eq!(ev.activity.tokens_in, 1_000_000);
        assert_eq!(ev.activity.model.as_deref(), Some("claude-opus-4-8"));
        assert!((ev.activity.cost_usd - 15.0).abs() < 1e-9); // 1M opus input @ $15
    }

    #[test]
    fn none_without_cwd_or_bad_ts() {
        let base = Interaction {
            session_id: "s".into(),
            ts: "2026-06-10T10:00:01Z".into(),
            cwd: None,
            model: "claude-sonnet-4-6".into(),
            tokens_in: 10,
            tokens_out: 5,
        };
        assert!(interaction_to_event(&base, "x").is_none()); // no cwd

        let bad_ts = Interaction { cwd: Some("/tmp".into()), ts: "not-a-date".into(), ..base };
        assert!(interaction_to_event(&bad_ts, "x").is_none());
    }
}
