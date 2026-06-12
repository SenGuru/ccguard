use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use ccguard_core::capture::{CapturedEvent, CapturedSession, EventKind};
use ccguard_core::event::Repo;

// ── serde types for the raw JSONL ────────────────────────────────────────────

#[derive(Deserialize)]
struct RawLine {
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(rename = "sessionId", default)]
    session_id: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(rename = "gitBranch", default)]
    git_branch: Option<String>,
    #[serde(rename = "isSidechain", default)]
    is_sidechain: bool,
    // for user/assistant lines
    #[serde(default)]
    message: Option<RawMessage>,
    // for ai-title lines
    #[serde(rename = "aiTitle", default)]
    ai_title: Option<String>,
    // for pr-link lines
    #[serde(rename = "prUrl", default)]
    pr_url: Option<String>,
    #[serde(rename = "prNumber", default)]
    pr_number: Option<i64>,
    #[serde(rename = "prRepository", default)]
    pr_repository: Option<String>,
}

#[derive(Deserialize)]
struct RawMessage {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<RawUsage>,
    // content is either a String or an array of content blocks
    #[serde(default)]
    content: Option<Value>,
}

#[derive(Deserialize, Default)]
struct RawUsage {
    #[serde(default)]
    input_tokens: i64,
    #[serde(default)]
    output_tokens: i64,
}

// ── parser ────────────────────────────────────────────────────────────────────

