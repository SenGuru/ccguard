//! OS-appropriate enforcement-policy discovery + a stable per-machine device id.
//!
//! The agent probes for Claude Code's enterprise **managed-settings.json** — the
//! highest-precedence policy file an admin deploys to lock a fleet — at the
//! standard per-OS system locations, then attests the contents against the
//! server-supplied expected `PolicyConfig`.
//!
//! NOTE: On Windows, an additional managed-settings source is the registry value
//! under `HKLM\SOFTWARE\Policies\ClaudeCode`. That registry plane is NOT probed
//! here; the file probe below covers the common enterprise deployment (MDM/GPO
//! that drops the JSON file into `C:\ProgramData\ClaudeCode\`).

use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// OS-appropriate managed-settings.json candidate paths (highest-precedence enterprise policy).
pub fn managed_settings_candidates() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        vec![
            PathBuf::from(r"C:\ProgramData\ClaudeCode\managed-settings.json"),
            PathBuf::from(r"C:\Program Files\ClaudeCode\managed-settings.json"),
        ]
    }
    #[cfg(target_os = "macos")]
    {
        vec![PathBuf::from(
            "/Library/Application Support/ClaudeCode/managed-settings.json",
        )]
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        vec![PathBuf::from("/etc/claude-code/managed-settings.json")]
    }
}

/// First existing candidate + its contents.
pub fn find_managed_settings() -> Option<(PathBuf, String)> {
    for p in managed_settings_candidates() {
        if let Ok(s) = std::fs::read_to_string(&p) {
            return Some((p, s));
        }
    }
    None
}

/// Stable-ish per-machine id: hash(hostname + os). Non-empty, deterministic within a machine.
pub fn device_id() -> String {
    let seed = format!("{}|{}", hostname(), os_str());
    let digest = Sha256::digest(seed.as_bytes());
    hex::encode(digest)[..16].to_string()
}

/// Best-effort machine hostname: `COMPUTERNAME` (Windows) / `HOSTNAME` (unix),
/// else `"unknown-host"`. No external crate — just env probing.
pub fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "unknown-host".to_string())
}

/// Human-readable OS string reported to the server (`windows` / `macos` / `linux` / other).
pub fn os_str() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "linux"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_list_is_non_empty_and_ends_with_managed_settings() {
        let cands = managed_settings_candidates();
        assert!(!cands.is_empty(), "candidate list must be non-empty");
        let first = cands[0].to_string_lossy();
        assert!(
            first.ends_with("managed-settings.json"),
            "first candidate should be a managed-settings.json path: {first}"
        );
    }

    #[test]
    fn device_id_is_non_empty_and_stable() {
        let a = device_id();
        let b = device_id();
        assert!(!a.is_empty(), "device_id must be non-empty");
        assert_eq!(a, b, "device_id must be deterministic within a machine");
    }

    #[test]
    fn hostname_is_non_empty() {
        assert!(!hostname().is_empty(), "hostname must be non-empty");
    }

    #[test]
    fn os_str_is_human_readable() {
        let os = os_str();
        assert!(
            matches!(os, "windows" | "macos" | "linux" | "unknown"),
            "os string must be human-readable, got {os}"
        );
    }
}
