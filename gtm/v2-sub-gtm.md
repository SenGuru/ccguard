I have the full corpus. Writing the v2 GTM now, grounding every channel claim and flagging where v1 over-reached.

---

# CCGuard / Claresso — GTM v2: The PLG Loop, Ranked $0-Ad Motions, and the Launch Sequence

*PLG/GTM re-grounding · 2026-06-27. Built on the locked positioning (PMC §11/§12, P1B §3) and re-checked against the fact base. Every channel number traces to RESEARCH-FINDINGS (RF) §8/§2/§5/§7 or a phase doc, or is marked **(assumption — not in research)**.*

**File key:** RF = RESEARCH-FINDINGS.md · PMC = product-marketing-context.md · P1 = phase1-market-truth.md · P1A = phase1-appendix.md · P1B = phase1b-positioning.md · P2 = phase2-gtm-blueprint.md · P3 = phase3-operating-plan-100k.md · TS = research/tracking-surface.md

---

## 0. What v2 changes vs v1 (the headline corrections)

v1 (P2 + P3) is directionally right on positioning and the loop, but it **launders three assumptions into the plan as if they were research-backed**, and it contains an internal contradiction. v2 keeps the loop, re-grounds the channels, and corrects these:

1. **The forecast contradiction.** P2 §3/§7 honestly lands at "**$30–50k MRR defensible base case, plan of record $50k; $100k is the stretch ceiling.**" P3 then re-bases the *plan of record* to **~$85k MRR** on the back of an expansion ladder (3%/mo donut upgrades, 6 enterprise deals, 1.2%→2.2% conversion ramp) for which **there is zero research support** — none of those rates appear in RF or any phase-1 doc. **v2 reverts the plan of record to P2's $30–50k band** and treats P3's $85k as a clearly-labelled upside model, not a forecast. (See §6.)

2. **Show HN framing.** v1 (P2 §4 launch table) writes "Show HN ~2.3% front-page → 8–15k visitors" and P2 §3 banks "1–2 Show HN front-page hits." RF §8/§5 actually say the front-page **rate** is ~2.3% and the 8–15k visitors materialise **only if** a post reaches the front page. **2.3% is a probability, not an expected outcome.** A single Show HN post should be modelled as *most likely a few hundred visitors, with a ~2.3% tail chance of a 8–15k breakout* — not as a planned 8–15k event. (See §5.)

3. **SEO traffic numbers are invented.** v1's "~3,000–6,000 organic tool-signups/mo by month 12" (P2 §3) and "~6,000–7,500/mo by M12 / ~25k cumulative" (P3 §8) have **no source in RF**. What research *does* support is that the **long-tail is winnable** (P1B rates SEO 4/10 — the highest in the territory set — because incumbents don't defend tool-seeking queries) and that there are **~21,000 indexed `.claude/` repos + ~300–500 new `.claude/` orgs/mo** as a real funnel-signal (RF §5, PMC §2). The *conversion of that into monthly signups is an assumption* and must be carried as a hypothesis with a kill-switch, not a number in the bank.

Everything below is re-derived from the fact base with these corrections applied.

---

## 1. The deliverability check that anchors the whole loop

Before any channel, confirm the land actually works — because P1B's entire winner-selection rests on CCGuard being able to *ship* the secrets door from existing code (P1B §3, "fully deliverable by the existing `findings.rs` + cross-tool capture").

**It can.** TS §0/§2 confirms Claude Code writes a complete append-only JSONL transcript to `~/.claude/projects/…` — every prompt, reply, tool call, token count, cwd, git branch — **zero config, already on disk** (TS §0.1, verified live). The scanner reads files the user already has; no API, no account, nothing leaves the machine. This is what makes the "5-minute, local, no-IT" promise honest (PMC §1, P1 §6).

**But the same fact is why detection is NOT the moat (P1B §2.1):** the OSS one-offs (`sensitive-canary`, `leakproof`, `Claudleak`, Sieve) read the *same on-disk transcripts*, and GitGuardian gives free `ggshield` AI hooks ≤25 devs (P1A "existing", PMC §12). So the loop's defensible pull is **cross-tool-in-one-scan + team rollup + donut + card-pay**, exactly as locked (PMC §12, P1B §3). v2 does not re-litigate this — it's correctly locked.

