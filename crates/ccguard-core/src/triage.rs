//! LLM-tier session triage — the part of the classifier that resolves the cases
//! the deterministic signal cascade leaves as UNCLASSIFIED (`Classification::Unknown`).
//!
//! This module is **pure**: it builds the prompt + output schema for a single
//! Claude call and parses the model's reply into a [`TriageVerdict`]. It does no
//! network I/O — the HTTP call lives in the server (`triage_client`), so the
//! prompt-shaping and parsing logic stays deterministic and unit-testable.
//!
//! Design notes (why it's shaped this way):
//! - The judge labels by **purpose, not location**. A brand-new module in its own
//!   directory is the canonical false-positive of the old git-allowlist classifier;
//!   the prompt explicitly forbids treating "separate directory / unfamiliar name"
//!   as a personal signal.
//! - `Unsure` is a first-class terminal answer. A wrong `Personal` is the expensive
//!   mistake, so the prompt requires an affirmative personal signal before choosing
//!   it and otherwise prefers `Unsure` over a low-confidence guess.
//! - The verdict feeds the dashboard label freely, but the server only lets it
//!   count toward usage-limiting/enforcement after a structural signal agrees or a
//!   human confirms — content is model-judged and gameable.

use serde::{Deserialize, Serialize};

/// Default judge model — cheap + fast, sized for high-volume per-session calls.
pub const DEFAULT_MODEL: &str = "claude-haiku-4-5";

/// Per-prompt character cap when assembling session context (keeps the call cheap
/// and bounds attacker-controlled content).
const PROMPT_CHAR_CAP: usize = 800;

/// The judge's verdict for one session.
#[derive(Debug, Clone, PartialEq)]
pub enum TriageLabel {
    Work,
    Personal,
    /// The signal genuinely does not determine work-vs-personal. Terminal, safe default.
    Unsure,
}

impl TriageLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriageLabel::Work => "work",
            TriageLabel::Personal => "personal",
            TriageLabel::Unsure => "unsure",
        }
    }

    /// Parse a model-returned label. Anything unrecognized maps to `Unsure`
    /// (conservative: never silently coerce garbage into Work/Personal).
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "work" => TriageLabel::Work,
            "personal" => TriageLabel::Personal,
            _ => TriageLabel::Unsure,
        }
    }
}

/// Parsed judge result.
#[derive(Debug, Clone, PartialEq)]
pub struct TriageVerdict {
    pub label: TriageLabel,
    /// Model's calibrated certainty, clamped to `[0.0, 1.0]`.
    pub confidence: f32,
    /// One short sentence citing the deciding signal.
    pub reason: String,
}

/// Why a model reply could not be turned into a verdict.
#[derive(Debug, Clone, PartialEq)]
pub enum TriageError {
    /// The reply was not valid JSON object text.
    NotJson,
    /// The JSON was valid but missing the required fields.
    MissingFields,
}

impl std::fmt::Display for TriageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriageError::NotJson => write!(f, "model reply was not valid JSON"),
            TriageError::MissingFields => write!(f, "model reply missing required fields"),
        }
    }
}
impl std::error::Error for TriageError {}

/// Everything the judge sees about one session. The server assembles this from
/// captured rows; the field order here is the order the prompt presents them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriageInput {
    pub repo_org: Option<String>,
    pub repo_name: Option<String>,
    pub repo_path: Option<String>,
    pub cwd: Option<String>,
    pub title: Option<String>,
    /// Developer prompts in seq order (already truncated/sampled by the caller).
    pub prompts: Vec<String>,
    /// File paths / shell command targets touched in the session.
    pub tool_targets: Vec<String>,
}

/// Operator/system prompt. `work_definition` is the tenant's own description of
/// what counts as work (free text); empty/None falls back to the general rule.
pub fn system_prompt(work_definition: Option<&str>) -> String {
    let def = match work_definition {
        Some(d) if !d.trim().is_empty() => d.trim(),
        _ => "No explicit definition provided — apply the general definition above.",
    };
    format!(
        "You are a classification assistant for an engineering-operations dashboard. \
You are given a summary of ONE Claude Code coding session that ran on company-provided \
tooling: the git repository it touched, the working directory, the session title, the \
developer's prompts, and the files and shell commands involved.\n\n\
Decide whether the session is WORK or PERSONAL.\n\
- WORK: it advances the company's own software, products, infrastructure, tooling, \
research, or business. This INCLUDES exploration, prototypes, spikes, refactors, a \
brand-new module or service in its own directory, internal scripts, configuration, and \
looking something up in service of a work task. A separate directory, an unfamiliar \
project name, or unusual tech does NOT by itself make a session personal — judge by \
PURPOSE, not location.\n\
- PERSONAL: it advances the developer's own side projects, personal accounts, hobby \
code, job hunting, or other activity unrelated to the company's business.\n\
- UNSURE: the available signal genuinely does not let you tell. Prefer UNSURE over a \
low-confidence guess. Calling something PERSONAL is a high-stakes judgement, so require \
a clear, affirmative personal signal before choosing it.\n\n\
<company_definition_of_work>\n{def}\n</company_definition_of_work>\n\n\
Return only: label (work | personal | unsure), confidence (0.0–1.0, your calibrated \
certainty), and reason (one short sentence naming the specific signal that decided it)."
    )
}

