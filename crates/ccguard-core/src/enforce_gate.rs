//! Enforcement gate — the decision the off-device proxy makes for one request.
//!
//! Hard-block is only ever reached under a conjunction of safeguards (design §3.5),
//! and the failure modes are asymmetric on purpose:
//!
//! - **Fail-OPEN** wins over everything: if the control plane is unreachable, the
//!   proxy MUST pass the request through. A Claresso outage can never block a
//!   developer's coding tool. (Written fail-open guarantee.)
//! - **Fail-CLOSED**: if the Claude Code version is outside the tested allowlist,
//!   or the runtime hook-precedence self-test fails, enforcement auto-disables and
//!   transparency continues — because the deny-bypass bugs mean we cannot trust the
//!   block on an untested version.
//! - A block is permitted ONLY on a session a Tier-G or two-independent-signal
//!   check confirms personal — **never UNCLASSIFIED, never a single work signal,
//!   never a content-only (soft) personal label** — and only when armed, the
//!   precision GO is met, and the seat is over its allowance, and only by gating
//!   the START of a new session (never a mid-flight kill).
//!
//! Pure; no I/O. The proxy gathers the inputs and acts on the [`Decision`].

/// The provenance class of the session the request belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionClass {
    Work,
    WorkProvisional,
    Unclassified,
    /// Structurally confirmed personal (Tier-G or two independent signals).
    PersonalConfirmed,
    /// Content/LLM-only personal — gameable, NEVER enforceable.
    PersonalSoft,
}

/// Everything the gate needs to decide. The proxy fills this per request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GateInputs {
    /// The tenant has explicitly armed enforcement (and accepted the AUP).
    pub armed: bool,
    /// The build-time precision GO/NO-GO currently reads GO.
    pub precision_go: bool,
    /// Provenance class of this session.
    pub class: SessionClass,
    /// This seat is over its personal allowance for the window.
    pub seat_over_allowance: bool,
    /// True only if this is the START of a new session (never gate mid-flight).
    pub is_session_start: bool,
    /// The Claude Code version is in the tested-and-passing allowlist.
    pub cc_version_supported: bool,
    /// The runtime hook-precedence self-test passed on this client.
    pub precedence_self_test_passed: bool,
    /// The proxy could reach the Claresso control plane for this decision.
    pub control_plane_reachable: bool,
}

impl Default for GateInputs {
    fn default() -> Self {
        GateInputs {
            armed: false,
            precision_go: false,
            class: SessionClass::Unclassified,
            seat_over_allowance: false,
            is_session_start: false,
            cc_version_supported: true,
            precedence_self_test_passed: true,
            control_plane_reachable: true,
        }
    }
}

/// What the proxy should do with the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Forward unchanged (the default; covers all non-enforced traffic).
    Allow,
    /// Control plane unreachable — forwarded to never block coding.
    FailOpenAllow,
    /// Version/self-test failed — enforcement disabled, transparency continues.
    FailClosedAllow(&'static str),
    /// Gate the start of a confirmed-personal, over-allowance session with a warm,
    /// one-click-recoverable message (never a silent refusal, never mid-flight).
    BlockNewSession,
}

/// Decide. Order encodes the safety precedence: fail-open, then fail-closed, then
/// the armed/GO/eligibility conjunction.
pub fn decide(i: &GateInputs) -> Decision {
    // 1. Fail-open beats everything — an outage must never block coding.
    if !i.control_plane_reachable {
        return Decision::FailOpenAllow;
    }
    // 2. Fail-closed — can't trust the block on an untested client.
    if !i.cc_version_supported {
        return Decision::FailClosedAllow("cc_version_not_in_tested_allowlist");
    }
    if !i.precedence_self_test_passed {
        return Decision::FailClosedAllow("precedence_self_test_failed");
    }
    // 3. Not armed, or precision GO not met → never block.
    if !i.armed || !i.precision_go {
        return Decision::Allow;
    }
    // 4. Only a structurally-confirmed personal session is ever eligible.
    //    Work / WorkProvisional (single-signal) / Unclassified / PersonalSoft → Allow.
    if i.class != SessionClass::PersonalConfirmed {
        return Decision::Allow;
    }
    // 5. Only the START of a new session, only when over allowance.
    if i.is_session_start && i.seat_over_allowance {
        Decision::BlockNewSession
    } else {
        Decision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fully-armed, eligible, over-allowance, session-start happy path.
    fn armed_block() -> GateInputs {
        GateInputs {
            armed: true,
            precision_go: true,
            class: SessionClass::PersonalConfirmed,
            seat_over_allowance: true,
            is_session_start: true,
            cc_version_supported: true,
            precedence_self_test_passed: true,
            control_plane_reachable: true,
        }
    }

    #[test]
    fn happy_path_blocks_only_when_everything_aligns() {
        assert_eq!(decide(&armed_block()), Decision::BlockNewSession);
    }

    #[test]
    fn fail_open_beats_a_would_be_block() {
        let i = GateInputs { control_plane_reachable: false, ..armed_block() };
        assert_eq!(decide(&i), Decision::FailOpenAllow);
    }

    #[test]
    fn unsupported_version_fails_closed_even_when_armed() {
        let i = GateInputs { cc_version_supported: false, ..armed_block() };
        assert!(matches!(decide(&i), Decision::FailClosedAllow(_)));
    }

    #[test]
    fn failed_self_test_fails_closed() {
        let i = GateInputs { precedence_self_test_passed: false, ..armed_block() };
        assert!(matches!(decide(&i), Decision::FailClosedAllow(_)));
    }

    #[test]
    fn unclassified_is_never_blocked_even_fully_armed() {
        let i = GateInputs { class: SessionClass::Unclassified, ..armed_block() };
        assert_eq!(decide(&i), Decision::Allow);
    }

    #[test]
    fn single_work_signal_provisional_is_never_blocked() {
        let i = GateInputs { class: SessionClass::WorkProvisional, ..armed_block() };
        assert_eq!(decide(&i), Decision::Allow);
    }

    #[test]
    fn soft_llm_personal_is_never_blocked() {
        let i = GateInputs { class: SessionClass::PersonalSoft, ..armed_block() };
        assert_eq!(decide(&i), Decision::Allow);
    }

    #[test]
    fn not_armed_allows() {
        let i = GateInputs { armed: false, ..armed_block() };
        assert_eq!(decide(&i), Decision::Allow);
    }

    #[test]
    fn precision_nogo_allows() {
        let i = GateInputs { precision_go: false, ..armed_block() };
        assert_eq!(decide(&i), Decision::Allow);
    }

    #[test]
    fn mid_flight_is_never_blocked_only_session_start() {
        let i = GateInputs { is_session_start: false, ..armed_block() };
        assert_eq!(decide(&i), Decision::Allow);
    }

    #[test]
    fn under_allowance_confirmed_personal_allows() {
        let i = GateInputs { seat_over_allowance: false, ..armed_block() };
        assert_eq!(decide(&i), Decision::Allow);
    }

    #[test]
    fn default_is_allow() {
        assert_eq!(decide(&GateInputs::default()), Decision::Allow);
    }
}