---

## 2. The PLG loop (Scan → Shock → Share → Rollup → Pay)

Unchanged in shape from P2 §4 — it's well-grounded — but v2 tightens two mechanics and labels every conversion assumption.

```
   long-tail SEO ┐                          ┌─ redacted Exposure Report card
   GitHub/HN/PH  ┼─▶ [SCAN] ─▶ [SHOCK] ─▶ [SHARE] ─┤   (cross-tool, screenshot-ready)
   shared card  ┘    free      "3 live      │        └─ claresso.dev/r/<id> public page
                     local     keys + 2     │           + CTA "scan your own" ──┐
                     CLI       PII across   ▼                                     │
                               3 tools"   [INVITE] ─▶ [TEAM ROLLUP] ─▶ [PAY]      │
                                            free        free workspace   Stripe   │
                                                                         card,    │
                                                                         no demo  │
        ◀───────────────────── backlinks + new scanners ──────────────────────────┘
```

| Stage | Mechanic | Grounding | Conversion assumption |
|---|---|---|---|
| **Scan** | `npx/brew/pipx ccguard scan`, no account, reads on-disk transcripts | TS §0.1, §2.3 (deliverable); P1 §4.4 (CLI-as-land) | — |
| **Shock** | "47 sessions, 3 tools, 3 LIVE secrets + 2 PII" — cross-tool framing is the activation moment OSS single-tool scripts can't reproduce | P1B §5, P2 §4 | The "holy crap" moment exists (incident-backed: Miasma, TrustFall, 2.1× — RF §2). **Share-rate is an assumption.** |
| **Share** | Redacted card + public `claresso.dev/r/<id>` page → backlinks feed Scan | P1B §5, P2 §4 | **>15–20% share rate is an assumption (P2 §6, P3 §8) — not in RF.** Must be measured, not assumed. The "defensible-virality test" (does cross-tool out-share single-tool? P1B §7.6) is a *pre-build validation*, not a settled fact. |
| **Invite/Rollup** | `ccguard invite` → free read-only team workspace; **gate sits on the rollup, never the individual scan** so the SEO/share loop never throttles | P2 §3, §5(a); P3 §5 | — |
| **Pay** | 2nd+ teammate or need for history/alerting/donut → Stripe card, no demo | P1B §3, P2 §3 | **Free→paying-TEAM conversion: RF §7 gives freemium free→paid 3–6% median; P2/P3 deliberately discount to 1.5–2.5% because the unit is a team, not a seat (P2 §3) — that discount is reasoned (assumption — not in research).** |

**v2 tightening #1 — add a research-backed conversion lever v1 ignored: the reverse-trial.** RF §7 reports **reverse-trial converts at ~24%** vs freemium 3–6% and card-required trial 25–44%. v1 never uses this. Offer the paid team dashboard as a **14-day reverse-trial** (full Growth features, card optional, auto-downgrade to free-rollup if not converted) at the `invite` moment. This is the single highest-leverage, fully-grounded conversion mechanic available and it costs nothing to build on top of the existing paywall. **Recommendation: A/B reverse-trial vs straight paywall at the invite gate.**

**v2 tightening #2 — capture the company email domain on `--share` from day one.** P3 §5 makes this the "enterprise-seed list," and it's the one piece of P3's enterprise motion that *is* sound: the scanner runs on individual laptops inside big orgs (RF §5 — 21k repos already), so a company-domain capture on the public-report email is a near-zero-cost identification signal. Keep it; just don't bank P3's "6 enterprise deals" on top of it (that number is an assumption — §6).

---

## 3. Ranked $0-ad organic motions

Ranked by **(compounding × deliverable-for-3-people × research-grounded-channel-fit)**. Newsletters are *excluded here* — they are paid (RF §8: $5–15k/issue) and belong in §4, the reinvestment bridge.

