# CCGuard Positioning Teardown — Locking the Self-Serve Front Door

## 1. Composite Ranking (demand + defensibility + content/SEO, gated on the last two)

Gating rule applied: a territory's ceiling is set by the LOWER of `defensibility` and `content_seo`. High demand on top of a leaky moat or an un-rankable SERP is a trap under the $0-ad / 3-person constraint, because you'd pour organic effort into traffic you can't convert defensibly or rank for at all. I say so explicitly per row.

| Rank | Territory | Demand | Defens. | SEO | Raw | Gated verdict |
|---|---|---|---|---|---|---|
| **1** | **#4 Full-transcript secret & PII leak detection for Claude Code** | 7 | **3** | **4** | **14** | **Only territory that tops BOTH gating axes. The door.** |
| 2 | #5 Shadow-AI discovery & governance (SMB, no MDM) | 7 | 2 | 3 | 12 | **LOSES despite tied-highest demand** — def 2 AND CCGuard literally cannot deliver discovery (agent only sees machines it's installed on). High-demand mirage. |
| 3 | #2 SOC2-ready audit trail for AI coding | 5 | 2 | 3 | 10 | Loses on gating — Gryph gives it away free (OSS) and SOC2 doesn't even require the transcripts it sells. Enterprise-ladder upsell, not a door. |
| 4 | #3 Flight recorder / black box for AI coding | 4 | 2 | 3 | 9 | Loses — the "recording" is free native JSONL; pure content hook, not a product. |
| 4 | #6 AI-coding AUP policy-in-a-box | 5 | 2 | **2** | 9 | **Lowest SEO of all (2).** Pincered by free templates + Anthropic-native enforce + Drata/Vanta. Lead-magnet only. |
| 6 | #1 "Self-serve IS the wedge" | 2 | 2 | 3 | 7 | Lowest demand — a buying *motion*, not a searchable category. Closer, not acquisition. |

**Explicit gating calls:** #5 and #2 both out-score #6/#3/#1 on raw points but are demoted/killed because their gating axes (defensibility 2, plus a hard deliverability gap for #5) cannot survive a $0-ad organic build. Demand is necessary but not sufficient; defensibility and SEO-winnability are the constraints that actually bind a 3-person bootstrap.

---

## 2. YC-Office-Hours Teardown of the Top 2

### TOP 1 — #4 Secret & PII leak detection (full transcript, local)

**Strongest objection ("a free tool already does this"):** Brutal and TRUE as stated. `sensitive-canary` ships the EXACT pitch — "zero-config, fully local Claude Code plugin that intercepts secrets AND PII before they reach the API" — for $0. GitGuardian's free `ggshield` hooks cover prompt input + tool output (not "just 3 checkpoints"), commit to "free forever," and the free wall sits at ≤25 devs — precisely CCGuard's SMB sweet spot. Sieve, leakproof, Claudleak all reproduce the "holy crap, 3 keys" moment in a 60-second brew/pip install. **The "better detection / full transcript, not 3 hooks" differentiator does NOT survive. It is already free.**

**Does the territory survive anyway? YES — but only when amputated from the detection claim.** The pain (demand 7) is real, searched without education, and incident-backed (npm `.claude/settings.local.json` leak, 294k secrets, GitGuardian 2.1× AI-commit leak rate). The door survives; the *differentiator* must move. Detection becomes the free-tier hook and SEO magnet, and the defensible, buyable layer is what the locked decisions already point at: **cross-tool team rollup + dashboard + work/personal attribution ("the donut") + card-pay self-serve no-demo.** That bundle is the one thing the free OSS one-offs and the contact-sales incumbents *cannot both* claim. Survives, reframed.

**Second objection ("can't win this organically"):** Partly real — GitGuardian's content engine + a dozen funded security blogs own the fat head terms. But the long tail is winnable: `scan Claude Code chat history for leaked API keys`, `ggshield Claude Code alternative`, `stop Cursor sending .env to AI` are tool-seeking, low-authority-competition queries a 3-person team can rank for with a free interactive tool as the link magnet. SEO score 4 is the highest in the set for a reason. Survives.

### TOP 2 — #5 Shadow-AI discovery (SMB, no MDM)