/// User-turn content: the session context rendered as a bounded text block.
pub fn user_prompt(input: &TriageInput) -> String {
    let mut s = String::with_capacity(1024);
    s.push_str("Session to classify:\n");

    let repo = match (input.repo_org.as_deref(), input.repo_name.as_deref()) {
        (Some(o), Some(n)) => format!("{o}/{n}"),
        (None, Some(n)) => n.to_string(),
        (Some(o), None) => o.to_string(),
        (None, None) => "(none / no git remote)".to_string(),
    };
    s.push_str(&format!("- Repository: {repo}\n"));
    if let Some(p) = field(&input.repo_path) {
        s.push_str(&format!("- Repo path: {p}\n"));
    }
    if let Some(c) = field(&input.cwd) {
        s.push_str(&format!("- Working directory: {c}\n"));
    }
    if let Some(t) = field(&input.title) {
        s.push_str(&format!("- Session title: {t}\n"));
    }

    if input.prompts.is_empty() {
        s.push_str("- Developer prompts: (none captured)\n");
    } else {
        s.push_str("- Developer prompts:\n");
        for p in &input.prompts {
            s.push_str("  • ");
            s.push_str(&one_line(p, PROMPT_CHAR_CAP));
            s.push('\n');
        }
    }

    if !input.tool_targets.is_empty() {
        s.push_str("- Files / commands touched:\n");
        for t in &input.tool_targets {
            s.push_str("  • ");
            s.push_str(&one_line(t, 200));
            s.push('\n');
        }
    }
    s
}

/// JSON Schema for `output_config.format` — forces a `{label, confidence, reason}`
/// object so the reply is always parseable.
pub fn output_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "label": { "type": "string", "enum": ["work", "personal", "unsure"] },
            "confidence": { "type": "number" },
            "reason": { "type": "string" }
        },
        "required": ["label", "confidence", "reason"],
        "additionalProperties": false
    })
}