### Motion 1 (durable engine) — Free-tool-led long-tail SEO/content
- **What:** the OSS cross-tool scanner as a link-magnet + the 10-article cluster (P1B §5; flagship "How to scan your Claude Code chat history for leaked API keys (free)").
- **Why #1:** the only motion that is simultaneously $0, compounding, and still paying out in month 12 (P2 §4 motion 1). It targets demand that is **searched without education** (P1 §5 wedge-2; P1A "intent": r/devops "How do you prevent credential leaks to AI tools?" asked unprompted), and the **long-tail is the highest-defensibility/SEO axis in the entire territory set (P1B §1, SEO 4/10 — the gating winner)** precisely because GitGuardian's content engine owns the fat head but not `ggshield Claude Code alternative` / `scan claude code chat for leaked keys` (P1B §2.1).
- **Grounded claim:** incumbents publish SEO content against these exact queries → confirms commercial search volume (P1A wedge-2 "intent"; GitGuardian/Doppler/Netwrix cited).
- **What is NOT grounded:** the *volume* of signups it produces. **Carry "≥X organic tool-signups/mo" as a hypothesis with a Q2 kill-switch, not a forecast** (corrects P2 §3 / P3 §8).
- **Effort/owner:** M ~70% ongoing; ~6–8 weeks to first rankings (P2 §4 — that ramp estimate is an assumption, but a conventional one).

### Motion 2 (zero-marginal-cost amplifier) — Product-led viral loop (Exposure Report share + invite)
- **What:** treat the redacted cross-tool card as a first-class growth surface; `share → backlink → scan` and `invite → rollup → Stripe` (P2 §4 motion 2).
- **Why #2:** turns every shocked user into an impression at $0 marginal cost; the cross-tool card is the one artifact single-tool OSS scripts structurally can't reproduce (P1B §5, PMC §12).
- **Grounded?** The *mechanism* is sound and deliverable (TS confirms cross-tool capture). The **K-factor is NOT grounded** — P3's "K≈0.15–0.30" and P2/P3's ">15–20% share rate" are assumptions. The honest pre-build step is P1B §7.6's defensible-virality test: ship the scanner, measure cross-tool vs single-tool share rate **before** betting the funnel on it.

### Motion 3 (evergreen discovery — under-weighted in v1) — GitHub-native distribution
- **What:** GitHub Marketplace CI Action listing + `Awesome-Claude-Code` / alternativeto ("ggshield alternative") / libhunt / console.dev, plus using the **`.claude/` org GitHub-API signal** for targeted content/outreach.
- **Why broken out from v1:** P2 bundled this inside "community launch." It deserves its own rank because it is **evergreen, not spiky**, and it is the most directly research-grounded distribution play: **GitGuardian became the #1 GitHub security app via the Marketplace** (P1 §4.2), and the **~21,000 indexed `.claude/` repos + ~300–500 new `.claude/` orgs/mo** are a real, queryable funnel signal (RF §5, PMC §2). Be present at the point of developer intent with one-click install (P1 §4.2).
- **Grounded magnitude?** The 21k/300–500 figures are facts; that they *convert* to signups is an assumption.

### Motion 4 (cold-start igniter, non-compounding) — Community launch cadence (HN / Reddit / PH)
- **What:** sequenced incident-anchored launches (Miasma, TrustFall/CVE-2025-59536, the 2.1× stat — RF §2) with the free scanner as payoff (P2 §4 motion 3).
- **Why #4 not higher:** spiky and non-compounding — it *ignites* Motions 1–3 (first users, the high-authority backlinks, the share events) but is not the engine (P2 §4 motion 3). **This is where v1's biggest channel over-reach lives — see §5 for the corrected numbers.**
- **Grounded channels:** Show HN, Product Hunt, r/netsec / r/devops / r/ClaudeAI are all named in RF §8. The VoC language for Reddit posts is banked in P1 §7.

### Motion 5 (slow-burn, founder-led) — Authentic community participation
- **What:** daily value-first participation in r/ClaudeAI, r/devops, r/devsecops, ai.stackexchange where the questions are *already being asked* (P1A wedge-2 "intent" lists the literal threads).
- **Why #5:** lowest leverage per hour and hardest to scale with 3 people, but it's $0 and it seeds Motions 1/4. **Compliance note:** the Reddit Responsible-Builder policy prohibits identical cross-posts and undisclosed automation — so this must be genuine human participation, which caps its throughput (consistent with P2 §6's "value-first" framing).

