# CCGuard / Claresso — 12-Month Roadmap, Hiring & Risk Plan
### Founder directive: drive toward $100k MRR, weight expansion + enterprise upsell heavily, stay honest

The base-case docs land at $30–50k MRR. The honest path to ~$100k is **not** "more free signups" — organic funnel-fill can't 3x. It's the **expansion ladder**: land teams cheap, drag a fraction up the $99 → $299 → $15–45k rungs. Below, every quarter is tied to the *specific revenue rung it unlocks*, and the math is built to show where $100k actually comes from (hint: ~40% of it is expansion/enterprise, not new-logo self-serve).

---

## 1. Product Roadmap by Quarter — each rung tied to the revenue it unlocks

### Q1 (Months 1–3) — **Free scanner + viral share. Unlocks: $0 direct, the entire top-of-funnel.**
Ship exactly the first-build spec: `ccguard scan` cross-tool CLI on `findings.rs`, redacted Exposure Report card, `--share` → `claresso.dev/r/<id>` + email capture. No auth, no billing.
- **Revenue unlocked:** none directly — this is the acquisition substrate. But it builds the two assets the entire ladder compounds on: (1) the **warm email list of teams who already found leaks**, (2) the hosted report-data substrate the dashboard claims later.
- **Aggressive add (cheap, high-leverage):** instrument the `invite` intent *now* even though rollup is read-only. Every "I want this for my team" click is a pre-qualified expansion lead. Capture **company email domain** on `--share` — this is how you later spot the 200-dev orgs hiding in your free base for enterprise outreach.
- **Exit metric:** scanner indexed + ranking on 3–6 long-tail queries; >15–20% share rate; 1 Show HN/PH breakout banked; cumulative free signups on pace for ~15k+/yr.

### Q2 (Months 4–6) — **Team dashboard + Stripe + donut teaser. Unlocks: $99 Team AND the $299 expansion trigger.**
This is the **single most important quarter for the $100k path** — not because $99 teams are lucrative, but because it installs the expansion ladder's first two rungs and the machinery that drags users up them.
- **Build:** cloud team rollup, 90-day history, scheduled/CI scans, Slack+email alerts → **Stripe $99 flat/10-dev paywall on the team boundary** (never the scan). Then the **$299 Growth tier with the work/personal donut** as the headline upsell, plus a **read-only donut teaser inside Team** (A/B'd — the spec's open question (d)).
- **Revenue unlocked:** $99 Team (new-logo self-serve) **and** the $299 Growth rung. Growth is where blended ARPA jumps from $99 to ~$169 — that lift alone is the difference between 1,010 teams ($100k all-Team, impossible) and ~592 teams ($100k blended, hard but real).
- **Expansion-weighting move (the under-used accelerator):** build **in-product expansion triggers** as first-class features, not afterthoughts — (a) seat-band overage nudges ("you're at 11/10 devs"), (b) the donut teaser showing a blurred company/personal split with "unlock attribution → Growth", (c) a "your scan found leaks in 4 repos across 3 teams" multi-team prompt that only Growth resolves. **Target: ≥25% of Team accounts touch a Growth trigger within 60 days.**
- **Exit metric:** first 5–20 paying teams; **≥15% of paying teams on Growth ($299)** by end of Q2; net revenue retention measured and >100%.

### Q3 (Months 7–9) — **Enterprise ladder: SSO/SCIM + MDM + enforcement + compliance. Unlocks: $15–45k ACV deals.**
This quarter is the **largest single $100k lever** and the one the base-case plan under-invests in. Two enterprise deals at $30k ACV = **$5k MRR each** — equivalent to ~50 Team accounts apiece, with near-zero CAC because they come from *inside your free base*.
- **Build (re-activate the deferred enterprise crates you already own):** SSO/SCIM, MDM/`managed-settings.json` fleet deploy, proxy enforcement (precision-gated), DPIA/LIA + eDiscovery export, self-host option. The architecture already exists in `ccguard-server`/`ccguard-proxy` — this is **re-activation, not greenfield**, which is why a single dev can do it.
- **The land-to-enterprise mechanic (this is the honest path, no sales team):** your free scanner is *already deployed on individual laptops inside big orgs*. Q1's company-domain capture surfaces them. A security lead who already ran the scanner and saw live secrets is the warmest enterprise lead possible. **Founder runs a light, async, "self-serve-assisted" enterprise motion** — a Calendly + a Loom-driven pilot, not a field sales org. Target **3–6 enterprise deals in 12 months**, not 50.
- **Revenue unlocked:** **$15–45k ACV → $1.25–3.75k MRR each.** Even at the conservative end, **4 deals ≈ $5–8k MRR; 6 deals at mid-band ≈ $12–18k MRR** — that's 12–18% of the $100k target from a handful of accounts.
- **Exit metric:** SSO + MDM + enforcement GA; 1–2 paid enterprise pilots closed or in contract; enterprise pipeline of 8–12 self-identified orgs from the free base.

