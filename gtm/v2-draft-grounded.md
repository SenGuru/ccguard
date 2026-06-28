Verification confirms the three most-attacked claims: "Aikido" appears only in the v1 docs (phase2/phase3), never in the research corpus — so the flat-per-team proof point is correctly demoted. Donut demand is confirmed at 2/10, composite 0.6, "near-zero pull… must be educated into existence." Wedge scores confirmed (secrets 5, spend 6, work-vs-personal 2). Proceeding to the merged draft.

---

# CCGuard / Claresso — v2 Go-to-Market & Operating Plan (research-grounded)

*Chief-of-staff consolidation · 2026-06-27 · Supersedes `phase2-gtm-blueprint.md` and `phase3-operating-plan-100k.md`. Merges four re-grounded v2 inputs (positioning, pricing/math, GTM, operating). Every material claim traces to a corpus citation or is tagged `(assumption — not in research)`. Built to be inspected: §7 is the consolidated ledger, §8 is the ranked list of every load-bearing assumption.*

**File key:** PMC = `product-marketing-context.md` · MT/P1 = `phase1-market-truth.md` · AP/P1A = `phase1-appendix.md` · POS/P1B = `phase1b-positioning.md` · GTM/P2 = `phase2-gtm-blueprint.md` (v1) · OP/P3 = `phase3-operating-plan-100k.md` (v1) · RF = `RESEARCH-FINDINGS.md` · TS = `research/tracking-surface.md` · TV = `research/total-visibility.md`. Code citations are file paths under `crates/`.

---

## 1. Strategy in one paragraph