**Motions NOT recommended as organic engines (grounded exclusions):**
- **LinkedIn organic/outbound** — RF §7: LinkedIn cost-per-SQL $400–3,000, "only viable >$300/mo ARPA." At the $99 Team ARPA, LinkedIn is structurally unviable; reserve only for Growth/Enterprise retargeting later (matches P3 §6).
- **Cold AI-SDR/Clay outbound** — RF §8 reports signal-triggered reply 15–25% vs 3.4% generic, *but* "human reviewer load-bearing" → not a 3-person $0 motion; defer.
- **Shadow-AI discovery content as a *product* claim** — P1B §2.2/§3: CCGuard structurally cannot deliver discovery; use only as a top-of-funnel angle routing to the deliverable secrets wedge, never as a promise.

---

## 4. The reinvestment bridge (first paid channel — not "$0", but revenue-funded)

PMC §11.4 caps month-1 ad spend at ~$100; thereafter only reinvested revenue. The **first dollar of reinvestment goes to dev newsletters from ~month 5** (P3 §6), and this is well-grounded:

- **Dev newsletters:** TLDR InfoSec ~**$167 CAC at scale** (RF §7, §5); **$5–15k/issue** sticker (RF §8). At $99 Team ARPA, a $167 CAC pays back in <2 months → viable. **Pitch the DATA story** ("AI tools leak secrets at 2.1×" — RF §2) with the tool as the link, not a product ad (P2 §4 week-8).
- **Avoid:** Pragmatic Engineer (takes no sponsors — RF §8). Google search ads only on the single best-converting query (CPL $87–200, RF §7) as a small test, not an engine.

This is the only channel where v1's CAC number is **fully grounded** — keep it as written.

---

## 5. The launch sequence (corrected channel outcomes)

Structure from P2 §4 is sound (SEO base before spikes; never two big launches in one week; HN before PH). v2 **corrects the expected outcomes** to match RF §8/§5 — the sequence is the same, the *numbers you plan against* are lower and probabilistic.

| Week | Launch | Asset | **v1 claim** | **v2 corrected expectation (grounded)** |
|---|---|---|---|---|
| 5 | GitHub repo public + first OSS release | README, install GIF, 3 seed articles | "100–300 organic scanners" (P2 §6) | Foundation, not a launch. Magnitude is an **assumption**; treat as soft-seed only. |
| 6 | Dev-tool directories + GitHub Marketplace | CI Action, Awesome-CC, alternativeto, libhunt, console.dev | evergreen backlinks | Grounded as a channel (P1 §4.2); **no signup number is in research** — measure. |
| 7 | **Show HN** (Tue–Thu 8–10am ET) | "Show HN: I scanned my AI coding history and found 3 leaked keys" | "~2.3% front-page → 8–15k visitors" stated as the outcome (P2 §4) | **CORRECTION:** RF §8/§5 — 2.3% is the *probability of reaching front page*; 8–15k visitors happen **only if** front page. Plan against **a few hundred visitors as the modal outcome, with a ~2.3% chance of an 8–15k breakout.** Do not bank the breakout. |
| 7 (+2d) | Reddit, staggered, natively rewritten | r/ClaudeAI → r/netsec → r/devops | "1–3k scanners" (P2 §6) | Channels grounded (RF §8); **the 1–3k number is an assumption.** Compliance: no identical cross-posts (Reddit policy). |
| 8 | Dev newsletters (paid, if revenue allows) | DATA story + HN traction | TLDR InfoSec primary | $167 CAC / $5–15k/issue — **grounded** (RF §7/§8). Funded by reinvested revenue, so realistically month ≥5. |
| 9 | **Product Hunt** (12:01am PT Tue/Wed) | gallery + 60s demo + card-pay tagline; **Team dashboard PAID GA + Stripe live** | "#1 dev tool ≈ 200–600 signups" (P2 §4) | RF §8: "#1 dev tool ≈ 200–600 signups." **Correction: this assumes you *win* #1.** Plan against 200–600 *conditional on a top finish*; a mid-pack finish is materially less. |
| 10–12 | Continuous/evergreen | 2 articles/wk; run share loop; convert cohort | second beat if first under-delivers | Sound. The "second beat" hedge is the right response to the probabilistic reality of HN/PH. |

