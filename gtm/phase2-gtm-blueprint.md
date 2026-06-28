# Phase 2 GTM Blueprint — CCGuard / Claresso

> Execution doc for the 3-person team (1 FT dev `D`, 1 FT marketer `M`, 1 founder `F` who manages + builds part-time). Bootstrapped, ~$0 ad budget, organic-led. Markets: US / Canada-ex-Quebec / Australia / Singapore / NZ. EU/UK inbound-only. Built on the locked decisions in `gtm/product-marketing-context.md` (§11, §12).

---

## 1. The Strategy in One Paragraph

We sell **cross-tool AI-coding exposure monitoring you can actually buy with a card.** We acquire on a **commodity we give away** — a free, local, one-command scanner that finds leaked API keys and PII across Claude Code, Cursor, and Copilot in a single pass — because that's the only thing devs are already searching for, and the funded incumbent (GitGuardian) doesn't defend the long-tail. We win the search and the share with the one thing single-tool OSS scripts structurally can't reproduce: **all three tools in one scan, on a screenshot-ready Exposure Report card.** Then we **defend and monetize on the moat nobody else can claim in one breath** — a team rollup dashboard, work/personal attribution (the donut), and self-serve card-pay above the free tier (every competitor hits a "Let's Talk" wall). Detection is the free front door; the un-copyable bundle is the product. We have a **12–20 month window** before Anthropic-native detection commoditizes the door, so we bank the moat into Google and into paying teams now, with a content/SEO flywheel + a product-led viral loop a 3-person team can actually run for $0.

---

## 2. Final Positioning

**Category:** Self-serve, local-first **cross-tool AI-coding exposure monitoring for teams** — the AI-session secret & PII scanner you can turn into a team dashboard with a card. (Deliberately NOT "secrets scanner," NOT "shadow-AI/AI-DLP governance," NOT "AI cost/FinOps" — each is either commoditized, undeliverable on a voluntary on-device agent, or given away free.)

**Wedge sentence (no competitor can honestly say it):**
> "Finds leaked keys & PII across every AI coding tool your team uses — Claude Code, Cursor, Copilot — runs entirely on your machine, and is the only one you can turn into a team dashboard by paying with a card, no demo, in 5 minutes."

**Locked headline hierarchy:**

| Surface | Headline |
|---|---|
| **Acquisition** | See every secret your AI coding tools already leaked — then stop the next one. Free, local, 5-minute scan across Claude Code, Cursor, and Copilot. |
| **Conversion** | The only AI-coding security you can buy with a card. No demo. No sales call. Live in 5 minutes. |
| **Retention / upsell** | One scan just became your team's exposure dashboard — now see which AI sessions were company work, and which weren't. |

**Four messaging pillars:** (1) One scan, every AI coding tool — not one tool, not three hooks, not a screen OCR. (2) Local-first & dev-transparent — content never leaves the machine, the dev sees their own findings first (the deliberate anti-Teramind motion). (3) Buy it with a card — no demo, no procurement, live in 5 minutes. (4) Your personal scan becomes the team's exposure dashboard, including work-vs-personal attribution (the donut).

**Top objections + answers:**

| Objection | Answer (1-line) |
|---|---|
| "Isn't detection already free (OSS / GitGuardian / Anthropic-native)?" | Yes — so we give detection away too; you pay for the un-copyable bundle: all 3 tools in one scan + team rollup + donut + card-pay past the free tier. None of them can say that in one breath. |
| "We don't want employee surveillance — devs will revolt." | We're the anti-Teramind: local-first, visible agent, no screen/keystroke capture, and the dev sees their own findings before any manager aggregate — which turns the dev into the person who installs it. |
| "Anthropic will ship this natively and kill you." | They're shipping it for *detection, on Claude Code only* — that's our clock (12–20 mo), not our killer; we defend on cross-tool coverage + dashboard + donut + card-pay, which Anthropic structurally won't build. |
| "Can't I fix this with .gitignore + a vault + pre-commit hooks?" | Those protect the repo/commit; the key was already in the prompt/AI context before any file hit disk (AI commits leak at 2.1×). We read the session transcript hooks never see — and we recommend you keep all three. |
| "Your free tier already scans my sessions — why pay?" | The solo local scan is free forever; you pay the moment "me" becomes "my team" — you can't ask 30 devs to run a CLI and Slack screenshots weekly. Paid = rollup, history, attribution. |

