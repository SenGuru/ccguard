//! Provenance Ledger — the deterministic, content-free classification cascade.
//!
//! Trust tiers, NOT cost tiers (per the 2026-06-11 design deliberation):
//!
//! - **Tier-G (ground truth)** — the ONLY signals that auto-resolve to WORK 0.95:
//!   a real push to a corporate-org remote (`W-PUSH`), or a *cryptographically
//!   signed* commit whose committer identity is in the corporate directory
//!   (`W-IDP-EMAIL`). The destination / signed identity is out-of-band-verifiable.
//! - **Tier-C (corroborators)** — content-free structural signals that are
//!   dev-mutable / spoofable (`git config` email, registry fingerprints, monorepo
//!   walk-up, tenant ticket regex, MDM env var, *unsigned* corp author email).
//!   They mark WORK-PROVISIONAL but NEVER auto-resolve alone.
//! - **UNCLASSIFIED** — a first-class terminal state, never a fallback to personal.
//! - **PERSONAL** — only ever reached by an affirmative personal signal confirmed
//!   by TWO independent (non-derivable) signals. New/separate/remote-less work is
//!   therefore never silently flagged personal *by construction*.
//!
//! This module is pure (no I/O). The agent collects [`RawSignals`] (content-free
//! facts); the server turns them into [`Signals`] via [`evaluate`] against a
//! tenant [`ProvenancePolicy`], then runs [`classify_provenance`]. Both steps are
//! unit-tested here, including the regression invariants the design demands:
//! an unsigned `--author` corp-email spoof must NOT auto-resolve to WORK, and a
//! PERSONAL verdict must NOT be buildable from correlated signals.

use serde::{Deserialize, Serialize};

use crate::event::Classification;

/// A remote's parsed identity (host + org), content-free.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RemoteRef {
    pub host: String,
    pub org: String,
}

/// Content-free structural facts the agent collects from the working tree.
/// NONE of these read prompt/code content — only git metadata + manifest config.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RawSignals {
    /// Parsed identities of all configured remotes.
    #[serde(default)]
    pub remotes: Vec<RemoteRef>,
    /// The current branch has an upstream (≈ has been pushed).
    #[serde(default)]
    pub pushed: bool,
    /// Committer email on HEAD (`%ce`).
    #[serde(default)]
    pub committer_email: Option<String>,
    /// HEAD commit carries a good signature (`%G?` ∈ {G,U}).
    #[serde(default)]
    pub commit_signed: bool,
    /// Resolved `git config user.email` (honors includeIf via git itself).
    #[serde(default)]
    pub config_email: Option<String>,
    /// A corporate MDM-injected env var (the policy `corp_env_name`) was present.
    #[serde(default)]
    pub env_corp_marker: bool,
    /// Private-registry fingerprints found in walked-up manifests (npm @scope,
    /// `registry=` host, GOPRIVATE, internal Cargo index, Artifactory/CodeArtifact).
    #[serde(default)]
    pub registry_fingerprints: Vec<String>,
    /// True when `cwd` is inside a repo but not at its root (monorepo leaf).
    #[serde(default)]
    pub monorepo_leaf: bool,
    /// The inherited (root) remote when `monorepo_leaf` — the workspace-root remote.
    #[serde(default)]
    pub monorepo_root: Option<RemoteRef>,
    /// Current branch name (for tenant ticket-prefix matching).
    #[serde(default)]
    pub branch: Option<String>,
}

/// Tenant policy that turns raw facts into trust signals. All matching is
/// case-insensitive. Lists are tenant-configured (no broad built-in regexes).
#[derive(Debug, Clone, Default)]
pub struct ProvenancePolicy {
    pub corp_hosts: Vec<String>,           // approved git hosts (reuse allowlist)
    pub corp_orgs: Vec<String>,            // approved orgs/owners (reuse allowlist)
    pub corp_email_domains: Vec<String>,   // e.g. "acme.com"
    pub personal_orgs: Vec<String>,        // explicitly-flagged personal destinations
    pub personal_email_domains: Vec<String>, // e.g. "gmail.com"
    pub ticket_prefixes: Vec<String>,      // exact JIRA-key prefixes, e.g. "ACME", "BILL"
    pub registry_patterns: Vec<String>,    // substrings marking a corp registry/scope
}