**Strongest objection ("we can't actually deliver this / a free tool already does this"):** FATAL, and it's not even close. CCGuard's voluntary on-device agent only sees machines where it's already installed — i.e. the *sanctioned* context. True shadow-AI discovery needs network/IdP/browser/MDM telemetry CCGuard does not have. Nudge Security already owns the agentless "no-MDM" wedge free-trial; Codacy AI Inventory does free repo-scan discovery with no agent; Microsoft Agent 365 bundles it native for M365. **CCGuard would be selling "discovery" while structurally being the worst-positioned tool to discover anything.** Does not survive. This is a category CCGuard cannot honestly stand in — drop as lead, keep at most as a top-of-funnel content angle routing to the deliverable secrets wedge.

**Verdict of the teardown:** #4 survives amputation and reframing; #5 fails on deliverability, which is the one objection you can't out-content. Winner is clear.

---

## 3. THE WINNER

**WINNER: Territory #4 — session-level secret/PII leak detection for AI coding — as the DOOR, repositioned onto the buyable layer: "the AI-coding secret & governance tool you can actually buy with a card, no demo, live in 5 minutes — across every tool your team uses."**

This is the synthesis both verdicts in the data converge on: lead the *wedge* (#4's searched, incident-backed secrets pain), drop the *differentiator* (better detection — it's free), and bolt on #1's card-pay closer plus the donut as the in-app upsell. It is the only configuration that is simultaneously (a) high-demand and searched without education, (b) the highest defensibility + SEO scores in the field, and (c) fully deliverable by the existing `findings.rs` + cross-tool capture + attribution code.

**Why it beats the runner-up (#5 Shadow-AI) given the constraints:**
- **Deliverability:** CCGuard can ship #4 today from existing code; it can *never* honestly ship #5's discovery promise. You cannot content-market your way out of a product that structurally can't do the job.
- **No-ad-budget fit:** #4 owns tool-seeking long-tail queries (`scan Claude Code chat for leaked keys`) with low-authority competition; #5's head terms are a red ocean of funded incumbents (Zenity, Reco, Palo Alto) that out-rank a 3-person team.
- **Anti-commoditization:** #5 is commoditized AND undeliverable — the worst quadrant. #4's commoditized *part* (detection) becomes the free top-of-funnel loop, while the moat moves to the un-copyable bundle: cross-tool + work/personal attribution + card-pay self-serve. No competitor can honestly say all three.
- **Self-serve fit:** #4 = high (dev installs in 5 min, sees own data first); #5 = low (needs org-wide telemetry, IT/security-led). #4 matches the pivot; #5 re-imports the enterprise motion the pivot is escaping.

---

## 4. Three Candidate Headlines (5-second-stranger test) + subheads

**A. "See every secret your AI coding tools already leaked — then stop the next one."**
*Subhead: Scan your Claude Code, Cursor, and Copilot sessions for exposed API keys, tokens, and PII. Free, local, 5-minute install — nothing leaves your machine.*

**B. "The only AI-coding security you can buy with a card. No demo. No sales call. Live in 5 minutes."**
*Subhead: Secret & PII leak detection across every AI coding tool your team uses — self-serve from free to team dashboard, the moment you decide.*

**C. "Your AI coding agent is leaking keys at 2× the human rate. Find out where in 5 minutes."**
*Subhead: Cross-tool secret & PII scanning for Claude Code, Cursor, and Copilot — runs locally, shows you your own exposure first, then your whole team's.*

> Lead recommendation: **A** as the acquisition headline (problem-first, searched), **B** as the pricing-page / conversion headline (the card-pay closer where it converts best).

---

## 5. Organic Content / SEO Engine Seed ($0 ads)

**10 high-intent articles/pages to build (mapped to live, low-competition tool-seeking queries):**
1. **"How to scan your Claude Code chat history for leaked API keys (free)"** — target `scan claude code chat history for leaked api keys`; the flagship, routes into the free tool.
2. **"ggshield for Claude Code: what it covers, what it misses, and the free alternatives"** — target `ggshield Claude Code alternative`; honest comparison, captures incumbent-aware traffic.
3. **"Stop Cursor and Claude Code from sending your .env to the cloud"** — target `stop Cursor sending .env to AI` / `prevent secrets leaking into AI prompts`.
4. **"Where does Claude Code store your session history? (and what's hiding in it)"** — target the how-to query, pivot from native JSONL into "what secrets are in those transcripts."
5. **"AI coding tools leak secrets at 2.1× the baseline rate — the 2026 data"** — target `do AI coding tools leak secrets`; cites GitGuardian SoSS, npm `.claude/settings.local.json` incident, 294k-secrets stat. Linkbait.
6. **"DLP for Claude Code: secrets + PII, fully local — a practical guide"** — target `DLP for Claude Code secrets PII`.
7. **"sensitive-canary vs leakproof vs ggshield vs CCGuard: the free AI-coding secret scanners compared"** — own the comparison SERP honestly; converts on cross-tool + team dashboard.
8. **"The Miasma worm and TrustFall (CVE-2025-59536): how AI coding configs exfiltrate your ~/.aws and ~/.ssh"** — incident explainer, high shareability, routes to scanner.
9. **"Auditing your whole team's AI coding secret exposure without an enterprise sales call"** — target `ai coding security tool no enterprise sales` / card-pay framing; the conversion bridge to paid.
10. **"Work vs. personal: how to tell which AI coding sessions are on the company card"** — seeds the donut upsell + an un-copyable angle no competitor ranks for.

(Stretch 11–12: "AI coding secret scanner that works with SOC 2 evidence" and "Claude Code secret detection plugin: setup in 5 minutes" — bridge content into the enterprise ladder.)

**The 1 free interactive tool / free-tier viral loop:**
**"Scan Your Own AI Sessions" — a one-command local scanner** (`brew`/`pip`/`npx`) that reads the user's existing Claude Code / Cursor / Copilot transcripts and returns a shareable, redacted "Exposure Report": *"We found 3 live secrets and 2 PII strings across 47 sessions in the last 30 days."* The "holy crap, 3 keys" moment is the share trigger; the redacted report card is the artifact that spreads on HN/Reddit. Local-first (nothing leaves the machine) is the trust unlock that the OSS one-offs *also* have — so the loop's defensible pull is **cross-tool in one scan + the team rollup upsell + card-pay**, which the single-tool free scripts cannot match. This is the land; the team dashboard + attribution is the expand.

---

## 6. The Wedge Sentence (no competitor can honestly say it)

**"CCGuard finds the leaked keys and PII across every AI coding tool your team uses — Claude Code, Cursor, and Copilot — runs entirely on your machine, and is the only one you can turn into a team dashboard by paying with a card, no demo, in 5 minutes."**

(GitGuardian/Snyk can't say "no demo / card-pay above free + cross-AI-tool session capture"; the free OSS one-offs can't say "cross-tool + team dashboard + work/personal attribution." Only CCGuard truthfully says all of it in one breath.)

---

## 7. Open Questions for Phase 2

1. **Free-tier size / gate placement:** Where exactly does free stop? (per-user local scan free forever vs. dev-count cap vs. "personal data free, team aggregate paid")? GitGuardian's free wall sits at ≤25 devs — do we undercut by gating on *team dashboard* rather than dev count, so the individual scan is always free and the loop never breaks?
2. **First buildable artifact:** Is the v1 the standalone free local scanner (fastest viral loop, weakest moat) or the scanner-with-account that auto-creates the team dashboard (slower, but the expand path)? Recommend scanner-first, account-optional, with a one-click "invite your team" that opens the paid rollup.
3. **Price point & unit:** $/seat vs flat $/team for the dashboard? Context anchors suggest $49–99/seat or ~$299/team flat. Which hits the ~150–400-team target for the $30–100k MRR trajectory without LinkedIn-CAC economics (ARPA must clear ~$300/mo to ever justify paid)?
4. **Donut timing:** Does work/personal attribution ship in the paid v1 as the headline upsell, or stay a phase-2 expand feature? It's the one un-copyable moat — how early do we expose it without inviting a fast-follow?
5. **Anthropic-native risk clock:** Issue #29328 (native in-chat secret masking) + Claude Code Security are live threats to the *detection* layer. How do we explicitly sequence content/SEO to bank the cross-tool + team + card-pay moat before native detection commoditizes the door entirely? (12–20 month window per the fact base.)
6. **Defensible-virality test:** Before building, can we validate that the cross-tool single-scan report demonstrably out-shares the existing free single-tool scripts? Cheap test: ship the scanner, measure share rate vs. a Claude-Code-only variant.