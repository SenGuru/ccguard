//! Claude Code **managed-settings.json** generator (pure, no I/O).
//!
//! Produces the enterprise managed-settings policy an admin deploys to lock a
//! fleet of Claude Code installs onto CCGuard: it forces telemetry on, pins the
//! corp login org, restricts managed hooks to the CCGuard server, wires the
//! `ccguard-agent --capture` SessionEnd hook, disables bypass-permissions mode,
//! and requires a minimum Claude Code version.
//!
//! All key names here are Claude Code's **real** enterprise managed-settings
//! keys — `allowManagedHooksOnly`, `forceLoginMethod`, `forceLoginOrgUUID`,
//! `requiredMinimumVersion`, `permissions.disableBypassPermissionsMode`, and the
//! OpenTelemetry `CLAUDE_CODE_ENABLE_TELEMETRY` / `OTEL_*` env vars — not
//! invented placeholders.
//!
//! The hash produced by [`policy_hash`] is computed over a **canonical** JSON
//! rendering (recursively key-sorted, compact) so it is stable across machines
//! and across builds regardless of whether `serde_json`'s `preserve_order`
//! feature is enabled.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// The inputs that vary per tenant/deployment for a managed-settings policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// CCGuard server base URL (trailing slash tolerated/stripped).
    pub server_url: String,
    /// Corp Claude org UUID that logins are locked to.
    pub org_uuid: String,
    /// OTLP collector endpoint the OTEL exporters point at.
    pub otel_endpoint: String,
    /// Minimum allowed Claude Code version.
    pub min_version: String,
    /// Name of the env var holding the ingest token the agent passes to the hook.
    pub token_env: String,
}

/// Build the managed-settings JSON document for `p`.
///
/// Uses Claude Code's real enterprise managed-settings keys. The SessionEnd hook
/// invokes `ccguard-agent --server <base> --token $<TOKEN_ENV> --capture`.
pub fn managed_settings(p: &PolicyConfig) -> Value {
    let base = p.server_url.trim_end_matches('/');
    json!({
        "allowManagedHooksOnly": true,
        "allowedHttpHookUrls": [format!("{base}/*")],
        "env": {
            "CLAUDE_CODE_ENABLE_TELEMETRY": "1",
            "OTEL_METRICS_EXPORTER": "otlp",
            "OTEL_LOGS_EXPORTER": "otlp",
            "OTEL_EXPORTER_OTLP_PROTOCOL": "http/protobuf",
            "OTEL_EXPORTER_OTLP_ENDPOINT": p.otel_endpoint,
            "OTEL_LOG_TOOL_DETAILS": "1"
        },
        "forceLoginMethod": "claudeai",
        "forceLoginOrgUUID": p.org_uuid,
        "hooks": {
            "SessionEnd": [ { "hooks": [ {
                "type": "command",
                "command": format!("ccguard-agent --server {base} --token ${} --capture", p.token_env),
                "timeout": 600
            } ] } ]
        },
        "permissions": { "disableBypassPermissionsMode": "disable" },
        "requiredMinimumVersion": p.min_version
    })
}

/// Recursively sort object keys so the rendering is deterministic, then serialize
/// compactly.
///
/// Objects become key-sorted (`BTreeMap`), arrays preserve their order, and
/// scalars are emitted as-is. This guarantees a stable hash whether or not
/// `serde_json`'s `preserve_order` feature is enabled anywhere in the build.
pub fn canonical_json(v: &Value) -> String {
    serde_json::to_string(&canonicalize(v)).expect("canonical json serialization")
}

