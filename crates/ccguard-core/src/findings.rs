//! Pure secret / PII / credential scanner.
//!
//! Stateless: feed it text, get back a list of [`Finding`]s. No DB, no I/O.
//! Findings store only a *redacted* preview of the matched value — never the
//! raw secret.
//!
//! The `regex` crate has no backtracking (so it is immune to catastrophic
//! backtracking) and no lookaround. The OpenAI-vs-Anthropic key ambiguity
//! (`sk-ant-...` is also matched by the broader `sk-...` OpenAI pattern) is
//! therefore resolved in code by recording the Anthropic match spans first and
//! skipping any OpenAI match that begins inside one (belt-and-suspenders: we
//! also skip OpenAI matches whose text starts with `sk-ant`).

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// One detected secret or piece of PII.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    pub kind: FindingKind,
    pub rule: String,
    pub severity: Severity,
    pub redacted: String, // safe preview — NEVER the full secret
    pub start: usize,     // byte offset in scanned content
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    Secret,
    Pii,
}

impl FindingKind {
    /// Stable lowercase string for storage/binding. Matches the serde repr.
    pub fn as_str(&self) -> &'static str {
        match self {
            FindingKind::Secret => "secret",
            FindingKind::Pii => "pii",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    High,
    Medium,
    Low,
}

impl Severity {
    /// Stable lowercase string for storage/binding. Matches the serde repr.
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
        }
    }
}

// --- Compiled patterns (compiled once, lazily) ---------------------------------

static AWS_ACCESS_KEY: Lazy<Regex> = Lazy::new(|| Regex::new(r"AKIA[0-9A-Z]{16}").unwrap());
static GITHUB_TOKEN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"gh[posru]_[A-Za-z0-9]{36,}").unwrap());
static ANTHROPIC_KEY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"sk-ant-[A-Za-z0-9_-]{20,}").unwrap());
static OPENAI_KEY: Lazy<Regex> = Lazy::new(|| Regex::new(r"sk-[A-Za-z0-9]{20,}").unwrap());
static SLACK_TOKEN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"xox[baprs]-[A-Za-z0-9-]{10,}").unwrap());
static STRIPE_SECRET: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:sk|rk)_live_[A-Za-z0-9]{16,}").unwrap());
static GOOGLE_API_KEY: Lazy<Regex> = Lazy::new(|| Regex::new(r"AIza[0-9A-Za-z_-]{35}").unwrap());
static JWT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"eyJ[A-Za-z0-9_-]{8,}\.eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}").unwrap()
});
static PRIVATE_KEY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"-----BEGIN (?:RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY-----").unwrap()
});
static EMAIL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap());
static US_SSN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap());
static CC_CANDIDATE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b(?:\d[ -]?){13,19}\b").unwrap());

/// Redact a matched value into a safe preview.
///
/// `len <= 6` → fully masked; otherwise first 4 chars + `…` + last 2 chars.
/// Secrets we match are ASCII, but we iterate over `chars()` to stay safe even
/// if a non-ASCII run slips through (e.g. the email rule).
fn redact(m: &str) -> String {
    let chars: Vec<char> = m.chars().collect();
    if chars.len() <= 6 {
        return "*".repeat(chars.len());
    }
    let first: String = chars[..4].iter().collect();
    let last: String = chars[chars.len() - 2..].iter().collect();
    format!("{first}…{last}")
}