/// Parse a JSONL transcript into a `CapturedSession`. `fallback_cwd` seeds the session cwd
/// if no line carries one. The caller is responsible for filling `user_email` and `repo`
/// after calling this (since they require external lookups).
pub fn parse_session(content: &str, fallback_cwd: Option<&str>) -> CapturedSession {
    let mut session_id = String::new();
    let mut title: Option<String> = None;
    let mut cwd: Option<String> = fallback_cwd.map(|s| s.to_string());
    let mut events: Vec<CapturedEvent> = Vec::new();
    let mut seq: i64 = 0;

    for raw in content.lines() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let line: RawLine = match serde_json::from_str(raw) {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Track session-level state
        if let Some(id) = &line.session_id {
            if !id.is_empty() && session_id.is_empty() {
                session_id = id.clone();
            }
        }
        if line.cwd.is_some() {
            cwd = line.cwd.clone();
        }

        let ts = parse_ts(line.timestamp.as_deref());
        let is_sidechain = line.is_sidechain;

        match line.kind.as_deref() {
            Some("ai-title") => {
                if let Some(t) = line.ai_title {
                    title = Some(t);
                }
            }

            Some("pr-link") => {
                if let Some(url) = line.pr_url {
                    let content_str = match (&line.pr_repository, line.pr_number) {
                        (Some(r), Some(n)) => format!("{r}#{n}"),
                        (Some(r), None) => r.clone(),
                        _ => url.clone(),
                    };
                    events.push(CapturedEvent {
                        seq,
                        ts,
                        kind: EventKind::Pr,
                        model: None,
                        tool_name: None,
                        target: Some(url),
                        content: Some(content_str),
                        tokens_in: 0,
                        tokens_out: 0,
                        is_sidechain,
                    });
                    seq += 1;
                }
            }

            Some("user") => {
                let msg = match line.message {
                    Some(m) => m,
                    None => continue,
                };
                let content_val = match msg.content {
                    Some(v) => v,
                    None => continue,
                };

                match &content_val {
                    // Simple string content (whole message as text)
                    Value::String(text) => {
                        if !text.is_empty() {
                            events.push(CapturedEvent {
                                seq,
                                ts,
                                kind: EventKind::UserPrompt,
                                model: None,
                                tool_name: None,
                                target: None,
                                content: Some(text.clone()),
                                tokens_in: 0,
                                tokens_out: 0,
                                is_sidechain,
                            });
                            seq += 1;
                        }
                    }
                    // Array of content blocks
                    Value::Array(blocks) => {
                        for block in blocks {
                            let block_type = block["type"].as_str().unwrap_or("");
                            match block_type {
                                "text" => {
                                    let text = block["text"].as_str().unwrap_or("").to_string();
                                    if !text.is_empty() {
                                        events.push(CapturedEvent {
                                            seq,
                                            ts,
                                            kind: EventKind::UserPrompt,
                                            model: None,
                                            tool_name: None,
                                            target: None,
                                            content: Some(text),
                                            tokens_in: 0,
                                            tokens_out: 0,
                                            is_sidechain,
                                        });
                                        seq += 1;
                                    }
                                }
                                "tool_result" => {
                                    // Content can be a string or array of text blocks
                                    let result_text =
                                        extract_tool_result_content(&block["content"]);
                                    // tool_use_id lets us find the original tool name (we skip
                                    // the deref here — stretch goal)
                                    events.push(CapturedEvent {
                                        seq,
                                        ts,
                                        kind: EventKind::ToolResult,
                                        model: None,
                                        tool_name: None, // deref of tool_use_id → name is stretch
                                        target: block["tool_use_id"]
                                            .as_str()
                                            .map(|s| s.to_string()),
                                        content: result_text,
                                        tokens_in: 0,
                                        tokens_out: 0,
                                        is_sidechain,
                                    });
                                    seq += 1;
                                }
                                _ => {} // skip unknown block types
                            }
                        }
                    }
                    _ => {}
                }
            }

            Some("assistant") => {
                let msg = match line.message {
                    Some(m) => m,
                    None => continue,
                };
                let content_val = match msg.content {
                    Some(v) => v,
                    None => continue,
                };
                let model = msg.model.clone();
                let (tokens_in, tokens_out) = msg
                    .usage
                    .map(|u| (u.input_tokens, u.output_tokens))
                    .unwrap_or((0, 0));

                // Attach tokens to the first event of this assistant turn
                let mut first_in_turn = true;

                match &content_val {
                    Value::String(text) => {
                        if !text.is_empty() {
                            events.push(CapturedEvent {
                                seq,
                                ts,
                                kind: EventKind::AssistantText,
                                model: model.clone(),
                                tool_name: None,
                                target: None,
                                content: Some(text.clone()),
                                tokens_in,
                                tokens_out,
                                is_sidechain,
                            });
                            seq += 1;
                        }
                    }
                    Value::Array(blocks) => {
                        for block in blocks {
                            let block_type = block["type"].as_str().unwrap_or("");
                            let (ti, to) = if first_in_turn {
                                first_in_turn = false;
                                (tokens_in, tokens_out)
                            } else {
                                (0, 0)
                            };
                            match block_type {
                                "text" => {
                                    let text = block["text"].as_str().unwrap_or("").to_string();
                                    if !text.is_empty() {
                                        events.push(CapturedEvent {
                                            seq,
                                            ts,
                                            kind: EventKind::AssistantText,
                                            model: model.clone(),
                                            tool_name: None,
                                            target: None,
                                            content: Some(text),
                                            tokens_in: ti,
                                            tokens_out: to,
                                            is_sidechain,
                                        });
                                        seq += 1;
                                    }
                                }
                                "thinking" => {
                                    let thinking =
                                        block["thinking"].as_str().unwrap_or("").to_string();
                                    if !thinking.is_empty() {
                                        events.push(CapturedEvent {
                                            seq,
                                            ts,
                                            kind: EventKind::Thinking,
                                            model: model.clone(),
                                            tool_name: None,
                                            target: None,
                                            content: Some(thinking),
                                            tokens_in: ti,
                                            tokens_out: to,
                                            is_sidechain,
                                        });
                                        seq += 1;
                                    }
                                }
                                "tool_use" => {
                                    let tool_name =
                                        block["name"].as_str().unwrap_or("").to_string();
                                    let input = &block["input"];
                                    // Derive target from well-known tool args
                                    let target = derive_target(&tool_name, input);
                                    // Determine event kind: Edit/Write are FileEdit
                                    let kind = if tool_name == "Edit"
                                        || tool_name == "Write"
                                        || tool_name == "MultiEdit"
                                    {
                                        EventKind::FileEdit
                                    } else {
                                        EventKind::ToolCall
                                    };
                                    let input_json = serde_json::to_string(input)
                                        .unwrap_or_else(|_| "{}".to_string());
                                    events.push(CapturedEvent {
                                        seq,
                                        ts,
                                        kind,
                                        model: model.clone(),
                                        tool_name: Some(tool_name),
                                        target,
                                        content: Some(input_json),
                                        tokens_in: ti,
                                        tokens_out: to,
                                        is_sidechain,
                                    });
                                    seq += 1;
                                }
                                _ => {} // skip unknown block types
                            }
                        }
                    }
                    _ => {}
                }
            }

            _ => {} // skip system, file-history-snapshot, attachment, permission-mode, etc.
        }
    }

    CapturedSession {
        session_id,
        // Caller fills these after the call:
        user_email: String::new(),
        repo: Repo {
            host: None,
            org: None,
            name: None,
            path: cwd.clone(),
            classification: None,
            confidence: 0.0,
        },
        title,
        cwd,
        signals: None, // filled by the caller after repo/signal resolution
        events,
    }
}