/// Evaluated trust signals — the booleans the cascade reasons over.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Signals {
    // Tier-G (ground truth, auto-resolve)
    pub w_push: bool,
    pub w_idp_email_signed: bool,
    // Tier-C (corroborators, never auto-resolve)
    pub c_corp_remote: bool,        // corp remote configured but not (yet) pushed
    pub c_unsigned_corp_email: bool,// committer corp email on an UNSIGNED commit (demoted from G)
    pub c_email_cfg: bool,          // git config user.email is corp domain
    pub c_mdm_env: bool,
    pub c_registry: bool,
    pub c_monorepo: bool,
    pub c_ticket: bool,
    // Affirmative personal (each a DISTINCT independence class)
    pub p_remote: bool,             // remote/destination on the personal denylist
    pub p_email_signed: bool,       // signed commit by a known personal email
}

/// The cascade's verdict class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceClass {
    Work,
    WorkProvisional,
    Unclassified,
    Personal,
}

impl ProvenanceClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProvenanceClass::Work => "work",
            ProvenanceClass::WorkProvisional => "work_provisional",
            ProvenanceClass::Unclassified => "unclassified",
            ProvenanceClass::Personal => "personal",
        }
    }

    /// Collapse to the coarse `Classification` stored on the session
    /// (`work_provisional` is still treated as work everywhere).
    pub fn to_classification(&self) -> Classification {
        match self {
            ProvenanceClass::Work | ProvenanceClass::WorkProvisional => Classification::Work,
            ProvenanceClass::Personal => Classification::Personal,
            ProvenanceClass::Unclassified => Classification::Unknown,
        }
    }
}

/// A provenance verdict with its rationale.
#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    pub class: ProvenanceClass,
    pub confidence: f32,
    pub provisional: bool,
    /// How it resolved: `tier_g` | `personal_confirmed` | `corroborator` | `unclassified`.
    pub resolved_by: &'static str,
    /// Signal codes that fired (e.g. `W-PUSH`, `C-MDM-ENV`), plus notes.
    pub reasons: Vec<String>,
}

fn domain_of(email: &str) -> Option<String> {
    email.rsplit_once('@').map(|(_, d)| d.trim().to_ascii_lowercase())
}

/// True if `domain` equals or is a subdomain of any entry in `domains`.
fn domain_matches(domain: &str, domains: &[String]) -> bool {
    domains.iter().any(|d| {
        let d = d.trim().to_ascii_lowercase();
        !d.is_empty() && (domain == d || domain.ends_with(&format!(".{d}")))
    })
}

fn ci_contains(haystack: &str, needles: &[String]) -> bool {
    let h = haystack.to_ascii_lowercase();
    needles
        .iter()
        .any(|n| !n.trim().is_empty() && h.contains(&n.trim().to_ascii_lowercase()))
}

fn list_has(list: &[String], value: &str) -> bool {
    list.iter().any(|x| x.eq_ignore_ascii_case(value))
}

/// Is this remote a corporate remote (host AND org both on the corp allowlist)?
fn remote_is_corp(r: &RemoteRef, policy: &ProvenancePolicy) -> bool {
    list_has(&policy.corp_hosts, &r.host) && list_has(&policy.corp_orgs, &r.org)
}

/// Turn raw collected facts into evaluated trust signals against tenant policy.
/// Pure and fully testable.
pub fn evaluate(raw: &RawSignals, policy: &ProvenancePolicy) -> Signals {
    let mut s = Signals::default();

    let corp_remote_present = raw.remotes.iter().any(|r| remote_is_corp(r, policy));
    if corp_remote_present {
        if raw.pushed {
            s.w_push = true;
        } else {
            s.c_corp_remote = true;
        }
    }

    // Committer email vs corp / personal directories, gated on signature.
    if let Some(dom) = raw.committer_email.as_deref().and_then(domain_of) {
        let corp = domain_matches(&dom, &policy.corp_email_domains);
        let personal = domain_matches(&dom, &policy.personal_email_domains);
        if corp {
            if raw.commit_signed {
                s.w_idp_email_signed = true; // Tier-G: signed + corp identity
            } else {
                s.c_unsigned_corp_email = true; // demoted: an unsigned --author is spoofable
            }
        }
        if personal && raw.commit_signed {
            s.p_email_signed = true; // affirmative personal, identity class
        }
    }

    // git config user.email (dev-mutable → corroborator only)
    if let Some(dom) = raw.config_email.as_deref().and_then(domain_of) {
        if domain_matches(&dom, &policy.corp_email_domains) {
            s.c_email_cfg = true;
        }
    }

    s.c_mdm_env = raw.env_corp_marker;

    if raw.registry_fingerprints.iter().any(|f| {
        ci_contains(f, &policy.registry_patterns) || ci_contains(f, &policy.corp_orgs)
    }) {
        s.c_registry = true;
    }

    if raw.monorepo_leaf {
        if let Some(root) = &raw.monorepo_root {
            if remote_is_corp(root, policy) {
                s.c_monorepo = true;
            }
        }
    }

    if let Some(branch) = raw.branch.as_deref() {
        if branch_has_ticket(branch, &policy.ticket_prefixes) {
            s.c_ticket = true;
        }
    }

    // Affirmative personal: a remote whose org is on the personal denylist
    // (destination class — independent from the identity class above).
    if raw.remotes.iter().any(|r| list_has(&policy.personal_orgs, &r.org)) {
        s.p_remote = true;
    }

    s
}

