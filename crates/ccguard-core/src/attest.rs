//! Enforcement **attestation evaluator** (pure, no I/O).
//!
//! Given the managed-settings JSON found on a device (or `None` if absent), the
//! expected [`PolicyConfig`], and the active Claude account/org, this module
//! produces an [`Attestation`] snapshot and a compliance [`verdict`].
//!
//! It is deliberately defensive: malformed on-disk JSON is treated as "present
//! but non-matching" (every config flag false) rather than panicking, so a
//! tampered/corrupt policy still yields a useful verdict.

use crate::enforce::{canonical_json, policy_hash, PolicyConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// A point-in-time snapshot of a device's enforcement posture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attestation {
    /// A managed-settings file was found on disk.
    pub policy_present: bool,
    /// Canonical SHA-256 (hex) of the on-disk policy, if it parsed.
    pub policy_hash: Option<String>,
    /// On-disk hash equals the expected policy hash.
    pub policy_match: bool,
    /// `env.CLAUDE_CODE_ENABLE_TELEMETRY == "1"`.
    pub telemetry_on: bool,
    /// A hook command references `ccguard-agent`.
    pub hook_present: bool,
    /// `forceLoginMethod` set AND `forceLoginOrgUUID` == expected org.
    pub login_locked: bool,
    /// `permissions.disableBypassPermissionsMode == "disable"`.
    pub bypass_disabled: bool,
    /// Logged-in account email (from `~/.claude.json`), if known.
    pub active_account: Option<String>,
    /// Logged-in org UUID, if known.
    pub active_org: Option<String>,
    /// Active org is known AND is not the expected corp org.
    pub personal_account: bool,
}

/// Overall compliance state derived from an [`Attestation`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Compliance {
    Compliant,
    Drifted,
    Tampered,
    NoncompliantAccount,
}

/// Evaluate a device's enforcement posture.
///
/// `on_disk` is the raw managed-settings JSON text (or `None` if no file was
/// found). `expected` is the policy the device *should* be running.
/// `active_account` / `active_org` describe the logged-in Claude identity.
pub fn evaluate(
    on_disk: Option<&str>,
    expected: &PolicyConfig,
    active_account: Option<&str>,
    active_org: Option<&str>,
) -> Attestation {
    let active_account = active_account.map(str::to_string);
    let active_org = active_org.map(str::to_string);

    // `personal_account` is independent of the on-disk policy: if we know the
    // active org and it isn't the corp org, the account is non-corp. Unknown org
    // is *not* treated as personal — don't false-accuse.
    let personal_account = match &active_org {
        Some(org) => org != &expected.org_uuid,
        None => false,
    };

    let Some(text) = on_disk else {
        return Attestation {
            policy_present: false,
            policy_hash: None,
            policy_match: false,
            telemetry_on: false,
            hook_present: false,
            login_locked: false,
            bypass_disabled: false,
            active_account,
            active_org,
            personal_account,
        };
    };

    // Present from here on. Parse defensively.
    let Ok(parsed) = serde_json::from_str::<Value>(text) else {
        // Malformed → present, but every config flag false and no hash.
        return Attestation {
            policy_present: true,
            policy_hash: None,
            policy_match: false,
            telemetry_on: false,
            hook_present: false,
            login_locked: false,
            bypass_disabled: false,
            active_account,
            active_org,
            personal_account,
        };
    };

    let hash = hex::encode(Sha256::digest(canonical_json(&parsed).as_bytes()));
    let policy_match = hash == policy_hash(expected);

    let telemetry_on = parsed
        .get("env")
        .and_then(|e| e.get("CLAUDE_CODE_ENABLE_TELEMETRY"))
        == Some(&Value::String("1".to_string()));

    let hook_present = hook_references_agent(&parsed);
    let login_locked = login_locked(&parsed, &expected.org_uuid);

    let bypass_disabled = parsed
        .get("permissions")
        .and_then(|p| p.get("disableBypassPermissionsMode"))
        == Some(&Value::String("disable".to_string()));

    Attestation {
        policy_present: true,
        policy_hash: Some(hash),
        policy_match,
        telemetry_on,
        hook_present,
        login_locked,
        bypass_disabled,
        active_account,
        active_org,
        personal_account,
    }
}