**The single most important sequencing point (grounded and correct in v1, kept):** the SEO base and the OSS link-magnet must be **live and indexed before** the HN/PH spikes, because the spikes are non-repeatable and the SEO is what captures the long-tail traffic the spikes send (P2 §4). The race is to **bank the cross-tool + donut + card-pay moat into Google and paying teams before Anthropic-native detection + GitGuardian free hooks commoditize the door** — the 12–20 month window (RF §1, PMC §2/§12, P2 §1). This is the correctly-identified #1 risk (P2 §7, P3 §7) and v2 endorses it without change.

---

## 6. Funnel math — reverting the plan of record to the honest band

**The contradiction:** P2 §7 says "$30–50k defensible base, plan of record $50k; $100k stretch." P3 §7 says "plan of record ~$85k." They cannot both be the plan of record. v2 sides with **P2's $30–50k band** as the forecast, for grounded reasons:

| Input | P3's assumption | Research grounding | v2 verdict |
|---|---|---|---|
| Free→paying-team conversion 1.2%→2.2% ramp | core driver | RF §7 gives freemium 3–6% *median* (seat, not team); the team-unit discount to ~2% is reasoned but **(assumption — not in research)** | Use as a **range with a kill-switch**, not a ramp curve |
| Donut upgrade 3%/mo of Team base (~42% of M12 MRR) | "the engine" | **Nothing in RF/P1 supports a 3%/mo upgrade rate.** Donut demand is rated **2/10, unsearched** (P1 §5, P1A wedge-3) | **Over-reach.** Donut is a retention/defense feature (PMC §11.1), not a forecastable expansion engine |
| 6 enterprise deals @ ~$2.2k MRR (~16% of M12) | levers 2/3 | RF §7: enterprise free-trial <10% convert; sales cycle 90–150 days; **no deal-count basis in research** | **Over-reach.** Possible upside from the company-domain seed list, not a banked number |
| Aikido "$300/mo, 10 users" flat-per-team proof | pricing justification (P2 §3, P3 §1) | **Not present anywhere in RF or the corpus I read** | **(assumption — not in research).** Flag and verify before quoting publicly |

**v2 forecast (grounded):** The only conversion/churn anchors in research are **freemium 3–6% (RF §7)** and **SMB churn 3–5%/mo (RF §7)**. The reachable identifiable-org pool is **~21,000 existing + ~3,600–6,000 new/yr ≈ ~25,000 orgs** (P1 §2, RF §5). At P2's reasoned ~1.5–2.5% free→paying-team conversion and a **flat blended ARPA ~$169** (P2 §3), **$30–50k MRR (~178–296 paying teams) is the honest 12-month band; $100k requires a launch breakout to compound (P2 §7).** Anything above $50k should be presented as **upside contingent on (a) a Show-HN/PH breakout that — per the corrected §5 — is a ~2.3% tail event per attempt, and (b) SEO out-ranking GitGuardian's free `ggshield` tail.** Report against **$50k; treat $85k–$100k as a ceiling, not a plan.**

**The reverse-trial (RF §7, ~24%) is the one grounded lever that could legitimately lift the conversion line** — and v1 left it on the table. That's the recommended swing to test, not the donut ladder.

---

## Grounding ledger

