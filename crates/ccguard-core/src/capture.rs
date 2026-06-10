use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::event::Repo;

/// The typed kind of one activity atom in a Claude Code session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    UserPrompt,
    AssistantText,
    Thinking,
    ToolCall,
    ToolResult,
    FileEdit,
    BashCommand,
    Pr,
}

impl EventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventKind::UserPrompt => "user_prompt",
            EventKind::AssistantText => "assistant_text",
            EventKind::Thinking => "thinking",
            EventKind::ToolCall => "tool_call",
            EventKind::ToolResult => "tool_result",
            EventKind::FileEdit => "file_edit",
            EventKind::BashCommand => "bash_command",
            EventKind::Pr => "pr",
        }
    }
}

/// One captured activity atom. `content` is the verbatim text/diff/command/output (becomes a deduped blob).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapturedEvent {
    pub seq: i64,
    pub ts: DateTime<Utc>,
    pub kind: EventKind,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tokens_in: i64,
    #[serde(default)]
    pub tokens_out: i64,
    #[serde(default)]
    pub is_sidechain: bool,
}

/// A full session capture: metadata + ordered events. Posted as one batch to /v1/capture.
/// `tenant_id` is set by the server from the ingest token (not the body).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapturedSession {
    pub session_id: String,
    pub user_email: String,
    pub repo: Repo,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    pub events: Vec<CapturedEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_and_kind_snake_case() {
        let json = r#"{
            "session_id":"s1","user_email":"dev@acme.com",
            "repo":{"host":"github.com","org":"acme-corp","name":"r"},
            "title":"Build the thing","cwd":"C:\\w\\r",
            "events":[
              {"seq":0,"ts":"2026-06-10T10:00:00Z","kind":"user_prompt","content":"do X"},
              {"seq":1,"ts":"2026-06-10T10:00:01Z","kind":"tool_call","tool_name":"Bash","target":"git status","content":"{\"command\":\"git status\"}"}
            ]
        }"#;
        let s: CapturedSession = serde_json::from_str(json).unwrap();
        assert_eq!(s.session_id, "s1");
        assert_eq!(s.events.len(), 2);
        assert_eq!(s.events[0].kind, EventKind::UserPrompt);
        assert_eq!(s.events[1].tool_name.as_deref(), Some("Bash"));
        assert_eq!(EventKind::ToolResult.as_str(), "tool_result");
        // serde uses snake_case:
        assert!(serde_json::to_string(&EventKind::UserPrompt).unwrap().contains("user_prompt"));
    }
}
