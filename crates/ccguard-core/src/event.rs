use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CcEvent {
    pub tenant_id: String,
    pub user: User,
    pub tool: String,
    pub session_id: String,
    pub ts: DateTime<Utc>,
    pub repo: Repo,
    #[serde(default)]
    pub content_ref: Option<String>,
    pub source_layer: String,
    pub activity: Activity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub email: String,
    #[serde(default)]
    pub seat_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Repo {
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub org: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub classification: Option<Classification>,
    #[serde(default)]
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Classification {
    Work,
    Personal,
    Unknown,
}

impl Classification {
    pub fn as_str(&self) -> &'static str {
        match self {
            Classification::Work => "work",
            Classification::Personal => "personal",
            Classification::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Activity {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub tokens_in: i64,
    #[serde(default)]
    pub tokens_out: i64,
    #[serde(default)]
    pub cost_usd: f64,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub decision: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_minimal_event_with_defaults() {
        let json = r#"{
            "tenant_id": "acme",
            "user": { "email": "dev@acme.com" },
            "tool": "claude-code",
            "session_id": "s1",
            "ts": "2026-06-09T21:13:00Z",
            "repo": { "host": "github.com", "org": "acme-corp", "name": "billing" },
            "source_layer": "endpoint_agent",
            "activity": { "type": "api_request", "cost_usd": 0.12, "tokens_in": 100 }
        }"#;
        let ev: CcEvent = serde_json::from_str(json).unwrap();
        assert_eq!(ev.tenant_id, "acme");
        assert_eq!(ev.user.email, "dev@acme.com");
        assert_eq!(ev.user.seat_id, None);
        assert_eq!(ev.repo.org.as_deref(), Some("acme-corp"));
        assert_eq!(ev.repo.classification, None);
        assert_eq!(ev.activity.cost_usd, 0.12);
        assert_eq!(ev.activity.tokens_out, 0);
    }

    #[test]
    fn classification_serializes_lowercase() {
        let s = serde_json::to_string(&Classification::Work).unwrap();
        assert_eq!(s, "\"work\"");
    }
}
