# Product Marketing Context — CCGuard / "Claresso"

> Foundation doc. Every market-research, positioning, GTM, content, and copy task reads this first.
> Status: living doc. Last updated start of GTM workstream (2026-06-27).

---

## 0. The strategic question this workstream answers
CCGuard is currently architected as an **enterprise compliance/governance** product (MDM-deployed agent, `managed-settings.json`, SSO/SCIM, works-council/DPIA generators, eDiscovery). The founder wants to **pivot to self-serve B2B SaaS** — buyer signs up, adopts in minutes, pays by card, *no 1-on-1 demos* — and reach **~$100k MRR within 12 months** with a **3-person equity-based team** (1 full-time dev, 1 marketer, 1 founder who manages + builds part-time), bootstrapped, reinvesting every dollar for the first 4 months, with possible light ad spend.

**The pivot tension:** the enterprise feature set is the *opposite* of self-serve. The job of this research is to find the **smallest self-serve-able wedge** inside CCGuard that one person can adopt in 5 minutes without IT/legal — then build positioning, pricing, GTM, and a content engine around it. **The lead wedge is NOT pre-decided; the research decides it from real demand evidence.**

---

## 1. What the product does (today, factual)
A multi-tenant system that gives employers visibility + governance over employee use of **Claude Code** (and Cursor/Copilot/Codex — cross-tool by design). Built in Rust, 4 crates:
- **ccguard-agent** — on-device, parses Claude Code / Codex / Copilot transcripts, collects git provenance, runs local LLM triage via the user's *own* `claude -p` seat, attests device compliance.
- **ccguard-server** — Axum + Postgres control plane: ingest, classification, dashboard (Maud HTML), fleet/attestation, enforcement decisions.
- **ccguard-proxy** — transparent reverse proxy for the Claude API; the only place hard enforcement happens; fail-open by design.
- **ccguard-core** — pure logic: classification, provenance cascade, secret/PII findings scanner, enforcement gate, conformal calibration.

**Capabilities that already exist in code (candidate wedges):**
1. **Secret / PII scanner** (`findings.rs`) — detects AWS keys, GitHub/OpenAI/Anthropic/Stripe tokens, JWTs, private keys, PII in captured AI-coding sessions. *Research note: RESEARCH-FINDINGS.md §4 says secrets is the validated painkiller; work/personal is a market you must educate.*
2. **Work-vs-personal repo attribution** ("the donut") — classifies each AI-coding session as company-work / personal / unknown and reports spend split. *The unique differentiator; no competitor has it.*
3. **AI-coding spend & usage visibility** — tokens, cost, sessions, models per dev/repo (note: raw token counts are unreliable ~46× undercount; honest dollars only come lagged from Claude Enterprise Analytics API).
4. **Cross-tool capture** — same event shape across Claude Code, Cursor, Copilot, Codex.
5. **Enforcement** (proxy) — warm, one-click-recoverable blocking of over-allowance personal sessions. Enterprise-only, code-locked until precision proven.
6. **Compliance/evidence** — eDiscovery search, retention, consent, DPIA/LIA generators. Enterprise.

**Safety/design principles (brand-relevant):** content never leaves the machine; PERSONAL never silently flagged; fail-open; transparent (visible agent, never covert); dev sees own data before any manager aggregate.

---

## 2. Validated market facts (from RESEARCH-FINDINGS.md, June 2026 — treat as fact base)
- **Whitespace confirmed:** no tool captures + governs Claude Code *terminal* sessions specifically. Anthropic's own Compliance API + 60 partners explicitly **exclude Claude Code / Cowork**. Window before it closes: ~12–20 months.
- **Real 2026 incidents = ammunition:** Miasma worm (planted malicious config in 73 MS GitHub repos targeting `.claude/settings.json`, auto-exfiltrated cloud creds); TrustFall / CVE-2025-59536 (one keypress → RCE in Claude Code/Cursor/Gemini/Copilot, exfiltrates `~/.ssh`, `~/.aws`); GitGuardian SoSS 2026 — **AI-assisted commits leak secrets at 3.2% vs 1.5% baseline (2.1×)**; 24,008 secrets in MCP configs; AI-credential leaks +81% YoY.
- **Adoption:** ~85% devs use AI tools; ~18% global / ~24% NA on Claude Code/Cursor; Claude Code enterprise subs quadrupled H1 2026; enterprise AI-coding bills tripled to ~$7M/yr avg.
- **GitHub signal:** 21,000+ Claude Code repos indexed; ~300–500 new orgs/mo with `.claude/` dirs detectable via GitHub API.

