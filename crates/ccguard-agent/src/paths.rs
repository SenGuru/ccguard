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
