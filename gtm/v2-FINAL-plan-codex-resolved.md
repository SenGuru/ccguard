## Codex objections — resolved

| Codex objection | Verdict | What changes in the plan |
|---|---|---|
| **Positioning is a thin conjunction** ("cross-tool + donut + card-pay") — buyers buy one painful job, not a conjunction | **PARTIAL** | Narrow the *headline* to one job — **"Find and prove the secrets your team leaked through Claude Code"** — and demote the conjunction from the pitch to the *defensibility* story (why an incumbent can't trivially copy us). Codex is right that you sell one job; the conjunction is a moat narrative, not a buyer message. |
| **The donut is not a retention layer** — its own evidence is demand 2/10, composite 0.6, "near-zero pull" | **ACCEPT** | Donut comes off the critical path and off the revenue model entirely. It becomes a **post-PMF experiment** (a `--label-personal` toggle we instrument), never a banked lever. The draft already half-conceded this; we finish the job. |
| **The paid pain ≠ the free pain** — free scanner solves "protect myself"; paying requires "govern my team," a different and weaker job | **ACCEPT (the core insight)** | This is the most important thing Codex said. We stop hoping the free→paid event happens organically and **manufacture a concrete, procurement-justifiable buying event**: a redacted **secrets-exposure evidence report** for SOC2 / customer security questionnaires / security review. That is an urgent, fundable team job — not "monitor your devs." Detailed in §2. |
| **Surveillance / trust kill-shot** — the moment it becomes a team dashboard it smells like employee monitoring | **ACCEPT** | Hard product changes: **agent-side scanning by default, no transcript upload unless explicitly enabled, dev sees own findings first, redaction-on-device.** Framing flips from "team dashboard" to "team evidence + rotation workflow." We sell "protect the team's secrets," never "see what your developers typed." |
| **Signups 4–10k, not 16–28k** | **PARTIAL** | Accept ~**4–10k** cumulative *self-serve scanner* users on pure organic — Codex is right the SEO/HN funnel is thinner than the draft modeled. **Reject** that this caps the business: the hybrid adds a *second, non-signup pipeline* (founder-led outbound to a known signal + partner-referred deals) that does not depend on scanner signup volume. |
| **Conversion 0.5–1.2%, not 1.2–2.4%** | **ACCEPT (for cold self-serve)** | Model cold free→paid-team at **0.6–1.0%**. We recover blended conversion not by disputing this number but by adding the higher-converting motions Codex himself recommends — reverse-trial (~24%) on the evidence-report gate, and paid-pilot→annual (60–90% per RF §7). |
| **Donut 3%/mo upgrade lever is fantasy** | **ACCEPT (reject the lever)** | Deleted from all math. |
| **Enterprise 0–1 by M12, not 6** | **ACCEPT** | The "6 enterprise deals" (~$45k ACV, SSO/SCIM, sales motion) is dead — we have no SSO, no SOC2, no sales team. **But** we replace it with **mid-market annual/pilot deals ($5–12k/yr, card or invoice, no SSO required)**, which is a *different, achievable* motion. Bank **0–1 true "enterprise"**; target 20–40 *annual team* deals. |
| **Share rate 1–5%, not 15–20%; K rounds to zero** | **ACCEPT** | Model **K ≈ 0**. Virality is removed as a growth assumption. The share card stays as a cheap top-of-funnel asset, not a funnel multiplier. People don't broadcast "my company leaked secrets" — Codex is obviously right. |
| **SEO is niche, attracts individuals not budget owners** | **PARTIAL** | Accept SEO is niche and individual-skewed; **reject** dropping it. Reframe SEO from *the team funnel* to *(a) credibility/proof and (b) individual-lead capture that feeds outbound* — when a dev installs from a company domain, that's an outbound signal, not a paid conversion. SEO is proof + lead source, not the revenue engine. |
| **The product is greenfield** — local scan, evidence report, Stripe, OAuth, invite, Cursor parser all net-new | **ACCEPT** | This is the whole business, not a detail. Q1 is rebuilt around shipping **the money path on a CC-only scope** (§6). Cursor and "cross-tool" claims are cut until the CC money path is live and demand is proven. |

**Net:** Codex is right on roughly 80% of the substance. The two places we push back — and defend below — are (1) the business is *not* capped at $14k because the draft already proposed light outbound that Codex's pure-PLG model ignores, and (2) the annual/pilot motion is *not* a betrayal of "no demos" if the buying event is a self-serve-generated evidence artifact rather than a live sales call.

---

## The reconciled strategy

**The central tension:** the founder wants pure self-serve (no demos, ~$0 ads) to reach $100k. Codex says pure self-serve organic cannot fill the funnel and wants founder-led outbound + partnerships + paid pilots + annual-first. Both are partly right. The honest resolution is a **hybrid with three layers**, where the outbound and partnership layers are *low-touch and asynchronous* — they do **not** require the live demos the founder is trying to avoid.

**The synthesis — what we keep, narrow, and change:**

1. **KEEP the self-serve PLG base.** Free, local, agent-side CC scanner → "holy crap, 3 live keys" → self-serve checkout. No salesperson required to transact. This honors the founder's core constraint.

2. **ADD light, founder-led outbound to a *known* signal (not cold enterprise sales).** The 21k indexed `.claude` repos + ~300–500 new orgs/mo (RF §5/§8) are a warm-ish, addressable signal. Founder sends *asynchronous, signal-triggered* email ("your org's public `.claude/` config matches the Miasma attack surface; here's a free 5-minute scan") — RF §8 puts signal-triggered reply at 15–25% vs 3.4% generic. **This is email, not demos.** It feeds the same self-serve checkout.

3. **ADD partnerships as the accelerator (the partner sells, we don't).** vCISO networks (one vCISO ≈ 30 clients; 67% of MSPs now offer vCISO) and AI-governance consultancies (McKenna, Clarista, DX Heroes + ~40 others sell $25–80k Claude Code governance engagements **with no tool to operationalize them** — a documented gap, RF §8). We are the tool. **They run the relationship; we collect the recurring revenue.** This is the single highest-leverage non-PLG channel and it requires zero demos from us.

**What we narrow:**
- **Claude Code only**, first. Drop "cross-tool," drop the Cursor claim (not in code), drop all cross-tool-depth messaging until the CC money path prints revenue. Cross-tool breadth is not worth lying or delaying the money path (Codex is right).
- **Donut = later experiment, not a core bet.** Off the roadmap critical path and off the MRR model.
- **Virality/K-factor = removed** as a growth assumption.

**How we solve the paid-conversion-event problem** (the kill-shot) — this is the strategic heart of v2:

The conversion event is **not** "invite your team to a monitoring dashboard." That triggers exactly the surveillance/trust/legal friction Codex names. Instead, the buying event is a **non-creepy, procurement-justifiable compliance artifact**:

- **Agent-side scanning is the default.** Findings are computed on each developer's machine. **No transcript content is uploaded unless a team explicitly enables aggregation.** This makes "nothing leaves your machine" *literally true*, kills the surveillance smell, and fixes the §5.5 honesty gap in one move.
- **The paid object is a redacted "Secrets Exposure Evidence Report"** — a per-repo, per-key, redacted artifact a team hands to a SOC2 auditor, a customer's security questionnaire, or an internal security review. It proves "we found, rotated, and now monitor leaked credentials in our AI tooling." *That* is an urgent, budgeted, defensible reason to pay — and it's framed as **protecting the team**, not watching developers.
- **Rotation workflow + repo attribution** ("which repo, not which person") turn the report into an actionable remediation deliverable.
- **Reverse-trial on this gate** (RF: ~24% vs 3–6% freemium) is the conversion mechanic.

This reframe — selling a compliance/evidence artifact instead of a surveillance dashboard — is what turns Codex's "$14k because the conversion event never materializes" into a conversion event that *can* materialize, while staying inside the founder's no-demo, protect-the-team guardrails.

---

## Re-baselined MRR projection

The draft's $85k and Codex's $14k are not actually that far apart once you separate *what they're modeling*. Codex models **pure self-serve PLG with zero outbound, zero partnerships, $99 ARPA**. The draft models self-serve PLG **plus** unbenchmarked donut upgrades and 6 invented enterprise deals. The honest reconciliation drops both fantasies, raises ARPA via annual-first pricing, and adds the *low-touch* outbound + partnership pipeline Codex himself recommended.

| Scenario | Self-serve teams | Annual/pilot deals | Blended ARPA | M12 MRR | What it requires |
|---|---|---|---|---|---|
| **Conservative** (Codex-aligned: PLG only, weak outbound) | ~40 | ~5 | ~$240 | **~$11–14k** | Free scanner works; conversion event weak; partnerships don't land |
| **Realistic (plan of record)** | ~55–70 | ~20–25 | ~$280 | **~$22–30k** | Evidence-report buying event converts; 1–2 vCISO/consultant partners active; reverse-trial + annual-first hold |
| **Aggressive** | ~80 | ~40 | ~$320 | **~$40–52k** | Partner channel resells at scale; pilot→annual hits 60–90%; one HN/PH breakout produces real buyers |
| **Ceiling** | ~120 | ~60 | ~$350 | **~$75–100k** | Everything above + multiple productive partners + a true breakout. ~5–10% probability. |

**The number I actually believe: ~$25k MRR at M12 (realistic band ~$22–30k).**

**Defense.** Codex's $11–14k floor is correct *for the motion he modeled* — pure organic PLG with a weak conversion event. I add ~$10–15k on top of that floor, and here's the grounded reason it's real, not hopium:
- The **annual/pilot motion** is the lever Codex *himself* endorsed (paid pilots $2.5k/30 days credited to annual; mid-market pilot→annual converts **60–90%** per RF §7). Just **20–25 annual deals at $5–10k/yr ≈ $8–17k MRR-equivalent** — from a pipeline (the 21k `.claude` signal + consultant referrals) that doesn't depend on scanner signup volume at all.
- The **evidence-report reframe** gives the self-serve base a buying event that survives the trust objection, lifting conversion off Codex's floor without disputing his 0.6–1.0% cold-conversion number.
- **Annual-first pricing** (below) roughly doubles ARPA vs the draft's $99, which is what makes both the math and paid acquisition viable.

**~$100k is very unlikely on this motion.** It requires either ~600 self-serve teams (absurd for a 3-person team in 12 months) or 15–20 meaningful annual/security deals (which starts to contradict the no-sales-team premise). **The realistic ceiling is ~$50k**, and even that is ahead of every applicable bootstrapped, no-audience comp in the research — Plausible took 42 months and Bannerbear 60 months to reach $50–83k MRR (RF §7). Hitting ~$25k in 12 months with no audience is already a *good* outcome.

**What would have to be true to beat ~$30k:** (1) the **partner channel actually resells** — even 2–3 productive vCISOs (≈30 clients each) changes the slope; (2) **pilot→annual converts at the high end** (80–90%); (3) **one launch breakout produces team buyers, not just GitHub stars** (the 2.3% HN front-page tail); (4) **annual ARPA holds at $5k+** as we move upmarket on the evidence-report value. If three of those four land, $40–50k is reachable. None of them is virality or donut upgrades.

---

## Revised pricing

Annual-first, evidence-report-anchored, ARPA-cleared-for-acquisition. Grounded in the secrets-WTP band ($5–15/dev standalone; $20–50/dev bundled team-governance, RF §6) and the comps (GitGuardian $30–117/dev, Snyk $52–98/dev, Semgrep $35/contributor).

| Tier | Price | Scope | What it is | Grounding |
|---|---|---|---|---|
| **Free (Door)** | $0 forever | 1 dev, local, agent-side scan | Unlimited local CC scans, full findings, redacted share card. **No upload.** The acquisition channel + outbound bait. | MT §4.3; PMC §12 |
| **Team (monthly entry)** | **$199/mo** (≤15 devs) or **$1,990/yr** | Team rollup + evidence report + rotation workflow + repo attribution | The self-serve magnet; annual saves ~2 months. Clears the ~$300 ARPA-with-mix floor when blended with Growth. | Bottom of bundled team band ($20–50/dev → ~$13/dev here) |
| **Pilot Pack** | **$2,500 / 30 days**, fully credited to an annual plan | Up to ~40 devs, hands-on evidence report for a security review | The **paid-pilot conversion motion** Codex recommended; self-serve checkout, no demo. Pilot→annual converts 60–90%. | RF §7 |
| **Growth / Business (annual-first)** | **$4,990–9,900/yr** (≤40 devs; overage banded) | Everything in Team + scheduled audits + SOC2-ready evidence pack + priority support | The annual deal the consultant/vCISO channel resells. The real ARPA driver. | $20–50/dev bundled band (RF §6); annual mid-market |
| **Enterprise (inbound-only ladder)** | ~$15–45k ACV | SSO/SCIM, managed-settings, enforcement, DPIA/eDiscovery | **Inbound-only. Bank 0–1 by M12.** Not on the critical path; the upsell for accounts that ask. | RF §6; greenfield SSO (§6) |

**Why this shape:** raising entry from the draft's $99 to **$199 + annual-first** roughly doubles ARPA, which is what makes founder-led outbound and (later) any paid newsletter spend viable at all — LinkedIn/cold motions only clear above ~$300/mo ARPA (RF §7). Per-seat is rejected (it's the procurement model the wedge dodges, RF §6). The donut is **not** a priced feature in v2.

---

## Revised GTM

A hybrid motion, sequenced. The self-serve loop is the base; outbound and partnerships are the accelerators that lift us off Codex's pure-PLG floor — all low-touch, no demos.

**Layer 1 — Self-serve loop, de-risked for trust (always on, M1+):**
`brew/npx/pipx install → local agent-side scan (no account) → "47 sessions, 3 live secrets" → redacted Exposure Report card → invite-to-team → generate Evidence Report → reverse-trial → card/annual checkout.`
The buying event is the **evidence report**, not a monitoring dashboard. No transcript upload unless aggregation is explicitly enabled. Dev sees own findings first. This is the trust-kill-shot mitigation baked into the funnel.

**Layer 2 — Founder-led outbound to the `.claude` signal (M2+):**
GitHub API → orgs with public `.claude/` dirs + identifiable company domains (21k indexed, +300–500/mo, RF §5/§8). Async, signal-triggered email tied to a real exposure data point (Miasma/TrustFall attack surface). Signal-triggered reply 15–25% (RF §8). **Email and a free scan link — not demos.** SEO/HN/PH leads from company domains feed this same list.

**Layer 3 — vCISO / AI-governance-consultant partnerships (M3+, the accelerator, RF §8):**
Consultancies (McKenna, Clarista, DX Heroes + ~40) sell Claude Code governance engagements **with no tool to operationalize them** — the documented gap we fill. vCISO networks (one ≈ 30 clients; 67% of MSPs offer vCISO). Offer a partner/referral plan on the annual tiers. **The partner owns the relationship; we collect recurring revenue.** Zero demos required of us. This is where the realistic-scenario upside lives.

**Layer 4 — Content/SEO as proof + lead source, not the team funnel (M1+):**
Comparison and how-to pages (`scan claude code history for leaked api keys`, `ggshield claude code alternative`) — accept Codex's point that these attract individuals. Treat them as **credibility + individual-lead capture that feeds Layer 2**, with a Q2 kill-switch on volume. Not modeled as the team-buyer funnel.

**Layer 5 — Launch channels to validate language + collect leads, not banked (M2, M3):**
Show HN and PH modeled at *modal* outcomes (a few hundred visitors; 2.3% front-page tail), used to validate messaging and capture leads. K ≈ 0; no virality banked. Pitch the data story (2.1× AI-commit leak rate), not a product ad. Newsletter (TLDR InfoSec, ~$167 CAC) only once revenue-funded.

**Grounded exclusions:** LinkedIn outbound (cost-per-SQL $400–3,000), virality as a growth lever, cross-tool messaging until CC money path is live, any spend-in-dollars claim (tokens undercount ~46×).

---

## Revised first build & roadmap

Codex is right that the product is greenfield where it matters most: the money path. Q1 is rebuilt around **the minimum that makes the conversion event real, Claude Code only.** Everything that exists but isn't on the path to the first dollar waits.

**Q1 (M1–3) — Ship the CC-only money path. Six net-new subsystems on a reusable engine:**
1. **`claresso scan` — local, agent-side, no upload.** Reuse `ccguard-core::findings` (real, tested, redacting) but **move the scan agent-side** (today it only runs server-side post-upload, `capture.rs:148`). This simultaneously delivers the local-first promise *and* kills the surveillance objection. **Highest-priority change in the whole plan.**
2. **Redacted Exposure Report card + `--share`** (top-of-funnel asset).
3. **The Evidence Report** (PDF/redacted export) — *the paid buying event.* SOC2/security-review-ready, per-repo attribution, rotation checklist.
4. **Self-serve onboarding: public signup + GitHub OAuth + team invite** (today auth is admin-password only, `users.rs:21`). PLG-non-negotiable.
5. **Stripe checkout + annual billing**, gated on the team/evidence-report boundary (zero billing code exists today).
6. **Team rollup** on the existing Maud dashboard (reuse).

**Explicitly cut from Q1:** Cursor parser (code supports Codex, not Cursor — don't claim it), the donut surface, cross-tool messaging, SSO/SCIM, enforcement. Slide the revenue curve right ~3–4 weeks vs the draft — this is real net-new engineering, and the founder should be **≥50% on dev** through Q1.

**Q2 (M4–6) — Convert + accelerate.** Reverse-trial on the evidence gate; partner/referral portal for vCISOs and consultants; **Cursor parser *only if* outbound/SEO demand proves it**; donut as an instrumented, optional `--label-personal` experiment (measure, don't bank).

**Q3 (M7–9) — Annual/mid-market hardening.** SOC 2 Type I (60–90 days, ~$15–30k — fund from revenue; it's the gating asset for annual deals and the evidence-report's own credibility). **SSO/SAML + SCIM** (greenfield) *only if* annual pipeline demands it. Re-activate the existing proxy/attestation/managed-settings for inbound enterprise.

**Q4 (M10–12) — Compound.** Scheduled audits, detector packs, Scale tier, partner co-marketing. Reconsider cross-tool breadth now that the CC money path is proven.

---

## The honest bottom line

**What this realistically becomes in 12 months:** a focused, profitable, **CC-only secrets-audit-and-evidence tool for engineering teams**, landing around **~$25k MRR (band $22–30k)**, with a ~$50k ceiling if the partner channel and annual motion overperform. That is a *good* bootstrapped outcome — ahead of every applicable no-audience comp in the research — but it is **not** the $100k self-serve rocket. The founder should internalize that now: report against ~$25k, treat $50k as a stretch win, and treat $100k as a launch-breakout lottery ticket, not a plan.

**The single biggest risk (Codex's kill-shot, accepted):** *the free tool works, but the paid conversion event never materializes* — devs run the scan once, rotate a key, and move on, and asking them to "invite the team" converts the product from "protect myself" into "monitor developers," which dies on trust, legal, and political friction. **The entire v2 redesign is a response to this one risk:** agent-side scanning with no upload by default, and a buying event (the redacted evidence report for SOC2/security review) that is *procurement-justifiable and protect-the-team-framed* rather than surveillance-framed. If that reframe doesn't convert, the business is Codex's $14k, not $25k.

**90-day go/no-go signals — watch these before over-investing:**
1. **Buying event is real:** ≥**8–12 paying teams** (self-serve + pilot) by day 90, *and* a meaningful share of them cite the **evidence report** as the reason they paid. If teams install but won't pay → the conversion event is dead → narrow further or pivot to a pure paid-audit consultancy.
2. **Trust objection rate:** track how many team-invite/aggregation attempts stall on surveillance concern. If >~30% stall even *with* agent-side/no-upload defaults → the kill-shot is winning → double down on "evidence artifact, never dashboard."
3. **Outbound works:** signal-triggered email to the `.claude` list replies at **≥10–15%**. Below that, the second pipeline isn't there and you're capped at the PLG floor.
4. **Partnership proof:** ≥**1–2 vCISO/consultant partners signed and ≥1 referred deal closed** by day 90. This is the highest-leverage upside signal — if it moves, push hard; if it's dead after real effort, model the conservative scenario.
5. **Activation:** ≥**40–50% of installs hit a real finding** (the "holy crap"). If the scanner rarely finds anything, there's no shock and no funnel.
6. **Virality:** measure K — **expect ~0**. Do not let a low number alarm you; it was never the plan. Alarm only if activation (signal 5) is also weak.

If signals 1, 3, and 4 are green at day 90, invest into the annual/partner motion and hire the 2nd dev before the SSO/SOC2 work. If 1 and 2 are red, stop building features and fix the conversion event before spending another dollar of runway — that is the whole ballgame.