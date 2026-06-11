//! Pure, testable splitter that breaks a large `CapturedSession` into byte-budgeted
//! chunks so each POST to `/v1/capture` stays well under the server's body limit.
//!
//! Each chunk carries identical session metadata (session_id, user_email, repo, title, cwd)
//! and a disjoint, ordered subset of events. The server inserts events idempotently
//! (`on conflict (tenant_id, session_id, seq) do nothing`) and recomputes aggregates from
//! all stored rows, so chunked posts accumulate into one correct session.

use ccguard_core::capture::{CapturedEvent, CapturedSession};

/// Target content-bytes per POST. 3 MiB keeps each request far under the server's 64 MiB cap.
pub const CHUNK_CONTENT_BUDGET: usize = 3 * 1024 * 1024;

/// Split a session's events into byte-budgeted chunks (same metadata, disjoint ordered seqs).
/// Always emits at least one event per chunk (a single oversized event goes alone), and emits
/// at least one (metadata-only) chunk even for an empty session.
pub fn chunk_session(s: &CapturedSession, budget: usize) -> Vec<CapturedSession> {
    let mut chunks: Vec<CapturedSession> = Vec::new();
    let mut cur: Vec<CapturedEvent> = Vec::new();
    let mut cur_bytes = 0usize;

    for e in &s.events {
        let sz = e.content.as_ref().map(|c| c.len()).unwrap_or(0);
        // Start a new chunk when adding this event would overflow the budget,
        // but never split into an empty chunk (a lone oversized event goes alone).
        if !cur.is_empty() && cur_bytes + sz > budget {
            chunks.push(meta_with(s, std::mem::take(&mut cur)));
            cur_bytes = 0;
        }
        cur.push(e.clone());
        cur_bytes += sz;
    }
    if !cur.is_empty() {
        chunks.push(meta_with(s, cur));
    }
    if chunks.is_empty() {
        // Empty session → one metadata-only post (keeps the session row in sync).
        chunks.push(meta_with(s, Vec::new()));
    }
    chunks
}

/// Build a CapturedSession that copies `s`'s metadata but carries the given events.
fn meta_with(s: &CapturedSession, events: Vec<CapturedEvent>) -> CapturedSession {
    CapturedSession {
        session_id: s.session_id.clone(),
        user_email: s.user_email.clone(),
        repo: s.repo.clone(),
        title: s.title.clone(),
        cwd: s.cwd.clone(),
        signals: s.signals.clone(),
        events,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccguard_core::capture::EventKind;
    use ccguard_core::event::Repo;
    use chrono::Utc;

    fn ev(seq: i64, content_len: usize) -> CapturedEvent {
        CapturedEvent {
            seq,
            ts: Utc::now(),
            kind: EventKind::AssistantText,
            model: None,
            tool_name: None,
            target: None,
            content: Some("x".repeat(content_len)),
            tokens_in: 0,
            tokens_out: 0,
            is_sidechain: false,
        }
    }

    fn session(events: Vec<CapturedEvent>) -> CapturedSession {
        CapturedSession {
            session_id: "sess-1".to_string(),
            user_email: "dev@acme.com".to_string(),
            repo: Repo {
                host: Some("github.com".to_string()),
                org: Some("acme".to_string()),
                name: Some("r".to_string()),
                path: Some("C:\\w".to_string()),
                classification: None,
                confidence: 0.0,
            },
            title: Some("the title".to_string()),
            cwd: Some("C:\\w".to_string()),
            signals: None,
            events,
        }
    }

    #[test]
    fn coverage_no_drop_no_dup_order_preserved() {
        // 5 events of 100 bytes each, budget 250 → multiple chunks.
        let s = session((0..5).map(|i| ev(i, 100)).collect());
        let chunks = chunk_session(&s, 250);
        assert!(chunks.len() > 1, "tiny budget should produce multiple chunks");

        // Concatenated events equal the original, in order, no drops/dups.
        let seqs: Vec<i64> = chunks
            .iter()
            .flat_map(|c| c.events.iter().map(|e| e.seq))
            .collect();
        assert_eq!(seqs, vec![0, 1, 2, 3, 4]);

        // Each chunk respects the budget where possible (chunks with >1 event are under budget).
        for c in &chunks {
            let bytes: usize = c
                .events
                .iter()
                .map(|e| e.content.as_ref().map(|s| s.len()).unwrap_or(0))
                .sum();
            if c.events.len() > 1 {
                assert!(bytes <= 250, "multi-event chunk over budget: {bytes}");
            }
        }
    }

    #[test]
    fn single_oversized_event_goes_alone() {
        // One event larger than the budget → exactly one chunk containing just that event.
        let s = session(vec![ev(0, 5000)]);
        let chunks = chunk_session(&s, 1000);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].events.len(), 1);
        assert_eq!(chunks[0].events[0].seq, 0);
    }

    #[test]
    fn oversized_event_amid_small_ones_is_isolated() {
        // small, HUGE, small with a small budget → the huge one ends up alone, all seqs preserved.
        let s = session(vec![ev(0, 10), ev(1, 5000), ev(2, 10)]);
        let chunks = chunk_session(&s, 1000);
        let seqs: Vec<i64> = chunks
            .iter()
            .flat_map(|c| c.events.iter().map(|e| e.seq))
            .collect();
        assert_eq!(seqs, vec![0, 1, 2]);
        // The 5000-byte event is in a chunk by itself.
        let huge_chunk = chunks
            .iter()
            .find(|c| c.events.iter().any(|e| e.seq == 1))
            .unwrap();
        assert_eq!(huge_chunk.events.len(), 1);
    }

    #[test]
    fn metadata_preserved_per_chunk() {
        let s = session((0..4).map(|i| ev(i, 100)).collect());
        let chunks = chunk_session(&s, 150);
        assert!(chunks.len() > 1);
        for c in &chunks {
            assert_eq!(c.session_id, "sess-1");
            assert_eq!(c.user_email, "dev@acme.com");
            assert_eq!(c.title.as_deref(), Some("the title"));
            assert_eq!(c.cwd.as_deref(), Some("C:\\w"));
            assert_eq!(c.repo.name.as_deref(), Some("r"));
        }
    }

    #[test]
    fn empty_session_yields_one_metadata_only_chunk() {
        let s = session(Vec::new());
        let chunks = chunk_session(&s, CHUNK_CONTENT_BUDGET);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].events.is_empty());
        assert_eq!(chunks[0].session_id, "sess-1");
    }
}
