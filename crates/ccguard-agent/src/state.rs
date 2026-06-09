use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Per-file byte offsets already processed (so re-runs only handle new bytes).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub offsets: HashMap<String, u64>,
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
}