Acquire on the commodity, retain on the bundle, race the clock. Session-level secret/PII detection is already free in the wild (`sensitive-canary`, `leakproof`, `Claudleak`, OSS `Sieve`, GitGuardian's free `ggshield` AI hooks ≤25 devs, and Anthropic-native Claude Code Security — POS §2.1; AP "existing"; PMC §12), so we **give detection away** to win the long-tail SERP and the share loop, then charge the moment "me" becomes "my team." The defensible asset is **not a moat** — the corpus rates single-feature defensibility 2–3/10 and names the **12–20 month window** as the real clock (POS §1/§3; PMC §12; RF §1). What we actually own is (a) a **configuration + motion no incumbent offers in one breath** (cross-tool scan + team rollup + work/personal attribution + card-pay above a real free tier), (b) a **time window** before Anthropic-native detection and GitGuardian free hooks close the door, and (c) a **retention/switching-cost layer** (accumulated team history, the "donut," dev-trust install base) that accrues *after* adoption. The honest 12-month landing zone is **$30–50k MRR**, not the $85–100k some v1 math implied; $100k is a launch-breakout ceiling, not a forecast.

---

## 2. Positioning

### 2.1 Category
**AI-coding exposure monitoring** — self-serve, cross-tool, local-first: the AI-session secret & PII scanner a team can turn into a shared dashboard with a card. We name the *session layer* incumbents are blind to, not the commoditized *detection act* (gap-map, MT §3). Deliberately **not**: a "secrets scanner" (commoditized — POS §2; locked PMC §11.2), "shadow-AI/AI-DLP governance" (undeliverable on a voluntary on-device agent — POS §2.2/§3), or "AI cost/FinOps" (undeliverable at dollar accuracy; tokens undercount ~46×; needs Anthropic's Enterprise Analytics API — P1 §5; RF §1).

### 2.2 The wedge sentence (no competitor can truthfully say it in one breath)
> *"Claresso finds the leaked keys and PII across every AI coding tool your team uses — Claude Code, Cursor, Copilot — runs the scan entirely on your machine, and is the only one that lets a whole team turn that into a shared exposure dashboard by paying with a card, no demo, in 5 minutes."*

The durable, true clause is the **conjunction** (cross-tool + AI-session level + whole-team rollup + card-pay above free), grounded in the gap-map sales walls (GitGuardian Business "Let's Talk," Semgrep Teams "Contact us," all shadow-AI sales-led — P1 §4; AP). **Corrected from v1:** the shorter boast *"the only AI-coding security you can buy with a card"* is **false** — Snyk ships a self-serve "Buy Now" Team tier ≤10 devs (POS ledger; AP "Snyk"). Scope every card-pay claim to *cross-tool, AI-session-level, whole-team* governance.

### 2.3 Headline hierarchy (deliberate ordering — detection acquires, card-pay converts, donut retains)

| Surface | Headline | Grounding |
|---|---|---|
| **Acquisition** (problem-first, searched) | **"See every secret your AI coding tools already leaked — then stop the next one."** *Sub: Scan your Claude Code, Cursor, and Copilot sessions for exposed API keys, tokens, and PII. Free, local, 5-minute install — the scan never leaves your machine.* | POS §4 (Headline A, winner); VoC bank P1 §7 |
| **Conversion** (pricing) | **"The only cross-tool AI-session security your whole team can buy with a card. No demo. Live in 5 minutes."** | P1 §4 (card-pay = the wedge); **corrected** from v1's over-broad claim (Snyk) |
| **Retention/upsell** (in-product only) | **"Your scan just became your team's exposure dashboard — now see which AI sessions were company work, and which weren't."** | TV §0.2 (classifier is the differentiator); P1 §5 (donut = upsell). The donut never appears at acquisition (demand 2/10, unsearched). |

### 2.4 Messaging pillars (each with its proof)
1. **One scan, every AI coding tool.** *Proof:* gap-map — GitGuardian sees 3 hook checkpoints, Semgrep post-file-write only, Teramind screen-OCR, Purview/Anthropic-Compliance exclude the CC terminal (RF §1; TV §7); cross-tool scanning is feasible (OSS Sieve — AP). **Calibration:** depth parity across Cursor/Copilot is asserted "by design" (PMC §1); deep-capture research is ~90% Claude-Code-specific — see §8.
2. **Detection is free; you pay for the team layer.** Concedes the #1 objection. *Proof:* POS §2.1; locked PMC §12 ("Detection is NOT the moat").
3. **Card-pay, no demo, whole-team self-serve.** *Proof:* P1 §4 (every incumbent sales-walls above free); AP. **Scope discipline:** never "the only AI security you can buy by card."
4. **Local-first, dev-transparent (anti-Teramind).** *Proof:* TV §0.4 ("maximal capture is the #1 sales-killer; transparency is the #1 differentiator"), §7.3; PMC §1. **Scope discipline:** "content never leaves the machine" is literally true only for the **free local scan** — see §5 / §8-#5.
5. **Your scan becomes the team's exposure dashboard with work/personal attribution (the donut) — the retention layer.** *Proof:* TV §0.2/§5.1 ("watch the repo, not the person"). Most differentiated *feature* (no named competitor) but a **switching-cost/expansion** layer, not a wedge (demand 2/10) and **not** "un-copyable."

### 2.5 The "secrets-door + bundle" thesis, stated honestly
- **Door = the commodity** (session-level secret/PII detection): heavily searched *without education*, incident-backed (Miasma; TrustFall/CVE-2025-59536; AI-commit leak 3.2% vs 1.5% = **2.1×** — RF §2), and free. We win the **long-tail SERP** GitGuardian doesn't defend (`scan claude code chat history for leaked api keys`; SEO 4/10, highest in the set — POS §1/§5) and the **share loop** (cross-tool card out-shares single-tool scripts).
- **"Moat" = temporary configuration + retention layer, not structural defensibility.** Every component is individually copyable (def 2–3/10 — POS §1/§3). Durability = **execution speed + accumulated data/history + SERP brand + dev-trust install base + incumbents' *strategic disinclination*** (card-pay/$99-team is off-strategy for $45–54k-ACV enterprise sales models — RF §6; Anthropic structurally won't build cross-tool or work/personal — P1 §8; TV §0.3). This corrects every v1 use of "un-copyable moat."

### 2.6 What we are NOT
Not a better secrets *scanner*; not employee surveillance; not a shadow-AI discovery tool (the agent only sees machines it's installed on — POS §2.2); not an authoritative AI-spend/FinOps dashboard (undeliverable — P1 §5); not enterprise sales-led/demo-gated at acquisition; not a compliance/eDiscovery platform at the door (that's the enterprise ladder — PMC §11.5).

---

## 3. Pricing + the honest path-to-$100k math

### 3.1 Pricing structure (sound; re-grounded justification)
**Free** = one individual scanning their own machine: unlimited local cross-tool scans, full findings, the shareable redacted Exposure Report card. **The gate sits on the team rollup, never the individual scan** — this keeps both viral mechanics (SEO-ranking scan + screenshot card) ungated, because the free scan *is* the acquisition channel (MT §4.3; POS §5).

**Flat-per-team with seat bands + priced overage** (not per-seat). **Corrected justification:** v1 leaned on *"Aikido proves flat-per-team ($300/mo, 10 users)"* — **Aikido appears nowhere in the corpus** (verified: only in the v1 docs themselves), so demote to `(assumption — not in research)`. The decision survives on three grounded legs: (a) per-seat is the procurement model the wedge rejects (GG/Snyk/Semgrep all per-dev *and* sales-wall above free — RF §6; MT §3); (b) ARPA must clear the ~$300/mo floor for paid acquisition to ever work (LinkedIn cost-per-SQL $400–3,000 — RF §7); (c) predictability for card-buyers `(assumption)`.

| Tier | v1 | **v2** | Eff. $/dev | Grounding |
|---|---|---|---|---|
| **Free (Door)** | $0 | **$0** forever, 1 individual | — | MT §4.3; PMC §12; POS §5 |
| **Team** | $99 / ≤10 | **$99 / ≤10** (hold) | ~$9.90 | Floor of standalone secrets band $5–15/dev (AP). The conversion magnet. |
| **Growth** | $299 / ≤30 | **$349 / ≤25** (overage $14/dev) | ~$14 | Still only the *bottom* of the bundled team-governance band $20–50/dev (AP "Secrets WTP"; MT §5). $299/30 was underpriced. |
| **Enterprise** | ~$15–45k ACV | **~$15–45k ACV** | — | Bracketed by GG/Snyk/Semgrep ~$45–54k (RF §6). Per-deal plausible; the *count* is the problem (§3.4). |

**Why raise Growth, not Team:** the $100k problem is ARPA, not logos. Raising Growth into a *cited* WTP band lifts blended ARPA from ~$140 to ~$165 — doing the same work as ~1%/mo of *invented* donut upgrades, but anchored to evidence. **Donut placement: KEEP as the Growth feature; REJECT as the revenue engine** (demand 2/10, composite 0.6, "must be educated into existence" — MT §5; POS §1; verified in AP).

### 3.2 The arithmetic frame (grounded)
$100k MRR needs ~**605 teams @ $165** (or ~1,010 @ $99 — matches OP §1). A 3-person organic team cannot acquire ~1,010 logos in 12 months, so $100k can only come from **ARPA concentration**, not logo count. That logic is sound and kept. The dispute is purely whether v1's specific concentration rates are grounded — they are not (§3.4).

### 3.3 The grounded base case (~$32k MRR)
Inputs at the conservative-but-grounded end: cumulative signups ~16k (low end of GTM's 15–25k, justified by the no-audience precedent below); free→paying-**team** conversion 1.2%→2.0% (RF §7 freemium median is 3–6% *per seat*; discounted because the unit is a team — MT §2); SMB churn 4%/mo (RF §7); blended ARPA $99→~$135. *Monthly shape is `(assumption — illustrative)`; inputs are grounded.*

| Mo | Signups | New teams | Active (4% churn) | ARPA | Ending MRR |
|----|------|------|------|------|------|
| M2 | 900 (HN) | 12 | 14 | $100 | ~$1.4k |
| M3 | 1,400 (PH) | 20 | 33 | $104 | ~$3.4k |
| M6 | 1,300 | 22 | 84 | $116 | ~$9.7k |
| M9 | 1,600 | 30 | 155 | $127 | ~$19.7k |
| M12 | 1,900 | 38 | **~241** | **$135** | **~$32.5k** |

### 3.4 Scenario comparison and recommended plan of record

| Scenario | Signups | Conv. | ARPA | Enterprise | M12 MRR | Requires |
|---|---|---|---|---|---|---|
| **Base** (all grounded) | ~16k | 2.0% | $135 | 0 | **~$32k** | One HN front-page + PH + modest SEO |
| **Plan of record (v2)** | ~22k | 2.4% | **$165** | 1–2 inbound | **~$48–55k** | SEO mid-range `(assumption)` + the §3.1 reprice + Growth mix climbs |
| **Stretch (= OP's "$85k")** | ~25k | 2.2% | $169 | **6 deals** | ~$82–88k | Donut 3%/mo upgrades (no benchmark) + 6 enterprise (no benchmark). **Inference-stacked.** |
| **Ceiling** | ~28–32k | 2.5% | ~$175 | 8–10 | ~$100k | OP's own "10–15% probability" stack |

**Recommended plan of record: ~$50k MRR.** This reconciles the v1 contradiction in GTM's favor: GTM §7 named **$30–50k base / $50k plan of record**; OP §1/§7 silently re-based the *same* plan of record to **~$85k** on two unbenchmarked levers (3%/mo donut upgrades + 6 enterprise) that OP §3 admits are **~58% of M12 MRR**. Drive toward the stretch; **report against $50k; treat $85–100k as upside, never as the forecast.**

**The framing fact both v1 docs underweight:** the only *no-audience, bootstrapped* comps in the research — **Plausible (42 months → ~$83k MRR) and Bannerbear (60 months → $50k MRR)** (RF §7) — took **3.5–5 years** to reach this MRR. The fast comps (CodeRabbit "VC+viral," RB2B "92k-follower founder audience") are explicitly disqualified, and the team has "no audience, no warm intros" (PMC §7). **Even $50k in 12 months is ahead of every applicable comp.**

---

## 4. GTM motion + launch

### 4.1 Deliverability anchor
The free local scan is what makes the "5-minute, local, no-IT" promise honest: Claude Code writes a complete append-only JSONL transcript to `~/.claude/projects/…` — zero config, already on disk (TS §0.1, verified live). **But the same fact is why detection is NOT the moat** — the OSS one-offs read the *same* transcripts (POS §2.1). *(Code caveat — §5: the engine exists but the local-scan delivery path is net-new.)*

### 4.2 The PLG loop: Scan → Shock → Share → Rollup → Pay
Free local CLI (`npx/brew/pipx`, no account) → "47 sessions, 3 tools, 3 LIVE secrets + 2 PII" (the cross-tool framing single-tool OSS can't reproduce) → redacted card + public `claresso.dev/r/<id>` → `invite` → free team workspace → Stripe card, no demo. The gate sits on the **rollup**, so the SEO/share loop never throttles.

**Two tightenings v1 missed:**
- **Reverse-trial at the `invite` gate.** RF §7: reverse-trial converts ~**24%** vs freemium 3–6%. v1 never used it. Offer the paid dashboard as a 14-day reverse-trial (card optional, auto-downgrade). **The single highest-leverage, fully-grounded conversion lever available — A/B it against the straight paywall.** This is the grounded substitute for OP's un-grounded donut ladder.
- **Capture the company email domain on `--share` from day one** → the enterprise-seed signal (the one sound piece of OP's enterprise motion — devs already run inside big orgs; 21k repos). Keep the mechanism; don't bank "6 deals" on it.

*Every conversion rate in the loop is flagged: share rate >15–20% and K≈0.15–0.30 are `(assumption)` — POS §7.6 calls them a **pre-build test**, not a known quantity.*

### 4.3 Ranked $0-ad organic motions
1. **Free-tool-led long-tail SEO/content** (durable engine) — the only motion that is $0, compounding, and still paying out in M12. Targets demand searched *without education*; long-tail is the highest-defensibility axis (POS §1, SEO 4/10). **Volume of signups is NOT grounded — carry "≥X organic signups/mo" as a hypothesis with a Q2 kill-switch.**
2. **Product-led viral loop** (Exposure Report share + invite) — zero-marginal-cost amplifier; mechanism deliverable, K-factor unvalidated.
3. **GitHub-native distribution** (Marketplace CI Action + Awesome-Claude-Code/alternativeto/libhunt + the `.claude/` org API signal) — evergreen; GitGuardian became #1 GitHub security app via Marketplace (P1 §4.2); ~21k indexed `.claude/` repos + ~300–500 new orgs/mo (RF §5). Conversion-to-signup is `(assumption)`.
4. **Community launch cadence (HN/Reddit/PH)** — cold-start igniter, non-compounding (see §4.4).
5. **Authentic community participation** — slow-burn, founder-led; capped by Reddit's Responsible-Builder policy (no identical cross-posts).

**Grounded exclusions:** LinkedIn organic/outbound (cost-per-SQL $400–3,000, unviable at $99 ARPA — RF §7); cold AI-SDR/Clay (human-reviewer load-bearing); shadow-AI discovery as a *product* claim (undeliverable — POS §2.2).

### 4.4 Launch sequence — corrected channel outcomes
Structure kept (SEO base indexed *before* spikes; HN before PH; never two big launches in one week). **Numbers corrected to be probabilistic:**
- **Show HN (wk 7):** v1 banked "8–15k visitors." RF §8/§5: **2.3% is the *probability of reaching front page*; 8–15k happens only IF front page.** Plan against **a few hundred visitors (modal), with a ~2.3% tail chance of an 8–15k breakout.** Do not bank the breakout.
- **Reddit (wk 7+2d):** channels grounded; the "1–3k scanners" number is `(assumption)`. No identical cross-posts.
- **Product Hunt (wk 9, paid Team GA + Stripe live):** 200–600 signups is **conditional on winning #1** (RF §8); mid-pack is materially less.
- **Dev newsletters (wk 8, revenue-funded — realistically M≥5):** TLDR InfoSec ~**$167 CAC** at scale, $5–15k/issue (RF §7/§8) — pays back in <2 months at $99 ARPA. **The only fully-grounded CAC number.** Pitch the *data story* (2.1×), not a product ad. Avoid Pragmatic Engineer (no sponsors).

**The #1 sequencing point (correct in v1, kept):** bank the cross-tool + donut + card-pay configuration into Google and paying teams *before* Anthropic-native detection + GitGuardian free hooks commoditize the door — the 12–20 month window (RF §1; PMC §12).

---

## 5. Operating plan (roadmap + hiring + first build)

### 5.1 The one finding that reshapes the build
v1 asserted the wedge is "fully deliverable by the existing `findings.rs` + cross-tool capture" (POS §3). **The detection engine exists and is good; the local-first delivery path does not.** In the actual code:
- `ccguard_core::findings::scan()` is a real, tested, redacting scanner — but invoked in exactly **one** production site: the **server** capture handler (`crates/ccguard-server/src/handlers/capture.rs:148`), running on content the agent has already **POSTed to `/v1/capture`**.
- The only no-network mode (`--dry-run`, `crates/ccguard-agent/src/main.rs:453`) shows tokens/repos/provenance but **never calls `findings::scan`** — it shows no secrets. The agent **requires `--server`** and uploads full transcript content.

So the literal acquisition wedge — *local scan, secrets on your machine, nothing leaves the device, shareable redacted card* — is **net-new engineering on a reusable core**, not wiring. Three more capabilities the GTM treats as present but **absent from code**: **Stripe/billing/checkout** (zero refs), **public signup + GitHub OAuth + team-invite** (auth is admin-provisioned password only — `users.rs:21`), and a **Cursor parser** (the agent parses Claude Code + **Codex** + Copilot, *not* Cursor — `paths.rs`).

### 5.2 Grounded code inventory (what's real)
| Capability | Reality | Source |
|---|---|---|
| Secret/PII engine | **Exists, solid** (8 secret + 3 PII rules, redacts, ~20 tests) | `ccguard-core/src/findings.rs` |
| Cross-tool capture: Claude Code + **Codex** + Copilot | **Exists** | `agent/src/main.rs:925-1041`, `codex.rs`, `copilot.rs` |
| **Cursor** capture | **Does NOT exist** | `agent/src/paths.rs` |
| Local-only secret scan output | **Does NOT exist** (scan is server-side post-upload) | `main.rs:453`; `capture.rs:148` |
| Donut (provenance + on-device `claude -p` judge, idle-gated, weekly-capped) | **Exists as pipeline** | `core/provenance.rs`, `ontask.rs`; `agent/local_judge.rs` |
| Control plane (Axum/Postgres, Maud dashboard, fleet/triage) | **Substantial** | `ccguard-server/src/**` |
| Enforcement proxy (fail-open, version-gated) | **Exists, 324 LOC** | `ccguard-proxy/src/main.rs` |
| Attestation + `gen-policy` (managed-settings) | **Exists** | `main.rs:208-303` |
| Billing / signup / OAuth / invite / SSO / SCIM | **Do NOT exist** | grep |

**Net:** the engine and enterprise machinery are further along than a typical pre-build; **the self-serve front door and money path are essentially greenfield** — the inverse of OP §5's "Enterprise is re-activation" framing. Enterprise *enforcement/attestation* is re-activation; **the scanner + billing + onboarding is the greenfield on the critical path to every dollar.**

### 5.3 Quarterly roadmap (sequenced to each revenue rung)
- **Q1 (M1–3) — Free door + Team $99.** Net-new critical path: (1) `claresso scan` local-only CLI (reuse engine + parsers; add local findings pass; make `--server` optional); (2) redacted Exposure Report card + `--share`; (3) **Cursor parser** *(decision §5.5)*; (4) self-serve onboarding + GitHub OAuth + team-invite ("non-negotiable" PLG lesson — MT §4); (5) Stripe checkout gated on the team-rollup boundary; (6) team dashboard rollup on the existing Maud surface. *Reuse: server ingest/findings storage/dashboard.* **v1's "Day-90: 5–20 paying teams" is achievable only if billing + OAuth + invite land by ~week 8 — flag the schedule as tight, slide the revenue curve right ~3–4 weeks.**
- **Q2 (M4–6) — Growth $349 + donut surface.** Donut *pipeline* exists; build the work/personal rollup view, the blurred Team teaser, the unlock; annual-prepay toggle; per-repo/per-dev drilldown. **Honest dependency:** donut freshness is best-effort (depends on devs idling for the local `claude -p` judge, idle-gate 300s / weekly cap 500) — instrument before banking "≥25% touch a Growth trigger in 60 days."
- **Q3 (M7–9) — Enterprise from inside the free base.** *Re-activate (exists):* proxy enforcement, attestation, `managed-settings`, fleet. *Greenfield (does NOT exist despite "re-activation" framing):* **SSO/SAML + SCIM**, DPIA/LIA/eDiscovery generators (spec'd, code unverified). 2nd dev must land **before** this quarter.
- **Q4 (M10–12) — Compound.** Scale ($599) + Enterprise-Lite rungs, SSO add-on, detector packs — config/packaging on rails already built.

### 5.4 Hiring & reinvestment (adjusted for the real build load)
| Trigger | ~Month | Move | Why |
|---|---|---|---|
| pre-revenue | M1–3 | No hire; **founder ≥50% to dev**, not "part-time" | Q1 ships 5–6 net-new subsystems `(assumption — my recommendation, not a corpus fact)` |
| $0–5k | M1–4 | 100% reinvest; no paid spend (PMC §11.4) | grounded |
| ~$8–12k | M5–6 | Newsletter spend ON; optional SEO contractor | SEO #1 durable lever |
| **~$15–25k** | **M6–8** | **2nd FT dev BEFORE Q3 SSO build** | SSO/SCIM is greenfield, not re-activatable |
| ~$45–60k | M10–12 | Part-time enterprise CS (0.5 FTE), not sales | churn risk > cost once 3+ $30k accounts |

Annual-prepay cash (~$120–150k, M2–6) is **downstream of Stripe shipping in Q1** — do not bank prepay before billing is live.

### 5.5 Decisions forced by the code (resolve before Q1)
1. **Cursor vs Codex:** all copy says Cursor; code supports Codex. **Build Cursor in Q1** (recommended — positioning/headlines/share-A/B all name Cursor) or rewrite every "Cursor" claim. *Cannot ship "Claude Code, Cursor, Copilot" honestly today.*
2. **Local-first for Team:** the paid rollup uploads transcript content, so "nothing leaves your machine" is true only for solo scan. Either message honestly ("solo = local; team = redaction-on-write upload") or **move `findings::scan` agent-side** (recommended by Q2 — local-first is the anti-Teramind moat).
3. **Free door account-less:** keep v1's "standalone CLI + one hosted `--share`, no auth." Do not gate the solo scan.

---

## 6. Honest landing zone

- **Plan of record: ~$50k MRR at M12** (~300 paying teams blended). Base case ~$32k if nothing breaks out. $85k requires two unbenchmarked levers; $100k is a ~10–15% ceiling.
- **The plan is one bet with a clock:** out-rank GitGuardian on the long tail and bank paying teams + dev-trust install base *inside the 12–20 month window*, before detection fully commoditizes. The two largest *unvalidated* dependencies are **(a) SEO signup volume** (winnability is grounded; volume is not) and **(b) cross-tool depth parity** (Cursor coverage doesn't exist in code yet).
- **What protects us is not a moat** — it's speed, accumulated history, SERP brand, dev-trust, and incumbents' strategic disinclination to run a $99-no-demo motion. All weaker and more time-bound than "moat."
- **The honest pre-build validations** (do these before betting the funnel): the defensible-virality test (does the cross-tool card out-share single-tool? — POS §7.6), the SEO Q2 kill-switch, and a product-truth audit of Cursor/Copilot coverage depth.

---

## 7. MASTER GROUNDING LEDGER

| # | Claim | Source (file §) | Confidence |
|---|---|---|---|
| 1 | Detection is a free commodity (OSS + ggshield ≤25 + Anthropic-native); not the moat | POS §2.1; PMC §12; AP "existing" | High |
| 2 | No tool captures the Claude Code *terminal session*; Anthropic Compliance API + 60 partners exclude it; 12–20mo window | RF §1; P1 §3; TV §0.3/§2; PMC §12 | High |
| 3 | Every secrets/SAST/shadow-AI incumbent hits a sales wall above free → card-pay is the wedge | P1 §4; AP (GG/Semgrep "Contact us") | High |
| 4 | **Snyk Team tier IS self-serve card-pay (≤10 devs)** → "only AI security buyable by card" is false | AP "Snyk"; POS ledger | High |
| 5 | AI-assisted commits leak at 2.1× (3.2% vs 1.5%); Miasma; TrustFall/CVE-2025-59536 | RF §2; PMC §2 | High |
| 6 | Cross-tool local secret *scanning* is feasible (OSS Sieve scans Cursor/Claude/Copilot) | AP "existing" | High |
| 7 | Donut: no named competitor, but demand **2/10, composite 0.6, "near-zero pull," must be educated** → feature/retention, not wedge | MT §5; POS §1; AP (verified) | High |
| 8 | Single-feature defensibility 2–3/10; the real asset is the 12–20mo window + retention | POS §1/§3; PMC §12 | High |
| 9 | Repo-attribution classifier = the differentiator ("watch the repo, not the person") | TV §0.2/§5.1/§7 | High |
| 10 | Transparency/dev-sees-own-data = #1 differentiator; maximal capture = #1 sales-killer | TV §0.4/§7.3; AP Teramind row | High |
| 11 | Accurate AI spend undeliverable self-serve (tokens undercount ~46×; needs Enterprise Analytics API) | P1 §5; RF §1; PMC §11.6 | High |
| 12 | Free tier sized to a team is the virality engine (GG 25 devs, Semgrep 10); gate on the rollup | MT §4.3; POS §5 | High |
| 13 | Per-seat = the procurement model the wedge rejects; ARPA only viable >$300/mo for paid acq | MT §3; RF §6/§7 | High |
| 14 | Standalone secrets WTP $5–15/dev; bundled team-governance $20–50/dev | AP "Secrets WTP"; MT §5 | High |
| 15 | Enterprise ACV $15–45k realistic per deal (GG/Snyk/Semgrep ~$45–54k) | RF §6; AP | High |
| 16 | SMB monthly churn 3–5% | RF §7 | High |
| 17 | Freemium free→paid median 3–6% (seat); team-unit discount to ~1.5–2.5% is reasoned | RF §7; MT §2 | Medium |
| 18 | Reverse-trial converts ~24% (vs freemium 3–6%, card-trial 25–44%) | RF §7 | High |
| 19 | Show HN front-page **rate** ~2.3%; 8–15k visitors **only if** front page | RF §8/§5 | High |
| 20 | PH #1 dev tool ≈ 200–600 signups (conditional on winning #1) | RF §8 | High (channel) / Med (top finish) |
| 21 | Newsletter (TLDR InfoSec) ~$167 CAC at scale; $5–15k/issue; Pragmatic Engineer no sponsors | RF §7/§8 | High |
| 22 | ~21,000 indexed `.claude/` repos + ~300–500 new orgs/mo = funnel signal | RF §5; PMC §2 | High (signal) / assumption (→signups) |
| 23 | GitGuardian = #1 GitHub security app via Marketplace → GitHub-native distribution works | P1 §4.2 | High |
| 24 | LinkedIn cost-per-SQL $400–3,000, unviable at $99 ARPA | RF §7 | High |
| 25 | No-audience comps: Plausible 42mo→~$83k MRR, Bannerbear 60mo→$50k; fast comps (CodeRabbit/RB2B) disqualified | RF §7; PMC §7 | High |
| 26 | $100k needs ~605 teams @ $165 (or ~1,010 @ $99) → ARPA concentration is the only path | OP §1 (derived); RF §6 | High |
| 27 | Long-tail SEO winnable (score 4, highest); head terms not | POS §1/§5 | Medium (analyst estimate, not rank data) |
| 28 | Free on-disk JSONL transcript (zero-config) makes local scanner deliverable | TS §0.1/§2.3 | High |
| 29 | Secret/PII engine is real, tested, redacting (8 secret + 3 PII) | `ccguard-core/src/findings.rs` | High |
| 30 | `findings::scan` runs **server-side only**, on POSTed content; not local | `capture.rs:148`; grep | High |
| 31 | Cross-tool capture exists for Claude Code + **Codex** + Copilot; **Cursor does NOT** | `agent/main.rs`, `paths.rs` | High |
| 32 | No Stripe/billing/checkout; no public signup/OAuth/invite (admin password only) | grep; `users.rs:21` | High |
| 33 | Donut pipeline exists (provenance + idle-gated `claude -p` judge) | `core/provenance.rs`; `agent/local_judge.rs` | High |
| 34 | Enforcement proxy + attestation + `gen-policy` exist; **SSO/SCIM does NOT** | `ccguard-proxy`; `server/auth.rs` | High |
| 35 | $30–50k = honest 12-mo band; $50k plan of record; $85–100k = breakout-contingent upside | GTM §7; OP §7; this doc (derived) | Medium |

---

## 8. EVIDENCE GAPS & v1 OVER-REACHES
*Ranked by how much the plan depends on the assumption (most load-bearing first). This is the inspection surface — attack here.*

1. **SEO signup volume is invented and the plan rests on it.** v1's "3,000–6,000 organic signups/mo by M12 → ~25k cumulative" (P2 §3; P3 §8) has **no source in RF** — RF supports only that the long-tail is *winnable* (SEO 4/10, analyst judgment not rank data) and sizes the org pool (~21k + ~300–500/mo). The entire organic, $0-ad GTM depends on out-ranking GitGuardian's free `ggshield` content. **Correction:** carry signup volume as a hypothesis with a Q2 kill-switch; plan the base on ~16k cumulative. **Highest-leverage unvalidated dependency in the plan.** `(assumption — not measured)`

2. **Cross-tool depth parity is asserted, not validated — and Cursor doesn't exist in code.** The whole wedge is "one scan, all three tools," yet deep-capture research is ~90% Claude-Code-specific (PMC §1 says "by design"), and the agent parses **Codex, not Cursor** (`paths.rs`). Cross-tool *scanning* is grounded (Sieve); cross-tool *deep governance + attribution* at Claude-Code parity is **unproven**. **Correction:** product-truth audit + build Cursor in Q1 before this is the headline claim. `(assumption — not validated; code gap confirmed)`

3. **The $85k "plan of record" is inference-stacked (~58% of M12 MRR un-grounded).** OP re-based GTM's $50k to ~$85k via a **3%/mo donut upgrade rate (~42% of M12 MRR)** and **6 enterprise deals (~16%)** — neither has any benchmark in RF, and the donut is the **lowest-demand feature in the entire ranking (2/10)**. **Correction:** revert plan of record to $50k; reclassify $85k as upside. **The most serious revenue over-reach.** `(invented)`

4. **The local-first front door + money path are greenfield, not "fully deliverable."** v1 (POS §3) called the wedge a wiring exercise; in code the local scan path, redacted card, `--share`, Stripe, OAuth, and team-invite are **all net-new** (engine runs server-side post-upload). **Correction:** Q1 is 5–6 net-new subsystems; slide the revenue curve right ~3–4 weeks. `(code reality vs v1 claim)`

5. **"Content never leaves the machine" is true only for the free solo scan.** The paid Team rollup uploads transcript content (`capture.rs:148`); v1 Pillar 2 states it as a universal product property. **Correction:** scope copy to the scan, or move `findings::scan` agent-side by Q2. `(scope over-reach; code-confirmed)`

6. **"Un-copyable moat" contradicts the corpus's own 2–3/10 defensibility finding.** Every component is individually copyable; durability = speed + accumulated history + SERP brand + dev-trust + strategic disinclination — all time-bound. **Corrected throughout** to "configuration + window + retention layer." `(v1 framing over-reach)`

7. **"Only AI-coding security you can buy with a card" is factually false (Snyk).** **Corrected** to *cross-tool, AI-session-level, whole-team* governance above a free tier. `(false-as-written)`

8. **Show HN modeled as an expected 8–15k event; it's a 2.3% tail probability.** **Corrected** to modal "a few hundred visitors, 2.3% breakout chance." Same for PH 200–600 (conditional on winning #1). `(probability mis-stated as outcome)`

9. **"Aikido proves flat-per-team" is not in the corpus** (verified: appears only in the v1 docs). Flat decision is correct on other grounds; **demoted** the proof point. `(assumption — not in research)`

10. **Paid "spend-visibility panel" (P2 §3 Growth) is an honesty risk** — the product can't deliver accurate dollars (tokens undercount ~46×; PMC §11.6 caps spend to a *content hook only*). **Demoted** to relative/exposure framing; never invoiced. `(over-promise)`

11. **OP §5 schedules "proxy enforcement GA by M9"** — contradicts PMC §1.5/§11.5 (enforcement is "precision unproven, code-locked until proven, fail-open, enterprise-only"). **Flag:** do not ship GA in 9 months. `(contradicts locked decision)`

12. **Share rate / K-factor (>15–20%, K≈0.15–0.30) are unvalidated** — POS §7.6 itself calls them a *pre-build test*. Must be measured before the funnel is bet on it. `(assumption)`

13. **Donut freshness is un-instrumented** — depends on devs idling for the local `claude -p` judge; "≥25% touch a Growth trigger in 60 days" assumes the teaser reliably has data. **Instrument early.** `(assumption; code-dependency confirmed)`

14. **Enterprise "6 deals, $15–45k ACV" over-specifies an unsizable motion** — per-deal ACV is grounded; the *count*, M9–M12 timing (vs 90–150-day cycle), and re-imported sales motion are not. **Keep the company-domain seed signal; drop the banked deal count.** `(count invented)`

15. **Timeline vs precedent gap (unstated in both v1 docs):** the only applicable comps took 42–60 months to reach $50–85k MRR; the plan targets it in 12. **Temper toward the conservative end.** `(unstated evidence gap)`

16. **Free→paying-**team** conversion (1.2%→2.2%)** sits below RF's 3–6% freemium median, reasonably (team ≠ seat), but the ramp shape is `(assumption)`. **Least objectionable** of the contested rates.

17. **"Enterprise is re-activation, not greenfield" (OP §5) is half-true** — proxy/attestation/managed-settings exist, but **SSO/SCIM is greenfield** (auth is password-only). The costliest enterprise unlock isn't re-activatable. `(misleading frame; code-confirmed)`

18. **DPIA/LIA/eDiscovery generators** are spec'd (`design §9`) but generating code unverified in `policy_template.rs`/`policy_draft.rs`. **Confirm before promising enterprise compliance artifacts.** `(gap — low confidence)`

19. **Annual-prepay cash ($120–150k) and blended-ARPA mix (65/30/5)** are reasonable planning inputs, unbenchmarked `(assumption)`. Prepay cash is downstream of Stripe shipping in Q1 — don't bank it early.

20. **Founder ≥50%-to-dev in Q1** is my recommendation driven by the six net-new subsystems, not a corpus fact. `(assumption — not in research)`