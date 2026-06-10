mod event;
mod parse;
mod paths;
mod poster;
mod pricing;
mod repo;
mod state;
mod transcript;

use std::path::{Path, PathBuf};

use clap::Parser;

use crate::event::interaction_to_event;
use crate::parse::parse_transcript;
use crate::poster::Poster;
use crate::state::State;

/// CCGuard endpoint agent — VISIBLE monitoring of this machine's Claude Code usage.
/// Sends metadata only (model, token counts, repo, timing) — never prompt or code content.
/// Use --capture to send full session transcripts (prompts, responses, tool calls, file edits).
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
    /// Full-capture mode: parse complete transcripts (prompts, responses, tool calls, file edits)
    /// and post CapturedSessions to /v1/capture instead of token-only events to /v1/events.
    #[arg(long)]
    capture: bool,
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
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(json_path).ok()?).ok()?;
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
        "CCGuard agent — VISIBLE Claude Code usage monitoring{}.\n  server: {}\n  claude dir: {}\n  user: {}",
        if args.capture { " (full-capture mode)" } else { " (metadata only)" },
        args.server,
        claude_dir.display(),
        email
    );

    let state_path = claude_dir.join("ccguard-agent-state.json");
    let mut st = State::load(&state_path);
    let poster = Poster::new(&args.server, &args.token);
    let mut repos = repo::RepoCache::new();

    if args.capture {
        // Full-capture mode: parse complete transcripts, post CapturedSessions
        let mut captured = 0usize;
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
            // Derive cwd from the file path (the encoded dir name encodes the project path,
            // but we let parse_session pick it up from the JSONL lines themselves).
            let mut session = transcript::parse_session(&chunk, None);

            // Populate identity + repo
            session.user_email = email.clone();
            if let Some(cwd) = session.cwd.as_deref() {
                session.repo = repos.resolve(cwd);
            }
            if session.session_id.is_empty() {
                // Use file stem as fallback session id
                session.session_id = file
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
            }

            match poster.post_capture(&session) {
                Ok(202) => captured += 1,
                Ok(code) => {
                    eprintln!("  POST capture returned HTTP {code}");
                    skipped += 1;
                }
                Err(e) => {
                    eprintln!("  POST capture error: {e}");
                    skipped += 1;
                }
            }
            st.set(&key, new_off);
        }
        st.save(&state_path)?;
        println!("CCGuard agent: captured {captured} session(s), skipped {skipped}.");
    } else {
        // Default mode: token-event metadata only (unchanged from Plans 1–4)
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
                match interaction_to_event(&interaction, &email, &mut repos) {
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
    }

    Ok(())
}