| Claim | Source (file §) | Confidence |
|---|---|---|
| No tool captures/governs Claude Code terminal sessions; Anthropic Compliance API + 60 partners exclude Claude Code; ~12–20mo window | RF §1; PMC §2 | High |
| Free on-disk JSONL transcript (zero-config) makes the local scanner deliverable today | TS §0.1, §2.3 | High |
| Detection is already free (sensitive-canary, ggshield ≤25 devs, OSS) → not the moat; acquire on commodity, defend on bundle | PMC §12; P1B §2.1; P1A "existing" | High |
| Secrets/credential-leak demand is searched without education (r/devops, ai.stackexchange threads) | P1 §5; P1A wedge-2 "intent" | High |
| 2.1× AI-commit leak rate; Miasma; TrustFall/CVE-2025-59536 = content ammunition | RF §2; PMC §2 | High |
| Show HN front-page **rate** ~2.3%; ~8–15k visitors **only if** front page | RF §8, §5 | High (v1 mis-stated as outcome — corrected §5) |
| Product Hunt **#1** dev tool ≈ 200–600 signups (conditional on winning #1) | RF §8 | High (channel) / Med (assumes top finish) |
| Newsletter (TLDR InfoSec) ~$167 CAC at scale; $5–15k/issue; Pragmatic Engineer no sponsors | RF §7, §8 | High |
| ~21,000 indexed `.claude/` repos + ~300–500 new `.claude/` orgs/mo = funnel signal | RF §5; PMC §2 | High (signal) / assumption (→signups) |
| GitGuardian = #1 GitHub security app via Marketplace → GitHub-native distribution works | P1 §4.2 | High |
| LinkedIn cost-per-SQL $400–3,000, only viable >$300/mo ARPA → exclude for $99 Team | RF §7 | High |
| Reverse-trial ~24% convert vs freemium 3–6% / card-trial 25–44% | RF §7 | High |
| SMB monthly churn 3–5% | RF §7 | High |
| Reachable Year-1 org pool ~25,000; ~1.5–2.5% free→paying-team conversion | P1 §2 (built on RF §5) | Med (reasoned assumption) |
| Flat-per-team pricing $99/$299, blended ARPA ~$169 | P2 §3 | Med (reasoned; Aikido proof unverified) |
| $30–50k MRR = defensible 12-mo band; $100k = breakout-contingent ceiling | P2 §3, §7 | Med (honest base case) |
| Gate on team rollup, never the individual scan (loop never throttles) | P2 §3/§5; P3 §5 | High (design principle) |
| Local-first / dev-sees-own-data-first = anti-Teramind trust unlock | PMC §1; P1A Teramind gap; TS §0.4 | High |

## Evidence gaps / v1 over-reaches

1. **P3's $85k "plan of record" contradicts P2's $50k and is built on un-sourced rates.** The 3%/mo donut upgrade (~42% of M12 MRR), 6 enterprise deals, and 1.2%→2.2% conversion ramp have **no basis in RF or phase-1 research**. The donut itself is rated demand 2/10 and unsearched (P1 §5). **Reverted: plan of record = $30–50k (P2); $85k/$100k = upside, clearly labelled.**
2. **Show HN stated as an expected 8–15k-visitor event.** RF §8/§5 make 2.3% a *front-page probability*; the visitor count is conditional. v1 banks "1–2 front-page hits" — that's an assumption about repeated attempts, not a research outcome. **Corrected to a probabilistic model (modal = a few hundred visitors).**
3. **SEO signup volumes ("3,000–6,000/mo", "25k cumulative") are invented.** RF supports that the long-tail is *winnable* (P1B SEO 4/10) and quantifies the `.claude/` org pool, but contains **no SEO-traffic-to-signup conversion figure**. Carry as a hypothesis with a Q2 kill-switch.
4. **Share rate / K-factor (">15–20%", "K≈0.15–0.30") are unvalidated.** P1B §7.6 itself flags this as a *pre-build test*, not a known quantity. Must be measured before the funnel is bet on it.
5. **"Aikido proves flat-per-team ($300/mo, 10 users)" is not in the corpus.** Quoted in P2 §3 and P3 §1 as load-bearing pricing evidence; I could not trace it to RF or any phase doc. **Verify before using publicly.** *(assumption — not in research)*
6. **PH "#1 dev tool" assumes you win #1.** RF §8's 200–600 figure is conditional on a top finish; a mid-pack PH launch is materially weaker. v1 plans against the win.
7. **Reverse-trial (RF §7, ~24%) was omitted from v1** despite being the single highest-leverage, fully-grounded conversion lever available. v2 adds it as the recommended A/B at the invite gate — a grounded substitute for P3's un-grounded donut ladder.
8. **Enterprise close model (P3 §5) over-specifies a motion the research can't size.** The company-domain seed list (RF §5 supports the signal) is sound; "6 deals, 4–8 range, $15–45k ACV" is an assumption stacked on a sales-cycle fact (90–150 days, RF §7) — keep the mechanism, drop the banked deal count.
9. **No research on time-to-first-ranking (v1's "6–8 weeks") or "100–300 organic scanners" at week 5.** Conventional but un-sourced; label as planning assumptions, not targets.