/// True if any `hooks.<event>[i].hooks[j].command` string contains
/// `"ccguard-agent"`. Walks the structure defensively.
fn hook_references_agent(parsed: &Value) -> bool {
    let Some(hooks) = parsed.get("hooks").and_then(Value::as_object) else {
        return false;
    };
    for matchers in hooks.values() {
        let Some(matchers) = matchers.as_array() else {
            continue;
        };
        for matcher in matchers {
            let Some(inner) = matcher.get("hooks").and_then(Value::as_array) else {
                continue;
            };
            for hook in inner {
                if let Some(cmd) = hook.get("command").and_then(Value::as_str) {
                    if cmd.contains("ccguard-agent") {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// `forceLoginMethod` present (non-null) AND `forceLoginOrgUUID` equals
/// `expected_org` — handling both a JSON string and a JSON array containing it.
fn login_locked(parsed: &Value, expected_org: &str) -> bool {
    let method_set = matches!(parsed.get("forceLoginMethod"), Some(v) if !v.is_null());
    if !method_set {
        return false;
    }
    match parsed.get("forceLoginOrgUUID") {
        Some(Value::String(s)) => s == expected_org,
        Some(Value::Array(arr)) => arr
            .iter()
            .any(|v| v.as_str() == Some(expected_org)),
        _ => false,
    }
}

/// Derive a compliance verdict and the list of drift reasons from `a`.
pub fn verdict(a: &Attestation) -> (Compliance, Vec<String>) {
    if !a.policy_present {
        return (Compliance::Tampered, vec!["managed-settings missing".into()]);
    }
    let mut reasons: Vec<String> = Vec::new();
    if a.personal_account {
        reasons.push("personal/non-corp account in use".into());
    }
    if !a.policy_match {
        reasons.push("policy hash drift".into());
    }
    if !a.telemetry_on {
        reasons.push("telemetry disabled".into());
    }
    if !a.hook_present {
        reasons.push("ccguard hook missing".into());
    }
    if !a.login_locked {
        reasons.push("login not locked to corp org".into());
    }
    if a.personal_account {
        return (Compliance::NoncompliantAccount, reasons);
    }
    if reasons.is_empty() {
        (Compliance::Compliant, reasons)
    } else {
        (Compliance::Drifted, reasons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enforce::managed_settings;
    use serde_json::json;

    fn cfg() -> PolicyConfig {
        PolicyConfig {
            server_url: "https://ccguard.corp.example/".into(),
            org_uuid: "org-1234-5678".into(),
            otel_endpoint: "https://otel.corp.example:4318".into(),
            min_version: "1.0.99".into(),
            token_env: "CCGUARD_TOKEN".into(),
        }
    }

    /// Serialize the canonical generated policy for a config.
    fn policy_str(c: &PolicyConfig) -> String {
        serde_json::to_string(&managed_settings(c)).unwrap()
    }

    #[test]
    fn compliant_policy_evaluates_all_true_and_compliant() {
        let c = cfg();
        let s = policy_str(&c);
        let a = evaluate(Some(&s), &c, Some("alice@corp"), Some(&c.org_uuid));

        assert!(a.policy_present);
        assert!(a.policy_match);
        assert!(a.telemetry_on);
        assert!(a.hook_present);
        assert!(a.login_locked);
        assert!(a.bypass_disabled);
        assert!(!a.personal_account);
        assert_eq!(a.active_account.as_deref(), Some("alice@corp"));
        assert_eq!(a.active_org.as_deref(), Some(c.org_uuid.as_str()));
        assert!(a.policy_hash.is_some());

        let (v, reasons) = verdict(&a);
        assert_eq!(v, Compliance::Compliant);
        assert!(reasons.is_empty());
    }

    #[test]
    fn removing_telemetry_drifts() {
        let c = cfg();
        let mut v = managed_settings(&c);
        // Drop the telemetry env key.
        v["env"]
            .as_object_mut()
            .unwrap()
            .remove("CLAUDE_CODE_ENABLE_TELEMETRY");
        let s = serde_json::to_string(&v).unwrap();
        let a = evaluate(Some(&s), &c, Some("alice@corp"), Some(&c.org_uuid));

        assert!(!a.telemetry_on);
        // Mutating the file also changes the hash.
        assert!(!a.policy_match);
        let (verdict_, reasons) = verdict(&a);
        assert_eq!(verdict_, Compliance::Drifted);
        assert!(reasons.iter().any(|r| r.contains("telemetry")));
    }

    #[test]
    fn stripping_hook_command_drops_hook_present() {
        let c = cfg();
        let mut v = managed_settings(&c);
        // Replace the command with something that does not mention the agent.
        v["hooks"]["SessionEnd"][0]["hooks"][0]["command"] = json!("echo noop");
        let s = serde_json::to_string(&v).unwrap();
        let a = evaluate(Some(&s), &c, Some("alice@corp"), Some(&c.org_uuid));

        assert!(!a.hook_present);
        let (verdict_, reasons) = verdict(&a);
        assert_eq!(verdict_, Compliance::Drifted);
        assert!(reasons.iter().any(|r| r.contains("hook")));
    }

    #[test]
    fn changing_org_uuid_unlocks_login_and_breaks_match() {
        let c = cfg();
        let mut v = managed_settings(&c);
        v["forceLoginOrgUUID"] = json!("org-not-the-corp");
        let s = serde_json::to_string(&v).unwrap();
        // active_org still the corp org so personal_account stays false.
        let a = evaluate(Some(&s), &c, Some("alice@corp"), Some(&c.org_uuid));

        assert!(!a.login_locked);
        assert!(!a.policy_match);
        let (verdict_, reasons) = verdict(&a);
        assert_eq!(verdict_, Compliance::Drifted);
        assert!(reasons.iter().any(|r| r.contains("login not locked")));
    }

    #[test]
    fn login_locked_accepts_array_org_form() {
        let c = cfg();
        let mut v = managed_settings(&c);
        v["forceLoginOrgUUID"] = json!([c.org_uuid.clone(), "org-other"]);
        let s = serde_json::to_string(&v).unwrap();
        let a = evaluate(Some(&s), &c, Some("alice@corp"), Some(&c.org_uuid));
        assert!(a.login_locked);
    }

    #[test]
    fn missing_policy_is_tampered() {
        let c = cfg();
        let a = evaluate(None, &c, Some("alice@corp"), Some(&c.org_uuid));
        assert!(!a.policy_present);
        assert!(a.policy_hash.is_none());
        assert!(!a.policy_match);
        assert!(!a.telemetry_on);
        assert!(!a.hook_present);
        assert!(!a.login_locked);
        assert!(!a.bypass_disabled);
        let (verdict_, reasons) = verdict(&a);
        assert_eq!(verdict_, Compliance::Tampered);
        assert_eq!(reasons, vec!["managed-settings missing".to_string()]);
    }

    #[test]
    fn personal_account_is_noncompliant_account() {
        let c = cfg();
        let s = policy_str(&c);
        // Policy is perfect, but the active org is not the corp org.
        let a = evaluate(Some(&s), &c, Some("bob@gmail.com"), Some("different-uuid"));
        assert!(a.personal_account);
        let (verdict_, reasons) = verdict(&a);
        assert_eq!(verdict_, Compliance::NoncompliantAccount);
        assert!(reasons
            .iter()
            .any(|r| r.contains("personal/non-corp account")));
    }

    #[test]
    fn unknown_active_org_is_not_personal() {
        let c = cfg();
        let s = policy_str(&c);
        let a = evaluate(Some(&s), &c, Some("alice@corp"), None);
        assert!(!a.personal_account);
        let (verdict_, _) = verdict(&a);
        assert_eq!(verdict_, Compliance::Compliant);
    }

    #[test]
    fn malformed_json_is_present_but_flags_false_no_panic() {
        let c = cfg();
        let a = evaluate(Some("{not json"), &c, Some("alice@corp"), Some(&c.org_uuid));
        assert!(a.policy_present);
        assert!(a.policy_hash.is_none());
        assert!(!a.policy_match);
        assert!(!a.telemetry_on);
        assert!(!a.hook_present);
        assert!(!a.login_locked);
        assert!(!a.bypass_disabled);
        // Account is corp, so verdict is Drifted (not NoncompliantAccount).
        let (verdict_, reasons) = verdict(&a);
        assert_eq!(verdict_, Compliance::Drifted);
        assert!(!reasons.is_empty());
    }

    #[test]
    fn attestation_serializes_snake_case_compliance() {
        let c = cfg();
        let a = evaluate(None, &c, None, None);
        let (v, _) = verdict(&a);
        assert_eq!(serde_json::to_string(&v).unwrap(), "\"tampered\"");
        let nc = Compliance::NoncompliantAccount;
        assert_eq!(serde_json::to_string(&nc).unwrap(), "\"noncompliant_account\"");
    }
}
