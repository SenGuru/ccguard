use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Per-file byte offsets already processed (so re-runs only handle new bytes).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub offsets: HashMap<String, u64>,
    /// Capture-mode high-water mark: the max event `seq` confirmed-sent (HTTP 202) per file.
    /// Separate from the legacy byte `offsets` (token mode); `serde(default)` so old state
    /// files without this field still load. Default per-file watermark is -1 (nothing sent).
    #[serde(default)]
    pub capture_seqs: HashMap<String, i64>,
    /// Triage (classification) budget bookkeeping — bounds how much of the dev's
    /// Claude Code weekly quota the agent may spend classifying. `serde(default)`.
    #[serde(default)]
    pub triage_week: String,
    #[serde(default)]
    pub triage_count: u32,
    /// Unix-epoch seconds until which the sweep is backing off (0 = none).
    #[serde(default)]
    pub triage_backoff_until: i64,
    /// `--service` mode: the calendar date (YYYY-MM-DD) the daily triage pass last
    /// ran. Lets the loop run triage once per day with catch-up if a day was missed
    /// (laptop asleep/off). Empty = never.
    #[serde(default)]
    pub triage_last_date: String,
}

impl State {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, serde_json::to_string_pretty(self).unwrap_or_default())
    }

    pub fn offset(&self, file: &str) -> u64 {
        *self.offsets.get(file).unwrap_or(&0)
    }

    pub fn set(&mut self, file: &str, off: u64) {
        self.offsets.insert(file.to_string(), off);
    }

    /// Capture-mode high-water mark for `file`: the max event seq confirmed-sent. Default -1.
    pub fn capture_watermark(&self, file: &str) -> i64 {
        *self.capture_seqs.get(file).unwrap_or(&-1)
    }

    /// Persist the capture high-water mark for `file` (only confirmed-sent seqs should be set).
    pub fn set_capture_watermark(&mut self, file: &str, seq: i64) {
        self.capture_seqs.insert(file.to_string(), seq);
    }

    /// Remaining classify budget for `week` (rolls over when the week key changes).
    pub fn weekly_remaining(&mut self, week: &str, cap: u32) -> u32 {
        if self.triage_week != week {
            self.triage_week = week.to_string();
            self.triage_count = 0;
        }
        cap.saturating_sub(self.triage_count)
    }

    /// Count one classify call against this week's budget.
    pub fn record_classify(&mut self, week: &str) {
        if self.triage_week != week {
            self.triage_week = week.to_string();
            self.triage_count = 0;
        }
        self.triage_count = self.triage_count.saturating_add(1);
    }

    /// Whether the sweep is currently backing off (e.g. after a rate-limit).
    pub fn in_backoff(&self, now_epoch: i64) -> bool {
        self.triage_backoff_until > now_epoch
    }

    pub fn set_backoff(&mut self, until_epoch: i64) {
        self.triage_backoff_until = until_epoch;
    }

    /// `--service` mode: has the daily triage pass already run on `date`
    /// (YYYY-MM-DD)? False also when a prior day was missed (→ catch-up runs).
    pub fn triage_ran_today(&self, date: &str) -> bool {
        self.triage_last_date == date
    }

    /// Record that the daily triage pass ran on `date`.
    pub fn mark_triage_date(&mut self, date: &str) {
        self.triage_last_date = date.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_and_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("ccg_state_{}.json", std::process::id()));
        let mut s = State::default();
        assert_eq!(s.offset("a"), 0);
        s.set("a", 42);
        s.save(&tmp).unwrap();

        let loaded = State::load(&tmp);
        std::fs::remove_file(&tmp).ok();
        assert_eq!(loaded.offset("a"), 42);
        assert_eq!(loaded.offset("missing"), 0);
    }

    #[test]
    fn capture_watermark_roundtrip_default_negative_one() {
        let tmp = std::env::temp_dir().join(format!("ccg_state_wm_{}.json", std::process::id()));
        let mut s = State::default();
        // Default for an unseen file is -1 (nothing confirmed-sent yet).
        assert_eq!(s.capture_watermark("f"), -1);
        s.set_capture_watermark("f", 7);
        s.save(&tmp).unwrap();

        let loaded = State::load(&tmp);
        std::fs::remove_file(&tmp).ok();
        assert_eq!(loaded.capture_watermark("f"), 7);
        assert_eq!(loaded.capture_watermark("other"), -1);
    }

    #[test]
    fn daily_triage_date_tracking_and_catchup() {
        let mut s = State::default();
        // Never run → not today, so the daily pass should run (incl. catch-up).
        assert!(!s.triage_ran_today("2026-06-13"));
        s.mark_triage_date("2026-06-13");
        assert!(s.triage_ran_today("2026-06-13"));
        // A new day → false again (today's pass is due).
        assert!(!s.triage_ran_today("2026-06-14"));
    }

    #[test]
    fn legacy_state_without_capture_seqs_still_loads() {
        // Old state files predate `capture_seqs`; serde(default) must let them load.
        let tmp =
            std::env::temp_dir().join(format!("ccg_state_legacy_{}.json", std::process::id()));
        std::fs::write(&tmp, r#"{"offsets":{"a":99}}"#).unwrap();
        let loaded = State::load(&tmp);
        std::fs::remove_file(&tmp).ok();
        assert_eq!(loaded.offset("a"), 99);
        assert_eq!(loaded.capture_watermark("a"), -1);
    }
}
