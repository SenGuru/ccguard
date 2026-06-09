use std::process::Command;

use ccguard_core::event::Repo;
use ccguard_core::remote::parse_remote_url;

/// Build a `Repo` from an optional git remote URL + the working directory.
/// Pure: no I/O. With a parseable remote → host/org/name filled; otherwise path only.
pub fn repo_from_remote(remote: Option<&str>, cwd: &str) -> Repo {
    match remote.and_then(parse_remote_url) {
        Some(id) => Repo {
            host: Some(id.host),
            org: Some(id.org),
            name: Some(id.name),
            path: Some(cwd.to_string()),
            classification: None,
            confidence: 0.0,
        },
        None => Repo {
            host: None,
            org: None,
            name: None,
            path: Some(cwd.to_string()),
            classification: None,
            confidence: 0.0,
        },
    }
}

/// Attribute a working directory to a repo by reading its git remote (`git -C <cwd> ...`).
pub fn repo_for_cwd(cwd: &str) -> Repo {
    repo_from_remote(git_remote(cwd).as_deref(), cwd)
}

fn git_remote(cwd: &str) -> Option<String> {
    let out = Command::new("git")
        .args(["-C", cwd, "config", "--get", "remote.origin.url"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_remote_to_host_org_name() {
        let r = repo_from_remote(Some("git@github.com:acme-corp/billing.git"), "C:\\work\\billing");
        assert_eq!(r.host.as_deref(), Some("github.com"));
        assert_eq!(r.org.as_deref(), Some("acme-corp"));
        assert_eq!(r.name.as_deref(), Some("billing"));
        assert_eq!(r.path.as_deref(), Some("C:\\work\\billing"));
    }

    #[test]
    fn no_remote_is_path_only() {
        let r = repo_from_remote(None, "C:\\scratch");
        assert!(r.host.is_none() && r.org.is_none());
        assert_eq!(r.path.as_deref(), Some("C:\\scratch"));
    }
}
