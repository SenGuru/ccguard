//! Anthropic Messages API client for the LLM triage tier.
//!
//! Rust has no official Anthropic SDK, so this is the documented raw-HTTP shape:
//! `POST {base}/v1/messages` with `x-api-key` + `anthropic-version: 2023-06-01`,
//! a `{model, max_tokens, system, messages, output_config}` body, and structured
//! JSON output (`output_config.format`) so the reply is always parseable.
//!
//! Self-host friendly: `ANTHROPIC_BASE_URL` lets an org point this at their own
//! Anthropic-compatible endpoint so session content never leaves their tenancy.

use ccguard_core::triage::{self, TriageInput, TriageVerdict};

/// Anthropic API version pin (raw-HTTP requirement).
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Why a triage call could not produce a verdict.
#[derive(Debug)]
pub enum TriageClientError {
    /// `ANTHROPIC_API_KEY` is not set — triage is unconfigured.
    NoApiKey,
    /// Network / transport failure talking to the API.
    Http(reqwest::Error),
    /// API returned a non-2xx status (status, first 300 chars of body).
    Status(u16, String),
    /// Reply body had no text block to parse.
    NoText,
    /// The model's text could not be parsed into a verdict.
    Parse(triage::TriageError),
}

impl std::fmt::Display for TriageClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriageClientError::NoApiKey => write!(f, "ANTHROPIC_API_KEY is not set"),
            TriageClientError::Http(e) => write!(f, "http error: {e}"),
            TriageClientError::Status(s, b) => write!(f, "api status {s}: {b}"),
            TriageClientError::NoText => write!(f, "no text block in reply"),
            TriageClientError::Parse(e) => write!(f, "parse error: {e}"),
        }
    }
}
impl std::error::Error for TriageClientError {}

/// True when an API key is present, i.e. triage can actually run.
pub fn api_key_present() -> bool {
    std::env::var("ANTHROPIC_API_KEY")
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false)
}

fn base_url() -> String {
    std::env::var("ANTHROPIC_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://api.anthropic.com".to_string())
        .trim_end_matches('/')
        .to_string()
}

/// Classify one session via Claude. `model`/`work_definition` come from tenant config.
pub async fn classify_session(
    client: &reqwest::Client,
    model: &str,
    work_definition: Option<&str>,
    input: &TriageInput,
) -> Result<TriageVerdict, TriageClientError> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
        .ok_or(TriageClientError::NoApiKey)?;

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 256,
        "system": triage::system_prompt(work_definition),
        "messages": [{ "role": "user", "content": triage::user_prompt(input) }],
        "output_config": {
            "format": { "type": "json_schema", "schema": triage::output_schema() }
        }
    });

    let resp = client
        .post(format!("{}/v1/messages", base_url()))
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(TriageClientError::Http)?;

    let status = resp.status();
    let text = resp.text().await.map_err(TriageClientError::Http)?;
    if !status.is_success() {
        return Err(TriageClientError::Status(
            status.as_u16(),
            text.chars().take(300).collect(),
        ));
    }

    let reply_text = first_text_block(&text).ok_or(TriageClientError::NoText)?;
    triage::parse_verdict(&reply_text).map_err(TriageClientError::Parse)
}

/// Pull the concatenated text from a Messages API reply body. The response shape
/// is `{ "content": [ { "type": "text", "text": "..." }, ... ] }`.
fn first_text_block(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let blocks = v.get("content")?.as_array()?;
    let mut out = String::new();
    for b in blocks {
        if b.get("type").and_then(|t| t.as_str()) == Some("text") {
            if let Some(t) = b.get("text").and_then(|t| t.as_str()) {
                out.push_str(t);
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_text_from_messages_reply() {
        let body = r#"{"id":"msg_1","type":"message","role":"assistant",
            "content":[{"type":"text","text":"{\"label\":\"work\",\"confidence\":0.9,\"reason\":\"corp repo\"}"}],
            "stop_reason":"end_turn"}"#;
        let t = first_text_block(body).unwrap();
        let v = ccguard_core::triage::parse_verdict(&t).unwrap();
        assert_eq!(v.label, ccguard_core::triage::TriageLabel::Work);
    }

    #[test]
    fn no_text_block_returns_none() {
        let body = r#"{"content":[{"type":"thinking","thinking":"hmm"}]}"#;
        assert!(first_text_block(body).is_none());
    }

    #[test]
    fn base_url_defaults_to_anthropic() {
        // Only asserts the default branch; env-dependent override is integration-tested.
        if std::env::var("ANTHROPIC_BASE_URL").is_err() {
            assert_eq!(base_url(), "https://api.anthropic.com");
        }
    }
}
