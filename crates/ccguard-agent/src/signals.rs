//! Collect content-free provenance signals from a working tree.
//!
//! Everything here is git metadata + manifest *config* — never prompt or code
//! content. The server turns these raw facts into trust signals against tenant
//! policy (`ccguard_core::provenance`). Cached per `cwd` so a transcript with
//! thousands of interactions in one dir does the git work once.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use ccguard_core::provenance::{RawSignals, RemoteRef};
use ccguard_core::remote::parse_remote_url;

/// Gather raw provenance facts for `cwd`. `corp_env_name` is the MDM-injected env
/// var the policy expects (e.g. `CCGUARD_CORP`); its presence (not value) is reported.
pub fn gather(cwd: &str, corp_env_name: &str) -> RawSignals {
    let mut s = RawSignals::default();

    // Remotes (dedup by host/org).
    s.remotes = git_remotes(cwd);

    // Pushed = current branch has an upstream.
    s.pushed = git_ok(
        cwd,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    );

    // HEAD committer email + signature status.
    s.committer_email = git_out(cwd, &["log", "-1", "--format=%ce"]).filter(|e| !e.is_empty());
    s.commit_signed = git_out(cwd, &["log", "-1", "--format=%G?"])
        .map(|g| is_signed(&g))
        .unwrap_or(false);

    // Resolved git config email (git honors includeIf for us).
    s.config_email = git_out(cwd, &["config", "user.email"]).filter(|e| !e.is_empty());

    // MDM corp env marker (presence only).
    s.env_corp_marker = std::env::var(corp_env_name)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);

    // Branch (for ticket-prefix matching server-side).
    s.branch = git_out(cwd, &["rev-parse", "--abbrev-ref", "HEAD"])
        .filter(|b| b != "HEAD" && !b.is_empty());

    // Monorepo leaf: cwd inside a repo but not at its root.
    if let Some(root) = git_out(cwd, &["rev-parse", "--show-toplevel"]) {
        if !same_path(&root, cwd) {
            s.monorepo_leaf = true;
            s.monorepo_root = s.remotes.first().cloned();
        }
    }

    // Registry fingerprints from manifests at cwd and the repo root.
    let mut roots = vec![cwd.to_string()];
    if let Some(root) = git_out(cwd, &["rev-parse", "--show-toplevel"]) {
        if !roots.iter().any(|r| same_path(r, &root)) {
            roots.push(root);
        }
    }
    for dir in &roots {
        s.registry_fingerprints
            .extend(registry_fingerprints(Path::new(dir)));
    }
    s.registry_fingerprints.sort();
    s.registry_fingerprints.dedup();

    s
}

/// `%G?` status: G(ood) and U(nknown-validity good) count as signed; everything
/// else (N none, B bad, E error, X expired, Y/R revoked) does not.
fn is_signed(g: &str) -> bool {
    matches!(g.trim(), "G" | "U")
}

fn same_path(a: &str, b: &str) -> bool {
    fn norm(p: &str) -> String {
        p.replace('\\', "/")
            .trim_end_matches('/')
            .to_ascii_lowercase()
    }
    norm(a) == norm(b)
}

