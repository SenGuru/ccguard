use crate::event::Classification;

/// A tenant's approved-resources allowlist.
#[derive(Debug, Default, Clone)]
pub struct Allowlist {
    pub hosts: Vec<String>,      // approved git hosts, e.g. "github.com"
    pub orgs: Vec<String>,       // approved orgs/owners, e.g. "acme-corp"
    pub path_roots: Vec<String>, // approved local path roots, e.g. "c:\\work"
}

/// Classify using the strongest available signal: git remote (host+org) first, then local path.
pub fn classify(
    repo_host: Option<&str>,
    repo_org: Option<&str>,
    repo_path: Option<&str>,
    allow: &Allowlist,
) -> (Classification, f32) {
    if let (Some(host), Some(org)) = (repo_host, repo_org) {
        let host_ok = allow.hosts.iter().any(|h| h.eq_ignore_ascii_case(host));
        let org_ok = allow.orgs.iter().any(|o| o.eq_ignore_ascii_case(org));
        return if host_ok && org_ok {
            (Classification::Work, 0.9)
        } else {
            (Classification::Personal, 0.8)
        };
    }
    if let Some(path) = repo_path {
        let p = path.replace('\\', "/").to_ascii_lowercase();
        let hit = allow
            .path_roots
            .iter()
            .any(|r| p.starts_with(&r.replace('\\', "/").to_ascii_lowercase()));
        return if hit {
            (Classification::Work, 0.6)
        } else {
            (Classification::Unknown, 0.3)
        };
    }
    (Classification::Unknown, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allow() -> Allowlist {
        Allowlist {
            hosts: vec!["github.com".into()],
            orgs: vec!["acme-corp".into()],
            path_roots: vec!["c:\\work".into()],
        }
    }

    #[test]
    fn host_and_org_match_is_work() {
        let (c, conf) = classify(Some("github.com"), Some("acme-corp"), None, &allow());
        assert_eq!(c, Classification::Work);
        assert!(conf > 0.5);
    }

    #[test]
    fn org_outside_allowlist_is_personal() {
        let (c, _) = classify(Some("github.com"), Some("dev-personal"), None, &allow());
        assert_eq!(c, Classification::Personal);
    }

    #[test]
    fn matching_is_case_insensitive() {
        let (c, _) = classify(Some("GitHub.com"), Some("ACME-Corp"), None, &allow());
        assert_eq!(c, Classification::Work);
    }

    #[test]
    fn path_root_match_is_work() {
        let (c, _) = classify(None, None, Some("C:\\work\\scratch"), &allow());
        assert_eq!(c, Classification::Work);
    }

    #[test]
    fn no_signal_is_unknown() {
        let (c, conf) = classify(None, None, None, &allow());
        assert_eq!(c, Classification::Unknown);
        assert_eq!(conf, 0.0);
    }
}
