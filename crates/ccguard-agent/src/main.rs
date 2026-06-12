mod chunk;
mod enforce_paths;
mod event;
mod local_judge;
mod parse;
mod paths;
mod poster;
mod pricing;
mod repo;
mod signals;
mod state;
mod transcript;

use std::path::{Path, PathBuf};

use chrono::Datelike;
use clap::Parser;

use crate::event::interaction_to_event;
use crate::parse::parse_transcript;
use crate::poster::Poster;
use crate::state::State;

/// CCGuard endpoint agent — VISIBLE monitoring of this machine's Claude Code usage.
/// Sends metadata only (model, token counts, repo, timing) — never prompt or code content.
/// Use --capture to send full session transcripts (prompts, responses, tool calls, file edits).
/// Use --attest to enroll + report this machine's enforcement posture.
/// Use gen-policy to print a managed-settings.json policy document (no network).
#[derive(Parser, Debug)]
#[command(name = "ccguard-agent")]
struct Args {
    /// CCGuard server base URL, e.g. http://localhost:8080
    #[arg(long)]
    server: String,
    /// Ingest token (ccg_...) for this tenant. Required for all modes except gen-policy.
    #[arg(long)]
    token: Option<String>,
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
    /// Name of the MDM-injected corporate env var whose presence marks a corp session
    /// (provenance signal C-MDM-ENV). Only its presence is reported, never its value.
    #[arg(long, default_value = "CCGUARD_CORP")]
    corp_env: String,
    /// Triage mode: classify the server's UNCLASSIFIED sessions by running THIS
    /// machine's logged-in Claude Code CLI (no separate API key; uses the company's
    /// existing Claude seat), then post the verdicts back.
    #[arg(long)]
    triage: bool,
    /// (triage) Model alias/id for the local Claude Code judge. Default: haiku.
    #[arg(long, default_value = "haiku")]
    judge_model: String,
    /// (triage) Max sessions to classify in one run (bounds per-run quota). Default: 25.
    #[arg(long, default_value_t = 25)]
    triage_limit: u32,
    /// (triage) Bypass the idle-gate and backoff — run the sweep now even if Claude
    /// Code is active. For testing / an explicit admin-triggered run.
    #[arg(long)]
    force: bool,
    /// Service mode: run forever — capture frequently (cheap, keeps secret-scanning
    /// fresh) and run the triage pass once per calendar day during an idle window
    /// (with catch-up if a day was missed). This is how the installed agent runs.
    #[arg(long)]
    service: bool,
    /// (service) Seconds between capture passes. Default 900 (15 min).
    #[arg(long, default_value_t = 900)]
    capture_interval: u64,
    /// Attestation mode: enroll this device, fetch the expected policy, evaluate the on-disk
    /// managed-settings, and POST the attestation to /v1/attest. Takes priority over harvest modes.
    #[arg(long)]
    attest: bool,
    /// Optional positional mode word. Currently only `gen-policy` is recognized — it prints a
    /// managed-settings.json document to stdout and exits (no network, no token). This lets an
    /// admin run `ccguard-agent --server <url> gen-policy --org-uuid <u> --otel <url> > managed-settings.json`.
    #[arg(value_name = "MODE")]
    mode: Option<String>,
    /// (gen-policy) Corp Claude org UUID logins are locked to.
    #[arg(long)]
    org_uuid: Option<String>,
    /// (gen-policy) OTLP collector endpoint the OTEL exporters point at.
    #[arg(long)]
    otel: Option<String>,
    /// (gen-policy) Minimum allowed Claude Code version. Default: 2.1.38
    #[arg(long)]
    min_version: Option<String>,
    /// (gen-policy) Env var holding the ingest token the hook passes. Default: CCGUARD_TOKEN
    #[arg(long)]
    token_env: Option<String>,
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

/// Read the active Claude identity from `~/.claude.json`: returns
/// `(oauthAccount.emailAddress, organization-uuid)`. Either may be `None`.
///
/// The org UUID's key name has varied across Claude Code versions, so we probe a
/// few likely shapes — `oauthAccount.organizationUuid` /
/// `oauthAccount.organization_uuid`, the nested `oauthAccount.organization.uuid`,
/// and top-level fallbacks — but NEVER fabricate one: absent → `None`.
fn read_active_account(claude_dir: &Path) -> (Option<String>, Option<String>) {
    let Some(parent) = claude_dir.parent() else {
        return (None, None);
    };
    let json_path = parent.join(".claude.json");
    let Ok(text) = std::fs::read_to_string(json_path) else {
        return (None, None);
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return (None, None);
    };

    let oauth = v.get("oauthAccount");
    let email = oauth
        .and_then(|o| o.get("emailAddress"))
        .and_then(|e| e.as_str())
        .map(str::to_string);

    let org = read_org_uuid(&v, oauth);
    (email, org)
}

/// Probe several likely org-UUID locations; return the first non-empty string.
fn read_org_uuid(root: &serde_json::Value, oauth: Option<&serde_json::Value>) -> Option<String> {
    let as_str = |v: Option<&serde_json::Value>| -> Option<String> {
        v.and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };

    // Under oauthAccount: organizationUuid / organization_uuid / organization.uuid.
    if let Some(o) = oauth {
        if let Some(s) = as_str(o.get("organizationUuid")) {
            return Some(s);
        }
        if let Some(s) = as_str(o.get("organization_uuid")) {
            return Some(s);
        }
        if let Some(org) = o.get("organization") {
            if let Some(s) = as_str(org.get("uuid")).or_else(|| as_str(org.get("organizationUuid")))
            {
                return Some(s);
            }
        }
    }
    // Top-level fallbacks.
    as_str(root.get("organizationUuid")).or_else(|| as_str(root.get("organization_uuid")))
}

/// Build a tenant `PolicyConfig` from CLI flags and print its managed-settings.json
/// document to stdout. Pure (no network) — intended for `... gen-policy > managed-settings.json`.
fn run_gen_policy(args: &Args) -> anyhow::Result<()> {
    let org_uuid = args
        .org_uuid
        .clone()
        .ok_or_else(|| anyhow::anyhow!("gen-policy requires --org-uuid"))?;
    let otel = args
        .otel
        .clone()
        .ok_or_else(|| anyhow::anyhow!("gen-policy requires --otel"))?;
    let cfg = ccguard_core::enforce::PolicyConfig {
        server_url: args.server.clone(),
        org_uuid,
        otel_endpoint: otel,
        min_version: args
            .min_version
            .clone()
            .unwrap_or_else(|| "2.1.38".to_string()),
        token_env: args
            .token_env
            .clone()
            .unwrap_or_else(|| "CCGUARD_TOKEN".to_string()),
    };
    println!("{}", ccguard_core::enforce::managed_settings_pretty(&cfg));
    Ok(())
}

/// Enroll this device, fetch the expected policy, evaluate the on-disk
/// managed-settings, and POST the attestation to /v1/attest.
fn run_attest(args: &Args, claude_dir: &Path, poster: &Poster) -> anyhow::Result<()> {
    let (email, org) = {
        let (e, o) = read_active_account(claude_dir);
        (args.email.clone().or(e), o)
    };

    let device_id = enforce_paths::device_id();
    let hostname = enforce_paths::hostname();
    let os = enforce_paths::os_str();

    let enroll_body = serde_json::json!({
        "device_id": device_id,
        "hostname": hostname,
        "os": os,
        "agent_version": env!("CARGO_PKG_VERSION"),
        "user_email": email,
    });

    let resp = match poster.post_enroll(&enroll_body) {
        Ok(r) => r,
        Err(e) => {
            // Treat "no tenant policy" as a non-fatal informational exit.
            if e.to_string().contains("tenant policy not set") {
                println!("server has no policy for this tenant; set it in the dashboard");
                return Ok(());
            }
            return Err(e);
        }
    };
    let expected = resp.expected;

    let found = enforce_paths::find_managed_settings();
    let on_disk = found.as_ref().map(|(_, s)| s.as_str());

    let att = ccguard_core::attest::evaluate(on_disk, &expected, email.as_deref(), org.as_deref());

    let attest_body = serde_json::json!({
        "device_id": device_id,
        "agent_version": env!("CARGO_PKG_VERSION"),
        "attestation": att,
    });
    let status = poster.post_attest(&attest_body)?;

    // Human summary.
    let mark = |b: bool| if b { "✓" } else { "✗" };
    let (compliance, reasons) = ccguard_core::attest::verdict(&att);
    match &found {
        Some((path, _)) => println!("  managed-settings: {}", path.display()),
        None => println!("  managed-settings: NO managed-settings found"),
    }
    println!("  telemetry on:       {}", mark(att.telemetry_on));
    println!("  ccguard hook:       {}", mark(att.hook_present));
    println!("  login locked:       {}", mark(att.login_locked));
    println!("  policy match:       {}", mark(att.policy_match));
    println!("  bypass disabled:    {}", mark(att.bypass_disabled));
    println!(
        "  corp account:       {} {}",
        mark(!att.personal_account),
        att.active_account.as_deref().unwrap_or("(unknown)")
    );
    if reasons.is_empty() {
        println!("  verdict: {:?}", compliance);
    } else {
        println!("  verdict: {:?} — {}", compliance, reasons.join(", "));
    }
    println!("  attest POST -> HTTP {status}");
    Ok(())
}

/// Default hard cap on classify calls per seat per week — the backstop that keeps
/// the monitor from ever eating the developer's own Claude Code quota.
const WEEKLY_CLASSIFY_CAP: u32 = 100;
/// Don't classify while the dev is actively coding (transcript touched recently).
const IDLE_GATE_SECS: i64 = 300;

/// Seconds since the most-recently-modified Claude Code transcript, or None if there
/// are no transcripts (treated as idle).
fn seconds_since_active(claude_dir: &Path) -> Option<i64> {
    let now = std::time::SystemTime::now();
    paths::list_transcripts(claude_dir)
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok()?.modified().ok())
        .filter_map(|m| now.duration_since(m).ok())
        .map(|d| d.as_secs() as i64)
        .min()
}

