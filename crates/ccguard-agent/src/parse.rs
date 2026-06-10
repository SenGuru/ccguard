use serde::Deserialize;

/// One billable assistant interaction extracted from a transcript.
#[derive(Debug, Clone, PartialEq)]
pub struct Interaction {
    pub session_id: String,
    pub ts: String, // RFC3339 timestamp string from the transcript
    pub cwd: Option<String>,
    pub model: String,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cache_read: i64,
    pub cache_creation: i64,
}

#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(rename = "sessionId", default)]
    session_id: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    message: Option<Msg>,
}

#[derive(Deserialize)]
struct Msg {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize, Default)]
struct Usage {
    #[serde(default)]
    input_tokens: i64,
    #[serde(default)]
    output_tokens: i64,
    #[serde(default)]
    cache_read_input_tokens: i64,
    #[serde(default)]
    cache_creation_input_tokens: i64,
}

/// Parse newline-delimited transcript JSON into billable assistant interactions.
/// `cwd` is tracked across lines (it usually appears on user lines, not assistant lines);
/// `fallback_cwd` seeds it. Non-assistant lines, lines without usage, and malformed lines are skipped.
pub fn parse_transcript(content: &str, fallback_cwd: Option<&str>) -> Vec<Interaction> {
    let mut last_cwd = fallback_cwd.map(|s| s.to_string());
    let mut out = Vec::new();
    for raw in content.lines() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let line: Line = match serde_json::from_str(raw) {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.cwd.is_some() {
            last_cwd = line.cwd.clone();
        }
        if line.kind.as_deref() != Some("assistant") {
            continue;
        }
        let msg = match line.message {
            Some(m) => m,
            None => continue,
        };
        let usage = match msg.usage {
            Some(u) => u,
            None => continue,
        };
        if usage.input_tokens == 0 && usage.output_tokens == 0 {
            continue;
        }
        out.push(Interaction {
            session_id: line.session_id.unwrap_or_default(),
            ts: line.timestamp.unwrap_or_default(),
            cwd: last_cwd.clone(),
            model: msg.model.unwrap_or_else(|| "unknown".to_string()),
            tokens_in: usage.input_tokens,
            tokens_out: usage.output_tokens,
            cache_read: usage.cache_read_input_tokens,
            cache_creation: usage.cache_creation_input_tokens,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_assistant_usage_and_carries_cwd() {
        // a user line carries cwd; the following assistant line carries usage but no cwd
        let content = r#"
{"type":"user","sessionId":"s1","cwd":"C:\\work\\repo","timestamp":"2026-06-10T10:00:00Z","message":{"role":"user","content":"hi"}}
{"type":"assistant","sessionId":"s1","timestamp":"2026-06-10T10:00:01Z","message":{"role":"assistant","model":"claude-opus-4-8","usage":{"input_tokens":1000,"output_tokens":200,"cache_read_input_tokens":5000,"cache_creation_input_tokens":300}}}
{"type":"file-history-snapshot"}
{"type":"assistant","sessionId":"s1","timestamp":"2026-06-10T10:00:02Z","message":{"role":"assistant","model":"claude-opus-4-8","usage":{"input_tokens":0,"output_tokens":0}}}
"#;
        let got = parse_transcript(content, None);
        assert_eq!(got.len(), 1);
        let i = &got[0];
        assert_eq!(i.session_id, "s1");
        assert_eq!(i.cwd.as_deref(), Some("C:\\work\\repo"));
        assert_eq!(i.model, "claude-opus-4-8");
        assert_eq!(i.tokens_in, 1000);
        assert_eq!(i.tokens_out, 200);
        assert_eq!(i.cache_read, 5000);
        assert_eq!(i.cache_creation, 300);
        assert_eq!(i.ts, "2026-06-10T10:00:01Z");
    }

    #[test]
    fn skips_garbage_lines() {
        let content = "not json\n{}\n{\"type\":\"assistant\"}\n";
        assert!(parse_transcript(content, None).is_empty());
    }
}
