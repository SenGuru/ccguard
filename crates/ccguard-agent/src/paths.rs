use std::path::{Path, PathBuf};

/// List transcript files: `<claude_dir>/projects/<encoded-cwd>/<session>.jsonl`.
/// (Subagent transcripts under `<session>/subagents/` are skipped in v1.)
pub fn list_transcripts(claude_dir: &Path) -> Vec<PathBuf> {
    let projects = claude_dir.join("projects");
    let mut out = Vec::new();
    if let Ok(dirs) = std::fs::read_dir(&projects) {
        for d in dirs.flatten() {
            let p = d.path();
            if p.is_dir() {
                if let Ok(files) = std::fs::read_dir(&p) {
                    for f in files.flatten() {
                        let fp = f.path();
                        if fp.extension().map(|e| e == "jsonl").unwrap_or(false) {
                            out.push(fp);
                        }
                    }
                }
            }
        }
    }
    out.sort();
    out
}

/// Default Codex home (`$CODEX_HOME` or `~/.codex`).
pub fn codex_home() -> Option<PathBuf> {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".codex")))
}

/// List Codex rollout transcripts: `<codex_home>/sessions/YYYY/MM/DD/rollout-*.jsonl`.
/// Walks the date-partitioned tree. (`.jsonl.zst` cold files are skipped in v1 —
/// the active/recent sessions we care about are uncompressed.)
pub fn list_codex_sessions(codex_home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let root = codex_home.join("sessions");
    // sessions/YYYY/MM/DD/rollout-*.jsonl — three levels of date dirs.
    fn walk(dir: &Path, depth: u8, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() && depth < 3 {
                walk(&p, depth + 1, out);
            } else if depth == 3
                && p.extension().map(|x| x == "jsonl").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("rollout-"))
                    .unwrap_or(false)
            {
                out.push(p);
            }
        }
    }
    walk(&root, 0, &mut out);
    out.sort();
    out
}

/// Default Copilot CLI home (`$COPILOT_HOME` or `~/.copilot`).
pub fn copilot_home() -> Option<PathBuf> {
    std::env::var_os("COPILOT_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".copilot")))
}

/// List Copilot CLI transcripts: `<copilot_home>/session-state/<id>/events.jsonl`.
/// Returns the `events.jsonl` files; the session id is the parent directory name.
pub fn list_copilot_sessions(copilot_home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let root = copilot_home.join("session-state");
    if let Ok(dirs) = std::fs::read_dir(&root) {
        for d in dirs.flatten() {
            let p = d.path();
            if p.is_dir() {
                let f = p.join("events.jsonl");
                if f.is_file() {
                    out.push(f);
                }
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_jsonl_transcripts_only() {
        let tmp = std::env::temp_dir().join(format!("ccg_paths_{}", std::process::id()));
        let proj = tmp.join("projects").join("C--Users-x-repo");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join("s1.jsonl"), "{}").unwrap();
        std::fs::write(proj.join("notes.txt"), "ignore").unwrap();

        let found = list_transcripts(&tmp);
        std::fs::remove_dir_all(&tmp).ok();

        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("s1.jsonl"));
    }
}