/// Is `e` a rate-limit / over-quota failure (→ back off the whole sweep)?
fn is_rate_limited(e: &anyhow::Error) -> bool {
    let s = e.to_string().to_ascii_lowercase();
    s.contains("429")
        || s.contains("rate limit")
        || s.contains("rate-limit")
        || s.contains("overloaded")
}

/// Triage UNCLASSIFIED sessions by running the machine's logged-in Claude Code on
/// server-built prompts, posting each verdict back. No CCGuard API key — uses the
/// company's existing Claude seat, and session content stays in the Claude Code
/// channel they already authorized. Idle-gated (never competes with the dev),
/// weekly-budget-capped, and backs off on rate limits.
fn run_triage(
    args: &Args,
    claude_dir: &Path,
    email: &str,
    poster: &Poster,
    st: &mut State,
    state_path: &Path,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now();
    let now_epoch = now.timestamp();
    let iso = now.iso_week();
    let week = format!("{}-W{:02}", iso.year(), iso.week());

    if !args.force && st.in_backoff(now_epoch) {
        println!("CCGuard agent: triage backing off (rate-limited recently); skipping this run.");
        return Ok(());
    }
    // Idle-gate: if Claude Code was touched in the last few minutes, the dev is
    // working — defer the whole sweep, interactive latency is sacred. (--force skips it.)
    if !args.force {
        if let Some(secs) = seconds_since_active(claude_dir) {
            if secs < IDLE_GATE_SECS {
                println!("CCGuard agent: Claude Code active {secs}s ago — deferring triage sweep (use --force to override).");
                return Ok(());
            }
        }
    }

    let remaining = st.weekly_remaining(&week, WEEKLY_CLASSIFY_CAP);
    if remaining == 0 {
        println!(
            "CCGuard agent: weekly classify budget reached ({WEEKLY_CLASSIFY_CAP}); deferring."
        );
        st.save(state_path)?;
        return Ok(());
    }
    let limit = args.triage_limit.min(remaining);

    let pending = poster.get_triage_pending(email, limit)?;
    if pending.is_empty() {
        println!("CCGuard agent: no unclassified sessions to triage.");
        return Ok(());
    }
    println!(
        "CCGuard agent: triaging {} session(s) via local Claude Code (model {}, {remaining} left in weekly budget)...",
        pending.len(),
        args.judge_model
    );
    let mut done = 0usize;
    let mut failed = 0usize;
    for item in &pending {
        if st.weekly_remaining(&week, WEEKLY_CLASSIFY_CAP) == 0 {
            println!("  weekly budget exhausted mid-sweep — stopping.");
            break;
        }
        match local_judge::classify(&item.prompt, &args.judge_model) {
            Ok(v) => {
                st.record_classify(&week);
                let body = serde_json::json!({
                    "session_id": item.session_id,
                    "label": v.label.as_str(),
                    "confidence": v.confidence,
                    "reason": v.reason,
                    "mixed": v.mixed,
                    "matched_clause": v.matched_clause,
                    "off_assignment": v.off_assignment,
                    "input_digest": item.input_digest,
                    "model": format!("claude-code/{}", args.judge_model),
                });
                match poster.post_triage_verdict(&body) {
                    Ok(s) if (200..300).contains(&s) => done += 1,
                    Ok(s) => {
                        eprintln!("  verdict POST HTTP {s} for {}", item.session_id);
                        failed += 1;
                    }
                    Err(e) => {
                        eprintln!("  verdict POST error: {e}");
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                if is_rate_limited(&e) {
                    st.set_backoff(now_epoch + 900); // 15 min
                    eprintln!("  rate-limited — backing off 15 min, stopping sweep.");
                    break;
                }
                eprintln!("  judge failed for {}: {e}", item.session_id);
                failed += 1;
            }
        }
    }
    st.save(state_path)?;
    println!(
        "CCGuard agent: triaged {done} session(s){}.",
        if failed > 0 {
            format!(", {failed} failed")
        } else {
            String::new()
        }
    );
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let claude_dir = args
        .claude_dir
        .clone()
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".claude")))
        .expect("could not determine claude dir");

    // gen-policy is pure (no network, no token) and takes priority over every other mode.
    match args.mode.as_deref() {
        Some("gen-policy") => return run_gen_policy(&args),
        Some(other) => {
            anyhow::bail!("unknown mode '{other}' (only 'gen-policy' is supported)");
        }
        None => {}
    }

    // All remaining modes require an ingest token.
    let token = args
        .token
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--token is required (except for gen-policy)"))?;
    let poster = Poster::new(&args.server, &token);

    // Attestation mode takes priority over the harvest modes and returns early.
    if args.attest {
        return run_attest(&args, &claude_dir, &poster);
    }

    let email = args
        .email
        .clone()
        .or_else(|| read_active_account(&claude_dir).0)
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

    // Triage mode: classify the server's UNCLASSIFIED sessions via local Claude Code.
    if args.triage {
        return run_triage(&args, &claude_dir, &email, &poster, &mut st, &state_path);
    }

    // Service mode: long-running loop (frequent capture + daily idle-gated triage).
    if args.service {
        return run_service(&args, &claude_dir, &email, &poster, &mut st, &state_path);
    }

    let mut repos = repo::RepoCache::new();
    let mut sigs = signals::SignalCache::new(&args.corp_env);

    if args.capture {
        run_capture_once(
            &claude_dir,
            &email,
            &poster,
            &mut st,
            &state_path,
            &mut repos,
            &mut sigs,
        )?;
    } else {
        run_token_events_once(
            &claude_dir,
            &email,
            &poster,
            &mut st,
            &state_path,
            &mut repos,
        )?;
    }

    Ok(())
}

