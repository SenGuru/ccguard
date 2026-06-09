mod event;
mod parse;
mod paths;
mod poster;
mod pricing;
mod repo;
mod state;

use std::path::{Path, PathBuf};

use clap::Parser;

use crate::event::interaction_to_event;
use crate::parse::parse_transcript;
use crate::poster::Poster;
use crate::state::State;

/// CCGuard endpoint agent — VISIBLE monitoring of this machine's Claude Code usage.
/// Sends metadata only (model, token counts, repo, timing) — never prompt or code content.
#[derive(Parser, Debug)]
#[command(name = "ccguard-agent")]
struct Args {
    /// CCGuard server base URL, e.g. http://localhost:8080
    #[arg(long)]
    server: String,
    /// Ingest token (ccg_...) for this tenant
    #[arg(long)]
    token: String,
    /// Claude config dir (default: ~/.claude)
    #[arg(long)]
    claude_dir: Option<String>,
    /// Override the reported user email (default: read from ~/.claude.json)
    #[arg(long)]
    email: Option<String>,
}

fn read_since(path: &Path, offset: u64) -> std::io::Result<(String, u64)> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    if offset >= len {
        return Ok((String::new(), len));
    }
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    Ok((buf, len))
}

fn read_claude_email(claude_dir: &Path) -> Option<String> {
    let json_path = claude_dir.parent()?.join(".claude.json");
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(json_path).ok()?).ok()?;
    v.get("oauthAccount")?
        .get("emailAddress")?
        .as_str()
        .map(|s| s.to_string())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let claude_dir = args
        .claude_dir
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".claude")))
        .expect("could not determine claude dir");

    let email = args
        .email
        .or_else(|| read_claude_email(&claude_dir))
        .unwrap_or_else(|| "unknown@local".to_string());

    println!(
        "CCGuard agent — VISIBLE Claude Code usage monitoring (metadata only).\n  server: {}\n  claude dir: {}\n  user: {}",
        args.server,
        claude_dir.display(),
        email
    );

    let state_path = claude_dir.join("ccguard-agent-state.json");
    let mut st = State::load(&state_path);
    let poster = Poster::new(&args.server, &args.token);

    let mut sent = 0usize;
    let mut skipped = 0usize;
    for file in paths::list_transcripts(&claude_dir) {
        let key = file.to_string_lossy().to_string();
        let (chunk, new_off) = match read_since(&file, st.offset(&key)) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if chunk.is_empty() {
            continue;
        }
        for interaction in parse_transcript(&chunk, None) {
            match interaction_to_event(&interaction, &email) {
                Some(ev) => match poster.post(&ev) {
                    Ok(202) => sent += 1,
                    Ok(code) => {
                        eprintln!("  POST returned HTTP {code}");
                        skipped += 1;
                    }
                    Err(e) => {
                        eprintln!("  POST error: {e}");
                        skipped += 1;
                    }
                },
                None => skipped += 1,
            }
        }
        st.set(&key, new_off);
    }
    st.save(&state_path)?;
    println!("CCGuard agent: sent {sent} event(s), skipped {skipped}.");
    Ok(())
}
