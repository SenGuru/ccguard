# CCGuard / Claresso — Market-Truth Report (Phase 1 Synthesis)

*Head of Market Strategy · 2026-06-27 · Decision-grade. Drives positioning, pricing, and the content engine.*

---

## 1. Executive Summary

**The single most important finding:** every serious competitor either (a) cannot see inside a Claude Code *terminal session* at all (Snyk, Purview, the entire Anthropic Compliance-API partner stack), or (b) sees only fragments of it — three hook checkpoints (GitGuardian), post-file-write scans (Semgrep), or blurry screen-OCR (Teramind) — and **none of the ones that get close offer true card-pay self-serve.** The whitespace is real, but it is bounded: it sits *underneath* a layer of free/OSS tools and *above* a wall of enterprise procurement, so the winnable market is narrow and the framing must be surgical.

**Recommended lead wedge:** lead self-serve with **session-level secrets/PII leak detection for Claude Code** — "see every credential that left your terminal in an AI session, in 5 minutes, no IT" — not generic "AI security monitoring" (red ocean) and not "spend visibility" (structurally undeliverable without Claude Enterprise + commoditized to $0 by Anthropic's own native analytics). Use **bill-shock/spend as the top-of-funnel content magnet** (highest search volume and emotion) that converts into the secrets product, then upsell the work-vs-personal "donut" and enterprise compliance. The lead ICP is the **security-conscious lead dev / DevSecOps engineer at a 20–200-dev company already running Claude Code** in a GREEN, Stripe-native market (US, Canada-ex-Quebec, Australia, Singapore).

---

## 2. TAM / SAM / Beachhead

Sized bottom-up. All derived figures marked **(inferred)**; anchor facts are from the product-marketing-context fact base (§2, §6, §8).

### TAM — all Claude Code / Cursor developers, globally
| Step | Figure | Source |
|---|---|---|
| Professional developers worldwide | ~30M **(inferred)** | industry baseline |
| × ~85% use AI coding tools | ~25.5M | fact base §2 |
| × ~18% on Claude Code / Cursor (global) | **~4.6M devs** | fact base §2 |
| TAM $ at blended $49/seat/mo | **~$2.7B/yr** **(inferred ceiling)** | 4.6M × $49 × 12 |

### SAM — self-serve-reachable, GREEN jurisdictions, 10–200-dev teams
| Step | Figure | Reasoning |
|---|---|---|
| GREEN + Stripe-native, high-WTP English subset (US, CA-ex-QC, AU, SG) share of CC/Cursor devs | ~45% → **~2.07M devs** **(inferred)** | NA adoption runs higher (~24% vs 18% global, §2); Claude Code skews US-heavy |
| × ~55% sit in 10–200-dev orgs (exclude solo-free + 200+ enterprise-procurement) | **~1.14M reachable seats** **(inferred)** | self-serve buyer band per §0, §9 |
| SAM $ at $49/seat/mo | **~$670M/yr** **(inferred)** | 1.14M × $49 × 12 |
| As orgs | ~21,000 indexed CC repos today + ~300–500 new `.claude/` orgs/mo (~3,600–6,000/yr) | fact base §2 — the real PLG funnel signal |

### Beachhead — Year-1 obtainable ($100k MRR / $1.2M ARR)
- At $49/seat: $100k MRR ≈ **2,040 seats ≈ 200–400 paying teams** (5–10 paid seats/team). Anchor from §8.
- Reachable identifiable-org pool Year 1: ~21,000 existing + ~4,000–6,000 new ≈ **~25,000 orgs**. Hitting 200–400 paying teams = **~1.5–2.5% paid conversion of identifiable orgs (inferred)** — aggressive but inside PLG norms (freemium free→paid 3–6% median, §5) *if* the free tier is genuinely viral.
- **Beachhead definition:** security-conscious **20–200-dev Claude Code teams in US / Canada-ex-QC / Australia / Singapore**, landed via a free self-audit, expanded on the first org-wide secret finding.

> **Pricing reality check:** the funnel only fills organically (no sales team, §7) if free→paid conversion clears ~2%. That is the gating assumption Phase 5 must defend.

---

## 3. Competitor Gap-Map

| Competitor | Self-serve? | Covers CC terminal? | The gap we exploit |
|---|---|---|---|
| **GitGuardian** | Freemium→sales (free ≤25 devs; Business = "Let's Talk") | **Partial** — 3 hook checkpoints (pre-prompt/pre-tool/post-tool), no transcript | Full session transcript + work/personal attribution + spend; **true card-pay self-serve past 25 devs** (their hard sales wall) |
| **Snyk** | Freemium→sales (self-serve hard-walls at 10 devs; SSO = Ignite) | **No** — post-commit/code-artifact only | Structurally blind to the session layer; "Snyk starts where the breach already happened — we catch it at the prompt" |
| **Semgrep** | Freemium→sales (**Teams = "Contact us," not self-serve**) | **Partial** — post-file-write scan only | Secrets-in-prompt *before* a file is written; no session capture; we are the layer beneath Semgrep |
| **Teramind** | Freemium→sales (endpoint agent, IT-led deploy) | **Partial** — terminal-title detection + screen OCR; brittle, **Windows-only full features** | Structured transcript vs blurry OCR; Mac/Linux (where CC lives); **dev-transparent, not dev-hostile surveillance** |
| **Microsoft Purview** | Freemium→sales (requires M365 E3/E5 + Claude Enterprise) | **No** — Microsoft's own docs exclude CC CLI; Enterprise web = metadata only | Terminal content capture with real secret/PII findings; **zero M365 dependency**, 5-min install |
| **Anthropic Compliance API + partners** (Wiz, Netskope, Nightfall, CrowdStrike) | **Sales-led only** | **Partial** — architecturally excludes CC CLI (local process bypasses server-side API); Nightfall = MCP-only | The local CLI's traffic never touches the Compliance API; we capture the actual transcript + git provenance on-device |
| **Shadow-AI bucket** (Portal26, WitnessAI, Prompt Security, Harmonic) | **Sales-led only** ($60k–$250k+/yr) | **Partial** — OTel/MDM-gated raw telemetry; no self-install | 5-min developer self-install vs MDM push; git provenance + work/personal attribution; **SMB price under $100/mo** |

**The single clearest opening (3 sentences):** There is a structurally underserved 20–200-developer company that has already outgrown the free OSS scanners but is *categorically locked out* of every enterprise option — too small for Purview/CrowdStrike/Nightfall procurement, blocked by GitGuardian's and Semgrep's "Contact us" walls, and unwilling to deploy Teramind surveillance over its own engineers. That buyer can answer "is a secret leaking into our repos?" with free tools but **cannot answer "what is Claude Code actually sending out of this terminal, before any commit?"** — the one question the whitespace research confirms no product answers. CCGuard owns that question if, and only if, it ships the one thing all seven competitors fail at simultaneously: a genuinely self-serve, card-pay, no-IT install at the session layer.

---

## 4. Self-Serve / PLG Lessons to Copy

From GitGuardian, Snyk, and Semgrep's motions — steal these:

1. **GitHub OAuth / social login at signup** (all three) — authenticated in <30s with an existing identity; zero new credentials. Non-negotiable.
2. **GitHub Marketplace listing as a discovery channel** (GitGuardian #1 security app) — be present at the point of developer intent, one-click install, no website visit. Pair with the 21,000-repo `.claude/` GitHub-API signal for targeted outbound-content.
3. **A genuinely useful, permanent free tier sized to a real team** — GitGuardian (25 devs), Semgrep (10 contributors). This is the virality engine; size CCGuard's free tier to 1–5 devs with *full session capture* so individuals get real value before any paywall.
4. **CLI / agent-skill-as-land, free, working before money changes hands** — ggshield AI hooks (MIT), Semgrep OSS CLI (60-second `brew`/`pip` install). The land motion must cost the user nothing and the buyer no approval.
5. **Pre-loaded demo project on first run** (Semgrep) — show a synthetic Claude Code session with detected AWS keys/JWTs *before* the user connects their own data. First "wow" must precede setup.
6. **Reverse-trial / PQL triggers** (Snyk, Semgrep) — auto-upgrade prompt when the user invites a teammate or hits the first org-wide finding; conversion is driven by product success, not a sales email.
7. **Programmatic-SEO live tracker at near-zero CAC** (Snyk Open Source Advisor) — build a public "AI session secret-leak checker" / "leaked secrets in AI commits" tracker to capture the exact queries vendors are already paying to rank for.

**The lesson that is also the wedge:** GitGuardian, Semgrep, and Snyk *all* fail at true self-serve above their free tier — every one hits a "Contact us" / sales-demo wall. **CCGuard's differentiated PLG act is to actually let a team pay by card.** That alone is a positioning line.

---

## 5. Demand-Ranked Wedges

Ranked by **demand_score × self_serve_fit** (fit weighting, inferred: high = 1.0, medium = 0.6, low = 0.3).

| Rank | Wedge | Score | Self-serve fit | Composite |
|---|---|---|---|---|
| 1 | AI-coding **spend & usage visibility** | 6 | medium (0.6) | **3.6** |
| 2 | **Secrets / credential leakage** via AI tools | 5 | medium (0.6) | **3.0** |
| 3 | **General** Claude Code security & monitoring | 5 | low (0.3) | **1.5** |
| 4 | **Work-vs-personal** acceptable-use ("the donut") | 2 | low (0.3) | **0.6** |

**1. Spend & usage visibility (composite 3.6).** *Who:* eng manager / VP Eng / founder-CTO at 10–150-dev startups on usage-based plans. *Searched-for?* Strongly — eng managers literally post "Any tools for tracking usage per dev?" and "our token bill hit six figures." *WTP:* the spend is huge and ROI math is obvious, but WTP for the *tool* is pressured to $0 by ccusage, LiteLLM, free New Relic, and **Anthropic's own native Analytics API**. *Verdict:* **highest demand, worst defensibility — and CCGuard cannot even deliver it honestly** (raw token counts undercount ~46×; accurate dollars require the Claude Enterprise Analytics API the SMB doesn't have, §1.3). **Use as a content/acquisition hook, not the core product promise.**

**2. Secrets / credential leakage (composite 3.0).** *Who:* AppSec/DevSecOps/platform-security engineer at a 20–200-dev company answering for a policy, SOC2 audit, or post-incident; secondarily the bill-shocked indie dev. *Searched-for?* Yes — genuine question-form demand ("how are you preventing secrets from leaking into prompts?"), backed by real 2026 incidents (Miasma, TrustFall, SoSS 2.1×). *WTP:* weak standalone (~$5–15/dev; GitGuardian gives hooks free, OSS exists), real when bundled into team governance (~$20–50/dev). *Verdict:* **the validated painkiller (RESEARCH-FINDINGS §4), closest to CCGuard's defensible moat (full transcript vs 3 hooks), and self-serve-able — the right lead if framed at the session/transcript level, not as "another secret scanner."**

**3. General security & monitoring (composite 1.5).** *Who:* DevSecOps at regulated/security-conscious orgs. *Searched-for?* Yes, but the acute buyer is procurement-led, the opposite of self-serve. *WTP:* concentrated in enterprise (SOC2-driven), undercut to $0 for the generic version by New Relic/Gryph/Dev Machine Guard. *Verdict:* **do not lead — red ocean of funded incumbents + free OSS; it buries CCGuard's moat under a claim everyone makes.**

**4. Work-vs-personal "donut" (composite 0.6).** *Who:* nominally eng/FinOps/HR, but the loud persona is the *employee hiding* a side project — the inverse of a buyer. *Searched-for?* No — must be educated into existence; native Cursor/Copilot/Claude admin dashboards already surface repo-level usage free. *WTP:* near-zero standalone. *Verdict:* **never the lead. A differentiating *feature* and upsell, not a wedge.**

---

## 6. Recommended Lead Wedge + ICP

### Lead wedge: **Session-level secrets/PII leak detection for Claude Code**
Framed precisely as: *"See every API key, token, JWT, SSH key, and PII string that left your terminal in a Claude Code session — the full transcript, not three hooks or a blurry screen recording. 5-minute install, your data never leaves the machine, no IT, no demo."*

**Why this over the higher-scoring spend wedge — the decisive reasoning:**
1. **Deliverability.** CCGuard *cannot honestly ship* the spend wedge in self-serve: token counts undercount ~46× and accurate dollars require the Claude Enterprise Analytics API the SMB buyer doesn't own (§1.3). Secrets detection runs fully on-device, today, in `findings.rs`.
2. **Defensibility.** Secrets is where CCGuard's moat is *real*: GitGuardian's free hooks see only 3 checkpoints, OSS scanners parse history files post-hoc — **CCGuard captures the full session transcript with git provenance**, which is differentiated from every free option. Spend tracking is undifferentiated token-counting that Anthropic ships natively.
3. **Validation + ammunition.** RESEARCH-FINDINGS §4 names secrets the validated painkiller; the 2026 incident set (Miasma, TrustFall/CVE-2025-59536, SoSS 2.1× leak rate) is all secrets/exfiltration — a ready-made content arsenal.
4. **Clean upsell ladder** (below).

**Exact lead ICP persona:** *"Sam, the security-owning senior/lead engineer."* A **lead developer or DevSecOps engineer at a 20–200-dev company already running Claude Code**, in the US / Canada-ex-Quebec / Australia / Singapore, who has been handed responsibility for "make sure we're not leaking keys into AI tools" for a SOC2 audit, a new AI-use policy, or after a scare — has budget influence but no desire for a 3-month enterprise procurement, and will pay by card to make the question go away. (= Candidate ICP (a), §9.)

**Why this ICP and not the eng-manager (spend) ICP:** the eng manager's pain is louder but their solution is free/native; Sam's pain is specific, audit-driven, recurring, and **not** answered by the free tools once you require *session-level* evidence. Sam is also the persona who converts the "dev-sees-own-data-first" trust model into an internal champion (the anti-Teramind motion).

**Upsell path:**
- **Land (free):** solo/lead dev self-audits their *own* Claude Code sessions, sees secret findings in <5 min (PQL: first finding).
- **Expand (paid team, $49/seat or $299/team flat — Phase 5 to test):** team-wide session secret governance + dashboard (PQL trigger: teammate invite or first org-wide finding).
- **Upsell to the differentiator (the "donut"):** once a team trusts the session-capture, work-vs-personal attribution becomes the "while you're here, here's where your AI budget actually went" expansion — sold *into* an existing account, never as the door-opener.
- **Enterprise:** compliance/evidence layer (eDiscovery, retention, DPIA), proxy enforcement, SSO/SCIM — kept entirely out of the self-serve path.

---

## 7. Voice-of-Customer Language Bank

Real phrases the ICP uses — for copy, landing-page headlines, and SEO targets:

1. "The **.claude and .cursor directories are the new .env files**."
2. "This is a **real blind spot** in most dev workflows right now."
3. "**How do you prevent credential leaks to AI tools?**"
4. "How are you **preventing secrets from leaking into prompts?**"
5. "**My agent stole my (api) keys** … he pulled out my API keys like it was nothing."
6. "**scan Cursor/Claude chat history for leaked API keys**"
7. "Am I **overthinking Claude Code security** or is this actually a risk?"
8. "How do you **handle security/monitoring of Claude Code** in your org?"
9. "what are these agents **actually doing on dev laptops, and can we prove it for SOC2?**"
10. "**Nobody has a good answer yet for how to govern AI coding.**"
11. "**burning through Claude / Cursor credits like crazy**" *(spend hook)*
12. "our **token bill hit six figures**" / "**Cursor charged us $1400 in one hour**" *(spend hook)*
13. "we track **AI spend per person and cost per commit**" *(spend hook)*
14. "**Start with a policy and centralization around a tool** … paid for and signed off."

*(11–14 = top-of-funnel bill-shock magnets that route to the secrets product.)*

---

## 8. Risks & Open Questions for Phase 2

**Positioning**
- **Commoditization risk (high):** "secrets scanner" reads as me-too against free GitGuardian hooks + OSS. Mitigation must be relentless: lead on **full session transcript / "what left your terminal"** and **true card-pay self-serve**, never on detection breadth. *Open Q: can we hold that distinction in a single headline a stranger understands in 5 seconds?*
- **Native Anthropic threat (existential):** Anthropic's Compliance API + Analytics API are expanding and explicitly exclude CC terminal *today* — but the 12–20-month window assumes they don't ship terminal capture themselves. *Open Q: what is the moat if Anthropic closes its own gap? (Answer likely: cross-tool + work/personal + local-first + dev-trust, none of which Anthropic will build.)*

**Pricing**
- Standalone secrets WTP is ~$5–15/dev; the $49 blended anchor needs the team-governance/dashboard framing to hold. *Open Q: per-seat ($49) vs team-flat ($299) — which converts better self-serve and survives churn (SMB 3–5%/mo)?* Test both in Phase 5.
- Free-tier sizing is the whole funnel: too generous = no conversion, too thin = no virality. *Open Q: 1 dev or 5?*

**What to cut from the enterprise product for self-serve**
- **Cut/defer from the self-serve SKU:** MDM / `managed-settings.json` deployment, SSO/SCIM gating, DPIA/LIA generators, eDiscovery, consent/works-council tooling, and **proxy enforcement** (precision unproven, enterprise-locked, fail-open). All are anti-self-serve and slow time-to-value.
- **Keep as the self-serve core:** agent self-install, local-first (content never leaves machine), dev-sees-own-data-first, session secret/PII findings. These *are* the wedge.

**Market / GTM**
- **Spend wedge is a trap as a product** (undeliverable + commoditized) but the **best acquisition hook** — *Open Q: how much spend functionality must we ship to make the bill-shock content credible without owning the dollar-accuracy problem?* Likely answer: ship session *counts/relative* signals, never authoritative dollars.
- **Jurisdiction discipline:** much of the loudest secrets/DLP framing (SoSS, EU AI Act) is EU-regulated (RED). Ensure all paid-acquisition and content target **US/CA-ex-QC/AU/SG**; treat EU as inbound-only.
- **The donut has near-zero pull (demand 2):** confirm it stays upsell-only. *Open Q: is there ANY trigger event (IP-ownership dispute, FinOps audit) that ever makes work/personal a lead — or is it permanently a feature?* Current evidence says permanently a feature.