/// Extract text content from a tool_result content field, which can be:
/// - a plain string
/// - an array of text/tool_reference blocks
fn extract_tool_result_content(content: &Value) -> Option<String> {
    match content {
        Value::String(s) => {
            if s.is_empty() {
                None
            } else {
                Some(s.clone())
            }
        }
        Value::Array(blocks) => {
            let texts: Vec<&str> = blocks
                .iter()
                .filter_map(|b| {
                    if b["type"].as_str() == Some("text") {
                        b["text"].as_str()
                    } else {
                        None
                    }
                })
                .collect();
            if texts.is_empty() {
                None
            } else {
                Some(texts.join("\n"))
            }
        }
        _ => None,
    }
}

/// Derive a human-readable `target` from well-known tool inputs.
fn derive_target(tool_name: &str, input: &Value) -> Option<String> {
    match tool_name {
        "Bash" => input["command"].as_str().map(|s| s.to_string()),
        "Read" | "Edit" | "Write" | "MultiEdit" => {
            input["file_path"].as_str().map(|s| s.to_string())
        }
        "WebFetch" => input["url"].as_str().map(|s| s.to_string()),
        "WebSearch" => input["query"].as_str().map(|s| s.to_string()),
        "Grep" => input["pattern"].as_str().map(|s| s.to_string()),
        "Glob" => input["pattern"].as_str().map(|s| s.to_string()),
        _ => None,
    }
}