## 3. Competitive landscape (to be deepened in Phase 1)
- **Anthropic Compliance API + 60 partners** (Purview, Netskope, Wiz, CrowdStrike, Relativity): claude.ai web chat only, **excludes Claude Code**. Enterprise.
- **Teramind** ($14–35/user/mo): screen/shell capture, no AI-response content, dev-toxic.
- **Microsoft Purview** (M365 E5 bundle): DLP/eDiscovery for Claude Enterprise web; not terminal replay.
- **GitGuardian** ($30–117/dev/mo; ~$45k ACV), **Snyk** ($52–98/dev), **Semgrep** ($35/contributor): secrets/SAST, post-commit, not session-level. *These are the secrets-wedge incumbents to position against.*
- **Coder AI Bridge, WitnessAI, Harmonic, Prompt Security, Nightfall, Cyberhaven, Portal26**: network/browser AI-DLP, shadow-AI; no terminal session capture.
- **Jellyfish / DX / Olakai / Faros**: AI cost/adoption analytics; no attribution, no governance.

## 4. Pricing comparables (factual)
GitGuardian $30–117/dev/mo (~$45k ACV) · Snyk $52–98/dev · Semgrep $35/contributor · CodeRabbit $24/dev · Cursor Business $40/seat · Copilot Enterprise $39/seat · ActivTrak $10–19/user · Teramind $14–35/user · Purview $10–60/user.

## 5. SaaS / self-serve benchmarks (factual)
SMB monthly churn 3–5%; freemium free→paid 3–6% median (reverse-trial ~24%); card-required trial 25–44% convert but 30–50% less volume; Google CPL $87–200; LinkedIn cost-per-SQL $400–3,000 (only viable >$300/mo ARPA); newsletter (TLDR InfoSec) ~$167 CAC at scale; Show HN ~2.3% front-page rate (~8–15k visitors if front page); PH #1 dev tool ≈ 200–600 signups.

## 6. Self-serve markets (factual)
GREEN to sell now: US, Canada-ex-Quebec, Singapore, Japan, Mexico, India (DPDP), Australia. RED/defer: EU, UK, Switzerland, Israel, Quebec, UAE free-zones. Best self-serve English markets (Stripe-native, high WTP): US, Canada, Australia, NZ, Singapore.

## 7. The team & constraints (drives every recommendation)
- 3 people, equity-based (no salary burn): 1 dev (full-time), 1 marketer (full-time), 1 founder (manage + part-time dev).
- Bootstrapped; reinvest 100% of revenue first 4 months; maybe light ad spend later.
- No connections, no audience, no warm intros at start.
- **Implication:** GTM must be self-serve + content/SEO/community-led + product-led, NOT enterprise sales. Every $100k-MRR path must be achievable by 3 people without a sales team.

## 8. Target outcome
~$100k MRR in 12 months. Example unit-economics anchors to pressure-test (Phase 5 finalizes): at $49/seat avg, $100k MRR ≈ 2,040 seats ≈ ~200–400 paying teams; at $99/seat ≈ 1,010 seats ≈ ~150–250 teams; at $299/team flat ≈ ~335 teams. Determines whether organic+light-ads can realistically fill the funnel.

## 9. Candidate ICPs (Phase 1/2 to validate & pick)
- (a) **Security/AppSec lead or DevSecOps engineer** at a 20–200-dev company already using Claude Code → secrets-leak wedge.
- (b) **Eng manager / VP Eng / founder-CTO** at a 10–100-dev startup → AI-spend visibility + work/personal wedge.
- (c) **Solo/lead dev** who wants to self-audit their own AI sessions → bottom-up free-tier land-and-expand.