**What we are NOT:** not a better secrets scanner, not employee surveillance, not a shadow-AI discovery tool, not a FinOps/spend dashboard, not an enterprise sales-led/demo-gated product, not a compliance/eDiscovery platform at acquisition.

**Naming:** Launch the public product as **Claresso** (already the user-facing brand in proxy messages, tool-agnostic, trademarkable); keep **CCGuard** as the repo/code name only. "CC" reads Claude-Code-specific and fights the cross-tool moat; "*Guard" signals the enterprise/sales motion we're escaping. Do NOT run a net-new naming project pre-PMF.

---

## 3. Final Pricing

**Free-tier line:** *Free stops at the **team boundary**, not the feature boundary.* Free = one person scanning their **own** machine — unlimited local scans, all tools in one pass, full secret/PII findings, the shareable redacted Exposure Report card. No seat cap, no scan cap, no card, no expiry, content never leaves the device. **Paid begins the moment a finding must leave one laptop and become a team artifact** (cloud rollup, history/trend, scheduled/CI scans, alerting, the donut). The two viral mechanics — the SEO-ranking free scan and the screenshot-shared card — are 100% free and ungated, so the acquisition loop never breaks.

**Tiers (lead with the flat headline):**

| Tier | Price | Who | What's in |
|---|---|---|---|
| **Free (the Door)** | $0 forever | Solo / lead dev self-auditing; top of the viral loop (ICP-c) | Local cross-tool scanner, unlimited scans, full findings, dev-sees-own-data, shareable Exposure Report card, CLI + optional pre-commit hook. No account, no card. |
| **Team** | **$99/mo flat, up to 10 devs** ($12/seat overage; $79/mo annual) | Eng manager / founder-CTO at 10–50 devs (ICP-b/c) | Free + cloud team dashboard, cross-tool rollup, 90-day history & trend, scheduled scans, CI/PR gate, Slack+email alerts, baseline policy. Card-pay, 5 min, no demo. |
| **Growth** | **$299/mo flat, up to 30 devs** ($10/seat overage; $249/mo annual) | AppSec/DevSecOps lead or VP Eng at 30–200 devs (ICP-a/b) | Team + **THE MOAT: work/personal donut**, spend-visibility panel, multi-team, 12-mo retention, custom detector packs/regex, per-repo & per-dev drilldown, audit export. Still self-serve. |
| **Enterprise** | Custom (~$15k–45k ACV) | Security/compliance buyer at 200+ devs | Growth + SSO/SCIM, MDM/managed-settings fleet deploy, proxy enforcement, DPIA/LIA, eDiscovery, self-host. The only sales-touch — intentionally. |

**Price-unit decision: HYBRID, leaning FLAT-per-team with seat bands + a clearly-priced per-seat overage.** Lead all marketing with the flat number ("$99/mo, up to 10 devs"); teams that outgrow a band pay a small overage rather than being forced up. Why: (1) predictability beats per-seat for card-buyers approving an expense; (2) **Aikido has proven this exact flat-per-team model in the exact adjacent space** ($300/mo, 10 users, "fixed rate, not doubling per head"); (3) per-seat couples revenue to volatile 2026 headcount — every layoff is an involuntary downgrade; (4) seat audits are a recurring downgrade ritual flat bands remove; (5) the pure-flat downside (capped expansion) is solved by overage + the $99→$299 band jump. **Avoid pure per-seat (Snyk/Semgrep/GitGuardian) — that's a procurement model, and our whole wedge is "buy it with a card."**

**MRR math (blended ARPA ~$169 at ~65% Team / 30% Growth / 5% Enterprise-as-Growth + modest overage):**

| Target MRR | All-Team ($99) | All-Growth ($299) | Blended ($169) |
|---|---|---|---|
| **$30k** | 303 teams | 100 teams | **~178 teams** |
| **$50k** | 505 teams | 167 teams | **~296 teams** |
| **$100k** | 1,010 teams | 335 teams | **~592 teams** |