fn parse_ts(ts: Option<&str>) -> DateTime<Utc> {
    ts.and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multi_event_transcript() {
        // Crafted transcript: user prompt → thinking → assistant text w/ tokens
        // → tool_call Bash → tool_result → ai-title sets session title
        let transcript = r#"
{"type":"user","sessionId":"sess-42","timestamp":"2026-06-10T09:00:00Z","cwd":"C:\\work\\repo","message":{"role":"user","content":[{"type":"text","text":"Please run git status"}]}}
{"type":"assistant","sessionId":"sess-42","timestamp":"2026-06-10T09:00:01Z","cwd":"C:\\work\\repo","message":{"role":"assistant","model":"claude-opus-4-8","usage":{"input_tokens":500,"output_tokens":10},"content":[{"type":"thinking","thinking":"I should run git status to see the repo state."}]}}
{"type":"assistant","sessionId":"sess-42","timestamp":"2026-06-10T09:00:01Z","cwd":"C:\\work\\repo","message":{"role":"assistant","model":"claude-opus-4-8","usage":{"input_tokens":500,"output_tokens":150},"content":[{"type":"tool_use","id":"toolu_abc","name":"Bash","input":{"command":"git status"}}]}}
{"type":"user","sessionId":"sess-42","timestamp":"2026-06-10T09:00:02Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_abc","content":"On branch main\nnothing to commit"}]}}
{"type":"assistant","sessionId":"sess-42","timestamp":"2026-06-10T09:00:03Z","message":{"role":"assistant","model":"claude-opus-4-8","usage":{"input_tokens":600,"output_tokens":30},"content":[{"type":"text","text":"The repo is clean."}]}}
{"type":"ai-title","sessionId":"sess-42","aiTitle":"Git Status Check"}
"#;
        let sess = parse_session(transcript, None);

        assert_eq!(sess.session_id, "sess-42");
        assert_eq!(sess.title.as_deref(), Some("Git Status Check"));
        assert_eq!(sess.cwd.as_deref(), Some("C:\\work\\repo"));

        // Expected events: UserPrompt, Thinking, ToolCall(Bash), ToolResult, AssistantText
        assert_eq!(
            sess.events.len(),
            5,
            "expected 5 events, got {:?}",
            sess.events.iter().map(|e| e.kind).collect::<Vec<_>>()
        );

        assert_eq!(sess.events[0].kind, EventKind::UserPrompt);
        assert_eq!(
            sess.events[0].content.as_deref(),
            Some("Please run git status")
        );

        assert_eq!(sess.events[1].kind, EventKind::Thinking);
        assert!(sess.events[1]
            .content
            .as_deref()
            .unwrap()
            .contains("git status"));
        // tokens attached to first event of assistant turn
        assert_eq!(sess.events[1].tokens_in, 500);
        assert_eq!(sess.events[1].tokens_out, 10);
        assert_eq!(sess.events[1].model.as_deref(), Some("claude-opus-4-8"));

        assert_eq!(sess.events[2].kind, EventKind::ToolCall);
        assert_eq!(sess.events[2].tool_name.as_deref(), Some("Bash"));
        assert_eq!(sess.events[2].target.as_deref(), Some("git status"));
        // tokens on first event of THIS assistant turn
        assert_eq!(sess.events[2].tokens_in, 500);
        assert_eq!(sess.events[2].tokens_out, 150);

        assert_eq!(sess.events[3].kind, EventKind::ToolResult);
        assert!(sess.events[3].content.as_deref().unwrap().contains("main"));
        assert_eq!(sess.events[3].target.as_deref(), Some("toolu_abc"));

        assert_eq!(sess.events[4].kind, EventKind::AssistantText);
        assert_eq!(
            sess.events[4].content.as_deref(),
            Some("The repo is clean.")
        );
        assert_eq!(sess.events[4].tokens_in, 600);
        assert_eq!(sess.events[4].tokens_out, 30);
    }

    #[test]
    fn parse_file_edit_kind() {
        let transcript = r#"
{"type":"assistant","sessionId":"s1","timestamp":"2026-06-10T10:00:00Z","message":{"role":"assistant","model":"claude-opus-4-8","usage":{"input_tokens":100,"output_tokens":50},"content":[{"type":"tool_use","id":"toolu_xyz","name":"Edit","input":{"file_path":"src/main.rs","old_string":"foo","new_string":"bar"}}]}}
"#;
        let sess = parse_session(transcript, Some("C:\\work"));
        assert_eq!(sess.events.len(), 1);
        assert_eq!(sess.events[0].kind, EventKind::FileEdit);
        assert_eq!(sess.events[0].tool_name.as_deref(), Some("Edit"));
        assert_eq!(sess.events[0].target.as_deref(), Some("src/main.rs"));
    }

    #[test]
    fn ai_title_only_sets_title_no_event() {
        let transcript = r#"
{"type":"ai-title","sessionId":"s1","aiTitle":"My Session"}
"#;
        let sess = parse_session(transcript, None);
        assert_eq!(sess.title.as_deref(), Some("My Session"));
        assert_eq!(sess.events.len(), 0);
    }

    #[test]
    fn skips_garbage_and_system_lines() {
        let transcript =
            "not json\n{}\n{\"type\":\"system\"}\n{\"type\":\"file-history-snapshot\"}\n";
        let sess = parse_session(transcript, None);
        assert_eq!(sess.events.len(), 0);
        assert_eq!(sess.session_id, "");
    }

    #[test]
    fn pr_link_produces_pr_event() {
        let transcript = r#"
{"type":"pr-link","sessionId":"s1","timestamp":"2026-06-10T10:00:00Z","prUrl":"https://github.com/acme/repo/pull/42","prNumber":42,"prRepository":"acme/repo"}
"#;
        let sess = parse_session(transcript, None);
        assert_eq!(sess.events.len(), 1);
        assert_eq!(sess.events[0].kind, EventKind::Pr);
        assert_eq!(
            sess.events[0].target.as_deref(),
            Some("https://github.com/acme/repo/pull/42")
        );
        assert_eq!(sess.events[0].content.as_deref(), Some("acme/repo#42"));
    }
}
