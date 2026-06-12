//! Server-side provenance: load tenant policy, evaluate the agent's content-free
//! signals through the deterministic cascade (`ccguard_core::provenance`), and
//! persist the verdict. This is the PRIMARY classifier; what it leaves
//! UNCLASSIFIED flows to the LLM triage tier.

use ccguard_core::capture::CapturedSession;
use ccguard_core::event::Classification;
use ccguard_core::provenance::{self, ProvenancePolicy, RawSignals, RemoteRef, Verdict};
use sqlx::{PgPool, Row};

/// Split a comma/newline/whitespace-separated config string into trimmed entries.
fn parse_list(text: &str) -> Vec<String> {
    text.split([',', '\n', '\r', ';'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Load the tenant's provenance policy: corp hosts/orgs from the existing
/// allowlist, plus the provenance_policy lists.
pub async fn load_policy(pool: &PgPool, tenant_id: &str) -> Result<ProvenancePolicy, sqlx::Error> {
    let mut policy = ProvenancePolicy::default();

    let rows = sqlx::query("select kind, value from allowlist_rules where tenant_id = $1")
        .bind(tenant_id)
        .fetch_all(pool)
        .await?;
    for r in rows {
        let kind: String = r.get("kind");
        let value: String = r.get("value");
        match kind.as_str() {
            "host" => policy.corp_hosts.push(value),
            "org" => policy.corp_orgs.push(value),
            _ => {}
        }
    }

    let p = sqlx::query(
        "select corp_email_domains, personal_orgs, personal_email_domains, \
                ticket_prefixes, registry_patterns from provenance_policy where tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?;
    if let Some(p) = p {
        policy.corp_email_domains = parse_list(&p.get::<String, _>("corp_email_domains"));
        policy.personal_orgs = parse_list(&p.get::<String, _>("personal_orgs"));
        policy.personal_email_domains = parse_list(&p.get::<String, _>("personal_email_domains"));
        policy.ticket_prefixes = parse_list(&p.get::<String, _>("ticket_prefixes"));
        policy.registry_patterns = parse_list(&p.get::<String, _>("registry_patterns"));
    }
    Ok(policy)
}

/// Build the raw signal set the cascade evaluates: the agent's collected signals
/// merged with the repo's parsed remote (so corp-remote detection works even for
/// older payloads that carry only `repo`, not full `signals`).
pub fn build_raw(s: &CapturedSession) -> RawSignals {
    let mut raw = s.signals.clone().unwrap_or_default();
    if let (Some(host), Some(org)) = (s.repo.host.as_deref(), s.repo.org.as_deref()) {
        let already = raw
            .remotes
            .iter()
            .any(|x| x.host.eq_ignore_ascii_case(host) && x.org.eq_ignore_ascii_case(org));
        if !already {
            raw.remotes.push(RemoteRef {
                host: host.to_string(),
                org: org.to_string(),
            });
        }
    }
    raw
}

/// Map a coarse override `Classification` onto the provenance class string.
fn override_class_str(c: Classification) -> &'static str {
    match c {
        Classification::Work => "work",
        Classification::Personal => "personal",
        Classification::Unknown => "unclassified",
    }
}

/// Classify the session and persist a `session_provenance` row. When
/// `override_class` is set (admin per-repo work-definition), that is authoritative
/// and recorded as `resolved_by='admin_override'`. Returns the coarse class to
/// store on `captured_sessions.classification`.
pub async fn classify_and_persist(
    pool: &PgPool,
    tenant_id: &str,
    s: &CapturedSession,
    override_class: Option<Classification>,
) -> Result<CaptureClassification, sqlx::Error> {
    // Always compute + persist the REAL provenance verdict (the corroborator /
    // safety net reads session_provenance.class — that is unchanged by AI-primary).
    let (class_str, confidence, provisional, resolved_by, reasons, prov_resolved) =
        match override_class {
            Some(c) => (
                override_class_str(c).to_string(),
                1.0f32,
                false,
                "admin_override".to_string(),
                String::new(),
                "admin_override".to_string(),
            ),
            None => {
                let policy = load_policy(pool, tenant_id).await?;
                let raw = build_raw(s);
                let v: Verdict = provenance::classify_raw(&raw, &policy);
                (
                    v.class.as_str().to_string(),
                    v.confidence,
                    v.provisional,
                    v.resolved_by.to_string(),
                    v.reasons.join("; "),
                    v.resolved_by.to_string(),
                )
            }
        };

    sqlx::query(
        "insert into session_provenance \
         (tenant_id, session_id, class, confidence, provisional, resolved_by, reasons, updated_at) \
         values ($1,$2,$3,$4,$5,$6,$7, now()) \
         on conflict (tenant_id, session_id) do update set \
           class = excluded.class, confidence = excluded.confidence, \
           provisional = excluded.provisional, resolved_by = excluded.resolved_by, \
           reasons = excluded.reasons, updated_at = now()",
    )
    .bind(tenant_id)
    .bind(&s.session_id)
    .bind(&class_str)
    .bind(confidence)
    .bind(provisional)
    .bind(&resolved_by)
    .bind(&reasons)
    .execute(pool)
    .await?;

    // AI-PRIMARY INVERSION: structural signals no longer own the dashboard label.
    // captured_sessions.classification is set to:
    //   - the admin override, if present (authoritative); or
    //   - 'work' as a FREE STRONG-WORK SHORTCUT only when a Tier-G ground-truth
    //     signal fired (real corp push / signed corp identity) — never structural
    //     'personal', never a corroborator-only 'work_provisional'; or
    //   - 'pending' — the AI judge now owns it (the agent's local Claude Code, or
    //     the opt-in server-API sweep, will resolve it).
    Ok(match override_class {
        Some(c) => CaptureClassification {
            stored: override_class_str(c).to_string(),
            coarse: c,
            shortcut: true,
        },
        None if prov_resolved == "tier_g" => CaptureClassification {
            stored: "work".to_string(),
            coarse: Classification::Work,
            shortcut: true,
        },
        None => CaptureClassification {
            stored: "pending".to_string(),
            coarse: Classification::Unknown,
            shortcut: false,
        },
    })
}

/// The classification decision for one captured session.
pub struct CaptureClassification {
    /// Value to store in `captured_sessions.classification`
    /// (`work` | `personal` | `unknown` | `pending`).
    pub stored: String,
    /// Coarse class for downstream on-task scoring (`pending` → `Unknown`).
    pub coarse: Classification,
    /// True when an override or a strong-work shortcut decided it (no AI call needed).
    pub shortcut: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccguard_core::event::Repo;

    fn session_with(repo: Repo, signals: Option<RawSignals>) -> CapturedSession {
        CapturedSession {
            session_id: "s".into(),
            user_email: "d@acme.com".into(),
            repo,
            title: None,
            cwd: None,
            signals,
            events: vec![],
        }
    }

    #[test]
    fn parse_list_handles_commas_newlines_semicolons() {
        let v = parse_list("acme.com,  eng.acme.com\n  foo.com ; bar.com");
        assert_eq!(v, vec!["acme.com", "eng.acme.com", "foo.com", "bar.com"]);
        assert!(parse_list("   ").is_empty());
    }

    #[test]
    fn build_raw_injects_repo_remote_when_signals_absent() {
        let repo = Repo {
            host: Some("github.com".into()),
            org: Some("acme-corp".into()),
            name: Some("r".into()),
            path: None,
            classification: None,
            confidence: 0.0,
        };
        let raw = build_raw(&session_with(repo, None));
        assert_eq!(raw.remotes.len(), 1);
        assert_eq!(raw.remotes[0].org, "acme-corp");
    }

    #[test]
    fn build_raw_does_not_duplicate_existing_remote() {
        let repo = Repo {
            host: Some("github.com".into()),
            org: Some("acme-corp".into()),
            name: Some("r".into()),
            path: None,
            classification: None,
            confidence: 0.0,
        };
        let signals = RawSignals {
            remotes: vec![RemoteRef { host: "github.com".into(), org: "acme-corp".into() }],
            pushed: true,
            ..Default::default()
        };
        let raw = build_raw(&session_with(repo, Some(signals)));
        assert_eq!(raw.remotes.len(), 1);
        assert!(raw.pushed);
    }
}