To hold ~300 paying teams (≈$50k) at month 12 with 4%/mo churn, you must gross-add ~35–45 paying teams/mo by end. At a 1.5–2.5% free→paying-**team** conversion (below the 3–6% freemium median because the unit is a *team*, not a seat), that needs **~15,000–25,000 cumulative free signups over 12 months.** Organic supply: 1–2 Show HN front-page hits (low thousands) + PH #1 (200–600) + the compounding SEO engine ramping to ~3,000–6,000 organic tool-signups/mo by month 12 if the content engine hits.

**Honest verdict:** **$30–50k MRR is the defensible base case** an all-organic 3-person team can hit in 12 months — and it's the **plan of record ($50k).** **$100k is the stretch ceiling,** reachable ONLY if (a) a Show HN/PH breakout compounds, (b) SEO out-ranks GitGuardian's free ggshield content, AND (c) revenue is reinvested into newsletter sponsorships from ~month 5. The flat-per-team model *helps* — it lifts blended ARPA to ~$169, so ~296 teams clears $50k instead of chasing 2,040 seats.

---

## 4. GTM Motion + Launch Sequence

**Core PLG loop — Scan → Shock → Share → Team rollup → Pay:**
1. **Acquire (free, local):** dev lands via long-tail SEO or a shared card, runs `npx ccguard scan` — no account, nothing leaves the machine.
2. **Shock:** "Scanned 47 sessions across 3 AI tools — found 3 LIVE secrets + 2 PII." The cross-tool framing is the activation moment single-tool scripts can't reproduce.
3. **Share:** scan emits a redacted, screenshot-ready Exposure Report card ("🔴 3 live secrets · 2 PII · Claude Code, Cursor, Copilot · 100% local"); `--share` mints a public `claresso.dev/r/<id>` page with a "scan your own" CTA → backlinks feed Step 1.
4. **Expand trigger:** CLI prints "Want this across your whole team? Run `ccguard invite`" → free team workspace, scans roll up. **The gate sits on the team rollup, never the individual scan.**
5. **Pay (card, no demo, 5 min):** 2nd+ teammate or need for continuous monitoring/history/donut hits the Stripe paywall.

**3 ranked motions:**
1. **Free-tool-led SEO / content engine (the flywheel).** Ship the cross-tool scanner as an OSS link-magnet + 10 high-intent long-tail articles (flagship: "How to scan your Claude Code chat history for leaked API keys (free)"). *Why #1:* the only motion that is simultaneously $0, compounding, and defensible; targets demand searched-without-education on a tail incumbents don't defend; still pays out in month 12. Effort: high upfront, compounding; ~6–8 wks to first rankings, M ~70% allocated ongoing.
2. **Product-led viral loop (Exposure Report share + team invite).** Treat the redacted cross-tool card as a first-class growth surface; A/B single-tool vs cross-tool to prove the share lift; `invite → rollup → Stripe` is the monetization engine. *Why #2:* zero-marginal-cost distribution that turns every shocked user into an impression and every team into an account; converts spiky launch traffic into a self-sustaining loop. Effort: medium, dev-heavy upfront (~3–4 wks).
3. **Community-led launch cadence + backlink seeding (HN / Reddit / PH / GitHub Marketplace / newsletters / directories).** Sequenced launches anchored by incident hooks (Miasma, TrustFall CVE-2025-59536, the 2.1× stat), free scanner as payoff. *Why #3:* provides the cold-start — first thousand users, the high-authority backlinks that bootstrap Motion 1, the share events that seed Motion 2 — but spiky and non-compounding, so it *ignites* the other two rather than being the engine. Effort: founder-time-intensive on launch days, concentrated weeks 5–9.

**Launch order & timing (SEO base exists BEFORE spikes; never two big launches in one week; HN before PH):**

