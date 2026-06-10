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
        let tmp =
            std::env::temp_dir().join(format!("ccg_state_wm_{}.json", std::process::id()));
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
