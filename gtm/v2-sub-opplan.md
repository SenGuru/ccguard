I have what I need. Here is the grounded v2 operating plan.

---

# CCGuard / Claresso — Operating Plan v2 (product-grounded)

*Product+ops re-grounding · 2026-06-27 · supersedes the roadmap/sequencing in `phase2-gtm-blueprint.md` and `phase3-operating-plan-100k.md` where they assume capabilities the code does not show. Strategy, positioning, pricing, and MRR math from v1 are accepted; what changes is the **build sequence**, because the code reality differs materially from how v1 described "what already exists."*

---

## 0. The one finding that reshapes the plan

v1 repeatedly asserts the wedge is **"fully deliverable by the existing `findings.rs` + cross-tool capture + attribution code"** (`phase1b-positioning.md` §3) and lists the week-1 task as building a **"free, local… nothing leaves your machine"** scanner that emits a redacted Exposure Report (`phase2-gtm-blueprint.md` §3, §6; `phase3-operating-plan-100k.md` §1).

**The detection engine exists and is good. The local-first delivery path does not.** In the actual code:

- `ccguard_core::findings::scan()` is a real, well-tested pure scanner (`crates/ccguard-core/src/findings.rs`) — but it is invoked in exactly **one** production site: the **server** capture handler, `crates/ccguard-server/src/handlers/capture.rs:148`, running on transcript content the agent has already **POSTed to `/v1/capture`**. (Grep: `scan(` appears only in `capture.rs:148` and unit tests.)
- The agent's only local, no-network mode (`--dry-run`, `crates/ccguard-agent/src/main.rs:453`) parses sessions and prints token/repo/provenance summaries but **never calls `findings::scan`** — it shows no secrets.
- The agent **requires `--server`** (a non-optional arg, `main.rs:37`) and in capture mode **uploads full transcript content** to the control plane.

So the literal acquisition wedge — *local scan, secrets shown on your machine, nothing leaves the device, no account, shareable redacted card* — is **net-new engineering on top of a reusable core**, not a wiring exercise. This is the single biggest correction v2 makes to the roadmap.

Three more capabilities the GTM plans treat as present that **are not in the code**: **Stripe / billing / self-serve checkout** (zero references; `subscriptions` is a spec table only), **public signup + GitHub OAuth + team-invite** (auth is admin-provisioned password only — `users.rs:21` `create_user`), and a **Cursor** parser (the agent parses Claude Code + **Codex** + Copilot, *not* Cursor — `paths.rs`, `codex.rs`, `copilot.rs`).

---

## 1. What the crates actually do today (grounded inventory)

| Capability | Reality in code | Source |
|---|---|---|
| Secret/PII detection engine | **Exists, solid.** 8 secret rules (AWS, GitHub, Anthropic, OpenAI, Slack, Stripe, Google, JWT, private key) + 3 PII (email, US SSN, Luhn+IIN credit card); redacts; pure/stateless; ~20 unit tests | `ccguard-core/src/findings.rs` |
| Cross-tool transcript capture | **Exists for Claude Code + Codex CLI + Copilot CLI** — same `CapturedSession` shape; incremental watermarks; install-baseline so dormant history is never sent | `agent/src/main.rs:925-1041`, `paths.rs`, `codex.rs`, `copilot.rs` |
| **Cursor** capture | **Does NOT exist.** No cursor path/parser anywhere | `agent/src/paths.rs` (only `codex_home`/`copilot_home`) |
| Local-only secret scan output | **Does NOT exist.** `--dry-run` shows sessions/tokens/repos/provenance, not findings; scan runs server-side post-upload | `main.rs:453-657`; `capture.rs:148` |
| Work/personal attribution (the donut) | **Exists as a pipeline:** provenance cascade (`classify_raw`) + on-device AI judge using the user's own `claude -p` seat, idle-gated, weekly-capped → posts verdicts to server | `core/src/provenance.rs`, `ontask.rs`; `agent/src/local_judge.rs`, `main.rs:339` |
| Control plane (server) | **Substantial.** Axum + Postgres, multi-tenant ingest/capture/findings storage, **Maud server-rendered** dashboard, fleet/attestation, triage, sessions/timeline/summary/search handlers | `ccguard-server/src/**` |
| Enforcement proxy | **Exists, 324 LOC.** Reverse-proxies Claude API, fail-open, version-allowlisted, warm-recoverable block of confirmed-personal over-allowance sessions; precision deliberately gated | `ccguard-proxy/src/main.rs`, `core/src/enforce_gate.rs` |
| Device attestation + managed-settings | **Exists.** `--attest` enroll/evaluate posture; `gen-policy` emits `managed-settings.json` (force-login-org, OTel, min-version, hooks) | `main.rs:208-303` |
| Billing / Stripe / checkout | **Does NOT exist** | grep: none |
| Public signup / GitHub OAuth / team invite / reverse-trial | **Does NOT exist** (password auth, admin-provisioned users) | `users.rs`, `auth.rs` |
| Spend/dollar accuracy | `pricing.rs` exists; token counts known-unreliable (~46× undercount) — v1 correctly relegates spend to a content hook only | `agent/src/pricing.rs`; `phase1-market-truth.md` §5 |