| Week | Launch | Asset / note |
|---|---|---|
| **5** | GitHub repo public + first OSS release | Polished README, install GIF, 3 seed articles live; soft-seed 10–20 friendly devs. (Foundation, not a launch.) |
| **6** | Dev-tool directories + GitHub Marketplace | Marketplace CI Action, Awesome-Claude-Code, alternativeto ("ggshield alternative"), libhunt, console.dev — evergreen backlinks indexed before the spike. |
| **7** | **Show HN** (Tue–Thu, 8–10am ET) | "Show HN: I scanned my AI coding history and found 3 leaked API keys — free local tool." Founder camps the thread all day. ~2.3% front-page → 8–15k visitors. |
| **7 (+2d)** | Reddit, staggered & natively rewritten | r/ClaudeAI (tool) → r/netsec (2.1× data + CVE) → r/devops (team rollup). Never identical cross-posts. |
| **8** | Dev newsletters | Pitch the DATA story ("AI tools leak secrets at 2.1×") + HN traction to TLDR InfoSec (primary), TLDR, Console.dev, Changelog, Pointer. Tool is the link. |
| **9** | **Product Hunt** (12:01am PT Tue/Wed) | Gallery + 60s demo + card-pay tagline; mobilize wk5–8 free users to upvote. #1 dev tool ≈ 200–600 signups. **Team dashboard PAID GA + Stripe live.** |
| **10–12** | Continuous / evergreen | 2 articles/wk, run the share loop, convert the cohort, re-pitch newsletters with "first paying teams." Schedule a 2nd smaller HN/Reddit beat if the first under-delivered. |

---

## 5. The 5 Open Questions — Resolved

| # | Question | **Decision** | Why (1 line) |
|---|---|---|---|
| **a** | Free-tier size / gate | **Free = unlimited cross-tool local scans for ONE individual; gate on the TEAM rollup, not features or scan count.** | Metering the scan would throttle the SEO/share loop that *is* the acquisition engine; charge only to turn many scans into a managed team surface. |
| **b** | First buildable artifact | **(a)+seam: a standalone free local CLI scanner with ONE hosted `--share` command** (mints a public redacted report URL + captures an email). No auth/RBAC/billing/dashboard in v1. | A signup wall on the acquisition artifact contradicts "acquire on the commodity, frictionless"; the seam quietly accrues the warm email list + report data substrate that v2's dashboard is built on. |
| **c** | Price unit | **Hybrid: flat-per-team headline ($99/$299) with seat bands + a clearly-priced per-seat overage.** | Predictable flat bills convert card-buyers (Aikido proves it in-space) and decouple revenue from volatile headcount; overage + band-jump recover expansion without per-seat's churn surface. |
| **d** | Donut timing | **Donut ships at Growth ($299), NOT at the door — with a read-only teaser A/B'd in Team.** | Demand for the donut is low/unsearched so it can't acquire, but it's the single most un-copyable retention/defense feature, so it gates expansion; teaser hedges the risk it's the real reason teams stay. |
| **e** | Anthropic-native sequencing | **Race them: acquire on the commodity (free detection) NOW and bank the moat (cross-tool + dashboard + donut + card-pay) into Google + paying teams during the 12–20 mo window.** | Anthropic ships detection for Claude Code *only*; their Compliance API + 60 partners explicitly exclude Claude Code terminal sessions — the moat is precisely what they structurally won't build. |

---

## 6. First 90 Days — Week-by-Week (D = dev, M = marketer, F = founder)

### Phase A — Build the Loop (Wks 1–4, no public launches)

| Wk | D | M | F |
|---|---|---|---|
| **1** | Scaffold free CLI `ccguard scan` on `findings.rs` + cross-tool capture; local terminal summary | Keyword map + lock 10-article calendar; draft flagship article #1 | Finalize free/paid gate (team-rollup gate), Stripe price decision ($99/$299 flat), set up analytics (PostHog/Plausible) |
| **2** | Build redacted Exposure Report card (terminal + PNG) + `--share` → public `claresso.dev/r/<id>` page | Publish articles #1 + #4; build landing page (headline A) | Register brew/npx/pipx packages; set up GitHub org + repo skeleton |
| **3** | Instrument scan→share→invite events; build `ccguard invite` → free read-only team rollup | Articles #5 (the 2.1× linkbait) + #8 (Miasma/TrustFall); write Show HN + Reddit drafts | Recruit 10–20 dogfood devs; line up a PH hunter |
| **4** | Stripe paywall on team dashboard + continuous monitoring v1; polish README + demo GIF | Articles #2 (ggshield alternative) + #7 (comparison SERP); prep Marketplace + directory listings | Private beta across 20 dogfooders, collect "3 keys" testimonials. **GATE: install→scan→report <5 min on a clean machine before any launch.** |