/// Parse the model's reply text (guaranteed JSON object by the schema) into a verdict.
/// Tolerant of stray prose around the JSON (extracts the first `{...}` span) so it
/// also works if a future model/path returns unstructured text.
pub fn parse_verdict(raw_text: &str) -> Result<TriageVerdict, TriageError> {
    let json_slice = extract_json_object(raw_text).ok_or(TriageError::NotJson)?;
    let v: serde_json::Value =
        serde_json::from_str(json_slice).map_err(|_| TriageError::NotJson)?;

    let label = v
        .get("label")
        .and_then(|x| x.as_str())
        .ok_or(TriageError::MissingFields)?;
    let reason = v
        .get("reason")
        .and_then(|x| x.as_str())
        .ok_or(TriageError::MissingFields)?;
    // Confidence may arrive as a number or a numeric string; default 0.5 if absent.
    let confidence = v
        .get("confidence")
        .and_then(|x| x.as_f64().or_else(|| x.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0.5) as f32;

    Ok(TriageVerdict {
        label: TriageLabel::from_str(label),
        confidence: clamp_confidence(confidence),
        reason: reason.trim().to_string(),
    })
}

fn clamp_confidence(c: f32) -> f32 {
    if c.is_nan() {
        0.5
    } else {
        c.clamp(0.0, 1.0)
    }
}

/// `Some(trimmed)` when the option holds non-empty text, else `None`.
fn field(o: &Option<String>) -> Option<&str> {
    o.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

/// Collapse to a single line and cap length (char-boundary safe).
fn one_line(s: &str, cap: usize) -> String {
    let flat: String = s
        .chars()
        .map(|c| if c == '\n' || c == '\r' || c == '\t' { ' ' } else { c })
        .collect();
    if flat.chars().count() <= cap {
        flat
    } else {
        let mut out: String = flat.chars().take(cap).collect();
        out.push('…');
        out
    }
}

/// Find the first balanced top-level `{...}` object in `text`. Cheap brace counter
/// that ignores braces inside strings. Returns the JSON slice, or None.
fn extract_json_object(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let start = text.find('{')?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escaped = false;
    for i in start..bytes.len() {
        let ch = bytes[i] as char;
        if in_str {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_roundtrip_and_unknown_is_unsure() {
        assert_eq!(TriageLabel::from_str("work"), TriageLabel::Work);
        assert_eq!(TriageLabel::from_str("PERSONAL"), TriageLabel::Personal);
        assert_eq!(TriageLabel::from_str(" Unsure "), TriageLabel::Unsure);
        assert_eq!(TriageLabel::from_str("banana"), TriageLabel::Unsure);
        assert_eq!(TriageLabel::Work.as_str(), "work");
    }

    #[test]
    fn parses_clean_verdict() {
        let v = parse_verdict(r#"{"label":"work","confidence":0.82,"reason":"Edits the billing service in the corp monorepo."}"#).unwrap();
        assert_eq!(v.label, TriageLabel::Work);
        assert!((v.confidence - 0.82).abs() < 1e-6);
        assert!(v.reason.contains("billing"));
    }

    #[test]
    fn parses_verdict_with_surrounding_prose() {
        let raw = "Here is my classification:\n{\"label\": \"personal\", \"confidence\": 0.7, \"reason\": \"A personal portfolio site under the user's home dir.\"}\nLet me know if you need more.";
        let v = parse_verdict(raw).unwrap();
        assert_eq!(v.label, TriageLabel::Personal);
    }

    #[test]
    fn confidence_is_clamped() {
        let hi = parse_verdict(r#"{"label":"work","confidence":3.5,"reason":"x"}"#).unwrap();
        assert_eq!(hi.confidence, 1.0);
        let lo = parse_verdict(r#"{"label":"unsure","confidence":-2,"reason":"x"}"#).unwrap();
        assert_eq!(lo.confidence, 0.0);
    }

    #[test]
    fn confidence_as_string_is_parsed() {
        let v = parse_verdict(r#"{"label":"work","confidence":"0.6","reason":"x"}"#).unwrap();
        assert!((v.confidence - 0.6).abs() < 1e-6);
    }

    #[test]
    fn missing_confidence_defaults_mid() {
        let v = parse_verdict(r#"{"label":"unsure","reason":"ambiguous"}"#).unwrap();
        assert_eq!(v.confidence, 0.5);
        assert_eq!(v.label, TriageLabel::Unsure);
    }

    #[test]
    fn non_json_errors() {
        assert_eq!(parse_verdict("totally not json").unwrap_err(), TriageError::NotJson);
    }

    #[test]
    fn missing_label_errors() {
        assert_eq!(
            parse_verdict(r#"{"confidence":0.5,"reason":"x"}"#).unwrap_err(),
            TriageError::MissingFields
        );
    }

    #[test]
    fn brace_in_string_does_not_confuse_extractor() {
        let raw = r#"{"label":"work","confidence":0.9,"reason":"Refactors the parser that emits {tokens}."}"#;
        let v = parse_verdict(raw).unwrap();
        assert_eq!(v.label, TriageLabel::Work);
        assert!(v.reason.contains("{tokens}"));
    }

    #[test]
    fn system_prompt_includes_work_definition_and_purpose_rule() {
        let p = system_prompt(Some("Anything in the acme-corp GitHub org or the internal GitLab."));
        assert!(p.contains("acme-corp"));
        assert!(p.contains("PURPOSE, not location"));
    }

    #[test]
    fn system_prompt_falls_back_when_no_definition() {
        let p = system_prompt(None);
        assert!(p.contains("No explicit definition"));
        let p2 = system_prompt(Some("   "));
        assert!(p2.contains("No explicit definition"));
    }

    #[test]
    fn user_prompt_renders_repo_prompts_and_targets() {
        let input = TriageInput {
            repo_org: Some("acme".into()),
            repo_name: Some("billing".into()),
            repo_path: Some("C:/work/billing".into()),
            cwd: None,
            title: Some("Fix invoice rounding".into()),
            prompts: vec!["why is the invoice total off by a cent".into()],
            tool_targets: vec!["src/invoice.rs".into()],
        };
        let up = user_prompt(&input);
        assert!(up.contains("acme/billing"));
        assert!(up.contains("Fix invoice rounding"));
        assert!(up.contains("invoice total off by a cent"));
        assert!(up.contains("src/invoice.rs"));
    }

    #[test]
    fn user_prompt_handles_no_remote() {
        let up = user_prompt(&TriageInput::default());
        assert!(up.contains("no git remote"));
        assert!(up.contains("(none captured)"));
    }

    #[test]
    fn long_prompt_is_truncated() {
        let big = "x".repeat(5000);
        let input = TriageInput {
            prompts: vec![big],
            ..Default::default()
        };
        let up = user_prompt(&input);
        assert!(up.contains('…'));
        // The whole 5000-char prompt must not survive verbatim.
        assert!(up.len() < 5000 + 200);
    }

    #[test]
    fn one_line_is_char_boundary_safe() {
        // Multi-byte chars must not panic the truncation.
        let s = "é".repeat(2000);
        let out = one_line(&s, 800);
        assert!(out.chars().count() <= 801);
    }

    #[test]
    fn output_schema_is_strict() {
        let sch = output_schema();
        assert_eq!(sch["additionalProperties"], serde_json::json!(false));
        assert_eq!(sch["required"], serde_json::json!(["label", "confidence", "reason"]));
    }
}