**Net:** the *engine and the enterprise machinery* are further along than a typical pre-build; the *self-serve front door and money path* are essentially greenfield. This is the inverse of the impression `phase3 §5` gives ("Enterprise is re-activation, not greenfield… the only reason one dev can ship it"). Enterprise enforcement/attestation IS largely re-activation; **the self-serve scanner + billing + onboarding is the greenfield**, and it is on the critical path to every dollar.

---

## 2. Quarterly product roadmap — sequenced to unlock each revenue rung

Each quarter lists the **rung it unlocks**, what **already exists** (reuse), and the **net-new build**. Pricing/rung definitions are from `phase2-gtm-blueprint.md` §3.

### Q1 (M1–3) — Unlock Rung 0 (Free door) + Rung 1 (Team $99)
**Revenue gate:** first paying teams require a free viral artifact **and** a card path. Both are net-new.

**Build (net-new, critical path):**
1. **`claresso scan` — local-only CLI.** Reuse `findings::scan` + the three existing parsers (`transcript.rs`, `codex.rs`, `copilot.rs`) and the `--dry-run` walk; **add a local findings pass** so secrets render on-device with **zero network**. Make `--server` optional (decouple from the control plane). *(Reuses core engine + parsers; new: local scan output, offline mode.)*
2. **Redacted Exposure Report card + `--share`.** PNG/terminal card; `--share` mints a hosted `claresso.dev/r/<id>` page and captures an email. *(New; the share endpoint is a tiny server surface, not the full dashboard.)*
3. **Cursor parser** — to make the "Claude Code, Cursor, Copilot" trio in all copy *true*. **Decision required (see §5):** build Cursor, or rewrite every "Cursor" claim to "Codex." Recommend **build Cursor in Q1** because the entire positioning, headlines, and the cross-tool share A/B (`phase2 §4`) name Cursor, and Codex has lower ICP salience.
4. **Self-serve onboarding + GitHub OAuth + team invite** — PLG lesson #1, "non-negotiable" (`phase1-market-truth.md` §4) — does not exist; build it.
5. **Stripe checkout + plan gating** on the **team-rollup boundary** (`phase2 §3`). Does not exist; build it.
6. **Team dashboard cloud rollup** — the server already ingests capture and stores findings; surface a team findings rollup + 90-day history + Slack/email alerts on the existing Maud dashboard (`web.rs`).

**Reuse as-is:** server ingest/capture/findings storage, Maud dashboard scaffold, multi-tenant isolation.

**v1 step this corrects:** `phase2 §6` Week-1 ("Scaffold free CLI on findings.rs… local terminal summary") and Week-4 ("Stripe paywall") are correct as *tasks* but are scheduled as if trivial; in reality items 1–5 are the quarter's whole engineering load for one FT dev + a part-time founder-dev. The v1 "Day-90: ~5–20 paying teams" target is achievable **only if** billing + OAuth + invite (none built) land by ~week 8. Flag the schedule as tight, not the destination.

### Q2 (M4–6) — Unlock Rung 2 (Growth $299, the donut)
**Revenue gate:** expansion (`phase3 §3` names the Team→Growth donut ladder as ~42% of M12 MRR — the load-bearing lever). The donut **pipeline exists**; the **product surface does not.**

**Build:**
1. **Donut dashboard surface + gating** — provenance + AI-judge verdicts already land on the server; build the work/personal/unknown rollup view, the **read-only blurred teaser** in Team (`phase2 §5e`), and the $299 unlock. *(Mostly re-activation of `provenance.rs`/`ontask.rs`/triage; new: UI + gate.)*
2. **Continuous-monitoring service mode for teams** — `--service` loop exists (`main.rs:1092`); package it as the installed agent + per-seat health.
3. **Annual prepay toggle** (cash engine, `phase3 §6`) — Stripe config once billing exists.
4. **Per-repo / per-dev drilldown + audit export** — sessions/timeline/summary handlers exist; assemble into the Growth views.