fn git_out(cwd: &str, args: &[&str]) -> Option<String> {
    let mut full = vec!["-C", cwd];
    full.extend_from_slice(args);
    let out = Command::new("git").args(&full).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn git_ok(cwd: &str, args: &[&str]) -> bool {
    let mut full = vec!["-C", cwd];
    full.extend_from_slice(args);
    Command::new("git")
        .args(&full)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn git_remotes(cwd: &str) -> Vec<RemoteRef> {
    let Some(text) = git_out(cwd, &["remote", "-v"]) else {
        return Vec::new();
    };
    let mut refs: Vec<RemoteRef> = Vec::new();
    for line in text.lines() {
        // "origin\thttps://github.com/acme/repo.git (fetch)"
        let url = line.split_whitespace().nth(1);
        if let Some(id) = url.and_then(parse_remote_url) {
            let r = RemoteRef {
                host: id.host,
                org: id.org,
            };
            if !refs.contains(&r) {
                refs.push(r);
            }
        }
    }
    refs
}

/// Extract private-registry fingerprints from manifest *config* in `dir`.
/// Reads only well-known config files; returns content-free host/scope tokens.
pub fn registry_fingerprints(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();

    // .npmrc / .yarnrc — registry hosts + scoped-registry + artifact stores.
    for f in [".npmrc", ".yarnrc", ".yarnrc.yml"] {
        if let Ok(t) = std::fs::read_to_string(dir.join(f)) {
            for line in t.lines() {
                let l = line.trim();
                if l.starts_with('#') || l.is_empty() {
                    continue;
                }
                if let Some(host) = registry_host(l) {
                    out.push(host);
                }
                let ll = l.to_ascii_lowercase();
                if ll.contains("artifactory") {
                    out.push("artifactory".into());
                }
                if ll.contains("codeartifact") {
                    out.push("codeartifact".into());
                }
                // "@scope:registry=..."
                if let Some(scope) = l
                    .strip_prefix('@')
                    .and_then(|r| r.split_once(':').map(|(s, _)| s))
                {
                    out.push(format!("@{scope}"));
                }
            }
        }
    }

    // package.json — @scope dependency names + a "publishConfig.registry".
    if let Ok(t) = std::fs::read_to_string(dir.join("package.json")) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
            for key in ["dependencies", "devDependencies", "peerDependencies"] {
                if let Some(obj) = v.get(key).and_then(|d| d.as_object()) {
                    for name in obj.keys() {
                        if let Some(scope) =
                            name.strip_prefix('@').and_then(|r| r.split('/').next())
                        {
                            out.push(format!("@{scope}"));
                        }
                    }
                }
            }
            if let Some(reg) = v
                .get("publishConfig")
                .and_then(|p| p.get("registry"))
                .and_then(|r| r.as_str())
            {
                if let Some(host) = registry_host(&format!("registry={reg}")) {
                    out.push(host);
                }
            }
        }
    }

    // go.mod — module path host (e.g. internal git host).
    if let Ok(t) = std::fs::read_to_string(dir.join("go.mod")) {
        for line in t.lines() {
            if let Some(rest) = line.trim().strip_prefix("module ") {
                if let Some(host) = rest.trim().split('/').next() {
                    if host.contains('.') {
                        out.push(host.to_ascii_lowercase());
                    }
                }
                break;
            }
        }
    }

    out.sort();
    out.dedup();
    out
}

/// Pull the host out of an npmrc `registry=` / scoped-registry line.
fn registry_host(line: &str) -> Option<String> {
    let val = line.split_once('=').map(|(_, v)| v.trim()).unwrap_or(line);
    let no_scheme = val.split("://").nth(1).unwrap_or(val);
    let host = no_scheme.split(['/', ':']).next()?.trim();
    if host.contains('.') && !host.eq_ignore_ascii_case("registry.npmjs.org") {
        Some(host.to_ascii_lowercase())
    } else {
        None
    }
}

/// Per-cwd cache so we run git/manifest reads once per directory.
#[derive(Default)]
pub struct SignalCache {
    corp_env: String,
    cache: HashMap<String, RawSignals>,
}

impl SignalCache {
    pub fn new(corp_env: &str) -> Self {
        SignalCache {
            corp_env: corp_env.to_string(),
            cache: HashMap::new(),
        }
    }

    pub fn resolve(&mut self, cwd: &str) -> RawSignals {
        if let Some(s) = self.cache.get(cwd) {
            return s.clone();
        }
        let s = gather(cwd, &self.corp_env);
        self.cache.insert(cwd.to_string(), s.clone());
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_status_mapping() {
        assert!(is_signed("G"));
        assert!(is_signed(" U "));
        assert!(!is_signed("N"));
        assert!(!is_signed("B"));
        assert!(!is_signed("E"));
        assert!(!is_signed(""));
    }

    #[test]
    fn registry_host_skips_public_npm() {
        assert_eq!(registry_host("registry=https://registry.npmjs.org/"), None);
        assert_eq!(
            registry_host("registry=https://artifactory.acme.com/api/npm/"),
            Some("artifactory.acme.com".into())
        );
    }

    #[test]
    fn fingerprints_pick_up_scope_and_artifactory() {
        let dir = std::env::temp_dir().join(format!("ccg_sig_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(".npmrc"),
            "@acme:registry=https://artifactory.acme.com/api/npm/\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("package.json"),
            r#"{"dependencies":{"@acme/ui":"1.0.0","react":"18"}}"#,
        )
        .unwrap();
        let fp = registry_fingerprints(&dir);
        std::fs::remove_dir_all(&dir).ok();
        assert!(fp.iter().any(|f| f == "@acme"));
        assert!(fp
            .iter()
            .any(|f| f == "artifactory" || f == "artifactory.acme.com"));
    }

    #[test]
    fn same_path_normalizes_slashes_and_case() {
        assert!(same_path("C:\\Work\\Repo", "c:/work/repo/"));
        assert!(!same_path("C:/work/repo/leaf", "C:/work/repo"));
    }
}