## 10. Naming / brand note
Product brand appears as **"Claresso"** in user-facing proxy messages; repo/code name is **CCGuard**. Brand naming is open for revisiting in the brand phase.

## 11. Locked decisions (GTM workstream — updated 2026-06-27 after Phase 1 review)
1. **Work/personal "donut" = in-app FEATURE + upsell, NOT the acquisition wedge.** Founder accepts (notes it conflicts with original vision, but optimizing for market/revenue).
2. **Do NOT lead with a generic "secrets scanner"** — GitGuardian (#1 GitHub security app, $50M Series C, gives Claude Code secret hooks FREE) commoditizes it. Must find a sharper, differentiated positioning territory that dodges this. The strongest differentiator surfaced in Phase 1 is that **NO competitor offers true card-pay self-serve above their free tier** ("the only AI-coding governance you can actually buy with a card, no demo"). Phase 1.5 validates territories.
3. **$100k MRR in 12 months = STRETCH direction, not a hard line.** Goal: get as close as possible; a strong trajectory (e.g. $30–50k MRR + clear ramp) is an acceptable win. Phase 5 must defend the math honestly.
4. **Paid budget ≈ $100 MAX in month 1, thereafter only revenue reinvested.** => GTM must be ~100% ORGANIC / content / SEO / community / product-led. Positioning is only viable if it is content/SEO-winnable and supports a viral free-tier loop. Paid ads are a later, revenue-funded accelerant, never the engine.
5. **Self-serve SKU cut list (confirmed):** defer MDM/managed-settings, SSO/SCIM, DPIA/eDiscovery, proxy enforcement out of the front door → they become the upsell/enterprise ladder. Keep: agent self-install, local-first, dev-sees-own-data-first, session secret/PII findings.
6. **Spend/bill-shock = top-of-funnel CONTENT hook only** (loud demand, but undeliverable as honest product self-serve). Never the core product promise.
7. **Jurisdiction discipline:** target US / Canada-ex-Quebec / Australia / Singapore for all paid + content; EU/UK = inbound-only (RED).

## 12. LOCKED POSITIONING (after Phase 1.5 — 2026-06-27)
**Hard truth accepted:** secret/PII *detection* is already free (OSS plugins sensitive-canary/leakproof/Claudleak, GitGuardian free ggshield hooks ≤25 devs, Gryph audit trail) and Anthropic is shipping native Claude Code Security. Detection is NOT the moat. Defensibility of any single feature is low (2-3/10). The 12–20 month window is the clock.

**Strategy = acquire on the commodity, retain & defend on the moat:**
- **Acquisition DOOR (free, heavily searched, commodity):** session-level secret/PII leak detection for AI coding tools. Free local one-command scanner → shareable redacted "Exposure Report" card. Wins long-tail tool-seeking SEO (`scan claude code chat for leaked api keys`, `ggshield claude code alternative`, `stop cursor sending .env to AI`).
- **The MOAT (paid, un-copyable bundle):** cross-tool (Claude Code + Cursor + Copilot in ONE scan) + team dashboard/rollup + **work/personal attribution (the donut)** + **card-pay self-serve, no demo**. No free tool and not even Anthropic can honestly claim all of this in one breath. The donut — founder's original baby — is the single most un-copyable element; it can't be the door (demand 2, unsearched) but IS the retention/defense layer.

**Wedge sentence (no competitor can honestly say it):** "Finds leaked keys & PII across every AI coding tool your team uses — Claude Code, Cursor, Copilot — runs entirely on your machine, and is the only one you can turn into a team dashboard by paying with a card, no demo, in 5 minutes."

**Headlines:** Acquisition = "See every secret your AI coding tools already leaked — then stop the next one." · Conversion/pricing = "The only AI-coding security you can buy with a card. No demo. Live in 5 minutes."

**Content engine seed (10 articles + 1 viral free tool):** see `gtm/phase1b-positioning.md` §5. Flagship = "How to scan your Claude Code chat history for leaked API keys (free)" → routes to the free scanner. Viral loop = cross-tool Exposure Report card (out-shares single-tool scripts).

**Reference artifacts:** `gtm/phase1-market-truth.md`, `gtm/phase1-appendix.md`, `gtm/phase1b-positioning.md`.