/// Standard Luhn checksum. `digits` must already be stripped to ASCII digits.
fn luhn_valid(digits: &str) -> bool {
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let mut sum = 0u32;
    let mut double = false;
    // Process right-to-left.
    for b in digits.bytes().rev() {
        let mut d = (b - b'0') as u32;
        if double {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
        double = !double;
    }
    sum % 10 == 0
}

/// Whether a digit string is a *real* credit-card number: Luhn-valid, plausible length,
/// AND carrying a known major-network IIN prefix (Visa / Mastercard / Amex / Discover /
/// Diners / JCB). The prefix check is what kills the false positives — roughly 1 in 10
/// random long numbers pass Luhn, but almost none carry a valid card prefix, so a bare
/// "Luhn + length" rule lights up on timestamps, IDs, and hashes across a real corpus.
fn looks_like_card(d: &str) -> bool {
    let n = d.len();
    if !(13..=19).contains(&n) || !luhn_valid(d) {
        return false;
    }
    let p = |k: usize| d.get(..k).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    let (p1, p2, p3, p4) = (p(1), p(2), p(3), p(4));
    if p1 == 4 {
        n == 13 || n == 16 || n == 19 // Visa
    } else if (51..=55).contains(&p2) {
        n == 16 // Mastercard
    } else if (2221..=2720).contains(&p4) {
        n == 16 // Mastercard 2-series
    } else if p2 == 34 || p2 == 37 {
        n == 15 // American Express
    } else if p4 == 6011 || p2 == 65 || (644..=649).contains(&p3) {
        n == 16 || n == 19 // Discover
    } else if (3528..=3589).contains(&p4) {
        (16..=19).contains(&n) // JCB
    } else if (300..=305).contains(&p3) || p2 == 36 || p2 == 38 || p2 == 39 {
        n == 14 // Diners Club
    } else {
        false
    }
}

/// Scan `content` for secrets and PII. Stateless and allocation-light.
pub fn scan(content: &str) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();

    // Track (start, end) spans of every match we keep so we can (a) dedup exact
    // spans and (b) implement the Anthropic-before-OpenAI disambiguation.
    let mut spans: Vec<(usize, usize)> = Vec::new();

    // Helper closure that pushes a finding if the exact span isn't already taken.
    // Returns whether it was pushed.
    fn push(
        findings: &mut Vec<Finding>,
        spans: &mut Vec<(usize, usize)>,
        kind: FindingKind,
        rule: &str,
        severity: Severity,
        matched: &str,
        start: usize,
    ) {
        let end = start + matched.len();
        if spans.iter().any(|&(s, e)| s == start && e == end) {
            return; // exact-span dedup: keep the higher-priority (earlier) rule
        }
        spans.push((start, end));
        findings.push(Finding {
            kind,
            rule: rule.to_string(),
            severity,
            redacted: redact(matched),
            start,
        });
    }

    // --- Secrets (High) ---

    for m in AWS_ACCESS_KEY.find_iter(content) {
        push(
            &mut findings,
            &mut spans,
            FindingKind::Secret,
            "aws_access_key",
            Severity::High,
            m.as_str(),
            m.start(),
        );
    }
    for m in GITHUB_TOKEN.find_iter(content) {
        push(
            &mut findings,
            &mut spans,
            FindingKind::Secret,
            "github_token",
            Severity::High,
            m.as_str(),
            m.start(),
        );
    }

    // Anthropic FIRST, recording its spans, so OpenAI can skip them.
    let mut anthropic_spans: Vec<(usize, usize)> = Vec::new();
    for m in ANTHROPIC_KEY.find_iter(content) {
        anthropic_spans.push((m.start(), m.end()));
        push(
            &mut findings,
            &mut spans,
            FindingKind::Secret,
            "anthropic_key",
            Severity::High,
            m.as_str(),
            m.start(),
        );
    }
    for m in OPENAI_KEY.find_iter(content) {
        // Skip if this OpenAI match starts inside an already-recorded Anthropic
        // span, or if the matched text is itself an Anthropic key.
        let inside_anthropic = anthropic_spans
            .iter()
            .any(|&(s, e)| m.start() >= s && m.start() < e);
        if inside_anthropic || m.as_str().starts_with("sk-ant") {
            continue;
        }
        push(
            &mut findings,
            &mut spans,
            FindingKind::Secret,
            "openai_key",
            Severity::High,
            m.as_str(),
            m.start(),
        );
    }

    for m in SLACK_TOKEN.find_iter(content) {
        push(
            &mut findings,
            &mut spans,
            FindingKind::Secret,
            "slack_token",
            Severity::High,
            m.as_str(),
            m.start(),
        );
    }
    for m in STRIPE_SECRET.find_iter(content) {
        push(
            &mut findings,
            &mut spans,
            FindingKind::Secret,
            "stripe_secret",
            Severity::High,
            m.as_str(),
            m.start(),
        );
    }
    for m in GOOGLE_API_KEY.find_iter(content) {
        push(
            &mut findings,
            &mut spans,
            FindingKind::Secret,
            "google_api_key",
            Severity::High,
            m.as_str(),
            m.start(),
        );
    }
    for m in JWT.find_iter(content) {
        push(
            &mut findings,
            &mut spans,
            FindingKind::Secret,
            "jwt",
            Severity::High,
            m.as_str(),
            m.start(),
        );
    }
    for m in PRIVATE_KEY.find_iter(content) {
        push(
            &mut findings,
            &mut spans,
            FindingKind::Secret,
            "private_key",
            Severity::High,
            m.as_str(),
            m.start(),
        );
    }

    // --- PII (Medium) ---

    for m in EMAIL.find_iter(content) {
        push(
            &mut findings,
            &mut spans,
            FindingKind::Pii,
            "email",
            Severity::Medium,
            m.as_str(),
            m.start(),
        );
    }
    for m in US_SSN.find_iter(content) {
        push(
            &mut findings,
            &mut spans,
            FindingKind::Pii,
            "us_ssn",
            Severity::Medium,
            m.as_str(),
            m.start(),
        );
    }
    for m in CC_CANDIDATE.find_iter(content) {
        let digits: String = m.as_str().chars().filter(|c| c.is_ascii_digit()).collect();
        if looks_like_card(&digits) {
            push(
                &mut findings,
                &mut spans,
                FindingKind::Pii,
                "credit_card",
                Severity::Medium,
                m.as_str(),
                m.start(),
            );
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(fs: &[Finding]) -> Vec<&str> {
        fs.iter().map(|f| f.rule.as_str()).collect()
    }

    fn find_rule<'a>(fs: &'a [Finding], rule: &str) -> Option<&'a Finding> {
        fs.iter().find(|f| f.rule == rule)
    }

    #[test]
    fn aws_access_key_positive_and_near_miss() {
        let fs = scan("key=AKIAIOSFODNN7EXAMPLE in config");
        let f = find_rule(&fs, "aws_access_key").expect("aws key found");
        assert_eq!(f.kind, FindingKind::Secret);
        assert_eq!(f.severity, Severity::High);
        // Near-miss: AKIA with too few chars is NOT matched.
        assert!(scan("AKIA123 too short").is_empty());
    }

    #[test]
    fn github_token_positive_and_near_miss() {
        let tok = format!("ghp_{}", "a".repeat(36));
        let fs = scan(&format!("token {tok} here"));
        let f = find_rule(&fs, "github_token").expect("gh token found");
        assert_eq!(f.severity, Severity::High);
        assert_eq!(f.kind, FindingKind::Secret);
        // Near-miss: too few chars after the prefix.
        assert!(find_rule(&scan("ghp_short"), "github_token").is_none());
    }

    #[test]
    fn anthropic_key_not_double_reported_as_openai() {
        let fs = scan("sk-ant-abcdefghij1234567890XYZ");
        assert_eq!(rules(&fs), vec!["anthropic_key"]);
        assert_eq!(fs.len(), 1);
        assert_eq!(fs[0].severity, Severity::High);
        assert_eq!(fs[0].kind, FindingKind::Secret);
    }

    #[test]
    fn plain_openai_key_is_reported() {
        let key = format!("sk-{}", "A1b2C3d4E5".repeat(3)); // 30 chars after prefix
        let fs = scan(&format!("OPENAI_API_KEY={key}"));
        let f = find_rule(&fs, "openai_key").expect("openai key found");
        assert_eq!(f.severity, Severity::High);
        assert!(find_rule(&fs, "anthropic_key").is_none());
    }

    #[test]
    fn slack_stripe_google_jwt_private_key() {
        let slack = scan("xoxb-1234567890-abcdef");
        assert!(find_rule(&slack, "slack_token").is_some());

        let stripe = scan(&format!("sk_live_{}", "a".repeat(20)));
        assert!(find_rule(&stripe, "stripe_secret").is_some());

        let google = scan(&format!("AIza{}", "B".repeat(35)));
        assert!(find_rule(&google, "google_api_key").is_some());

        let jwt = scan("eyJhbGciOiJI.eyJzdWIiOiIx.SflKxwRJSM_pole");
        assert!(find_rule(&jwt, "jwt").is_some());

        let pk = scan("-----BEGIN RSA PRIVATE KEY-----\nMIIE...");
        assert!(find_rule(&pk, "private_key").is_some());
        let pk_plain = scan("-----BEGIN PRIVATE KEY-----");
        assert!(find_rule(&pk_plain, "private_key").is_some());
    }

    #[test]
    fn credit_card_luhn_valid_spaced() {
        let fs = scan("card 4242 4242 4242 4242 on file");
        let f = find_rule(&fs, "credit_card").expect("valid CC found");
        assert_eq!(f.kind, FindingKind::Pii);
        assert_eq!(f.severity, Severity::Medium);
    }

    #[test]
    fn credit_card_luhn_invalid_not_found() {
        let fs = scan("1234 5678 9012 3456");
        assert!(find_rule(&fs, "credit_card").is_none());
    }

    #[test]
    fn credit_card_visa_test_number() {
        let fs = scan("4111111111111111");
        assert!(find_rule(&fs, "credit_card").is_some());
    }

    #[test]
    fn credit_card_requires_real_iin_prefix() {
        // A real Amex (37 prefix, 15 digits) is still caught.
        assert!(find_rule(&scan("amex 378282246310005 here"), "credit_card").is_some());
        // Luhn-VALID but with a non-card leading digit (7) -> rejected. This is the
        // false-positive class that lit up across a real corpus (timestamps/ids/hashes).
        assert!(luhn_valid("7000000000000005"));
        assert!(find_rule(&scan("snowflake 7000000000000005"), "credit_card").is_none());
        // Direct helper checks.
        assert!(looks_like_card("4242424242424242")); // Visa
        assert!(looks_like_card("378282246310005")); // Amex
        assert!(!looks_like_card("7000000000000005")); // Luhn-valid non-card
    }

    #[test]
    fn email_is_pii_medium() {
        let fs = scan("contact alice@example.com please");
        let emails: Vec<_> = fs.iter().filter(|f| f.rule == "email").collect();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].kind, FindingKind::Pii);
        assert_eq!(emails[0].severity, Severity::Medium);
    }

    #[test]
    fn redact_bounds() {
        let full = "AKIAIOSFODNN7EXAMPLE";
        let r = redact(full);
        assert_ne!(r, full);
        assert!(r.chars().count() < full.chars().count());
        assert!(r.starts_with("AKIA"));
        assert!(r.ends_with("LE"));
        // Short input is fully masked.
        assert_eq!(redact("abc"), "***");
        assert_eq!(redact("123456"), "******");
    }

    #[test]
    fn no_secrets_empty() {
        assert!(scan("just some normal prose with no secrets").is_empty());
    }

    #[test]
    fn ssn_found_phone_not_matched_as_ssn() {
        let fs = scan("SSN 123-45-6789 on record");
        let ssn = find_rule(&fs, "us_ssn").expect("ssn found");
        assert_eq!(ssn.kind, FindingKind::Pii);
        assert_eq!(ssn.severity, Severity::Medium);
        // A phone-like number must NOT match the SSN rule.
        assert!(find_rule(&scan("call 123-456-7890"), "us_ssn").is_none());
    }

    #[test]
    fn luhn_helper_direct() {
        assert!(luhn_valid("4242424242424242"));
        assert!(luhn_valid("4111111111111111"));
        assert!(!luhn_valid("1234567890123456"));
        assert!(!luhn_valid(""));
        assert!(!luhn_valid("not-digits"));
    }

    #[test]
    fn enum_as_str_matches_serde_repr() {
        assert_eq!(FindingKind::Secret.as_str(), "secret");
        assert_eq!(FindingKind::Pii.as_str(), "pii");
        assert_eq!(Severity::High.as_str(), "high");
        assert_eq!(Severity::Medium.as_str(), "medium");
        assert_eq!(Severity::Low.as_str(), "low");
    }

    #[test]
    fn finding_serializes_snake_case() {
        let fs = scan("AKIAIOSFODNN7EXAMPLE");
        let json = serde_json::to_string(&fs[0]).unwrap();
        assert!(json.contains("\"secret\""));
        assert!(json.contains("\"high\""));
        // The raw secret must never appear in the serialized finding.
        assert!(!json.contains("AKIAIOSFODNN7EXAMPLE"));
    }
}
