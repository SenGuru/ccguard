//! Parser for OpenAI Codex CLI rollout transcripts.
//!
//! Codex writes one JSONL file per session under
//! `~/.codex/sessions/YYYY/MM/DD/rollout-<ts>-<uuid>.jsonl` (CODEX_HOME overrides
//! the root). Each line is `{timestamp, type, payload}`. The `type` values we care
//! about:
//!   - `session_meta`  : first line; payload has `id`, `cwd`, git fields, model.
//!   - `response_item` : payload.type in {message, function_call,
//!                       function_call_output, reasoning}.
//!   - `event_msg`     : payload.type == "token_count" carries cumulative tokens.
//!   - `turn_context`  : payload.model (the model for the upcoming turn).
//!
//! We normalize all of that into the same `CapturedSession`/`CapturedEvent` shape
//! the Claude Code parser produces, so the whole server pipeline (triage, on-task,
//! secrets, spend) is reused unchanged. Sessions are tagged `tool = "codex_cli"`.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use ccguard_core::capture::{CapturedEvent, CapturedSession, EventKind};
use ccguard_core::event::Repo;

#[derive(Deserialize)]
struct RolloutLine {
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    payload: Option<Value>,
}

/// Parse a Codex rollout JSONL transcript into a `CapturedSession`. The caller
/// fills `user_email`, identity, and re-resolves `repo`/`signals` from `cwd`.
pub fn parse_session(content: &str) -> CapturedSession {
    let mut session_id = String::new();
    let mut cwd: Option<String> = None;
    let mut title: Option<String> = None;
    let mut cur_model: Option<String> = None;
    let mut events: Vec<CapturedEvent> = Vec::new();
    let mut seq: i64 = 0;
    // token_count is cumulative per session; we diff and attach the delta to the
    // most recent assistant/tool event so per-event sums stay correct in aggregate.
    let (mut prev_in, mut prev_out) = (0i64, 0i64);
    let mut last_assistant_idx: Option<usize> = None;

    for raw in content.lines() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let line: RolloutLine = match serde_json::from_str(raw) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let ts = parse_ts(line.timestamp.as_deref());
        let p = match &line.payload {
            Some(v) => v,
            None => continue,
        };

        match line.kind.as_deref() {
            Some("session_meta") => {
                if let Some(id) = p["id"].as_str() {
                    if session_id.is_empty() {
                        session_id = id.to_string();
                    }
                }
                if let Some(c) = p["cwd"].as_str() {
                    cwd = Some(c.to_string());
                }
                if let Some(m) = p["model"].as_str().or_else(|| p["model_provider"].as_str()) {
                    cur_model = Some(m.to_string());
                }
            }

            Some("turn_context") => {
                if let Some(m) = p["model"].as_str() {
                    cur_model = Some(m.to_string());
                }
            }

            Some("event_msg") => {
                match p["type"].as_str() {
                    Some("token_count") => {
                        let (ci, co) = read_token_count(p);
                        let din = (ci - prev_in).max(0);
                        let dout = (co - prev_out).max(0);
                        prev_in = ci;
                        prev_out = co;
                        if let Some(i) = last_assistant_idx {
                            events[i].tokens_in += din;
                            events[i].tokens_out += dout;
                        }
                    }
                    // agent_message duplicates assistant text already captured via
                    // response_item; skip to avoid double-counting.
                    _ => {}
                }
            }

            Some("response_item") => {
                match p["type"].as_str() {
                    Some("message") => {
                        let role = p["role"].as_str().unwrap_or("");
                        let text = join_content_text(&p["content"]);
                        if text.trim().is_empty() {
                            continue;
                        }
                        match role {
                            "user" => {
                                if is_injected_context(&text) {
                                    continue; // skip Codex-injected env/permission turns
                                }
                                events.push(ev(seq, ts, EventKind::UserPrompt, None, None, None, Some(text)));
                                seq += 1;
                            }
                            "assistant" => {
                                events.push(ev(
                                    seq, ts, EventKind::AssistantText, cur_model.clone(), None, None, Some(text),
                                ));
                                last_assistant_idx = Some(events.len() - 1);
                                seq += 1;
                            }
                            // developer / system / tool framing — not real activity.
                            _ => {}
                        }
                    }
                    Some("reasoning") => {
                        let text = p["summary"]
                            .as_array()
                            .map(|a| join_content_text(&Value::Array(a.clone())))
                            .filter(|s| !s.trim().is_empty())
                            .or_else(|| p["text"].as_str().map(|s| s.to_string()))
                            .unwrap_or_default();
                        if !text.trim().is_empty() {
                            events.push(ev(
                                seq, ts, EventKind::Thinking, cur_model.clone(), None, None, Some(text),
                            ));
                            last_assistant_idx = Some(events.len() - 1);
                            seq += 1;
                        }
                    }
                    Some("function_call") | Some("local_shell_call") | Some("custom_tool_call") => {
                        let name = p["name"]
                            .as_str()
                            .unwrap_or_else(|| p["type"].as_str().unwrap_or("tool"))
                            .to_string();
                        let args = parse_arguments(p);
                        let target = derive_target(&name, &args);
                        let kind = if is_file_edit(&name) {
                            EventKind::FileEdit
                        } else {
                            EventKind::ToolCall
                        };
                        let body = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string());
                        events.push(ev(seq, ts, kind, cur_model.clone(), Some(name), target, Some(body)));
                        last_assistant_idx = Some(events.len() - 1);
                        seq += 1;
                    }
                    Some("function_call_output") | Some("local_shell_call_output")
                    | Some("custom_tool_call_output") => {
                        let out = extract_output(&p["output"]);
                        if out.as_deref().map(str::is_empty).unwrap_or(true) {
                            continue;
                        }
                        let target = p["call_id"].as_str().map(|s| s.to_string());
                        events.push(ev(seq, ts, EventKind::ToolResult, None, None, target, out));
                        seq += 1;
                    }
                    _ => {}
                }
            }

            _ => {} // compacted, etc. — skipped
        }
    }

    // Title: first real user prompt, truncated — Codex has no AI title line.
    if title.is_none() {
        if let Some(first) = events.iter().find(|e| e.kind == EventKind::UserPrompt) {
            if let Some(c) = first.content.as_deref() {
                let t: String = c.chars().take(60).collect();
                title = Some(t);
            }
        }
    }

    CapturedSession {
        session_id,
        user_email: String::new(),
        device_id: None,
        hostname: None,
        plan: None,
        tool: Some("codex_cli".to_string()),
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

/// Codex injects synthetic `user` turns (environment context, permission notes,
/// and a guard telling it not to read ~/.claude). These are not human prompts.
fn is_injected_context(text: &str) -> bool {
    let t = text.trim_start();
    t.starts_with("<environment_context>")
        || t.starts_with("<user_instructions>")
        || t.starts_with("<permissions")
        || t.starts_with("## My request for Codex")
        || t.starts_with("# AGENTS.md instructions")
        || t.starts_with("<INSTRUCTIONS>")
        || t.contains("Do NOT read or execute any files under ~/.claude")
}

/// Pull cumulative input/output token totals out of a token_count payload. Codex
/// has used a few shapes across versions; be permissive.
fn read_token_count(p: &Value) -> (i64, i64) {
    // Newer: {"info": {"total_token_usage": {"input_tokens":..,"output_tokens":..}}}
    for base in [&p["info"]["total_token_usage"], &p["total_token_usage"], p] {
        let i = base["input_tokens"].as_i64();
        let o = base["output_tokens"].as_i64();
        if i.is_some() || o.is_some() {
            return (i.unwrap_or(0), o.unwrap_or(0));
        }
    }
    (0, 0)
}

/// Concatenate the `text`/`input_text`/`output_text` fields of a content array
/// (or return a bare string as-is).
fn join_content_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter_map(|b| {
                b["text"]
                    .as_str()
                    .or_else(|| b.as_str())
                    .map(|s| s.to_string())
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// `arguments` is usually a JSON string; double-parse it. Falls back to the raw
/// object if it's already structured.
fn parse_arguments(p: &Value) -> Value {
    match &p["arguments"] {
        Value::String(s) => serde_json::from_str(s).unwrap_or(Value::Null),
        Value::Object(_) | Value::Array(_) => p["arguments"].clone(),
        _ => {
            // local_shell_call carries an `action` object instead of `arguments`.
            if !p["action"].is_null() {
                p["action"].clone()
            } else {
                Value::Null
            }
        }
    }
}

fn is_file_edit(name: &str) -> bool {
    matches!(name, "apply_patch" | "edit" | "write_file" | "str_replace")
}

/// Best-effort human-readable target from a tool's arguments.
fn derive_target(name: &str, args: &Value) -> Option<String> {
    // shell-style: {"command": ["bash","-lc","git status"]} or a string
    if let Some(cmd) = args["command"].as_array() {
        let joined: Vec<&str> = cmd.iter().filter_map(|v| v.as_str()).collect();
        if !joined.is_empty() {
            return Some(joined.join(" "));
        }
    }
    if let Some(cmd) = args["command"].as_str() {
        return Some(cmd.to_string());
    }
    if is_file_edit(name) {
        if let Some(p) = args["path"].as_str().or_else(|| args["file_path"].as_str()) {
            return Some(p.to_string());
        }
        // apply_patch: first "*** Update File: <path>" / "*** Add File: <path>"
        if let Some(patch) = args["input"].as_str().or_else(|| args["patch"].as_str()) {
            for ln in patch.lines() {
                if let Some(rest) = ln.strip_prefix("*** ") {
                    if let Some((_, path)) = rest.split_once("File: ") {
                        return Some(path.trim().to_string());
                    }
                }
            }
        }
    }
    None
}

/// function_call_output `output` is often `{"output": "...", "metadata": {...}}`
/// (as a JSON string) or a plain string.
fn extract_output(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => {
            // It may itself be a JSON object string with an "output" field.
            if let Ok(Value::Object(o)) = serde_json::from_str::<Value>(s) {
                if let Some(inner) = o.get("output").and_then(|x| x.as_str()) {
                    return Some(inner.to_string());
                }
            }
            Some(s.clone())
        }
        Value::Object(o) => o
            .get("output")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
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
    fn parses_real_shape_session() {
        // Mirrors the real on-disk format: session_meta, injected developer/user
        // turns (skipped), a real user prompt, assistant text, a shell tool call +
        // output, and a cumulative token_count.
        let t = r#"
{"timestamp":"2026-06-05T15:34:43.522Z","type":"session_meta","payload":{"id":"019e986c","cwd":"C:\\work\\repo","cli_version":"0.135.0","model_provider":"openai"}}
{"timestamp":"2026-06-05T15:34:48.524Z","type":"response_item","payload":{"type":"message","role":"developer","content":[{"type":"input_text","text":"<permissions instructions>blah"}]}}
{"timestamp":"2026-06-05T15:34:48.524Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context>\n<cwd>C:\\work\\repo</cwd>\n</environment_context>"}]}}
{"timestamp":"2026-06-05T15:34:49.000Z","type":"turn_context","payload":{"model":"gpt-5.4-codex"}}
{"timestamp":"2026-06-05T15:34:50.000Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"run the tests and fix failures"}]}}
{"timestamp":"2026-06-05T15:34:51.000Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"On it — running the suite."}]}}
{"timestamp":"2026-06-05T15:34:52.000Z","type":"response_item","payload":{"type":"function_call","name":"shell","arguments":"{\"command\":[\"bash\",\"-lc\",\"npm test\"]}","call_id":"c1"}}
{"timestamp":"2026-06-05T15:34:55.000Z","type":"response_item","payload":{"type":"function_call_output","call_id":"c1","output":"12 passing"}}
{"timestamp":"2026-06-05T15:34:55.100Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":4821,"output_tokens":312}}}}
"#;
        let s = parse_session(t);
        assert_eq!(s.session_id, "019e986c");
        assert_eq!(s.cwd.as_deref(), Some("C:\\work\\repo"));
        assert_eq!(s.tool.as_deref(), Some("codex_cli"));

        let kinds: Vec<_> = s.events.iter().map(|e| e.kind).collect();
        // developer + env-context user turns are skipped; real prompt kept.
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
        assert_eq!(s.events[0].content.as_deref(), Some("run the tests and fix failures"));
        assert_eq!(s.events[1].model.as_deref(), Some("gpt-5.4-codex"));
        assert_eq!(s.events[2].tool_name.as_deref(), Some("shell"));
        assert_eq!(s.events[2].target.as_deref(), Some("bash -lc npm test"));
        assert_eq!(s.events[3].content.as_deref(), Some("12 passing"));
        // cumulative token_count attached to the last assistant/tool event (the shell call)
        let total_in: i64 = s.events.iter().map(|e| e.tokens_in).sum();
        let total_out: i64 = s.events.iter().map(|e| e.tokens_out).sum();
        assert_eq!(total_in, 4821);
        assert_eq!(total_out, 312);
        assert_eq!(s.title.as_deref(), Some("run the tests and fix failures"));
    }

    #[test]
    fn apply_patch_is_file_edit_with_path() {
        let t = r#"
{"type":"response_item","payload":{"type":"function_call","name":"apply_patch","arguments":"{\"input\":\"*** Begin Patch\\n*** Update File: src/main.rs\\n@@\\n-foo\\n+bar\\n*** End Patch\"}","call_id":"c2"}}
"#;
        let s = parse_session(t);
        assert_eq!(s.events.len(), 1);
        assert_eq!(s.events[0].kind, EventKind::FileEdit);
        assert_eq!(s.events[0].tool_name.as_deref(), Some("apply_patch"));
        assert_eq!(s.events[0].target.as_deref(), Some("src/main.rs"));
    }

    #[test]
    fn empty_and_garbage_lines_skipped() {
        let s = parse_session("not json\n{}\n{\"type\":\"compacted\",\"payload\":{}}\n");
        assert!(s.events.is_empty());
    }
}