### Q4 (Months 10–12) — **Expansion/retention engine + optional 2nd surface. Unlocks: NRR >110% (compounds all rungs) + ceiling lift.**
At this stage, **expansion revenue should be doing more work than new logos** — that's the bootstrapped-to-$100k signature.
- **Build (retention/expansion — priority):** continuous-monitoring alerts (the stickiness hook), annual-plan push (locks logos, kills churn surface), usage-based expansion nudges, a self-serve "upgrade to Growth/Enterprise" flow that needs zero founder touch. Harden the donut from teaser → daily-value surface so Growth retains.
- **Optional 2nd surface (only if Q1–Q3 hit):** the highest-ROI candidate is a **CI/PR-gate GitHub App** (the spec already notes the CI hook as organic distribution) — it converts the free scanner into an always-on org-wide check, which is a natural Growth/Enterprise expansion driver, *not* a net-new product. **Do NOT build a 2nd surface if behind plan** — focus beats breadth for a 1-dev team.
- **Revenue unlocked:** NRR lift. Going from 100% to 110% NRR on a $50k base adds ~$5k MRR/quarter *with zero new acquisition* — this is the compounding that closes the last gap to $100k.
- **Exit metric:** NRR ≥110%; ≥20–25% of MRR from Growth+Enterprise; logo churn <3.5%/mo.

### Honest $100k bridge (where the money actually is)
| Source | Plausible M12 contribution | Note |
|---|---|---|
| Team ($99) new-logo self-serve | ~$35–45k | The base-case engine; realistic ceiling for organic 3-person fill |
| Growth ($299) expansion | ~$20–30k | Driven by Q2 expansion triggers + donut; the ARPA multiplier |
| Enterprise (3–6 deals) | ~$10–20k | The Q3 ladder; highest $/effort, lands from free base |
| **Blended toward** | **~$70–95k MRR** | **$100k is the stretch ceiling; ~$70–90k is the aggressive-but-honest target** |

**Honest verdict:** weighting expansion + enterprise pulls the credible outcome from the doc's $30–50k base case up to **~$70–90k MRR**, with $100k reachable only if a launch breakout compounds AND ≥1 enterprise deal lands at the top of the $15–45k band. This is aggressive, not fantasy — the delta over base case is *expansion mechanics*, not invented signups.

---

## 2. Hiring / Reinvestment Schedule — discipline + when revenue funds heads

**Iron rule (Months 1–4): 100% of revenue reinvested, $0 founder/team salary draw.** Equity-only team means no payroll burn — every dollar goes to: package registrars/hosting (~$0–50/mo), then the *first* discretionary spend is **newsletter sponsorships from ~Month 5** (TLDR InfoSec ~$167 CAC at scale), not a hire.

| Trigger (MRR) | ~Month | Hire | Why this role, this order |
|---|---|---|---|
| **$0–5k** | M1–4 | **No hires.** Reinvest 100% into infra + content tooling. | Cash is oxygen pre-PMF; the 3-person team builds the loop. Founder-dev fills gaps. |
| **~$8–12k** | M5–6 | **Spend on newsletters/SEO tools first, not people.** Optional **part-time content/SEO contractor** (1–2 articles/wk) to keep the flywheel compounding while D builds Q2 billing. | The SEO engine is the #1 durable risk (see §3). M can't write 2 articles/wk *and* run launches *and* community. Contractor is cheaper than FT and directly funds the acquisition engine. |
| **~$15–25k** | M6–8 | **2nd dev (FT, first real hire).** | Q3 enterprise ladder (SSO/MDM/enforcement/compliance) is too much for 1 dev *while also* maintaining the scanner + Q2 billing. The 2nd dev is what makes the $15–45k rung shippable on time — it directly unlocks the highest-margin revenue. Hire *before* enterprise pilots, not after. |
| **~$30–40k** | M9–10 | **Content/SEO specialist (convert contractor to FT) OR promote.** | At this MRR the flywheel is proven; making it FT compounds the durable engine. Funds itself if each article drives >$167-CAC-equivalent signups. |
| **~$45–60k** | M10–12 | **Part-time enterprise CS / solutions (0.5 FTE).** | Once 3+ enterprise deals exist, churn risk on $30k accounts >> cost of a part-time CS. Protects the highest-ACV revenue and frees founder from pilot babysitting. Deliberately part-time — we are NOT building a sales org. |