/// Produce a `Value` whose objects are recursively key-sorted.
fn canonicalize(v: &Value) -> Value {
    match v {
        Value::Object(map) => {
            // BTreeMap gives a stable (lexicographic) key order independent of
            // the underlying serde_json map type.
            let sorted: BTreeMap<String, Value> = map
                .iter()
                .map(|(k, val)| (k.clone(), canonicalize(val)))
                .collect();
            let mut out = Map::new();
            for (k, val) in sorted {
                out.insert(k, val);
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

/// Stable SHA-256 (hex) of the canonical managed-settings document for `p`.
pub fn policy_hash(p: &PolicyConfig) -> String {
    hex::encode(Sha256::digest(canonical_json(&managed_settings(p)).as_bytes()))
}

/// Pretty-printed managed-settings JSON (for writing the file to disk).
pub fn managed_settings_pretty(p: &PolicyConfig) -> String {
    serde_json::to_string_pretty(&managed_settings(p)).expect("valid json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> PolicyConfig {
        PolicyConfig {
            server_url: "https://ccguard.corp.example/".into(),
            org_uuid: "org-1234-5678".into(),
            otel_endpoint: "https://otel.corp.example:4318".into(),
            min_version: "1.0.99".into(),
            token_env: "CCGUARD_TOKEN".into(),
        }
    }

    #[test]
    fn managed_settings_uses_real_enterprise_keys() {
        let c = cfg();
        let v = managed_settings(&c);
        assert_eq!(v["forceLoginOrgUUID"], json!(c.org_uuid));
        assert_eq!(v["allowManagedHooksOnly"], json!(true));
        assert_eq!(v["env"]["CLAUDE_CODE_ENABLE_TELEMETRY"], json!("1"));
        assert_eq!(
            v["permissions"]["disableBypassPermissionsMode"],
            json!("disable")
        );
        assert_eq!(v["requiredMinimumVersion"], json!(c.min_version));
        assert_eq!(v["forceLoginMethod"], json!("claudeai"));
    }

    #[test]
    fn server_url_trailing_slash_is_stripped() {
        let c = cfg();
        let v = managed_settings(&c);
        // No double slash before the wildcard.
        assert_eq!(
            v["allowedHttpHookUrls"][0],
            json!("https://ccguard.corp.example/*")
        );
    }

    #[test]
    fn session_end_hook_command_has_agent_and_token_env() {
        let c = cfg();
        let v = managed_settings(&c);
        let cmd = v["hooks"]["SessionEnd"][0]["hooks"][0]["command"]
            .as_str()
            .expect("command is a string");
        assert!(cmd.contains("ccguard-agent"), "command has the agent: {cmd}");
        assert!(
            cmd.contains("$CCGUARD_TOKEN"),
            "command references the token env: {cmd}"
        );
        assert!(cmd.contains("--capture"));
        assert_eq!(v["hooks"]["SessionEnd"][0]["hooks"][0]["timeout"], json!(600));
    }

    #[test]
    fn otel_env_block_is_complete() {
        let c = cfg();
        let v = managed_settings(&c);
        assert_eq!(v["env"]["OTEL_METRICS_EXPORTER"], json!("otlp"));
        assert_eq!(v["env"]["OTEL_LOGS_EXPORTER"], json!("otlp"));
        assert_eq!(v["env"]["OTEL_EXPORTER_OTLP_PROTOCOL"], json!("http/protobuf"));
        assert_eq!(
            v["env"]["OTEL_EXPORTER_OTLP_ENDPOINT"],
            json!(c.otel_endpoint)
        );
        assert_eq!(v["env"]["OTEL_LOG_TOOL_DETAILS"], json!("1"));
    }

    #[test]
    fn policy_hash_is_deterministic_for_same_config() {
        let c = cfg();
        assert_eq!(policy_hash(&c), policy_hash(&c));
    }

    #[test]
    fn policy_hash_differs_for_different_org() {
        let a = cfg();
        let mut b = cfg();
        b.org_uuid = "org-9999-0000".into();
        assert_ne!(policy_hash(&a), policy_hash(&b));
    }

    #[test]
    fn canonical_json_sorts_keys_recursively() {
        // Same data, different insertion order, must canonicalize identically.
        let a = json!({ "b": 1, "a": { "y": 2, "x": 3 } });
        let b = json!({ "a": { "x": 3, "y": 2 }, "b": 1 });
        assert_eq!(canonical_json(&a), canonical_json(&b));
        // Keys really are sorted in the output.
        assert_eq!(canonical_json(&a), r#"{"a":{"x":3,"y":2},"b":1}"#);
    }

    #[test]
    fn canonical_json_preserves_array_order() {
        let v = json!({ "xs": [3, 1, 2] });
        assert_eq!(canonical_json(&v), r#"{"xs":[3,1,2]}"#);
    }

    #[test]
    fn managed_settings_pretty_round_trips_to_equal_value() {
        let c = cfg();
        let pretty = managed_settings_pretty(&c);
        let reparsed: Value = serde_json::from_str(&pretty).expect("pretty is valid json");
        assert_eq!(reparsed, managed_settings(&c));
    }
}
