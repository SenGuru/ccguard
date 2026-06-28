I've read the full corpus. Here is the re-grounded v2 pricing + path-to-$100k math.

---

# CCGuard / Claresso — Pricing v2 & Path-to-$100k Math (Re-Grounded)

*Pricing strategist re-grounding · 2026-06-27 · Supersedes pricing in `phase2-gtm-blueprint.md §3` and the projection in `phase3-operating-plan-100k.md`. Every material number traces to a corpus citation or is tagged `(assumption — not in research)`.*

**Shorthand:** PMC = `product-marketing-context.md` · MT = `phase1-market-truth.md` · AP = `phase1-appendix.md` · POS = `phase1b-positioning.md` · GTM = `phase2-gtm-blueprint.md` (v1) · OP = `phase3-operating-plan-100k.md` (v1) · RF = `RESEARCH-FINDINGS.md`.

---

## 0. Headline verdict (what changed from v1)

1. **The two v1 docs contradict each other on the plan-of-record and v2 sides with GTM, not OP.** GTM §7 names **$30–50k MRR the defensible base case and $50k the plan of record**; OP §1/§7 escalates the *same plan of record* to **~$85k**. The escalation is produced entirely by two levers — a **3%/mo Team→Growth "donut" upgrade rate** and **6 enterprise deals** — that **have no benchmark anywhere in RF §5–§8 or the phase docs.** OP's own decomposition (OP §3) admits these two levers = **~58% of M12 MRR**. So **~58% of OP's flagship number rests on un-grounded assumptions.** Re-grounded plan-of-record returns to **~$45–55k MRR** (GTM's number), with $85k reclassified from "plan of record" to "upside requiring two unbenchmarked levers to hold."

2. **The single most important grounding fact both v1 docs underweight:** the only two bootstrapped, *no-audience* comparables in the research — **Plausible (42 months to $1M ARR ≈ $83k MRR) and Bannerbear (60 months to $50k MRR)** (RF §7) — took **3.5–5 years** to reach the MRR this plan targets in **12 months.** The fast comps RF cites (CodeRabbit, RB2B) are explicitly disqualified: CodeRabbit was "VC+viral," RB2B hit $1M ARR in 16 weeks "off a 92k-follower founder audience" (RF §7) — and the team has **"no connections, no audience, no warm intros at start"** (PMC §7). **The grounded precedent says even $50k MRR in 12 months is ahead of every applicable comp.** This must frame the whole plan.

3. **Pricing structure (free-tier line, flat-per-team, bands + overage) is sound and largely grounded — but its headline justification ("Aikido proves flat-per-team") is not in the corpus.** Re-grounded below on facts that *are* (RF §7 ARPA floor; the procurement-model argument from POS/GTM).

4. **The pricing is underpriced against its own WTP evidence.** Effective per-dev at $99/10-dev and $299/30-dev is **~$9.90/dev** — the *floor* of the standalone secrets band ($5–15/dev, AP "Secrets WTP") and well **below the $20–50/dev bundled team-governance band** the same research says is reachable (AP "Secrets WTP," MT §5, POS §8). **Raising Growth is a grounded ARPA lever that closes the logo gap more honestly than an invented 3%/mo upgrade rate.**

---

## 1. Pricing v2

### 1.1 Free-tier line — KEEP v1 (well grounded)

**Free = one individual scanning their own machine: unlimited local cross-tool scans, full secret/PII findings, the shareable redacted Exposure Report card. The gate sits on the *team rollup*, never the individual scan.** (GTM §3, GTM §5 open-Q-a.)

This is the best-grounded decision in the pricing. It is a direct application of the PLG lesson the research extracted from the actual incumbents: *"a genuinely useful, permanent free tier sized to a real team — GitGuardian (25 devs), Semgrep (10 contributors) — is the virality engine"* (MT §4.3). Gating on the team artifact rather than a dev-count or scan cap keeps the two viral mechanics (SEO-ranking scan + screenshot card) ungated, which is correct because **the free scan IS the acquisition channel** (POS §5, GTM §4). Detection is already free in the wild (sensitive-canary, ggshield ≤25 devs, leakproof, Claudleak — POS §2.1, AP "Secrets existing"), so charging for it would be both uncompetitive and against the locked "acquire on the commodity" strategy (PMC §12).

### 1.2 Per-seat vs flat — KEEP FLAT-PER-TEAM, but re-grounded

**Decision: flat-per-team with seat bands + a priced per-seat overage** (unchanged from GTM §3 / OP §5). **What changes is the justification.** v1 leads on *"Aikido has proven this exact flat-per-team model... $300/mo, 10 users"* (GTM §3, OP §1). **Aikido appears nowhere in the corpus** — not in RF §6 pricing comparables, not in the appendix. That is a load-bearing external claim presented as proof; **demote it to `(assumption — not in research)`.**

The flat decision survives on three grounded legs without Aikido:
- **Per-seat is the procurement model the entire wedge rejects.** GitGuardian, Snyk, Semgrep all price per-dev/contributor (RF §6) *and all three hard-wall into "Contact us" sales above the free tier* (MT §3, AP). The differentiated act is "buy it with a card, no demo" (POS §3, PMC §12) — a per-seat sales motion contradicts it.
- **ARPA must clear a floor for the economics to ever work.** RF §7: *"LinkedIn cost-per-SQL $400–3,000 (only viable >$300/mo ARPA)."* Flat bands lift ARPA toward that floor; a $9.90/dev × few-seats invoice never gets there. (This is also why the §1.3 reprice matters.)
- **Predictability for card-buyers + decoupling from volatile 2026 headcount** (GTM §3 reasoning) — defensible as logic, though not separately benchmarked `(assumption)`.

### 1.3 Tiers — KEEP the structure, RAISE Growth (grounded ARPA lever)

| Tier | v1 price | **v2 price** | Effective $/dev (v2) | Grounding |
|---|---|---|---|---|
| **Free (Door)** | $0 | **$0** forever, 1 individual | — | MT §4.3; PMC §12; POS §5 |
| **Team** | $99 / ≤10 devs | **$99 / ≤10 devs** (hold) | ~$9.90 | Floor of standalone secrets band $5–15/dev (AP). Hold as the conversion magnet — predictability + lowest friction. |
| **Growth** | $299 / ≤30 devs | **$349 / ≤25 devs** (overage $14/dev) | ~$14 | Still only the *bottom* of the bundled team-governance band $20–50/dev (AP, MT §5). $299/30 was **underpriced**; $349/25 recovers ARPA and tightens the band so overage triggers sooner. |
| **Enterprise** | Custom ~$15–45k ACV | **Custom ~$15–45k ACV** | — | Bracketed by GitGuardian ~$45k, Snyk ~$45k, Semgrep ~$54k median ACV (RF §6, AP). Plausible **per deal**; the *count* is the problem (§3). |

**Why raise Growth and not Team:** the entire $100k problem is ARPA, not logos (OP §1 is right about that). The research hands you a **grounded** ARPA lever — the gap between current ~$10/dev pricing and the $20–50/dev bundled band (AP "Secrets WTP": *"a team-governance/dashboard angle could reach ~$20-50/dev/mo"*). Pricing Growth at $14/dev is still conservative inside that band, yet a 65/30 Team/Growth mix at $99/$349 lifts blended ARPA from ~$140 to **~$165** — which does the same work as ~1%/mo of invented donut upgrades, but is anchored to a cited WTP band instead of a guessed behavior rate.

**Donut placement — KEEP as the Growth-tier feature, REJECT it as the revenue engine.** The donut belongs at Growth as an upsell/retention feature (GTM §5-d) — that is consistent with the research, which ranks it **dead last (composite 0.6), demand 2/10, "near-zero pull," "must be educated into existence"** (MT §5, POS §1, AP "donut demand"). What the research does **not** support is OP making donut upgrades **"THE load-bearing lever (~42% of M12 MRR)"** (OP §3). You cannot build 42% of revenue on the single lowest-demand item in the entire demand-ranking. The donut is a *defensibility/retention* asset (it's the un-copyable moat element — PMC §12, POS §3), **not** a self-serve expansion driver with a known conversion rate.

---

## 2. Path-to-$100k — re-grounded scenario math

### 2.1 The arithmetic frame (grounded)

$100k MRR requires, at the v2 blended ARPA of **~$165**: **~605 active paying teams.** At pure Team $99: **~1,010 teams** (matches OP §1). The structural problem is real and correctly stated by v1: *a 3-person organic team cannot acquire ~1,010 logos in 12 months* (OP §1). So $100k can only come from **ARPA concentration** (expansion + enterprise), not logo count. **That logic is sound and I keep it.** The dispute is purely about *whether the specific concentration rates v1 uses are grounded* — they are not (§3).

### 2.2 Funnel inputs — what is grounded vs inferred

| Input | v1 value | Grounded? | Source / flag |
|---|---|---|---|
| SMB monthly churn | ~4%/mo (3.5% stretch) | **Grounded** | RF §7: SMB churn 3–5%/mo |
| Free→paying-**team** conversion | 1.2% → 2.2% | **Partly** | RF §7 freemium median is **3–6%**; v1 sets *below* it because the unit is a team not a seat (MT §2 reality check, GTM §3). The *adjustment direction* is reasoned; the exact 1.2→2.2 ramp is `(assumption)`. |
| Show HN front-page spike | "1–3k scanners" | **Grounded (range)** | RF §8: ~2.3% front-page rate; **8–15k visitors IF front page** |
| Product Hunt | 200–600 signups | **Grounded** | RF §8: PH #1 dev tool ≈ 200–600 |
| **Sustained SEO signups → 3,000–6,000/mo by M12** | core fuel | **NOT grounded** | No SEO-traffic benchmark exists in RF. GTM §3 itself hedges *"if the content engine hits."* The identifiable new-org pool is only **~300–500 new `.claude/` orgs/mo** (RF §5). The 4,100/mo signups OP needs by M12 is an inference, not a benchmark. |
| Newsletter CAC | ~$167 | **Grounded** | RF §7 (TLDR InfoSec at scale) |
| Cumulative signups (12mo) | ~25,200 (OP) / 15–25k (GTM) | **Inferred** | Built from the un-benchmarked SEO ramp above. GTM's 15–25k is the more honest band. |

### 2.3 Re-grounded BASE CASE (conservative, all-grounded inputs at the conservative end)

Inputs: signups ramp to a **~16–17k** cumulative (low end of GTM's 15–25k, justified by the Bannerbear/Plausible no-audience precedent, RF §7); conversion 1.2%→2.0%; churn 4%/mo (RF §7); blended ARPA ramping $99→~$135 (Team-heavy + light overage, minimal Growth). Illustrative precision — the inputs are grounded, the monthly shape is `(assumption)`:

| Mo | Signups | New paying teams | Active teams (4% churn) | ARPA | **Ending MRR** |
|----|------|------|------|------|------|
| M1 | 150 | 2 | 2 | $99 | ~$0.2k |
| M2 | 900 (HN) | 12 | 14 | $100 | ~$1.4k |
| M3 | 1,400 (PH) | 20 | 33 | $104 | ~$3.4k |
| M4 | 1,100 | 16 | 48 | $108 | ~$5.2k |
| M5 | 1,200 | 19 | 65 | $112 | ~$7.3k |
| M6 | 1,300 | 22 | 84 | $116 | ~$9.7k |
| M7 | 1,400 | 25 | 106 | $120 | ~$12.7k |
| M8 | 1,500 | 28 | 130 | $124 | ~$16.1k |
| M9 | 1,600 | 30 | 155 | $127 | ~$19.7k |
| M10 | 1,700 | 33 | 182 | $130 | ~$23.7k |
| M11 | 1,800 | 36 | 211 | $133 | ~$28.1k |
| M12 | 1,900 | 38 | **~241** | **$135** | **~$32.5k** |

**Base case ≈ $30–35k MRR, ~241 active teams.** Cumulative signups ~16k. This is the honest "everything works but nothing breaks out" outcome and it sits inside GTM §7's $30–50k band.

### 2.4 Scenario comparison

| Scenario | Signups (cum) | Conversion (M12) | Blended ARPA | Enterprise | M12 MRR | What it requires |
|---|---|---|---|---|---|---|
| **Base** (all grounded, conservative) | ~16k | 2.0% | $135 | 0 | **~$32k** | One HN front-page + PH + modest SEO. Grounded. |
| **Plan-of-record (v2)** | ~22k | 2.4% | **$165** (v2 reprice, 30% Growth) | 1–2 inbound | **~$48–55k** | SEO hits mid-range (still `(assumption)`) + the §1.3 reprice + Growth mix climbs. = GTM's honest $50k. |
| **Stretch (= OP's "$85k")** | ~25k | 2.2% | $169 | **6 deals** | ~$82–88k | **Donut 3%/mo upgrades (no benchmark) + 6 enterprise (no benchmark for count, re-imports cut motion). Inference-stacked.** |
| **Ceiling** | ~28–32k | 2.5% | ~$175 | 8–10 deals | ~$100–105k | OP §7's own "low-probability stack (~10–15%)." Two simultaneous compounding launches + everything above. |

**Recommended plan of record: ~$50k MRR (v2 plan-of-record row)** — reconciling to GTM §7, not OP §1. Drive toward the stretch, but report the stretch as *upside contingent on two unbenchmarked levers*, never as the forecast.

---

## 3. Stress-test of v1's $85k "aggressive" projection

The task: are OP's load-bearing assumptions (3%/mo donut upgrade, 6 enterprise deals, 1.2→2.2% conversion, ~25k signups) supported by cited benchmarks, or invented? Verdict per assumption:

| OP assumption | OP's claimed impact | Grounded? | Finding |
|---|---|---|---|
| **3%/mo Team→Growth donut upgrade** | **~42% of M12 MRR** (OP §3, "the load-bearing lever") | **NO — invented** | No expansion/upgrade-rate benchmark exists in RF §7 or anywhere in the corpus. Worse: it is built on the **lowest-demand feature in the research** — donut = demand 2/10, composite 0.6, "near-zero pull," "must be educated into existence" (MT §5, POS §1, AP). Building the single largest revenue block on the single weakest-demand feature is a direct contradiction of the demand evidence. **Most serious over-reach in the plan.** |
| **6 enterprise deals @ ~$2.2k MRR** | **~16% of M12 MRR** (OP §3) | **Per-deal ACV grounded; count + timing invented; motion contradicts the pivot** | $15–45k ACV is bracketed by RF §6 (GG/Snyk/Semgrep ~$45–54k). But (a) the **count "6"** has no benchmark; (b) RF §7 puts **mid-market security sales cycle at 6–9 weeks and enterprise at 90–150 days** — 6 deals closing M9–M12 from a base that only opens in M1 is tight; (c) it **re-imports the enterprise sales motion the entire pivot exists to escape** (PMC §0, §11.5: "defer SSO/SCIM, MDM, eDiscovery, proxy enforcement out of the front door"). OP §5 even schedules **"SSO/SCIM + MDM + enforcement GA"** and hires a 4th head to build it — see §4 over-reaches. |
| **Conversion 1.2% → 2.2%** | drives self-serve base ±35% | **Partly grounded** | Sits *below* RF §7's 3–6% freemium median, correctly adjusted down because the unit is a team (MT §2, GTM §3). The *level* is defensible-conservative; the **ramp shape** is `(assumption)`. This is the **least objectionable** of the four. |
| **~25k cumulative signups** | the "fuel" | **NOT grounded at the top end** | Built on a sustained SEO ramp to 3–6k signups/mo that **has no benchmark** and that GTM itself hedges with "if the content engine hits." Grounded supply = one HN front-page (8–15k *one-time*, RF §8) + PH (200–600, RF §8) + a finite 21,000-repo stock and ~300–500 new orgs/mo (RF §5). 25k is the optimistic edge of an inferred range, not a benchmarked number. GTM's 15–25k band is more honest; base case should plan ~16k. |

**Net:** of the four pillars under $85k, **two (~58% of M12 MRR) are invented**, one (signups) is inferred at its optimistic edge, and only conversion is reasonably grounded. **$85k is inference-stacked and should not be the plan of record.** OP §7 partially admits this (floor $48k, ceiling "10–15% probability") but still prints **$85k as "the number I will report against"** (OP §2) — that is the over-reach to correct.

---

## Grounding ledger

| Claim | Source (file §) | Confidence |
|---|---|---|
| Free tier sized to a team is the virality engine; gate on team rollup | MT §4.3; GTM §3,§5a; POS §5 | High |
| Detection is already free (ggshield ≤25, sensitive-canary, OSS) → can't charge for it | POS §2.1; AP "Secrets existing"; PMC §12 | High |
| Per-seat = the procurement model the wedge rejects; competitors all sales-wall above free | MT §3,§4; AP; RF §6 | High |
| ARPA only viable above ~$300/mo for paid acquisition → flat bands lift ARPA | RF §7 (LinkedIn cost-per-SQL) | High |
| Standalone secrets WTP $5–15/dev; bundled team-governance $20–50/dev | AP "Secrets WTP"; MT §5; POS §8 | High |
| Growth $299/30-dev (~$10/dev) is underpriced vs bundled band → raise to ~$349/25 | AP "Secrets WTP" (derived) | Medium |
| Enterprise ACV $15–45k is realistic per deal | RF §6; AP (GG/Snyk/Semgrep $45–54k) | High |
| SMB monthly churn 3–5% | RF §7 | High |
| Freemium free→paid median 3–6%; team-unit conversion reasonably set below it | RF §7; MT §2; GTM §3 | Medium |
| Show HN 8–15k visitors *if* front page (2.3% rate); PH #1 dev tool 200–600 | RF §8 | High |
| No-audience bootstrapped comps: Plausible 42mo→$1M ARR, Bannerbear 60mo→$50k MRR | RF §7 | High |
| Fast comps (CodeRabbit, RB2B) disqualified — VC/viral/92k-audience; team has none | RF §7; PMC §7 | High |
| Identifiable org supply ~300–500 new `.claude/` orgs/mo + 21k stock | RF §5; PMC §2 | High |
| Newsletter CAC ~$167 at scale | RF §7 | High |
| $100k needs ~605 teams @ $165 (or ~1,010 @ $99) → ARPA concentration is the only path | OP §1 (derived); RF §6 | High |
| Base-case ~$32k, plan-of-record ~$50k, stretch ~$85k, ceiling ~$100k | This doc (derived from above); GTM §7; OP §7 | Medium |
| Donut belongs at Growth as retention/upsell feature, not the door | GTM §5d; MT §5; POS §1 | High |
| Monthly MRR table shapes / ramp curves | — | **Assumption (illustrative)** |

## Evidence gaps / v1 over-reaches

1. **OP's 3%/mo donut upgrade rate (~42% of M12 MRR) is invented** — no expansion-rate benchmark in the corpus, and it is built on the lowest-demand feature in the entire demand ranking (donut demand 2/10, MT §5). **The biggest over-reach.** Correct: donut is a retention/defense asset, not a benchmarked revenue engine.
2. **OP's 6-enterprise-deal count is invented** (per-deal ACV is fine; the count and M9–M12 close timing are not) and the underlying motion **contradicts the self-serve pivot** (PMC §0/§11.5).
3. **OP §5 schedules "proxy enforcement GA" by M9** — directly contradicts PMC §1.5/§11.5: enforcement is *"precision unproven, code-locked until precision proven, fail-open, enterprise-only."* Shipping it GA in 9 months with a 3–4 person team is an over-reach against the locked decision.
4. **"Aikido proves flat-per-team ($300/mo, 10 users)"** (GTM §3, OP §1) — **Aikido is not in the corpus** (absent from RF §6 and AP). The flat decision is correct, but this specific proof point is `(assumption — not in research)`; re-grounded on RF §7 + the procurement-model argument instead.
5. **The SEO signup ramp (3–6k/mo by M12 → ~25k cumulative)** has no benchmark; RF contains no SEO-traffic data. GTM hedges it ("if the content engine hits"); OP treats the top of it as plan-of-record fuel. Plan the base on ~16k.
6. **The two v1 docs disagree on plan-of-record ($50k vs $85k) and neither flags the conflict.** v2 sides with GTM's $50k.
7. **Timeline vs precedent gap (un-addressed in both v1 docs):** the only applicable comps (Plausible, Bannerbear — RF §7) took 3.5–5 years to reach this MRR. Neither v1 doc reconciles a 12-month $50–85k target against the 42–60-month no-audience precedent. This is the largest *unstated* evidence gap and should temper the plan-of-record toward the conservative end.
8. **Annual-prepay cash ($120–150k from ~50 prepays)** (OP §6) and **blended-ARPA mix (65/30/5)** (GTM §3) are reasonable but unbenchmarked `(assumption)` — fine as planning inputs, not as facts.
9. **$49/seat anchors in PMC §8 are superseded** by the flat-per-team decision; any math still quoting "2,040 seats" should be retired to avoid two inconsistent pricing frames in the corpus.