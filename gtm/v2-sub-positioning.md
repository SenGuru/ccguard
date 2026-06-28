I have read the full corpus and the product-reality specs. Here is the v2 positioning, re-grounded and with v1 over-reaches corrected.

---

# CCGuard / Claresso — v2 Positioning (evidence-grounded)

## 0. What changed from v1, in one paragraph

v1 (phase2 §1–2, phase3 §1) was directionally right on the **acquire-on-commodity / retain-on-bundle** thesis, but it inflated that bundle into an "un-copyable moat" — language the research explicitly contradicts. Phase1b §3 and the locked decision (product-marketing-context §12) both state plainly: *"Defensibility of any single feature is low (2-3/10). The 12–20 month window is the clock."* So the honest asset is **not** a moat in the structural sense — it is (a) a **configuration + motion no incumbent currently offers in one breath**, (b) a **time window**, and (c) a **retention/switching-cost layer** (team history + the donut) that accrues *after* adoption. v2 keeps the winning wedge and corrects every place v1 claimed durability the evidence doesn't support — most importantly the blanket "only AI-coding security you can buy with a card" (false: Snyk ships a self-serve "Buy Now" Team tier — phase1-appendix, Snyk) and the paid "spend-visibility panel" (the product cannot honestly deliver accurate dollars — phase1 §5, RESEARCH-FINDINGS §1).

---

## 1. Category

**AI-coding exposure monitoring** — self-serve, cross-tool, local-first; the AI-session secret & PII scanner a team can turn into a shared dashboard with a card.

Deliberately **not**: "secrets scanner" (commoditized — phase1b §2; locked §11.2), "shadow-AI / AI-DLP governance" (structurally undeliverable on a voluntary on-device agent — phase1b §2 / TOP-2 teardown), or "AI cost/FinOps" (undeliverable at dollar-accuracy + commoditized by Anthropic-native — phase1 §5, RESEARCH-FINDINGS §1).

*Why a new category label and not "secrets scanner":* the gap-map (phase1-market-truth §3) shows every incumbent is blind to or fragmentary on the **Claude Code terminal session** layer; "exposure monitoring" names the session layer rather than the commoditized detection act.

---

## 2. Wedge sentence (no competitor can truthfully say it in one breath)

> **"Claresso finds the leaked keys and PII across every AI coding tool your team uses — Claude Code, Cursor, Copilot — runs the scan entirely on your machine, and is the only one that lets a whole team turn that into a shared exposure dashboard by paying with a card, no demo, in 5 minutes."**

This is carried over from product-marketing-context §12 / phase1b §6 **with one correction**: the durable, true clause is *"a whole team … shared dashboard … by card, no demo"* (phase1 §4 lesson; gap-map sales walls — GitGuardian Business "Let's Talk", Semgrep Teams "Contact us", all shadow-AI sales-led). The shorter v1 boast *"the only AI-coding security you can buy with a card"* is **false as written** — Snyk's Team tier is a self-serve "Buy Now" up to 10 devs (phase1-appendix, Snyk). The honest differentiator is the **conjunction** (cross-tool + AI-session level + whole-team rollup + card-pay above a real free tier), not any single clause.

---

## 3. Headline hierarchy

| Surface | Headline | Grounding |
|---|---|---|
| **Acquisition** (problem-first, searched) | **"See every secret your AI coding tools already leaked — then stop the next one."** *Sub: Scan your Claude Code, Cursor, and Copilot sessions for exposed API keys, tokens, and PII. Free, local, 5-minute install — the scan never leaves your machine.* | phase1b §4 (Headline A, winner); VoC bank phase1 §7 ("scan Cursor/Claude chat history for leaked API keys") |
| **Conversion** (pricing page) | **"The only cross-tool AI-session security your whole team can buy with a card. No demo. Live in 5 minutes."** | phase1 §4 (card-pay = the wedge); **corrected** from v1's over-broad "only AI-coding security you can buy with a card" |
| **Retention / upsell** (in-product) | **"Your scan just became your team's exposure dashboard — now see which AI sessions were company work, and which weren't."** | total-visibility §0.2 (classifier is the moat); phase1 §5 (donut = upsell, not door) |

