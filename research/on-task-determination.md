# CCGuard — Determining "On-Task" Usage (research, 2026-06-10)

> Requirement from Senthil: not just "work repo vs personal repo," but **"make sure employees are on-task — working on what they're supposed to — no matter what."** Role is admin-assigned (manual). Five parallel research passes: (1) eng-intelligence work-alignment, (2) workforce on-task monitoring, (3) AI-coding on-task signals, (4) UEBA/anomaly/insider-risk, (5) ethics/legal/market reality.

---

## 0. The headline (read first)

1. **"On-task" is a SECOND axis CCGuard doesn't have yet.** Today CCGuard answers *"whose repo?"* (work/personal). On-task asks *"did the work land, and was it the assigned work?"* A session can be on a company repo and wasted (token-burn, no commit); or on an unknown repo and productive. The strong product is **two axes: repo-attribution (have) × output-landing + task-alignment (new)**.

2. **You can build a robust on-task score from metadata ALONE — no prompt reading.** Tiers 1–2 below (commit/PR/merge survival, ticket alignment, session shape) give a decisive signal without ever reading prompt content. Content matching (tier 3) is a consent-gated booster, never default.

3. **The single biggest finding — and the one to internalize:** the literal goal *"on-task no matter what"* is the exact thing that gets monitoring products **rejected, churned, and sued.** The research is one-directional: individual, content-capturing, "gotcha" surveillance destroys trust, *raises* turnover (~50% of companies saw more), and often *lowers* the productivity it claims to protect (gaming, sabotage). And CCGuard's subjects are **developers — the group that rejects individual surveillance hardest of all** (McKinsey-dev-productivity revolt; 54% of devs fear individual eval from activity data). **The winning reframe: "alignment & capacity visibility," delivered transparently, aggregate-first, with an employee self-view and "indicators, not verdicts."** Same buyer pain solved, without the landmines. Drop "no matter what."

---

## 1. The on-task SIGNAL MAP (how you actually tell)

Strength = how decisively it separates on-task/off-task (1–5). Privacy load: **1 = metadata/defensible, 5 = content/radioactive** (lower better).

