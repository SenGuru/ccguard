//! Parse a git remote URL into (host, org, name). Handles scp-like (`git@host:org/repo.git`),
//! https, and ssh forms, with or without a trailing `.git`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteIdentity {
    pub host: String,
    pub org: String,
    pub name: String,
}

pub fn parse_remote_url(url: &str) -> Option<RemoteIdentity> {
    let mut s = url.trim();
    if let Some(stripped) = s.strip_suffix(".git") {
        s = stripped;
    }

    let (host, path) = if let Some(rest) = s.strip_prefix("git@") {
        // scp-like: git@github.com:org/repo
        let (h, p) = rest.split_once(':')?;
        (h.to_string(), p.to_string())
    } else {
        // strip scheme://
        let no_scheme = match s.find("://") {
            Some(i) => &s[i + 3..],
            None => s,
        };
        // strip optional user@
        let no_user = match no_scheme.split_once('@') {
            Some((_, after)) => after,
            None => no_scheme,
        };
        let (h, p) = no_user.split_once('/')?;
        (h.to_string(), p.to_string())
    };

    let parts: Vec<&str> = path.split('/').filter(|x| !x.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }
    let org = parts[0].to_string();
    let name = parts[parts.len() - 1].to_string();
    if host.is_empty() || org.is_empty() || name.is_empty() {
        return None;
    }
    Some(RemoteIdentity {
        host: host.to_ascii_lowercase(),
        org,
        name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(host: &str, org: &str, name: &str) -> RemoteIdentity {
        RemoteIdentity {
            host: host.into(),
            org: org.into(),
            name: name.into(),
        }
    }

    #[test]
    fn parses_scp_like() {
        assert_eq!(
            parse_remote_url("git@github.com:acme-corp/billing.git"),
            Some(id("github.com", "acme-corp", "billing"))
        );
    }

    #[test]
    fn parses_https_with_and_without_git_suffix() {
        assert_eq!(
            parse_remote_url("https://github.com/acme-corp/billing.git"),
            Some(id("github.com", "acme-corp", "billing"))
        );
        assert_eq!(
            parse_remote_url("https://github.com/acme-corp/billing"),
            Some(id("github.com", "acme-corp", "billing"))
        );
    }

    #[test]
    fn parses_ssh_scheme_with_user() {
        assert_eq!(
            parse_remote_url("ssh://git@gitlab.acme.com/group/billing.git"),
            Some(id("gitlab.acme.com", "group", "billing"))
        );
    }

    #[test]
    fn subgroup_org_is_first_segment_name_is_last() {
        assert_eq!(
            parse_remote_url("https://gitlab.acme.com/group/sub/billing.git"),
            Some(id("gitlab.acme.com", "group", "billing"))
        );
    }

    #[test]
    fn host_is_lowercased() {
        assert_eq!(
            parse_remote_url("git@GitHub.com:Acme/Repo.git"),
            Some(id("github.com", "Acme", "Repo"))
        );
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_remote_url("not-a-url"), None);
        assert_eq!(parse_remote_url("https://github.com/onlyone"), None);
    }
}