/// `BRANCH` matches a tenant ticket prefix if it contains `<PREFIX>-<digits>`.
fn branch_has_ticket(branch: &str, prefixes: &[String]) -> bool {
    let b = branch.to_ascii_uppercase();
    prefixes.iter().any(|p| {
        let p = p.trim().to_ascii_uppercase();
        if p.is_empty() {
            return false;
        }
        // look for "<P>-<digit>"
        let needle = format!("{p}-");
        b.match_indices(&needle).any(|(i, _)| {
            b[i + needle.len()..]
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        })
    })
}

/// Run the cascade over evaluated signals. The ONLY auto-resolve to WORK is a
/// Tier-G signal; PERSONAL needs two independent affirmative personal signals.
pub fn classify_provenance(s: &Signals) -> Verdict {
    let mut reasons: Vec<String> = Vec::new();

    let g_signals = [("W-PUSH", s.w_push), ("W-IDP-EMAIL", s.w_idp_email_signed)];
    let c_signals = [
        ("C-CORP-REMOTE", s.c_corp_remote),
        ("C-UNSIGNED-CORP-EMAIL", s.c_unsigned_corp_email),
        ("C-EMAIL-CFG", s.c_email_cfg),
        ("C-MDM-ENV", s.c_mdm_env),
        ("C-REGISTRY", s.c_registry),
        ("C-MONOREPO", s.c_monorepo),
        ("C-TICKET", s.c_ticket),
    ];
    // Two DISTINCT independence classes — destination vs identity. A test asserts
    // a PERSONAL verdict cannot be built from a single class.
    let p_signals = [("P-REMOTE", s.p_remote), ("P-EMAIL-SIGNED", s.p_email_signed)];

    let g_fired: Vec<&str> = g_signals.iter().filter(|(_, b)| *b).map(|(c, _)| *c).collect();
    let c_fired: Vec<&str> = c_signals.iter().filter(|(_, b)| *b).map(|(c, _)| *c).collect();
    let p_count = p_signals.iter().filter(|(_, b)| *b).count();
    let p_fired: Vec<&str> = p_signals.iter().filter(|(_, b)| *b).map(|(c, _)| *c).collect();

    // Tier-G wins (ground truth). Note any contradicting personal signal for review.
    if !g_fired.is_empty() {
        for c in &g_fired {
            reasons.push((*c).to_string());
        }
        if p_count > 0 {
            reasons.push(format!("contested-by:{}", p_fired.join("+")));
        }
        return Verdict {
            class: ProvenanceClass::Work,
            confidence: 0.95,
            provisional: false,
            resolved_by: "tier_g",
            reasons,
        };
    }

    // PERSONAL: affirmative + two independent confirmations (both distinct classes).
    if p_count >= 2 {
        for c in &p_fired {
            reasons.push((*c).to_string());
        }
        return Verdict {
            class: ProvenanceClass::Personal,
            confidence: 0.9,
            provisional: false,
            resolved_by: "personal_confirmed",
            reasons,
        };
    }

    // Corroborators → WORK-PROVISIONAL (treated as work, flagged provisional).
    if !c_fired.is_empty() {
        for c in &c_fired {
            reasons.push((*c).to_string());
        }
        // A lone personal signal alongside corroborators is noted but does not flip
        // the verdict (it needs two independent confirmations to reach PERSONAL).
        if p_count == 1 {
            reasons.push(format!("personal-hint:{}", p_fired.join("")));
        }
        return Verdict {
            class: ProvenanceClass::WorkProvisional,
            confidence: 0.6,
            provisional: true,
            resolved_by: "corroborator",
            reasons,
        };
    }

    // Nothing decisive (incl. a lone affirmative personal signal) → terminal-safe
    // UNCLASSIFIED, never personal.
    if p_count == 1 {
        reasons.push(format!("personal-hint:{}", p_fired.join("")));
    }
    Verdict {
        class: ProvenanceClass::Unclassified,
        confidence: 0.0,
        provisional: false,
        resolved_by: "unclassified",
        reasons,
    }
}

