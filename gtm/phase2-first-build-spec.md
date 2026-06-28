# CCGuard v1 — Smallest Buildable Artifact: Build Spec

## Recommendation: **(a) standalone free local CLI scanner — with a deliberate "moat seam," not a full account.**

Not pure (a), not (b). Ship the free local CLI as the hero artifact, but wire in **one** optional hosted command (`--share`) that mints a public redacted Exposure Report URL and captures an email. That single seam *starts* the moat without building auth/RBAC/billing/dashboard.

**Why not (b) — scanner-with-account-that-auto-creates-a-dashboard:**
- A signup wall on the *acquisition* artifact directly contradicts the locked strategy ("acquire on the **commodity**, free, zero-friction; retain & defend on the moat" — §12). The whole point of leading with a commodity is that the scanner has to out-frictionless OSS hooks (ggshield, sensitive-canary). An account gate loses the Show HN / SEO viral loop you're acquiring with.
- (b) re-summons the exact enterprise machinery you're fleeing: multi-tenant Postgres, sessions, auth, billing. That is 10+ weeks for one dev (the `ccguard-server` crate already proves the cost), not 4–6, and it front-loads the moat *before* you have proof the door converts.
- The moat's defensibility is "the only one you can turn into a team dashboard by paying with a card" — that promise only needs to be **credible and demoable**, not *built*, to acquire and start a waitlist. Build it in v2 once the door is throwing off signups.

**Why (a)+seam genuinely starts the moat (so you're not just "weakest moat"):** the `--share` endpoint quietly accrues the two assets v2's dashboard is built on — (1) an opt-in **email list of teams who already self-identified leaks** (warmest possible expand audience) and (2) the **hosted redacted-report data substrate** the team rollup later claims. The donut/dashboard is then a v2 layer *on top of data you already own*, not a cold start.

---

## What to reuse (this is why 4–6 weeks is real)

| Reuse | From | Note |
|---|---|---|
| `Finding`, `FindingKind`, `Severity`, the detector regexes | `ccguard-core/src/findings.rs` | **Already stateless, pure, no DB/I/O, stores only redacted previews** — it is *built to be a CLI engine*. This is the whole product core; lift as-is. |
| Cross-tool transcript discovery | `ccguard-agent/src/paths.rs` (`list_transcripts`, `codex_home`, etc.) | Claude Code `~/.claude/projects/**.jsonl`, Codex `~/.codex`, plus copilot/codex parsers. |
| Transcript → normalized session parsing | `ccguard-agent/src/{transcript.rs, codex.rs, copilot.rs}` | Emits `CapturedSession`/`CapturedEvent` — the one cross-tool event shape. Feed each event's text into `findings::scan`. |
| Normalized event model | `ccguard-core/src/{capture.rs, event.rs}` | The "same event shape across tools" abstraction; keep it, it's your cross-tool moat primitive. |
| Redaction discipline + "content never leaves machine" | core invariant in `findings.rs` | The redacted preview is what makes `--share` safe to ship. |

## What to add (the only new code)
1. **New thin crate `ccguard-scan`** (a `[[bin]]`): single-shot, no daemon, no attestation. `ccguard scan [path]` → discover transcripts (all tools) → parse → scan → report. Depends on `ccguard-core` + the parser modules only.
2. **Terminal report**: grouped by tool, severity-sorted, redacted previews, count summary, non-zero exit code on findings (CI-friendly — a free CI hook is organic distribution).
3. **Exposure Report card generator**: redacted findings → static HTML → PNG (e.g. `resvg`/headless). This is the *shareable viral object* (§12 viral loop). Single-tool scripts can't produce a cross-tool card; that's the share-worthiness.
4. **`ccguard scan --share`**: POST the **redacted summary JSON only** to one serverless endpoint → returns `claresso.dev/r/<id>` public report page with a "Claim your team dashboard →" email capture. *This is the entire backend in v1: one function + object store/one table + email field.* No auth, no users, no tenancy.
5. **Install paths**: `curl | sh`, Homebrew tap, single static binary release. One-command run is the SEO landing-page payoff.

## What to cut from v1 (defer to upsell ladder, per §11.5)
- The entire `ccguard-server` control plane (Axum/Postgres, `tenants/users/sessions/auth/ingest/summary/timeline/search`), dashboard/Maud UI, fleet/attestation.
- `ccguard-proxy` enforcement (whole crate).
- **The donut / work-vs-personal attribution** (`classify.rs`, `provenance.rs`) — v2 dashboard feature, *not* the door (§11.1).
- Local LLM triage / conformal calibration / precision gate (`local_judge.rs`, `triage.rs`, `conformal.rs`, `precision_gate.rs`) — overkill for a deterministic regex+entropy scan; keep v1 detection **purely deterministic** so it's trustworthy and offline.
- SSO/SCIM, MDM/managed-settings, DPIA/eDiscovery, billing, RBAC.

## Rough sequence (1 FT dev, ~4–6 weeks)
- **Wk 1 — Carve the binary.** New `ccguard-scan` crate; wire `findings.rs`; get a clean Claude Code-only scan + terminal report working end-to-end. *(De-risk: prove the lift is trivial.)*
- **Wk 2 — Go cross-tool.** Add Cursor/Copilot/Codex via `paths.rs` + existing parsers into the normalized event shape. This *is* the differentiator vs every single-tool OSS script — prioritize it.
- **Wk 3 — Trust + UX.** False-positive hardening (precision = trust = the thing OSS hooks fumble), redaction audit, exit codes, `--json`, `--ci` mode, install scripts (curl/brew). Ship binary releases.
- **Wk 4 — Viral object.** Exposure Report card (HTML→PNG) + the `--share` serverless endpoint + public report page with email capture CTA. **Moat seam live.**
- **Wk 5 — Distribution wiring.** SEO landing page hooks for the flagship article ("scan your Claude Code chat for leaked API keys"), opt-in anonymous telemetry (counts only), Show HN / Product Hunt assets, README.
- **Wk 6 — Buffer/dogfood/launch.** Real-repo dogfooding, precision tuning on live data, harden the share endpoint, launch.

**Done = v1 success criteria:** a dev runs one command, sees real redacted leaks across all their AI tools in <60s with zero signup, and `--share` produces a card worth posting + drops an email into the expand list. The dashboard, donut, and card-pay billing are **v2 (wks 7–12)**, built directly on the share-endpoint data and waitlist this artifact generates.

Reference files: `crates/ccguard-core/src/findings.rs`, `crates/ccguard-agent/src/{paths.rs,transcript.rs,codex.rs,copilot.rs}`, `crates/ccguard-core/src/{capture.rs,event.rs}`.