/// One full-capture pass: parse complete transcripts, chunk by content budget, and
/// post CapturedSessions. Reads the WHOLE file each run (the parser needs all lines
/// for session metadata/seqs); the per-file seq watermark — advanced only on
/// confirmed 202s — prevents redundant re-POSTs and silent loss of an unsent tail.
#[allow(clippy::too_many_arguments)]
fn run_capture_once(
    claude_dir: &Path,
    email: &str,
    poster: &Poster,
    st: &mut State,
    state_path: &Path,
    repos: &mut repo::RepoCache,
    sigs: &mut signals::SignalCache,
) -> anyhow::Result<()> {
    let mut captured = 0usize; // files that fully sent (all chunks 202)
    let mut failed = 0usize; // files with an unsent tail (will retry next run)
    for file in paths::list_transcripts(claude_dir) {
        let key = file.to_string_lossy().to_string();
        let content = match std::fs::read_to_string(&file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if content.is_empty() {
            continue;
        }
        let mut session = transcript::parse_session(&content, None);

        // Populate identity + repo
        session.user_email = email.to_string();
        if let Some(cwd) = session.cwd.as_deref() {
            session.repo = repos.resolve(cwd);
            // Content-free provenance signals (git + manifest config) for this dir.
            session.signals = Some(sigs.resolve(cwd));
        }
        if session.session_id.is_empty() {
            // Use file stem as fallback session id
            session.session_id = file
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
        }

        // Only send events past the confirmed-sent high-water mark for this file.
        let wm = st.capture_watermark(&key);
        let new_events: Vec<_> = session
            .events
            .iter()
            .filter(|e| e.seq > wm)
            .cloned()
            .collect();
        if new_events.is_empty() {
            continue;
        }
        let trimmed = ccguard_core::capture::CapturedSession {
            events: new_events,
            ..session.clone()
        };

        let chunks = chunk::chunk_session(&trimmed, chunk::CHUNK_CONTENT_BUDGET);
        let mut max_sent = wm;
        let mut had_error = false;
        for c in &chunks {
            let chunk_max = c.events.iter().map(|e| e.seq).max();
            match poster.post_capture(c) {
                Ok(202) => {
                    if let Some(m) = chunk_max {
                        if m > max_sent {
                            max_sent = m;
                        }
                    }
                }
                Ok(code) => {
                    eprintln!("  POST capture returned HTTP {code} for {key} — stopping this file (will retry)");
                    had_error = true;
                    break;
                }
                Err(e) => {
                    eprintln!(
                        "  POST capture error for {key}: {e} — stopping this file (will retry)"
                    );
                    had_error = true;
                    break;
                }
            }
        }

        // Persist only the confirmed-sent high-water mark — no silent loss.
        if max_sent > wm {
            st.set_capture_watermark(&key, max_sent);
        }
        if had_error {
            failed += 1;
        } else {
            captured += 1;
        }
    }
    st.save(state_path)?;
    if failed > 0 {
        println!(
            "CCGuard agent: captured {captured} session(s), {failed} had send errors (will retry)."
        );
    } else {
        println!("CCGuard agent: captured {captured} session(s).");
    }
    Ok(())
}

/// One token-event (metadata-only, legacy) pass — unchanged from Plans 1-4.
fn run_token_events_once(
    claude_dir: &Path,
    email: &str,
    poster: &Poster,
    st: &mut State,
    state_path: &Path,
    repos: &mut repo::RepoCache,
) -> anyhow::Result<()> {
    let mut sent = 0usize;
    let mut skipped = 0usize;
    for file in paths::list_transcripts(claude_dir) {
        let key = file.to_string_lossy().to_string();
        let (chunk, new_off) = match read_since(&file, st.offset(&key)) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if chunk.is_empty() {
            continue;
        }
        for interaction in parse_transcript(&chunk, None) {
            match interaction_to_event(&interaction, email, repos) {
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
    st.save(state_path)?;
    println!("CCGuard agent: sent {sent} event(s), skipped {skipped}.");
    Ok(())
}

/// Service mode: run forever. Capture every `capture_interval` seconds (cheap —
/// keeps the server's secret-scanning + token meter fresh); run the triage pass
/// once per calendar day during an idle window, with catch-up if a day was missed
/// (laptop asleep/off). Both passes already self-gate (capture is idempotent via
/// the watermark; triage is idle-gated + weekly-budget-capped + backs off on
/// rate limits), so the loop just paces them.
fn run_service(
    args: &Args,
    claude_dir: &Path,
    email: &str,
    poster: &Poster,
    st: &mut State,
    state_path: &Path,
) -> anyhow::Result<()> {
    let interval = std::time::Duration::from_secs(args.capture_interval.max(30));
    println!(
        "CCGuard agent: service mode — capture every {}s, triage once daily (idle-gated). Ctrl-C to stop.",
        interval.as_secs()
    );
    let mut repos = repo::RepoCache::new();
    let mut sigs = signals::SignalCache::new(&args.corp_env);
    loop {
        // Capture pass (cheap, every tick). A failure here must not kill the loop.
        if let Err(e) = run_capture_once(
            claude_dir, email, poster, st, state_path, &mut repos, &mut sigs,
        ) {
            eprintln!("CCGuard agent: capture pass error (will retry next tick): {e}");
        }

        // Daily triage pass: once per calendar day, only when not already done today.
        // run_triage itself enforces the idle-gate, weekly budget, and backoff, and
        // returns early (without doing work) when the dev is active — so on an active
        // day we keep trying each tick until an idle window opens, then mark the date.
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        if !st.triage_ran_today(&today) {
            let idle_ok = args.force
                || seconds_since_active(claude_dir)
                    .map(|s| s >= IDLE_GATE_SECS)
                    .unwrap_or(true);
            if idle_ok {
                if let Err(e) = run_triage(args, claude_dir, email, poster, st, state_path) {
                    eprintln!("CCGuard agent: triage pass error: {e}");
                } else {
                    // Mark the day done only after a real (idle) attempt ran.
                    st.mark_triage_date(&today);
                    let _ = st.save(state_path);
                }
            }
        }

        std::thread::sleep(interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gen_policy_core_output_is_valid_json_with_force_login_org() {
        // Pure: build a sample PolicyConfig and render it via the core generator.
        let cfg = ccguard_core::enforce::PolicyConfig {
            server_url: "https://ccguard.corp.example".into(),
            org_uuid: "org-abc-123".into(),
            otel_endpoint: "https://otel.corp.example:4318".into(),
            min_version: "2.1.38".into(),
            token_env: "CCGUARD_TOKEN".into(),
        };
        let out = ccguard_core::enforce::managed_settings_pretty(&cfg);
        let parsed: serde_json::Value =
            serde_json::from_str(&out).expect("gen-policy output is valid JSON");
        assert_eq!(parsed["forceLoginOrgUUID"], "org-abc-123");
        assert!(out.contains("forceLoginOrgUUID"));
    }

    #[test]
    fn read_active_account_parses_email_and_org() {
        let dir = std::env::temp_dir().join(format!("ccg_acct_{}", std::process::id()));
        let claude_dir = dir.join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            dir.join(".claude.json"),
            r#"{"oauthAccount":{"emailAddress":"alice@corp.com","organizationUuid":"org-xyz"}}"#,
        )
        .unwrap();

        let (email, org) = read_active_account(&claude_dir);
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(email.as_deref(), Some("alice@corp.com"));
        assert_eq!(org.as_deref(), Some("org-xyz"));
    }

    #[test]
    fn read_active_account_missing_org_is_none_not_fabricated() {
        let dir = std::env::temp_dir().join(format!("ccg_acct_noorg_{}", std::process::id()));
        let claude_dir = dir.join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            dir.join(".claude.json"),
            r#"{"oauthAccount":{"emailAddress":"bob@gmail.com"}}"#,
        )
        .unwrap();

        let (email, org) = read_active_account(&claude_dir);
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(email.as_deref(), Some("bob@gmail.com"));
        assert_eq!(org, None);
    }
}