Note the deliberate ordering: detection-pain headline acquires (it is what is searched without education — phase1 §5 item 2), card-pay headline converts, donut headline *retains*. The donut never appears at acquisition — its demand is 2/10 and must be educated into existence (phase1 §5 item 4; appendix work/personal section).

---

## 4. Messaging pillars (each with its research proof)

**Pillar 1 — One scan, every AI coding tool.** Not one tool, not three hook checkpoints, not a blurry screen OCR.
*Proof:* gap-map (phase1 §3) — GitGuardian sees 3 hook checkpoints, Semgrep post-file-write only, Teramind screen-OCR (Mac/Linux degraded), Purview/Anthropic-Compliance exclude the CC terminal entirely (RESEARCH-FINDINGS §1; total-visibility §7). Cross-tool local scanning is *demonstrated feasible* — OSS tool Sieve already scans Cursor/Claude/Copilot/Cline chat history on macOS (phase1-appendix, secrets section). **Calibration:** depth-of-coverage parity across Cursor/Copilot is asserted "by design" (product-marketing-context §1) but the deep capture research (tracking-surface, total-visibility) is ~90% Claude-Code-specific — see Evidence Gaps.

**Pillar 2 — Detection is free; you pay for the team layer.** We give the scanner away because the detection moment is already a commodity.
*Proof:* phase1b §2 — `sensitive-canary`, `leakproof`, `Claudleak`, free `ggshield` hooks (≤25 devs), and Anthropic-native Claude Code Security all reproduce the "holy crap, 3 keys" moment for $0; locked §12 ("Detection is NOT the moat"). This pillar pre-empts the #1 objection by conceding it.

**Pillar 3 — Card-pay, no demo, whole-team self-serve.** The team buys session-level AI-exposure governance the moment it decides, not after a procurement cycle.
*Proof:* phase1 §4 ("GitGuardian, Semgrep, and Snyk all fail at true self-serve above their free tier — every one hits a 'Contact us' / sales wall … CCGuard's differentiated PLG act is to actually let a team pay by card"); gap-map sales walls. **Scope discipline:** claim *team-level cross-tool AI-session governance by card*, never "the only AI security you can buy by card" (Snyk Team self-serve exists — phase1-appendix).

**Pillar 4 — Local-first, dev-transparent (the anti-Teramind motion).** The raw session content stays on the device; the dev sees their own findings before any manager aggregate.
*Proof:* total-visibility §0.4 ("Maximal capture is the #1 sales-killer; transparency is the #1 differentiator"); design principles (product-marketing-context §1: "content never leaves the machine; PERSONAL never silently flagged; dev sees own data first"); Teramind gap-map row ("dev-transparent, not dev-hostile surveillance"). **Scope discipline:** "content never leaves the machine" is literally true only for the **free local scan**; the **paid** dashboard sends *redacted findings + metadata* to the cloud rollup (phase2 pricing §3 concedes this). Copy must say "the scan runs locally / only redacted findings leave the device," not imply the paid product is air-gapped.

**Pillar 5 — Your scan becomes the team's exposure dashboard, with work-vs-personal attribution (the donut) — the retention layer.**
*Proof:* total-visibility §0.2 ("The moat is the classifier, not the capture … repo-attribution … no one has it"); §5.1 (repo-allowlist classifier = "we watch the repo, not the person"). This is the single most differentiated *feature* (no named competitor — phase1 §5 / appendix), but it is a **switching-cost / expansion** layer, not an acquisition wedge (demand 2/10) and not "un-copyable" (native Cursor/Copilot/Claude admin dashboards already surface repo-level usage free — appendix work/personal section).

---