### Phase B — Cold-Start Launches (Wks 5–9)

| Wk | D | M | F |
|---|---|---|---|
| **5** | Monitor install telemetry, fix day-1 bugs fast | Start daily authentic Reddit/community participation (value-first) | Repo + first OSS release public (3+ articles live). Target: 100–300 organic scanners |
| **6** | Ship share-rate instrumentation + single-tool-vs-cross-tool A/B | Submit Marketplace + all directories; publish articles #3 + #6 | Oversee directory/Marketplace copy |
| **7** | On-call for traffic spike + bug fixes; capture every visitor into scan funnel | Help rewrite Reddit posts per sub | **Show HN (camps thread); staggered Reddit.** Target: 1–3k scanners, first team workspaces |
| **8** | Harden team dashboard for paid GA | Newsletter outreach (DATA story + HN traction); publish articles #9 + #10 (donut angle) | Follow up warm newsletter replies |
| **9** | Team dashboard PAID GA + Stripe live | Drive email/notify list to PH | **Product Hunt launch; works the thread.** Target: 200–600 signups, **FIRST PAYING TEAMS** |

### Phase C — Convert & Compound (Wks 10–13)

| Wk | D | M | F |
|---|---|---|---|
| **10** | Fix the single biggest drop-off before `invite`; tighten paywall trigger to <5-min time-to-paid | 2 articles/wk; double down on whichever query is ranking | Full-funnel review |
| **11** | Ship the work/personal donut as the headline paid upsell, activate in-dashboard | Re-pitch newsletters with "first paying teams" proof; refresh winning HN/Reddit angle | Warm follow-ups |
| **12** | Ship continuous-monitoring alerts (retention hook) | Optimize proven SEO pages (internal links, schema, expand cluster) | Review unit economics vs ~$300 ARPA floor + churn |
| **13** (buffer/retro) | Support 2nd beat | Second smaller Show HN/Reddit tied to dashboard GA or a fresh incident | Decide on the ~$100 month-1 ad test against the best-converting query; set next-quarter cadence |

**Day-90 target state:** scanner is the indexed link-magnet free tool; 3–6 long-tail pages ranking; Exposure Report loop running with a measured share rate (target >15–20% of shocked scans); **~5–20 paying teams**; a defensible cross-tool + donut + card-pay moat in market.

---

## 7. The Honest Bottom Line

**Best realistic 12-month MRR:** **$30–50k is the defensible base case and the plan of record; $100k is the stretch ceiling, not the line.** A 3-person, $0-ad, all-organic team can credibly reach ~178–296 paying teams in 12 months. Plan, staff, and report against $50k; treat $100k as upside that requires a launch breakout to compound.

**What has to go right:** (1) the free CLI is genuinely frictionless and beats OSS hooks on the activation moment (cross-tool shock in <60s, zero signup); (2) the SEO engine actually *ranks* on the long-tail GitGuardian doesn't defend — this is the durable month-12 engine and the single biggest dependency; (3) at least one Show HN/PH breakout lands to bootstrap the backlinks; (4) the Exposure Report card out-shares single-tool scripts (the defensible-virality test); (5) free champions convert to paying *teams* at ≥1.5–2.5%; (6) revenue gets reinvested into newsletters from ~month 5.

**The single biggest risk: GitGuardian + Anthropic commoditize the door faster than we bank the moat.** GitGuardian gives free ggshield Claude Code hooks and Anthropic is shipping native Claude Code security — if free detection becomes a table-stakes OS feature before our SEO ranks and our paid moat is in market, the entire organic funnel-fill assumption erodes. **The 12–20 month window is the clock, and everything above is a race to convert commodity acquisition into the un-copyable, card-payable team bundle before that window closes.** Mitigation is baked into the sequencing: acquire on the commodity now, defend on cross-tool + donut + card-pay — the exact things Anthropic structurally won't build and OSS scripts can't reproduce.