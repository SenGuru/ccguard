//! Classify a session by running the machine's OWN Claude Code, headless.
//!
//! The whole product premise is that the employee has company-provided Claude Code
//! installed and logged into the corp org. So instead of CCGuard buying a separate
//! API key, the agent shells out to the local `claude` CLI in print mode — it uses
//! the existing OAuth session, costs nothing extra, and the session content never
//! leaves the machine's already-authorized Claude Code channel.
//!
//! Invocation (live-tested 2026-06-12 on a logged-in Max seat):
//!   claude -p "<instruction>" --model <m> --output-format json --max-turns 1
//! NOTE: `--bare` must NOT be used — it skips loading the stored OAuth login and
//! every call fails "Not logged in" even with valid credentials on disk.
//! `--max-turns 1` makes it a non-interactive, can't-hang text call.
//! We strip `ANTHROPIC_API_KEY` from the child env so it uses the logged-in session
//! rather than a key (a set key would otherwise take precedence).

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use ccguard_core::capture::{CapturedSession, EventKind};
use ccguard_core::triage::{self, TriageVerdict};

/// The stable opening of every judge call's first user turn. The local `claude -p`
/// classification invocation writes its OWN transcript into `~/.claude/projects/`,
/// which capture would otherwise re-ingest as a 3-event session that then needs
/// judging — a self-amplifying loop. We detect those by this fixed prefix (we send
/// it, so it's reliable regardless of cwd) and skip them at capture.
pub const JUDGE_SENTINEL: &str = "Classify the Claude Code session described in the input.";

/// True if `session` is one of the agent's own local-Claude judge calls (not a real
/// developer session) — so capture can drop it and never feed the loop.
pub fn is_own_judge_session(session: &CapturedSession) -> bool {
    session
        .events
        .iter()
        .find(|e| e.kind == EventKind::UserPrompt)
        .and_then(|e| e.content.as_deref())
        .map(|c| c.trim_start().starts_with(JUDGE_SENTINEL))
        .unwrap_or(false)
}

/// Wall-clock cap on a single local classification — a hung `claude` must never
/// stall the sweep (or the dev's machine).
const JUDGE_TIMEOUT: Duration = Duration::from_secs(90);

/// The fixed `-p` instruction. The server-built prompt (rules + session context) is
/// piped via STDIN, so the only thing that touches the shell is this constant — the
/// arbitrary, attacker-influenceable session text never reaches a command line.
/// Kept free of cmd metacharacters (`| < > { } " & ^ %`) so the Windows `cmd /C`
/// path can't mis-parse it; the JSON shape with braces/pipes lives in the piped
/// prompt instead.
const INSTRUCTION: &str = "Classify the Claude Code session described in the input. \
Respond with only a single JSON object using exactly the fields the input asks for: at minimum label \
(exactly one of work, personal, or unsure), confidence (a number from 0 to 1), and reason (one short sentence), \
plus any other fields the input specifies. Output only that JSON object and nothing else.";

/// Run the local Claude Code judge on a server-built prompt; returns the verdict.
/// The prompt is delivered on STDIN (safe for arbitrary content); only fixed,
/// special-char-free flags hit the command line.
pub fn classify(prompt: &str, model: &str) -> Result<TriageVerdict> {
    // NO `--bare`: live-tested 2026-06-12 — `--bare` skips loading the stored OAuth
    // login, so every call failed "Not logged in" even on a valid Max session;
    // without it the logged-in session answers. `--max-turns 1` keeps this a pure,
    // non-interactive, can't-hang text call; `--output-format json` wraps the
    // answer. (No `--permission-mode`: a single text turn has no tools to gate, and
    // the valid modes don't include a "deny-all".)
    let args = [
        "-p",
        INSTRUCTION,
        "--model",
        model,
        "--output-format",
        "json",
        "--max-turns",
        "1",
    ];

    // On Windows the npm `claude` shim is a `.cmd`, which CreateProcess can't launch
    // directly — go through the command processor. On Unix run `claude` directly.
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg("claude").args(args);
        c
    } else {
        let mut c = Command::new("claude");
        c.args(args);
        c
    };

    let mut child = cmd
        .env_remove("ANTHROPIC_API_KEY") // force the logged-in Claude Code session
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            anyhow!("could not run `claude` (is Claude Code installed and logged in?): {e}")
        })?;

    // Write the prompt to stdin, then close it (drop) so claude sees EOF.
    {
        let mut si = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("could not open claude stdin"))?;
        si.write_all(prompt.as_bytes())?;
    }

    // Wall-clock timeout: poll for exit; kill if it overruns. (Classification output
    // is tiny, so the unread stdout pipe can't fill / deadlock before we collect it.)
    let start = Instant::now();
    loop {
        match child.try_wait()? {
            Some(_) => break,
            None => {
                if start.elapsed() > JUDGE_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(anyhow!(
                        "claude timed out after {}s",
                        JUDGE_TIMEOUT.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }

    let out = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() {
        // `--output-format json` puts the real reason (e.g. "Not logged in") in the
        // envelope's `result` even on a non-zero exit — surface it, not empty stderr.
        let msg = extract_result(&stdout)
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| String::from_utf8_lossy(&out.stderr).trim().to_string());
        return Err(anyhow!("claude exited {}: {}", out.status, msg));
    }

    // `--output-format json` wraps the answer in {result, ...}; fall back to raw
    // stdout if the envelope is absent. parse_verdict tolerates surrounding prose.
    let text = extract_result(&stdout).unwrap_or_else(|| stdout.to_string());
    triage::parse_verdict(&text).map_err(|e| anyhow!("could not parse verdict: {e}"))
}

/// Pull the `result` text out of the `--output-format json` envelope.
fn extract_result(stdout: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    v.get("result").and_then(|r| r.as_str()).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_result_from_envelope() {
        let env = r#"{"result":"{\"label\":\"work\",\"confidence\":0.8,\"reason\":\"corp repo\"}","session_id":"s","total_cost_usd":0.0001}"#;
        let text = extract_result(env).unwrap();
        let v = triage::parse_verdict(&text).unwrap();
        assert_eq!(v.label, triage::TriageLabel::Work);
    }

    #[test]
    fn non_envelope_returns_none() {
        assert!(extract_result("just some text, not json").is_none());
    }
}