## 5. The "secrets-door + bundle-moat" thesis — grounded and corrected

**The thesis, stated honestly:**

1. **Door = the commodity** (session-level secret/PII detection). It is heavily searched *without education* and incident-backed (Miasma, TrustFall/CVE-2025-59536, GitGuardian SoSS 3.2% vs 1.5% = 2.1× — RESEARCH-FINDINGS §2), and it is *free* (phase1b §2). We win the **long-tail SERP** GitGuardian doesn't defend (`scan claude code chat history for leaked api keys`, `ggshield claude code alternative` — phase1b §5; SEO score 4, highest in the set — phase1b §1) and the **share loop** (cross-tool Exposure Report card out-shares single-tool scripts — phase1b §5).

2. **"Moat" = a temporary configuration + a retention layer, not structural defensibility.** This is the core correction. Phase1b §1/§3 score every component's defensibility 2-3/10. What CCGuard actually owns is:
   - a **bundle no competitor offers in one breath today** (cross-tool + team rollup + donut + card-pay) — true, but *each clause is individually copyable*;
   - a **12–20 month window** before Anthropic-native detection + GitGuardian free hooks close the door (locked §12; phase2 §7 "the single biggest risk");
   - a **retention/switching-cost layer** that accrues *after* adoption — accumulated team history, the donut, the dev-trust install base (total-visibility §0.2, Three Defensible Angles §7.1–7.3).

   **Defensibility therefore = execution speed + data/history accumulation + SERP brand + dev-trust install base — NOT an "un-copyable" feature.** Every place v1 said "un-copyable moat" / "the moat nobody else can claim" (phase2 §1, §2) should read "the configuration no incumbent currently offers, which we must convert into retention before the window closes."

