//! Parser for GitHub Copilot CLI session transcripts.
//!
//! The Copilot CLI persists each session under
//! `~/.copilot/session-state/<session-id>/events.jsonl` (COPILOT_HOME overrides
//! the root). Each line is a JSON event `{type, timestamp, data}`. Known types:
//!   - `session.start`            : data.context.{repository, branch}; the prompt.
//!   - `user.message` / prompt    : the developer's prompt text.
//!   - `assistant.message`        : the model's response text.
//!   - `tool.execution_start`     : data.{toolName, arguments}.
//!   - `tool.execution_complete`  : data.{success, output}.
//!   - `session.shutdown`         : data.modelMetrics.<model>.usage.{inputTokens,outputTokens}.
//!
//! Microsoft does not publish a formal schema, so this is built to the observed
//! shape (jonmagic teardown + GitHub docs) and is intentionally permissive about
//! field names. Output is the same `CapturedSession` shape as the other parsers,
//! tagged `tool = "copilot_cli"`. Repo is taken from `context.repository`
//! (owner/repo) since the CLI does not always record a local cwd.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use ccguard_core::capture::{CapturedEvent, CapturedSession, EventKind};
use ccguard_core::event::Repo;

#[derive(Deserialize)]
struct EventLine {
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    data: Option<Value>,
}

/// Parse a Copilot CLI `events.jsonl` into a `CapturedSession`. The caller fills
/// `user_email`/identity and supplies the session id from the directory name when
/// the stream itself doesn't carry one.
pub fn parse_session(content: &str) -> CapturedSession {
    let mut session_id = String::new();
    let mut title: Option<String> = None;
    let mut repo = Repo {
        host: None,
        org: None,
        name: None,
        path: None,
        classification: None,
        confidence: 0.0,
    };
    let mut events: Vec<CapturedEvent> = Vec::new();
    let mut seq: i64 = 0;
    let mut last_assistant_idx: Option<usize> = None;
    let mut last_tool_target: Option<Option<String>> = None;

    for raw in content.lines() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let line: EventLine = match serde_json::from_str(raw) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let ts = parse_ts(line.timestamp.as_deref());
        let d = line.data.unwrap_or(Value::Null);

        match line.kind.as_deref() {
            Some("session.start") => {
                if let Some(id) = first_str(&d, &["sessionId", "id", "session_id"]) {
                    if session_id.is_empty() {
                        session_id = id;
                    }
                }
                if let Some(r) = first_str(&d["context"], &["repository", "repo"])
                    .or_else(|| first_str(&d, &["repository", "repo"]))
                {
                    set_repo_from_slug(&mut repo, &r);
                }
                // Some builds put the opening prompt directly on session.start.
                if let Some(t) = first_str(&d, &["prompt", "initialPrompt", "message"]) {
                    if !t.trim().is_empty() {
                        events.push(ev(seq, ts, EventKind::UserPrompt, None, None, None, Some(t)));
                        seq += 1;
                    }
                }
            }

            // Prompt / response message events (a few naming variants seen in the wild).
            Some("user.message") | Some("user_prompt") | Some("prompt") => {
                if let Some(t) = message_text(&d) {
                    events.push(ev(seq, ts, EventKind::UserPrompt, None, None, None, Some(t)));
                    seq += 1;
                }
            }
            Some("assistant.message") | Some("agent.message") | Some("response")
            | Some("model.response") => {
                if let Some(t) = message_text(&d) {
                    let model = first_str(&d, &["model", "modelName"]);
                    events.push(ev(seq, ts, EventKind::AssistantText, model, None, None, Some(t)));
                    last_assistant_idx = Some(events.len() - 1);
                    seq += 1;
                }
            }
            // Generic `message` with an explicit role.
            Some("message") => {
                let role = first_str(&d, &["role"]).unwrap_or_default();
                if let Some(t) = message_text(&d) {
                    if role == "assistant" || role == "agent" {
                        let model = first_str(&d, &["model", "modelName"]);
                        events.push(ev(seq, ts, EventKind::AssistantText, model, None, None, Some(t)));
                        last_assistant_idx = Some(events.len() - 1);
                    } else {
                        events.push(ev(seq, ts, EventKind::UserPrompt, None, None, None, Some(t)));
                    }
                    seq += 1;
                }
            }

            Some("tool.execution_start") | Some("tool.start") => {
                let name = first_str(&d, &["toolName", "tool", "name"]).unwrap_or_else(|| "tool".into());
                let args = &d["arguments"];
                let target = derive_target(&name, args);
                let kind = if is_file_edit(&name) {
                    EventKind::FileEdit
                } else {
                    EventKind::ToolCall
                };
                let body = serde_json::to_string(args).unwrap_or_else(|_| "{}".into());
                events.push(ev(seq, ts, kind, None, Some(name), target.clone(), Some(body)));
                // remember target so we can label the completion
                last_tool_target = Some(target);
                seq += 1;
            }
            Some("tool.execution_complete") | Some("tool.complete") => {
                let out = first_str(&d, &["output", "result", "stdout"])
                    .filter(|s| !s.is_empty());
                let target = last_tool_target.clone().flatten();
                events.push(ev(seq, ts, EventKind::ToolResult, None, None, target, out));
                seq += 1;
            }

            Some("session.shutdown") | Some("session.end") => {
                let (ti, to) = read_model_metrics(&d["modelMetrics"]);
                if ti + to > 0 {
                    let idx = last_assistant_idx.or_else(|| events.len().checked_sub(1));
                    if let Some(i) = idx {
                        events[i].tokens_in += ti;
                        events[i].tokens_out += to;
                    }
                }
            }

            _ => {}
        }
    }

    if title.is_none() {
        if let Some(first) = events.iter().find(|e| e.kind == EventKind::UserPrompt) {
            if let Some(c) = first.content.as_deref() {
                title = Some(c.chars().take(60).collect());
            }
        }
    }

    CapturedSession {
        session_id,
        user_email: String::new(),
        device_id: None,
        hostname: None,
        plan: None,
        tool: Some("copilot_cli".to_string()),
        repo,
        title,
        cwd: None, // Copilot CLI repo comes from context.repository, not a local cwd
        signals: None,
        events,
    }
}