**Honest dependency:** the donut's accuracy depends on the local `claude -p` judge consuming the user's own seat under an idle-gate/weekly cap (`main.rs:309-445`). This is clever but means donut freshness is **best-effort, not real-time**, and degrades for teams whose devs rarely go idle. v1's "≥25% of Team accounts touch a Growth trigger in 60 days" (`phase3 §8`) assumes the donut teaser reliably has data to show; instrument early.

### Q3 (M7–9) — Unlock Rung 3 (Enterprise) from inside the free base
**Revenue gate:** `phase3 §3` puts 6 Enterprise deals at ~16% of M12 MRR. Here the code genuinely is **re-activation, not greenfield** — but not uniformly.

**Re-activate (exists):** proxy enforcement (`ccguard-proxy`), device attestation (`--attest`), `managed-settings.json` generation (`gen-policy`), fleet handlers.
**Build greenfield (does NOT exist, despite "re-activation" framing):** **SSO/SAML + SCIM** (auth is password-only today), DPIA/LIA/eDiscovery generators (spec'd in `2026-06-09-ccguard-design.md` §9 but no code found — `policy_template.rs`/`policy_draft.rs` exist but were not verified to generate these artifacts).
**Hire dependency:** this quarter is why the 2nd dev must land **before** the Enterprise build (`phase3 §6`) — correct call, kept.

### Q4 (M10–12) — Compound: make expansion out-earn acquisition
Harden the donut ladder, ship Scale ($599) + Enterprise-Lite rungs, SSO-as-add-on, retention/detector-pack a-la-carte (`phase3 §3` levers 5–8). No new platform; ARPA engineering only. This matches v1 and is grounded — these are config/packaging on rails already built in Q1–Q3.

---

## 3. First-build spec (v1 product — the Free door + Team rung)

**Goal:** the smallest shippable thing that (a) reproduces the "holy crap, 3 keys across 3 tools" moment **fully locally**, and (b) has a card path to a team rollup. Built on the existing engine; explicitly enumerating the net-new pieces.

**A. `claresso scan` (local CLI) — REUSE engine, NEW delivery**
- Walk Claude Code (`paths::list_transcripts`), Codex (`list_codex_sessions`), Copilot (`list_copilot_sessions`) — **exists**; add **Cursor** (new).
- For each session's content, run `ccguard_core::findings::scan` **locally** — engine exists, local call site is **new**.
- Print a per-tool summary: `Scanned N sessions across {tools} — found X live secrets + Y PII`. Findings carry only the **redacted** preview (`findings.rs` `redact()` guarantees the raw secret never serializes — verified by the `finding_serializes_snake_case` test).
- **Offline by default:** make `--server`/`--token` optional (currently required, `main.rs:37`). Nothing leaves the machine unless the user runs `--share`.

**B. Exposure Report card + `--share` — NEW**
- Redacted screenshot-ready card (counts + tool list + "100% local").
- `--share` POSTs **only the redacted aggregate** (never content) to a minimal hosted endpoint → public report URL + email capture (the warm list, `phase2 §5b`).

**C. Account + Team rollup — NEW (onboarding/OAuth/invite) + REUSE (ingest/findings/dashboard)**
- GitHub OAuth signup + `claresso invite` → free team workspace (new).
- Team agents run in `--capture`/`--service` (exists) → server stores findings (exists) → team rollup + history + alerts on the Maud dashboard (extend existing `web.rs`).
- **Trust note that must be made explicit in product + docs:** in Team mode, **transcript content is uploaded to the control plane** so the server-side scan (`capture.rs:148`) and rollup work. The "local-first, nothing leaves your machine" promise is **true for free solo scan, not for the paid team rollup.** v1 copy blurs this (`phase2 §2` pillar 2 implies local-first universally). Either (i) keep server-side scan but message honestly ("solo = local; team = encrypted upload, redaction-on-write"), or (ii) move `findings::scan` to run **agent-side** and upload only redacted findings — a larger build but the only way to keep the local-first claim end-to-end. **Recommend (ii) for Team within Q1–Q2** because local-first is the explicit anti-Teramind moat (`phase1b §2`, `product-marketing-context.md` §0).

**D. Stripe billing — NEW**
- Checkout + plan gating on the **team-rollup boundary** (free solo scan never gated, `phase2 §3`).

**Out of scope for first build (kept on the enterprise ladder, per locked decisions §11):** SSO/SCIM, MDM/managed-settings deploy, proxy enforcement, DPIA/eDiscovery, the spend/dollar panel.

---

## 4. Hiring & reinvestment schedule (v2 — adjusted for the real build load)

v1's schedule (`phase3 §6`) is directionally right but front-loads too little engineering for a front door that is greenfield. Adjustment: **the part-time founder-dev is insufficient for the Q1 net-new stack (local scanner + Cursor + OAuth + invite + Stripe + rollup) alongside 1 FT dev.** Pull one lever earlier.

| Trigger | ~Month | Move | Why (grounded) |
|---|---|---|---|
| $0 (pre-revenue) | M1–3 | **No hire**, but **founder allocates ≥50% to dev in Q1**, not "part-time." | Q1 ships 5–6 net-new subsystems (none in code today); v1 assumed a wiring job. |
| $0–5k | M1–4 | 100% reinvest to infra/tooling; **no paid spend** (locked §11.4). | Matches v1. |
| ~$8–12k | M5–6 | Newsletter spend ON; **optional part-time content/SEO contractor**. | SEO is the #1 durable lever; unchanged from v1. |
| **~$15–25k** | **M6–8** | **2nd FT dev — hire BEFORE the Q3 Enterprise/SSO build.** | SSO/SCIM is **greenfield** (not re-activation); proxy/attestation are re-activation. One dev can't ship SSO + maintain scanner + billing. Kept from v1, reason corrected. |
| ~$30–40k | M9–10 | Contractor → FT content/SEO. | Unchanged. |
| ~$45–60k | M10–12 | Part-time enterprise CS (0.5 FTE), **not sales**. | Unchanged; once 3+ $30k accounts exist, churn risk > cost. |

**Reinvestment engine unchanged and grounded:** annual prepay injects ~$120–150k cash M2–6 (`phase3 §6`) — but this **depends on the annual toggle existing in Stripe**, which depends on billing shipping in Q1. The cash engine is downstream of the billing build; do not bank prepay cash before Stripe is live.

---

## 5. Decisions forced by the code (must resolve before Q1 build)

1. **Cursor vs Codex.** Code supports Codex; all copy says Cursor. **Build Cursor in Q1** (recommended) or rewrite positioning. Cannot ship "scan Claude Code, Cursor, Copilot" honestly today.
2. **Local-first for Team rung.** Keep server-side scan (honest-message it) or move scan agent-side (preserve the moat). Recommend agent-side for Team by Q2.
3. **Free-door account-less vs account-seam.** v1 chose "standalone CLI + one hosted `--share`, no auth" (`phase2 §5b`) — grounded and buildable; keep it. Do not gate the solo scan.

---

## 6. What v1's MRR math still needs (unchanged but re-flagged)
The $48k floor / ~$85k plan-of-record / $100k ceiling band (`phase3 §7`) is accepted. **But every dollar in it is downstream of subsystems not yet built** (scanner, OAuth, invite, Stripe, donut surface). v1's month-by-month curve (`phase3 §2`) shows MRR from M1 ($99) and 16 paying teams by M2 — that **requires billing + onboarding live by ~M1–M2**, i.e. ~6–8 weeks of greenfield. Treat M1–M2 revenue as **aspirational**; realistic first paid team is more likely M2–M3 once Stripe + invite ship. Slide the curve right by ~3–4 weeks.

---

## Grounding ledger

| Claim | Source (file §) | Confidence |
|---|---|---|
| Secret/PII scanner is real, tested, redacts, narrow ruleset (8 secret + 3 PII) | `ccguard-core/src/findings.rs` | High |
| `findings::scan` runs **server-side only**, on POSTed content; not local | `ccguard-server/src/handlers/capture.rs:148`; grep of `scan(` | High |
| Cross-tool capture exists for Claude Code + **Codex** + Copilot | `agent/src/main.rs:925-1041`, `paths.rs`, `codex.rs`, `copilot.rs` | High |
| **Cursor** parser does NOT exist | `agent/src/paths.rs` (no cursor home/parser) | High |
| `--dry-run` is the only no-network mode; shows tokens/repos/provenance, **not findings** | `agent/src/main.rs:453-657` | High |
| Agent **requires** `--server`; capture mode uploads transcript content | `agent/src/main.rs:37, 735-754` | High |
| Donut pipeline (provenance + on-device `claude -p` judge, idle-gated, weekly-capped) exists | `core/provenance.rs`, `ontask.rs`; `agent/local_judge.rs`, `main.rs:339-445` | High |
| No Stripe/billing/checkout/subscription code | grep across `crates/` | High |
| No public signup / GitHub OAuth / team-invite; auth is admin-provisioned password | `server/handlers/users.rs:21`, `auth.rs`, `passwords.rs` | High |
| Enforcement proxy exists (fail-open, version-gated, warm-block) | `ccguard-proxy/src/main.rs`; `core/enforce_gate.rs` | High |
| Attestation + `gen-policy` (managed-settings) exist | `agent/src/main.rs:208-303` | High |
| Dashboard is server-rendered **Maud**, not React/Next as spec proposed | `server/src/web.rs`; spec §4.1 proposed React | High |
| SSO/SCIM does NOT exist (Enterprise "re-activation" is partial) | `server/auth.rs` (password only); `phase3 §5` claims re-activation | High |
| DPIA/LIA/eDiscovery generators: spec'd, code unverified | spec `2026-06-09…§9`; `core/policy_template.rs`/`policy_draft.rs` not deep-read | Low |
| Strategy/positioning/pricing/MRR band accepted from v1 | `phase2 §1-3`, `phase3 §1,7` | High (as inputs) |
| 12–20 mo window; GitGuardian/Anthropic commoditize detection | `product-marketing-context.md` §12; `RESEARCH-FINDINGS.md` §1 | High |

## Evidence gaps / v1 over-reaches

1. **OVER-REACH (material): "fully deliverable by the existing `findings.rs` + cross-tool capture + attribution code"** (`phase1b §3`). The detection *engine* exists, but the **local-first scan path, redacted Exposure Report, and `--share`** do not — the scan runs server-side after content upload (`capture.rs:148`). The flagship free tool is net-new engineering, not wiring. **Correction in §0/§3.**
2. **OVER-REACH: "Cursor" in every cross-tool claim** (`phase2 §1,2,4`; headlines A/B/C). Code parses **Codex**, not Cursor. Either build Cursor (recommended, Q1) or rewrite copy. **§5.1.**
3. **OVER-REACH: universal "local-first, nothing leaves your machine"** (`phase2 §2` pillar 2; `phase1b §2`). True for solo scan; **false for the paid Team rollup**, which uploads transcript content. Message honestly or move scan agent-side. **§3.C, §5.2.**
4. **UNDER-STATED BUILD: Stripe/billing/checkout** assumed live by M1–M2 (`phase3 §2` shows $99 MRR in M1). **Zero billing code exists.** ~6–8 wks greenfield; slide the revenue curve right. **§6.**
5. **UNDER-STATED BUILD: signup + GitHub OAuth + team-invite** ("non-negotiable" PLG lesson, `phase1-market-truth.md` §4) — not in code; auth is admin-provisioned password. **§2 Q1.**
6. **MISLEADING FRAME: "Enterprise is re-activation, not greenfield"** (`phase3 §5`). Partly true (proxy, attestation, managed-settings exist) but **SSO/SCIM is greenfield** — the costliest Enterprise unlock isn't re-activatable. **§2 Q3.**
7. **GAP (low confidence): DPIA/LIA/eDiscovery generators** are spec'd (`design §9`) but I did not verify generating code in `policy_template.rs`/`policy_draft.rs`. Confirm before promising Enterprise compliance artifacts.
8. **GAP: donut freshness** depends on devs going idle for the local `claude -p` judge (idle-gate 300s, weekly cap 500). v1's "≥25% touch a Growth trigger in 60 days" (`phase3 §8`) assumes the teaser reliably has data; un-instrumented assumption. **§2 Q2.**
9. **ASSUMPTION (not in research): founder must allocate ≥50% to dev in Q1**, not "part-time" as `product-marketing-context.md` §7 / `phase3` assume — driven by the six net-new Q1 subsystems above. Flag as my recommendation, not a corpus fact.
10. **GAP: spend/dollar panel** (Growth tier, `phase2 §3`) — token counts are research-confirmed unreliable (~46× undercount, `phase1-market-truth.md` §5). Ship as relative/counts signal only; never authoritative dollars. v1 mostly honors this; ensure the Growth "spend-visibility panel" line doesn't reintroduce the over-promise.