/// Convenience: evaluate raw facts against policy and classify in one step.
pub fn classify_raw(raw: &RawSignals, policy: &ProvenancePolicy) -> Verdict {
    classify_provenance(&evaluate(raw, policy))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> ProvenancePolicy {
        ProvenancePolicy {
            corp_hosts: vec!["github.com".into()],
            corp_orgs: vec!["acme-corp".into()],
            corp_email_domains: vec!["acme.com".into()],
            personal_orgs: vec!["my-personal".into()],
            personal_email_domains: vec!["gmail.com".into()],
            ticket_prefixes: vec!["ACME".into(), "BILL".into()],
            registry_patterns: vec!["artifactory.acme.com".into(), "@acme".into()],
        }
    }
    fn corp_remote() -> RemoteRef {
        RemoteRef { host: "github.com".into(), org: "acme-corp".into() }
    }

    // ---- cascade (on evaluated Signals) ----

    #[test]
    fn tier_g_push_auto_resolves_work_095() {
        let v = classify_provenance(&Signals { w_push: true, ..Default::default() });
        assert_eq!(v.class, ProvenanceClass::Work);
        assert_eq!(v.confidence, 0.95);
        assert!(!v.provisional);
        assert_eq!(v.resolved_by, "tier_g");
    }

    #[test]
    fn signed_idp_email_alone_is_work() {
        let v = classify_provenance(&Signals { w_idp_email_signed: true, ..Default::default() });
        assert_eq!(v.class, ProvenanceClass::Work);
        assert_eq!(v.confidence, 0.95);
    }

    #[test]
    fn single_corroborator_is_provisional_not_work() {
        let v = classify_provenance(&Signals { c_mdm_env: true, ..Default::default() });
        assert_eq!(v.class, ProvenanceClass::WorkProvisional);
        assert!(v.provisional);
        assert!(v.confidence < 0.95);
    }

    #[test]
    fn two_corroborators_still_only_provisional() {
        // A pure sum of Tier-C cannot reach auto-WORK (kills the walk-up + stale-env spoof).
        let v = classify_provenance(&Signals {
            c_mdm_env: true,
            c_corp_remote: true,
            ..Default::default()
        });
        assert_eq!(v.class, ProvenanceClass::WorkProvisional);
    }

    #[test]
    fn no_signals_is_unclassified_never_personal() {
        let v = classify_provenance(&Signals::default());
        assert_eq!(v.class, ProvenanceClass::Unclassified);
        assert_eq!(v.confidence, 0.0);
    }

    #[test]
    fn lone_affirmative_personal_signal_is_unclassified_not_personal() {
        let v = classify_provenance(&Signals { p_remote: true, ..Default::default() });
        assert_eq!(v.class, ProvenanceClass::Unclassified);
        assert!(v.reasons.iter().any(|r| r.contains("personal-hint")));
    }

    #[test]
    fn two_independent_personal_signals_reach_personal() {
        let v = classify_provenance(&Signals {
            p_remote: true,
            p_email_signed: true,
            ..Default::default()
        });
        assert_eq!(v.class, ProvenanceClass::Personal);
        assert_eq!(v.resolved_by, "personal_confirmed");
    }

    #[test]
    fn personal_signal_with_corroborator_does_not_flip_to_personal() {
        let v = classify_provenance(&Signals {
            p_remote: true,
            c_mdm_env: true,
            ..Default::default()
        });
        assert_eq!(v.class, ProvenanceClass::WorkProvisional);
    }

    // ---- REGRESSION: unsigned --author spoof must NOT auto-resolve to WORK ----

    #[test]
    fn unsigned_author_corp_email_spoof_is_not_auto_work() {
        // `git commit --author="dev@acme.com"` on an UNSIGNED commit:
        let raw = RawSignals {
            committer_email: Some("dev@acme.com".into()),
            commit_signed: false,
            ..Default::default()
        };
        let v = classify_raw(&raw, &policy());
        assert_ne!(v.class, ProvenanceClass::Work, "unsigned corp email must not be Tier-G");
        assert_eq!(v.class, ProvenanceClass::WorkProvisional); // demoted to corroborator
        assert!(v.reasons.iter().any(|r| r == "C-UNSIGNED-CORP-EMAIL"));
    }

    #[test]
    fn signed_commit_with_corp_email_is_tier_g() {
        let raw = RawSignals {
            committer_email: Some("dev@acme.com".into()),
            commit_signed: true,
            ..Default::default()
        };
        let v = classify_raw(&raw, &policy());
        assert_eq!(v.class, ProvenanceClass::Work);
        assert_eq!(v.confidence, 0.95);
    }

    // ---- REGRESSION: PERSONAL cannot be built from correlated signals ----

    #[test]
    fn no_remote_and_non_corp_email_do_not_make_personal() {
        // "no remote" + "non-corp git email" are NOT independent (both derive from
        // working outside the corp checkout) and are not even affirmative-personal.
        let raw = RawSignals {
            remotes: vec![],
            config_email: Some("dev@somewhere-else.io".into()),
            ..Default::default()
        };
        let v = classify_raw(&raw, &policy());
        assert_eq!(v.class, ProvenanceClass::Unclassified);
    }

    // ---- the hard edge case: new module, separate dir, no remote ----

    #[test]
    fn new_module_mdm_env_pre_git_is_work_provisional() {
        let raw = RawSignals { env_corp_marker: true, ..Default::default() };
        let v = classify_raw(&raw, &policy());
        assert_eq!(v.class, ProvenanceClass::WorkProvisional);
        assert_eq!(v.class.to_classification(), Classification::Work);
    }

    #[test]
    fn truly_bare_new_dir_is_unclassified_not_personal() {
        let v = classify_raw(&RawSignals::default(), &policy());
        assert_eq!(v.class, ProvenanceClass::Unclassified);
        assert_eq!(v.class.to_classification(), Classification::Unknown);
    }

    // ---- evaluate() against policy ----

    #[test]
    fn corp_remote_pushed_is_w_push_unpushed_is_corroborator() {
        let pushed = evaluate(
            &RawSignals { remotes: vec![corp_remote()], pushed: true, ..Default::default() },
            &policy(),
        );
        assert!(pushed.w_push && !pushed.c_corp_remote);
        let unpushed = evaluate(
            &RawSignals { remotes: vec![corp_remote()], pushed: false, ..Default::default() },
            &policy(),
        );
        assert!(!unpushed.w_push && unpushed.c_corp_remote);
    }

    #[test]
    fn registry_scope_and_ticket_branch_corroborate() {
        let raw = RawSignals {
            registry_fingerprints: vec!["@acme/ui".into()],
            branch: Some("feature/BILL-204-fix".into()),
            ..Default::default()
        };
        let s = evaluate(&raw, &policy());
        assert!(s.c_registry);
        assert!(s.c_ticket);
        // both corroborators, no Tier-G → provisional
        assert_eq!(classify_provenance(&s).class, ProvenanceClass::WorkProvisional);
    }

    #[test]
    fn ticket_prefix_needs_digits() {
        let p = policy();
        assert!(branch_has_ticket("ACME-12-thing", &p.ticket_prefixes));
        assert!(!branch_has_ticket("ACMETHING", &p.ticket_prefixes));
        assert!(!branch_has_ticket("ACME-x", &p.ticket_prefixes));
    }

    #[test]
    fn monorepo_leaf_inherits_corp_root_remote() {
        let raw = RawSignals {
            monorepo_leaf: true,
            monorepo_root: Some(corp_remote()),
            ..Default::default()
        };
        assert!(evaluate(&raw, &policy()).c_monorepo);
    }

    #[test]
    fn personal_org_remote_plus_signed_personal_email_is_personal() {
        let raw = RawSignals {
            remotes: vec![RemoteRef { host: "github.com".into(), org: "my-personal".into() }],
            committer_email: Some("me@gmail.com".into()),
            commit_signed: true,
            ..Default::default()
        };
        let v = classify_raw(&raw, &policy());
        assert_eq!(v.class, ProvenanceClass::Personal);
    }

    #[test]
    fn email_subdomain_matches_corp_domain() {
        let s = evaluate(
            &RawSignals { config_email: Some("dev@eng.acme.com".into()), ..Default::default() },
            &policy(),
        );
        assert!(s.c_email_cfg);
    }
}