#[allow(clippy::too_many_arguments)]
fn ev(
    seq: i64,
    ts: DateTime<Utc>,
    kind: EventKind,
    model: Option<String>,
    tool_name: Option<String>,
    target: Option<String>,
    content: Option<String>,
) -> CapturedEvent {
    CapturedEvent {
        seq,
        ts,
        kind,
        model,
        tool_name,
        target,
        content,
        tokens_in: 0,
        tokens_out: 0,
        is_sidechain: false,
    }
}

fn first_str(v: &Value, keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Some(s) = v[*k].as_str() {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}

/// A message's text may live under text/content/message, where `content` can be a
/// string or an array of `{text}` blocks.
fn message_text(d: &Value) -> Option<String> {
    if let Some(s) = first_str(d, &["text", "message"]) {
        return Some(s);
    }
    match &d["content"] {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Array(blocks) => {
            let joined: Vec<String> = blocks
                .iter()
                .filter_map(|b| b["text"].as_str().or_else(|| b.as_str()).map(str::to_string))
                .collect();
            if joined.is_empty() {
                None
            } else {
                Some(joined.join("\n"))
            }
        }
        _ => None,
    }
}

fn set_repo_from_slug(repo: &mut Repo, slug: &str) {
    let slug = slug.trim().trim_end_matches(".git");
    if let Some((org, name)) = slug.split_once('/') {
        repo.host = Some("github.com".to_string());
        repo.org = Some(org.to_string());
        repo.name = Some(name.to_string());
    } else if !slug.is_empty() {
        repo.name = Some(slug.to_string());
    }
}

fn is_file_edit(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("edit") || n.contains("write") || n.contains("patch") || n.contains("str_replace")
}

fn derive_target(name: &str, args: &Value) -> Option<String> {
    if let Some(c) = args["command"].as_str() {
        return Some(c.to_string());
    }
    if let Some(arr) = args["command"].as_array() {
        let j: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
        if !j.is_empty() {
            return Some(j.join(" "));
        }
    }
    if is_file_edit(name) {
        return first_str(args, &["path", "file_path", "filePath", "file"]);
    }
    first_str(args, &["path", "file_path", "url", "query", "pattern"])
}

/// Sum input/output tokens across all models in a `modelMetrics` map.
fn read_model_metrics(m: &Value) -> (i64, i64) {
    let mut ti = 0i64;
    let mut to = 0i64;
    if let Value::Object(models) = m {
        for (_model, v) in models {
            let usage = &v["usage"];
            ti += usage["inputTokens"]
                .as_i64()
                .or_else(|| usage["input_tokens"].as_i64())
                .unwrap_or(0);
            to += usage["outputTokens"]
                .as_i64()
                .or_else(|| usage["output_tokens"].as_i64())
                .unwrap_or(0);
        }
    }
    (ti, to)
}

fn parse_ts(ts: Option<&str>) -> DateTime<Utc> {
    ts.and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cli_events() {
        let t = r#"
{"type":"session.start","timestamp":"2026-03-15T10:00:00Z","data":{"sessionId":"abc","context":{"repository":"acme/widgets","branch":"main"}}}
{"type":"user.message","timestamp":"2026-03-15T10:00:01Z","data":{"text":"add a retry to the http client"}}
{"type":"assistant.message","timestamp":"2026-03-15T10:00:02Z","data":{"model":"gpt-4o","text":"Sure, I'll add retries."}}
{"type":"tool.execution_start","timestamp":"2026-03-15T10:00:03Z","data":{"toolName":"shell","arguments":{"command":"go test ./..."}}}
{"type":"tool.execution_complete","timestamp":"2026-03-15T10:00:05Z","data":{"success":true,"output":"ok  acme/widgets"}}
{"type":"session.shutdown","timestamp":"2026-03-15T10:00:06Z","data":{"modelMetrics":{"gpt-4o":{"usage":{"inputTokens":1920,"outputTokens":210}}}}}
"#;
        let s = parse_session(t);
        assert_eq!(s.session_id, "abc");
        assert_eq!(s.tool.as_deref(), Some("copilot_cli"));
        assert_eq!(s.repo.org.as_deref(), Some("acme"));
        assert_eq!(s.repo.name.as_deref(), Some("widgets"));
        assert_eq!(s.repo.host.as_deref(), Some("github.com"));

        let kinds: Vec<_> = s.events.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![
                EventKind::UserPrompt,
                EventKind::AssistantText,
                EventKind::ToolCall,
                EventKind::ToolResult
            ],
            "got {kinds:?}"
        );
        assert_eq!(s.events[0].content.as_deref(), Some("add a retry to the http client"));
        assert_eq!(s.events[1].model.as_deref(), Some("gpt-4o"));
        assert_eq!(s.events[2].tool_name.as_deref(), Some("shell"));
        assert_eq!(s.events[2].target.as_deref(), Some("go test ./..."));
        assert_eq!(s.events[3].content.as_deref(), Some("ok  acme/widgets"));
        // shutdown tokens attached to the assistant turn
        assert_eq!(s.events.iter().map(|e| e.tokens_in).sum::<i64>(), 1920);
        assert_eq!(s.events.iter().map(|e| e.tokens_out).sum::<i64>(), 210);
    }

    #[test]
    fn generic_message_role_routing() {
        let t = r#"
{"type":"message","data":{"role":"user","content":"hello"}}
{"type":"message","data":{"role":"assistant","content":[{"text":"hi there"}]}}
"#;
        let s = parse_session(t);
        assert_eq!(s.events.len(), 2);
        assert_eq!(s.events[0].kind, EventKind::UserPrompt);
        assert_eq!(s.events[1].kind, EventKind::AssistantText);
        assert_eq!(s.events[1].content.as_deref(), Some("hi there"));
    }
}
