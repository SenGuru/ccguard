//! Co-Owned Ledger — the humane personal-usage split.
//!
//! v1 is **transparency**, not enforcement. Per the 2026-06-11 deliberation, the
//! denominator is **session-count fraction**, never dollars: the JSONL token
//! fields undercount input 100–174× and output 10–17×, so a dollar meter on that
//! data fires on fiction. There is therefore **no money type in this module by
//! construction** — `split` deals only in session counts, and a `personal ≤ N% of
//! spend` figure literally cannot be expressed here.
//!
//! UNCLASSIFIED sessions are excluded from BOTH numerator and denominator — they
//! can never trip the meter. Only **confirmed** personal sessions (affirmative
//! personal + two independent signals, or a human-confirmed verdict) count toward
//! the personal share; an LLM-judged "personal" alone does not.

/// Rolling-window session counts. The ONLY units this module accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UsageCounts {
    /// Sessions classified work (incl. work-provisional).
    pub work: u32,
    /// Sessions confirmed personal (structural personal OR human-confirmed verdict).
    pub personal_confirmed: u32,
    /// Sessions in the terminal-safe UNCLASSIFIED state — excluded from the meter.
    pub unclassified: u32,
}

/// The computed split. `personal_share_pct` is over the work+personal denominator
/// (UNCLASSIFIED excluded), labeled "estimated split, not billed dollars."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsageSplit {
    pub work: u32,
    pub personal: u32,
    /// Surfaced explicitly so the meter's exclusion is visible, never silent.
    pub unclassified_excluded: u32,
    /// work + personal_confirmed (UNCLASSIFIED is not in here).
    pub denominator: u32,
    /// personal / denominator, rounded; 0 when the denominator is 0.
    pub personal_share_pct: u32,
    pub allowance_pct: u32,
    /// allowance − share (negative when over allowance).
    pub headroom_pct: i32,
    pub over_allowance: bool,
}

/// Compute the personal/work session-count split against the tenant allowance.
/// Pure; UNCLASSIFIED is excluded from the denominator by construction.
pub fn split(counts: &UsageCounts, allowance_pct: u32) -> UsageSplit {
    let denominator = counts.work + counts.personal_confirmed;
    let personal_share_pct = if denominator == 0 {
        0
    } else {
        // round half up
        ((counts.personal_confirmed as u64 * 100 + denominator as u64 / 2) / denominator as u64)
            as u32
    };
    let headroom_pct = allowance_pct as i32 - personal_share_pct as i32;
    UsageSplit {
        work: counts.work,
        personal: counts.personal_confirmed,
        unclassified_excluded: counts.unclassified,
        denominator,
        personal_share_pct,
        allowance_pct,
        headroom_pct,
        over_allowance: personal_share_pct > allowance_pct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unclassified_is_excluded_from_denominator() {
        // 8 work, 2 personal, 90 unclassified → share is 2/10 = 20%, NOT 2/100.
        let s = split(&UsageCounts { work: 8, personal_confirmed: 2, unclassified: 90 }, 20);
        assert_eq!(s.denominator, 10);
        assert_eq!(s.personal_share_pct, 20);
        assert_eq!(s.unclassified_excluded, 90);
    }

    #[test]
    fn headroom_and_over_allowance() {
        let under = split(&UsageCounts { work: 9, personal_confirmed: 1, unclassified: 0 }, 20);
        assert_eq!(under.personal_share_pct, 10);
        assert_eq!(under.headroom_pct, 10);
        assert!(!under.over_allowance);

        let over = split(&UsageCounts { work: 6, personal_confirmed: 4, unclassified: 0 }, 20);
        assert_eq!(over.personal_share_pct, 40);
        assert_eq!(over.headroom_pct, -20);
        assert!(over.over_allowance);
    }

    #[test]
    fn empty_denominator_is_zero_share_not_panic() {
        let s = split(&UsageCounts { work: 0, personal_confirmed: 0, unclassified: 5 }, 20);
        assert_eq!(s.personal_share_pct, 0);
        assert_eq!(s.denominator, 0);
        assert!(!s.over_allowance);
    }

    #[test]
    fn at_allowance_is_not_over() {
        let s = split(&UsageCounts { work: 8, personal_confirmed: 2, unclassified: 0 }, 20);
        assert_eq!(s.personal_share_pct, 20);
        assert!(!s.over_allowance); // strictly greater than triggers over
        assert_eq!(s.headroom_pct, 0);
    }

    #[test]
    fn rounds_half_up() {
        // 1 of 3 = 33.3% → 33; 1 of 8 = 12.5% → 13
        assert_eq!(split(&UsageCounts { work: 2, personal_confirmed: 1, unclassified: 0 }, 20).personal_share_pct, 33);
        assert_eq!(split(&UsageCounts { work: 7, personal_confirmed: 1, unclassified: 0 }, 20).personal_share_pct, 13);
    }
}