**Reinvestment priority ladder (what each new dollar buys, in order):** (1) keep the lights on, (2) newsletter/SEO acquisition from M5, (3) part-time content contractor, (4) 2nd dev for the enterprise ladder, (5) FT content/SEO, (6) part-time enterprise CS, (7) only then a small paid-ads test against the best-converting query. **Never hire ahead of the rung the hire unlocks** — each head must trace to a revenue rung it makes possible.

---

## 3. Top Execution Risks for 1 FT Dev + Part-Time Founder-Dev — and Mitigations

**Risk 1 — Single-dev bottleneck collides head-on with the Q3 enterprise ladder.**
The roadmap asks one dev to maintain the free scanner, ship Q2 billing, *and* re-activate SSO/MDM/enforcement/compliance in Q3 — the exact enterprise machinery the plan deliberately fled. This is the #1 threat to the $100k path because enterprise is the biggest lever and the most code.
- **Mitigation:** Hire the 2nd dev at ~$15–25k MRR (M6–8), *before* enterprise build, not after. Re-activate (don't rewrite) the existing `ccguard-server`/`ccguard-proxy` crates — they already exist, which is the only reason this is feasible. Gate enterprise scope hard: SSO + MDM + enforcement *first* (deal-blockers), DPIA/eDiscovery *second* (closeable as "on roadmap"). If 2nd dev slips, **sell enterprise pilots against a near-term roadmap rather than delaying revenue.**

**Risk 2 — The door commoditizes (GitGuardian free hooks + Anthropic-native) before the moat is banked.**
The plan's own stated #1 risk: if free detection becomes an OS feature before SEO ranks and the paid ladder is live, the funnel erodes. For the *aggressive* $100k target this is worse — it caps the top-of-funnel that feeds expansion.
- **Mitigation:** This is precisely *why* expansion + enterprise weighting de-risks the founder's target — revenue concentrates in cross-tool + donut + dashboard + card-pay + enterprise, the things Anthropic structurally won't build and OSS can't reproduce. **Pull Q2 (dashboard/Stripe/donut) as early as possible** so the moat is in-market while the door still acquires. Race the clock: bank the moat into Google + paying teams during the 12–20 mo window. Don't over-invest dev time defending the free door — invest it in the ladder.

**Risk 3 — Expansion triggers don't fire (the $100k bridge collapses to the $30–50k base case).**
The entire aggressive case depends on dragging a fraction of Team accounts to Growth and a handful to Enterprise. If the donut/expansion nudges underperform, you're back to the base case — because new-logo self-serve alone tops out around $35–45k.
- **Mitigation:** Treat expansion triggers as **first-class A/B'd product surfaces, instrumented from Q1**, not afterthoughts. Ship the read-only donut teaser in Team (open question (d) resolved toward teasing). Set a hard checkpoint: if **<15% of Team accounts touch a Growth trigger by end of Q2 / NRR <100%**, stop and fix monetization before scaling acquisition — pouring more free signups into a leaky expansion funnel just burns the SEO lead. Measure NRR weekly from first paying team.

**Risk 4 — Founder is the bottleneck on three incompatible jobs (part-time dev + manager + enterprise closer).**
In Q3 the founder must run an async enterprise motion (pilots, Loom demos, contracts) *while* still part-time-coding *while* managing. These compete; enterprise closing usually loses to firefighting, and enterprise is the highest $/effort rung.
- **Mitigation:** Founder **stops coding by ~Q3** — convert founder-dev hours into enterprise-close + management once the 2nd dev lands (sequencing depends on Risk 1's hire). Keep enterprise deliberately **self-serve-assisted** (Calendly + Loom + a pilot template), never field sales, so it fits part-time. Add the part-time enterprise CS at ~$45–60k MRR to absorb post-sale load. If founder time is the binding constraint, **prioritize closing 2–3 enterprise deals over chasing the 200th Team account** — the ACV math favors it overwhelmingly.

---

*Source docs: `C:/Users/gsent/Desktop/2027-q1-projects/CCGuard/gtm/product-marketing-context.md`, `phase2-gtm-blueprint.md`, `phase2-first-build-spec.md`. Roadmap re-weights the blueprint's base case toward expansion + enterprise per the founder's $100k directive; numbers stay inside the docs' honest ranges (blended ARPA ~$169, $15–45k enterprise ACV, 12–20 mo window).*