3. **What makes the bundle *durable enough to matter* is the gap-map structure, not the features:** the incumbents who *could* copy any one clause are structurally disinclined to copy the **motion** — GitGuardian/Snyk/Semgrep's economics are enterprise-sales ACVs (~$45–54k median — RESEARCH-FINDINGS §6), so "card-pay, no demo, $99/team" is *off-strategy* for them, not impossible (phase1 §4). Anthropic structurally won't build cross-tool or work/personal (it's a single-vendor platform — phase1 §8; total-visibility §0.3). That **strategic disinclination**, plus the window, is the realistic defense — and it is weaker and more time-bound than "moat" implies.

---

## 6. Top objections + answers

| # | Objection | Answer | Grounding |
|---|---|---|---|
| 1 | "Isn't detection already free (OSS / ggshield / Anthropic-native)?" | Yes — so we give detection away too. You pay for the layer none of them offer in one breath: all three tools in one scan + team rollup + work/personal attribution + card-pay above the free tier. | phase1b §2; locked §12 |
| 2 | "We don't want employee surveillance — devs will revolt." | We're the anti-Teramind: the scan runs locally, the agent is visible, no screen/keystroke capture, and the dev sees their own findings before any manager aggregate — which is why the dev is the one who installs it. | total-visibility §0.4, §7.3; Teramind gap-map row |
| 3 | "Anthropic will ship this natively and kill you." | They're shipping detection, for Claude Code only — that's our clock (12–20 mo), not our killer. Their Compliance API + 60 partners explicitly exclude the Claude Code terminal, and they won't build cross-tool or work/personal. We bank those into retention before the window closes. | RESEARCH-FINDINGS §1; phase1 §8; **honest: this is a window, not immunity** |
| 4 | ".gitignore + a vault + pre-commit hooks already fixes this." | Those protect the repo/commit; the key was already in the prompt and AI context before any file hit disk (AI commits leak at 2.1×). We read the session transcript hooks never see — keep all three. | RESEARCH-FINDINGS §2; gap-map (Snyk "starts where the breach already happened") |
| 5 | "Your free tier already scans my sessions — why pay?" | The solo local scan is free forever. You pay the moment "me" becomes "my team" — you can't ask 30 devs to run a CLI and Slack screenshots weekly. Paid = cloud rollup, history, alerting, attribution. | phase2 §3 (gate on the team boundary, not the feature) |
| 6 | "Is your cross-tool coverage real, or just Claude Code with two logos?" | Honest answer: Claude Code is deepest (full on-disk transcript, zero config); Cursor/Copilot we scan their local chat history for secrets/PII (same as OSS Sieve). Deep session governance is strongest on Claude Code today and expanding cross-tool. | tracking-surface §2; phase1-appendix (Sieve); **flagged gap below** |
| 7 | "What actually stops GitGuardian from copying this next quarter?" | Nothing structural — and we won't pretend otherwise. What protects us is speed, the long-tail SERP and dev-trust install base we build in the window, and that card-pay/no-demo is off-strategy for their $45k-ACV sales model. The donut + accumulated team history are the switching costs that make leaving us expensive once you're in. | phase1b §1/§3 (def 2-3/10); phase1 §4; **corrects v1 "un-copyable"** |
| 8 | "Can you show me our real AI spend / which seats cost what?" | We show **relative** session counts and where exposure lives — not authoritative dollars. Accurate AI spend needs Anthropic's Enterprise Analytics API you don't have, and raw token counts undercount ~46×. Spend is how we get your attention, not what we invoice you to fix. | phase1 §5; RESEARCH-FINDINGS §1; locked §11.6 — **corrects v1's paid "spend-visibility panel"** |

---

## 7. What we are NOT
Not a better secrets *scanner* (detection is free); not employee surveillance; not a shadow-AI discovery tool (the agent only sees machines it's installed on — phase1b TOP-2 teardown); not an authoritative AI-spend/FinOps dashboard (undeliverable — phase1 §5); not an enterprise sales-led/demo-gated product at acquisition; not a compliance/eDiscovery platform at the door (that's the enterprise ladder — locked §11.5).

---

## Grounding ledger

| Claim | Source (file §) | Confidence |
|---|---|---|
| Detection is a free commodity (OSS + ggshield + Anthropic-native); not the moat | phase1b §2; product-marketing-context §12 | High |
| No competitor captures the Claude Code *terminal session*; Anthropic Compliance API + 60 partners exclude it | RESEARCH-FINDINGS §1; phase1 §3; total-visibility §0.3, §2 | High |
| Every secrets/SAST/shadow-AI incumbent hits a sales wall above free → card-pay is the wedge | phase1 §4; phase1-appendix (GitGuardian/Semgrep "Contact us") | High |
| **Snyk Team tier IS self-serve card-pay (≤10 devs)** → "only AI security you can buy by card" is false | phase1-appendix (Snyk: "self-serve 'Buy Now' button, no demo required") | High |
| AI-assisted commits leak secrets at 2.1× baseline (3.2% vs 1.5%) | RESEARCH-FINDINGS §2; product-marketing-context §2 | High |
| Cross-tool local secret scanning is feasible (OSS Sieve scans Cursor/Claude/Copilot) | phase1-appendix (secrets/existing) | High |
| Donut (work/personal repo attribution) has no named competitor but demand 2/10, unsearched → feature/retention, not wedge | phase1 §5; appendix work/personal; locked §11.1 | High |
| Defensibility of any single feature is low (2-3/10); the real asset is the 12–20 mo window | phase1b §1, §3; product-marketing-context §12 | High |
| Repo-attribution classifier = the differentiating asset ("watch the repo, not the person") | total-visibility §0.2, §5.1, §7 | High |
| Transparency/dev-sees-own-data is the #1 differentiator; maximal capture is the #1 sales-killer (anti-Teramind) | total-visibility §0.4, §7.3 | High |
| Accurate AI spend is undeliverable self-serve (tokens undercount ~46×; needs Enterprise Analytics API) | phase1 §5; RESEARCH-FINDINGS §1; locked §11.6 | High |
| Acquisition headline A + card-pay conversion headline | phase1b §4 | High (carried, B reworded) |
| Long-tail SEO (score 4, highest) is winnable; head terms are not | phase1b §1, §5 | Medium (SEO scores are analyst estimates, not measured) |
| $30–50k MRR base case / $100k stretch | phase2 §7; phase3 §7; locked §11.3 | Medium (projection, not fact) |
| Cross-tool *deep governance* (not just scanning) at parity across Cursor/Copilot | product-marketing-context §1 ("by design") | **Low — asserted, not validated** |

---

## Evidence gaps / v1 over-reaches

1. **"Un-copyable moat" → corrected to "configuration + window + retention layer."** v1 (phase2 §1–2, "the un-copyable bundle," "the moat nobody else can claim") directly contradicts the corpus's own finding that single-feature defensibility is 2-3/10 and the window is the clock (phase1b §1/§3; locked §12). Every component is individually copyable; durability comes from speed, accumulated history, dev-trust install base, and incumbents' *strategic disinclination* — all weaker and more time-bound than "moat." **Corrected throughout.**

2. **"The only AI-coding security you can buy with a card" is false.** Snyk ships a self-serve "Buy Now" Team tier (phase1-appendix). The true claim must be scoped to **cross-tool, AI-session-level, whole-team** governance above a free tier. v1's conversion headline (phase2 §2) over-reaches. **Reworded.**

3. **Paid "spend-visibility panel" (phase2 §3 Growth tier) is an honesty risk.** The product cannot deliver accurate dollars (phase1 §5; RESEARCH-FINDINGS §1) and locked §11.6 restricts spend to a *content hook only*. Selling a paid spend panel invites churn on a promise the product can't keep. **Demote to relative/exposure framing; do not invoice it.**

4. **Cross-tool depth is asserted, not validated — the single biggest product-reality risk to the entire positioning.** The whole differentiated wedge rests on "one scan, all three tools," yet the deep-capture research (tracking-surface, total-visibility) is ~90% Claude-Code-specific, and product-marketing-context §1 marks cross-tool only as "by design." Cross-tool *scanning* is grounded (Sieve precedent); cross-tool *deep session governance + work/personal attribution + git provenance* at Claude-Code parity is **unproven in the corpus**. If Cursor/Copilot coverage is shallow, the moat narrows to "Claude Code + thin cross-tool." **Recommend an explicit product-truth audit before this becomes the headline claim.** (assumption — not validated in research)

5. **"Content never leaves the machine" is true only for the free local scan.** The paid cloud rollup sends redacted findings + metadata off-device (phase2 §3 concedes it). v1 Pillar 2 (phase2 §2) states it as an unqualified product property. **Scope copy to the scan, not the paid product.**

6. **MRR ambition is internally inconsistent across v1 docs.** phase2 §7 lands at "$30–50k base case, plan of record $50k"; phase3 §1/§7 moves the plan-of-record to ~$85k on an expansion ladder (3%/mo donut upgrades + 6 enterprise deals). Both flag $100k as a low-probability ceiling, but the **plan-of-record number itself diverges by ~$35k** between the two deliverables. The donut-ladder that carries phase3's ~42% of M12 MRR depends on monetizing the *lowest-demand* feature in the research (donut demand 2/10) — a real tension. (Not a positioning claim, but it affects how aggressively the donut can be leaned on as the revenue engine.) **Flag for reconciliation; positioning treats the donut as retention, not as the primary revenue thesis.**

7. **SEO-winnability (the funnel-fill assumption) is an analyst estimate, not measured.** phase1b §1 scores long-tail SEO 4/10 (best in set) but it is judgment, not rank data. The entire organic, $0-ad GTM (locked §11.4) rests on out-ranking GitGuardian's free ggshield content on the long tail — the largest unvalidated dependency in the plan (phase2 §7 names it the #2 "what has to go right"). (assumption — not measured)