### Tier 1 — build first (strong + metadata-only)
| Signal | On-task vs off-task | How to compute | Str | Priv |
|---|---|---|---|---|
| **Repo attribution** (HAVE) | company repo vs personal/unknown | git remote host+org vs allowlist (`classify()`) | 4 | 1 |
| **Session→commit linkage** | productive session yields a commit; spinning = big token burn, 0 commits | OTel `tool_result` carries `git_commit_id` on a successful `git commit`; or read from transcript | 5 | 1 |
| **Commit→PR→merge survival** | on-task work reaches a *merged* PR; off-task = dead branch never merged | SCM API: PR state for branches with the AI commits | 5 | 1 |
| **Ticket alignment** | branch/commit references an issue **assigned to this dev, in current sprint** | issue-key regex `[A-Z]{2,10}-\d+` in branch/commit/PR → tracker API (Jira/Linear/GitHub Issues) → check assignee+sprint | 5 | 2 |
| **Code churn / survival** | off-task/low-value = AI lines reverted within ~2wk; on-task = survives | git-blame over time (GitClear's "throwaway code" metric) | 4 | 1 |
| **AI-commit trailers in company repos** | `Co-Authored-By: Claude` commits landing in company repos | `git log` trailers × repo classification | 4 | 1 |
| **Tokens-per-surviving-line / abandoned session** | huge cost, ~0 lines that survive = waste | cost ÷ surviving lines; flag long+costly sessions with 0 commits | 4 | 1 |

### Tier 2 — fast-follow (counters)
Accept rate (`code_edit_tool.decision` accept/reject), lines-accepted volume, refusal/error rate, **company seat used on personal repos** (the existing donut, extended cross-tool via the `tool` field), evasion/bypass detection (direct hits to api.anthropic.com off the sanctioned path; telemetry/trailers disabled on an active machine).

### Tier 3 — consent-gated only (never default-on)
Prompt↔ticket embedding match; topic drift; file-path↔component match. High strength, high privacy load — mirror CCGuard's capture-tier model (`metadata → repo-attribution → content`).

**The score (entirely Tiers 1–2, zero prompt content):**
```
on_task_score(session) =
   w1·repo_is_company  + w2·output_landed(commit/merged PR)
 + w3·output_survived(not reverted in 2wk)
 + w4·ticket_aligned(issue assigned to dev, in sprint)
 + w5·efficient_shape(tokens/surviving-line normal; NOT abandoned-spin)
 − penalty·evasion(bypass/shadow-AI)
```
Degrades gracefully: SCM-only gives repo+commit+merge+churn; add a tracker connector for ticket terms; add OTel/agent for shape terms.

**Two structural cautions (forced by the data):**
- **Outcome > shape.** Faros/GitClear show "lines accepted"/"accept rate" are gameable and weakly tied to value (high accept + high churn = waste that looks productive). Weight *survival* (merge, non-revert) above *acceptance*.
- **Score the work, not the worker.** Surface session/repo-level on-task and *aggregate* team views — never a per-developer "productivity verdict" (the ghost-engineer + Productivity-Score landmine).

First-party metadata sources (free, employer-authorized): **Claude Code Analytics API** (commits/PRs by Claude Code, accept/reject per tool, tokens, cost, per-user — but NO repo/dir, confirming the wedge) · **Claude Code OTel** (`commit.count`, `pull_request.count`, `code_edit_tool.decision`, `active_time`, and `git_commit_id` on `tool_result` = the session→commit join) · **Copilot/Cursor metrics** (acceptances, lines accepted; engaged-vs-active users).

---

## 2. How admins DEFINE "work" (the "looks unrelated but is" case)

Universal pattern across both eng-intel and workforce tools: **role-agnostic default + override hierarchy.**
- **Org default** (everything under `acme-corp` = work) as baseline.
- **Per-repo override + context note** — admin marks any repo work/personal AND records *what it is / which team / what kind of work*. This is the mechanism for "an oddly-named repo that's actually a real project." (New vs current org-only allowlist.)
- **Unknown-repo triage queue** — ambiguous repos surface for admin labeling (already in spec).
- **Developer self-flag** — dev proposes "this IS work, here's why" → admin approves (already in spec).
- **Issue-key join** (the buildable primitive every incumbent uses): regex a tracker key in PR title/body, branch name, commit message → resolve to ticket. "No link" ≈ untracked/unplanned work. LinearB adds *sprint-timing* (added/finished after sprint start = unplanned) to avoid crying wolf on "planned but unlinked."
- **Investment categories** (Feature / KTLO / Bug / Tech-debt) live on the *ticket/epic* and flow to PRs via the link (Swarmia's precedence chain is a ready blueprint).

The one thing **every incumbent refuses to ship** (for cultural, not technical, reasons): the `issue.assignee == commit-author` check — "is this person on THEIR assigned ticket." That's CCGuard's whitespace, but it's also the highest false-positive / most surveillance-coded signal (pairing, handoffs, reviews legitimately break it). Ship it only as *an indicator for a conversation*, never a verdict.

---

## 3. Role-based "off-profile" detection (the marketer-who-codes case)

Best-fit method (UEBA research): **role-based expected-action profiles** + **per-user self-baseline**, composed transparently, delivered as indicators.
- **Role profile** (admin assigns role → expected action set): a non-coder coding, finance in source control = *role-inconsistent category*, flagged even at low volume. Most explainable, lowest false-positive.
- **Self-baseline** (z-score/histogram on a few metrics: active hours, repos touched, volume, off-hours ratio): catches "abnormal for *themselves*." Cold-start trick (Exabeam): replay recent history → score from day one; keep re-baselining so legitimate growth (upskilling) re-normalizes instead of alerting forever.
- **Peer-group** comparison (same-role outlier) — add once roles are well-populated.
- **Compose** via expert anchor-scores + Bayesian frequency down-weighting (Exabeam) — a single *explainable* number, NOT a neural net. Avoid autoencoders/sentiment/keystroke.
- **Wrap in "indicators, not verdicts" + human triage queue.** Every deviation → review queue → dismiss/escalate → feedback retrains. This is what handles the upskilling-vs-misuse ambiguity the user raised: surface "this doesn't fit the role/pattern, please review," never auto-accuse.
- **Pseudonymize by default** (Purview/DTEX): users as `ANON####`, score on metadata, identity reveal gated + audit-logged. Privacy-by-design = legal necessity + sales differentiator (and matches Senthil's pseudonymity posture).

---

## 4. Ethics / legal / market reality (the part that reshapes the product)

**The "creepy line" is crossed at four thresholds — stay on the safe side of ALL four:** (a) individual identification, (b) content capture (screenshots/keystrokes/messages), (c) covert/silent operation, (d) punishment/"gotcha" framing.

- **Microsoft Productivity Score (2020)** is the worked example: *same data* was fine as an org aggregate and toxic as a named-individual score → Microsoft stripped individual names after ~5 weeks of backlash.
- **Trust/turnover/output:** ~50% of companies that deployed device monitoring saw *increased* turnover (VMware); monitored employees report worse mental health (APA: 45% vs 29% negative); 1/3 of managers saw no productivity impact, ~1/4 said it drove job-hunting, 20% saw sabotage (15Five); 72% of employees said monitoring software had negative/no effect.
- **Gaming:** valuing "active time" (mouse/keystrokes) just makes people *look* busy while deep work gets flagged idle. Never ship active-time as a productivity proxy.
- **"Productivity paranoia"** (Microsoft 2022): 87% of employees feel productive, only 12% of leaders are confident — the boss's anxiety is mostly *perceptual*. A tool that resolves the **anxiety** (trustworthy visibility/alignment) beats one hunting a deficit that's mostly not real.
- **Legal (design to GDPR even US-first):** consent is NOT a valid basis (power imbalance) → rely on *legitimate interest* + documented **proportionality** + mandatory **DPIA**; content monitoring (screenshots/keystrokes/email) "typically fails proportionality for general productivity purposes." EU works-council co-determination. US notice laws: **NY** (written notice+ack, eff. 2022), **CT**, **DE**. Architect so content capture is *impossible by default*.
- **Who buys happily** (CCGuard ICP): **call centers / BPO**, **finance/compliance**, **distributed ops** — metric-accepting cultures, legible legitimate-interest purposes. **Who rejects hardest:** **engineering/dev orgs** on an individual-metrics pitch. ← CCGuard's subjects are developers, so this is the live strategic risk.

**Design patterns that WIN (Viva model):** aggregate/de-identified by default with minimum-group thresholds; transparency (explaining *why* raised comfort 10%→30%); employee-facing self-view (strongest single trust lever); alignment/capacity framing not punishment; opt-in for any content depth; visible-not-covert always.

**Hard DON'Ts (each maps to a documented failure):** ❌ individual leaderboards/scores · ❌ screenshots/keylogging/webcam/message content · ❌ covert/silent agent · ❌ automated disciplinary triggers · ❌ selling "catch slackers / on-task no matter what" · ❌ leading with engineering orgs on individual metrics.

**One-line positioning:** *"You don't need to prove they're working — you need to see where the work and capacity actually are, with the team able to see it too. Build that and it sells; build 'catch them no matter what' and it gets rejected, churned, or sued."*

---

## 5. How this folds into CCGuard (recommended)

Keep the artifact/attribution core (already on the defensible side). Add, in order:
1. **Richer work definition** — per-repo overrides + context labels + "kind of work" tag (on top of org allowlist + triage + dev self-flag).
2. **On-task score (metadata-only)** — repo × session→commit × merge-survival × churn; then a **tracker connector** (Jira/Linear/GitHub Issues) for ticket-alignment terms.
3. **Role profiles + self-baseline** — admin-assigned role → expected-action profile + per-user z-score baseline → **indicators into a human review queue**, pseudonymized, never auto-verdict.
4. **Framing/architecture guardrails baked in** — aggregate-first with min-group thresholds, employee self-view, transparency/notice (NY/CT/DE) + DPIA template, no content by default, no individual leaderboards, visible agent only.

Sells as **"AI-coding alignment & capacity assurance"**, not "on-task-no-matter-what surveillance."

*Full per-agent reports (with all vendor mechanisms + ~120 source URLs) summarized above; key sources inline.*
