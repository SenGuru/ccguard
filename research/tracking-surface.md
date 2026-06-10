# CCGuard — The Complete Tracking Surface

> Research compiled 2026-06-09. Six parallel research passes (Claude Code native telemetry + 5 layer-specific agents). Purpose: map **every** technical vector an employer could use to monitor company-issued Claude Code (and AI coding tools generally), then tier them by richness × tamper-resistance × legal exposure so we can decide CCGuard's capture architecture.
>
> Context: authorized monitoring of **company-issued** tooling on **company-managed** devices. Legitimate, but only sellable if notice/consent/config are built as product features (see §6).

---

## 0. The headline findings (read this first)

1. **The single richest source is free and already on disk.** Claude Code writes a complete, append-only JSONL transcript of every session to `~/.claude/projects/<encoded-cwd>/<sessionId>.jsonl` — every prompt, assistant reply, tool call, shell stdout/stderr, file edit, model, token count, cost, git branch, and cwd. **No flags, no env vars, nothing enabled.** The encoded folder name *is* the working directory, so it gives repo attribution for free. Verified live: 876 MB across 149 sessions in a single project folder. This is strictly a *superset* of what OTel-with-prompt-logging would yield.

2. **The moat is the classifier, not the capture.** Anthropic's own console already exposes per-user token/cost. What nobody has is **repo-attribution** — matching the session's git remote (host + org) against a company allowlist to answer "company work vs personal project." That's the one signal the whole wedge rests on, and it's privacy-defensible: *we watch the repo, not the person.*

3. **The wedge is OPEN — but Anthropic is the platform-owner risk.** No product does {Claude Code repo-attribution + work-vs-personal + cost governance + compliance workflow}. Anthropic's Analytics API is "a single sprint" from exposing working-directory and closing it (6–18 mo window). Mitigation: be **cross-tool** (Cursor + Copilot + Claude) and own the **compliance workflow** Anthropic will never build.

4. **Maximal capture is the #1 sales-killer; transparency is the #1 differentiator.** The legal research and the competitive research point the same way: the winning product leads with metadata + repo-attribution + **developer-friendly transparency**, and offers deep content capture as a *gated, consented* tier. Teramind-style covert screen-surveillance is exactly the thing enterprise/GDPR/eng-culture buyers reject.

---

## 1. Master map — tracking layers ranked

| Tier | Layer | Richness | Tamper-resist | Legal load | Role in CCGuard |
|---|---|---|---|---|---|
| **1** | **Endpoint agent → on-disk Claude artifact harvest** | ★★★★★ | High (if MDM-deployed) | High (content) | **Primary content + attribution spine** |
| **2** | **Repo-allowlist classifier** (git remote host+org vs company orgs) | ★★★☆☆ (decisive) | Med→High (verify server-side) | **Lowest / favorable** | **The wedge / differentiator** |
| **3** | **Official OTel telemetry** (metrics+events, managed-settings-enforced) | ★★★☆☆ | High (central push) | Low-Med | Clean second channel + agentless mode |
| **4** | **Network / cloud content capture** (Bedrock/Vertex logging, gateway, SASE) | ★★★★★ | High (infra-level) | High→clean (own cloud) | Cross-tool deep content (enterprise) |
| **5** | **Git / SCM server-side** (trailers, audit-log streaming, push hooks, secret-scan) | ★★★★☆ | **Highest** (server-side) | Low | Tamper-proof "what landed in company repos" |
| **6** | **OS/endpoint extras** (process, active-window, screenshots, clipboard, keystroke) | ★★–★★★★★ | High (kernel) | Low→**Highest** | Tamper tripwires; heavy vectors gated |
| **7** | **Coarse network** (DNS / egress / NetFlow) | ★☆☆☆☆ | Very high | Low | Bypass-detection backstop |

---

## 2. Layer 1 — Claude Code on-disk artifacts (the spine)

Root: `~/.claude/` and `~/.claude.json` (identical layout on Win/macOS/Linux; on macOS the OAuth token is in Keychain `Claude Code-credentials`).

### 2.1 Session transcripts — the crown jewel
`~/.claude/projects/<encoded-cwd>/<sessionId>.jsonl` — append-only event log. **Encoded folder = absolute cwd** (separators + drive colon → `-`), e.g. `C--Users-gsent-Desktop-altkey-homepage` → `C:\Users\gsent\Desktop\altkey-homepage`. Folder name alone attributes every session to a repo/dir.

Each line is one JSON object. `user` lines carry verbatim `message.content` (full prompt), `cwd`, `gitBranch`, `version`, `sessionId`, `timestamp`. `assistant` lines carry `model`, full content incl. `tool_use` blocks with **exact tool + full input args**, `stop_reason`, and a complete `usage` block (input/output/cache tokens, service_tier). Tool results (stdout/stderr, file contents) land on the matching `user` line under `toolUseResult`.
- Subagent transcripts: `…/<sessionId>/subagents/agent-<id>.jsonl` (+ `.meta.json`) — separate full transcripts per Task spawn.
- Large tool outputs offloaded to `…/<sessionId>/tool-results/toolu_<id>.txt`.

**Net: tailing these by byte offset = near-real-time, lossless capture of the entire conversation with zero config.**

### 2.2 Supporting artifacts (all verified)
| Artifact | Path | Value |
|---|---|---|
| Identity + spend ledger | `~/.claude.json` | `oauthAccount` (email, accountUuid, organizationUuid/Name, role), every project path ever opened, per-project `lastCost`/tokens/`lastSessionId`/lines, `history[]`, `githubRepoPaths`, `userID` |
| OAuth tokens (plaintext on Win/Linux) | `~/.claude/.credentials.json` | access/refresh tokens, subscriptionType, per-MCP creds |
| Settings + MCP secrets + permissions | `~/.claude/settings.json`, `settings.local.json` | enabled plugins, **MCP API keys in plaintext**, allow/deny/ask command lists (tooling habits) |
| Global prompt history | `~/.claude/history.jsonl` | every prompt typed, keyed by project |
| Pre-edit file copies | `~/.claude/file-history/<sessionId>/` | exfiltrable source even if repo later cleaned |
| Shell snapshots | `~/.claude/shell-snapshots/*.sh` | env, aliases, install path |
| Todos | `~/.claude/todos/*.json` | task intent |
| Usage cache | `~/.claude/stats-cache.json` | per-day/per-hour activity + spend profile (no transcript parse needed) |
| Paste cache | `~/.claude/paste-cache/*.txt` | verbatim large pastes |
| Telemetry spool | `~/.claude/telemetry/1p_failed_events.*.json` | buffered first-party events |

### 2.3 Harvest recipe
Watch `~/.claude/projects/**/*.jsonl` (+ subagents/, tool-results/), `history.jsonl`, `.claude.json`, `paste-cache/`, `file-history/` via `ReadDirectoryChangesW` (Win) / `FSEvents` (mac) / `inotify` (Linux). Tail JSONL by byte offset. Attribute: repo from folder + per-line `cwd`/`gitBranch`; identity from `oauthAccount.emailAddress`; spend from `usage` + `stats-cache.json`.

---

## 3. Layer 3 — Official OTel telemetry (the clean channel)

Enable: `CLAUDE_CODE_ENABLE_TELEMETRY=1`, `OTEL_METRICS_EXPORTER=otlp`, `OTEL_LOGS_EXPORTER=otlp`, `OTEL_EXPORTER_OTLP_ENDPOINT=…`, `OTEL_EXPORTER_OTLP_HEADERS="Authorization=…"`. Traces (beta) via `CLAUDE_CODE_ENHANCED_TELEMETRY_BETA=1`.

**Metrics:** `claude_code.session.count`, `claude_code.token.usage` (by `token.type`/`model`), `claude_code.cost.usage`, `claude_code.lines_of_code.count`, `claude_code.commit.count`, `claude_code.pull_request.count`, `claude_code.code_edit_tool.decision` (accept/reject), `claude_code.active_time.total`.
**Events:** `claude_code.user_prompt` (length only by default), `claude_code.api_request`, `claude_code.tool_result`, `claude_code.tool_decision`, `claude_code.api_request_body`/`response_body` (opt-in), etc.
**Attributes always attached:** `user.id`, `user.email`, `organization.id`, `session.id`, `terminal.type`, model. **Inject `OTEL_RESOURCE_ATTRIBUTES="tenant.id=…,enduser.id=…"`** for tenant/user tagging.
**Content flags (opt-in):** `OTEL_LOG_USER_PROMPTS=1` (prompt text), `OTEL_LOG_TOOL_DETAILS=1` (tool inputs/paths/commands), `OTEL_LOG_TOOL_CONTENT=1` (full tool I/O), `OTEL_LOG_RAW_API_BODIES=1` (full API payloads).

**Critical gap:** OTel does **NOT** expose working directory / repo name / file paths / git branch in standard attributes. Repo attribution must come from **hooks** or **transcript harvest**.

**Enforcement:** managed-settings can force telemetry + endpoint and lock hooks (`allowManagedHooksOnly: true`). Server-managed (Teams/Enterprise) is strongest; OS-level/file-based for Console/Pro. Managed-settings paths: macOS/Linux `/Library/Application Support/ClaudeCode/managed-settings.json` · `/etc/claude-code/managed-settings.json`; Windows `C:\ProgramData\ClaudeCode\managed-settings.json` or `HKLM\SOFTWARE\Policies\ClaudeCode`.

### 3.1 Hooks (repo attribution without an agent)
Events: `SessionStart`, `UserPromptSubmit`, `PreToolUse` (can block), `PostToolUse`, `Stop`, `SessionEnd`, `ConfigChange`. Hooks run in `${CLAUDE_PROJECT_DIR}` and can `git rev-parse --show-toplevel` / `git config --get remote.origin.url` → **reliable repo identity per session**, plus `tool_name`/`tool_input`. Deployable + lockable via managed-settings. This is the agentless path to the Layer-2 signal.

---

## 4. Layer 4 — Network / cloud content capture (cross-tool, deep)

**Enabling fact:** Claude Code does **not** certificate-pin — trusts bundled + OS CA store (`CLAUDE_CODE_CERT_STORE=bundled,system`), supports `NODE_EXTRA_CA_CERTS`, `HTTPS_PROXY`, `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, mTLS. So full prompt+completion content is interceptable with no tool cooperation.

| Vector | Captures | Mechanism | Tamper-resist | Legal |
|---|---|---|---|---|
| **Bedrock model-invocation logging** | **Full prompt+completion** + tokens + `identity.arn` per principal | `CLAUDE_CODE_USE_BEDROCK=1`; admin `PutModelInvocationLoggingConfiguration` → S3/CloudWatch | **Very high** (account-level, dev can't disable) | **Cleanest** — own AWS account |
| **Vertex request-response logging** | Full prompts+completions → BigQuery | `CLAUDE_CODE_USE_VERTEX=1`; REST `loggingConfig` | Very high (project admin) | Clean — own GCP |
| **Microsoft Foundry** | Tenant-side request/response logging | route to Foundry endpoint | High (tenant admin) | Clean — own Azure |
| **Self-hosted gateway** (LiteLLM / Portkey / Cloudflare AI Gateway / Kong+log / Helicone) | Full request+response bodies, tokens, per-user via key/JWT | `ANTHROPIC_BASE_URL=https://gateway…` + auth | Med (env-bypassable → pair with egress firewall) | High (3rd-party processor) |
| **SASE TLS-inspection** (Zscaler / Netskope / Palo Alto / CrowdStrike) | **Everything to api.anthropic.com, all AI tools** + DLP | corporate root CA in OS store or `NODE_EXTRA_CA_CERTS`; transparent SASE agent | **Strong** (device-managed) | **Highest** — decrypts all HTTPS; must domain-scope + consent |
| **altkey transparent mode** (user's own product) | Full bodies at relay (transparent) / metadata only (tunnel relay is E2E by design) | local CA + intercept; tools point at altkey | Med (strong if company-deployed) | High in transparent mode |

**Best legal × richness:** Bedrock/Vertex native logging — full content, dev-undisable, data stays in the customer's own cloud. **Force-routed self-hosted gateway** is the pragmatic build for standard-API customers (pair with an egress firewall blocking direct `api.anthropic.com` so it can't be bypassed). Note: env-var routing alone is employee-bypassable.

---

## 5. Layer 5 — Git / SCM server-side (tamper-proof, privacy-defensible)

- **AI commit trailers:** Claude Code appends `Co-Authored-By: Claude <noreply@anthropic.com>` + `🤖 Generated with Claude Code` by default. Detect: `git log --format='%(trailers:key=Co-authored-by)'` / GitHub Search API. **Trivially disabled client-side** (`attribution`/`includeCoAuthoredBy` in settings.json) → **must enforce via managed settings** to be trustworthy. Also detect Copilot/Cursor/Aider markers for cross-tool.
- **GitHub audit log git events** (`git.clone/fetch/push`): `GET /orgs/{org}/audit-log?include=git` — REST/stream only, **7-day retention** → **stream to SIEM** (Splunk/Datadog/S3) for durability + out of employee reach.
- **Copilot Metrics API** = the *precedent* that per-user platform-side AI-usage measurement is legit & shipping; Claude Code has no equivalent first-party org API → CCGuard's wedge.
- **Server-side push hooks** (Enterprise Server / GitLab DC / Bitbucket DC `pre/post-receive`): capture author + repo + files + message + AI-trailer on **100% of pushes**, **highest tamper-resistance**. (Not available on GitHub.com SaaS — use audit-log streaming there.)
- **Branch protection / require-signed-commits**: check whether signing is enforced as a *confidence modifier* on author attribution (unsigned = spoofable).
- **Secret-scanning / push-protection join**: AI-trailered commit ∩ secret-push-attempt = compelling risk story.
- **GitLab**: `/api/v4/audit_events` (+ `/events` push actions) + GraphQL audit streaming. **Bitbucket Cloud** is the weak link (admin-only audit, no push diffs) → lean on trailers/hooks.

### 5.1 Repo-allowlist classifier (Layer 2 — the wedge)
1. Session repo identity: `git remote get-url origin` + `git rev-parse --show-toplevel`.
2. Normalize URL → host + org/owner (handle `git@host:org/repo.git` and `https://host/org/repo`).
3. Match vs company allowlist, **auto-built** from `GET /orgs/{org}/repos` (GitLab `/groups/{id}/projects`, Bitbucket `/repositories/{ws}`). Match on **org/owner** so new repos auto-classify.
- Reliability **high** on host+org boundary; weak edges = personal forks of company repos, SSH/HTTPS form, monorepos → treat unknown hosts as personal/unknown, verify server-side via push event/hook so it can't be faked at push time.
- **This is the only vector that draws the work-vs-personal line, and the strongest "we watch the repo, not the person" talking point.**

---

## 6. Legal / compliance boundary (what's actually sellable)

Monitoring company tooling is legal **only** with notice/consent/config as features. Covert + content-without-notice = the top dealbreaker.

### Compliance tiers
- **(a) Safe-by-default (ship on):** token/usage/cost counts, model/feature, session timestamps, **repo/file names within company orgs**, **aggregate** team/org metrics, anonymized/pseudonymized reporting. Still needs: one-time install-time notice + acknowledgment, retention window, company-resource scoping.
- **(b) Require notice/consent config (build gating, off until enabled):** full prompt+response content, code/diff content, raw keystroke content, screenshots, **individual named reporting/drill-down**, any capture touching all-party-consent-state users, EU/UK deployment (DPIA+LIA+residency), Germany (works-council gate).
- **(c) High-risk — avoid or hard-gate:** covert/stealth (default), webcam/voiceprint, **keystroke dynamics/behavioral biometrics**, **emotion/sentiment inference (EU AI Act Art.5 — PROHIBITED, €35M/7%)**, capture of personal accounts/repos, cross-border transfer of EU data without SCC/DPF + residency.

### Reusable compliance primitives (these are *sales accelerators*, not just compliance)
Notice-template + e-acknowledgment workflow · per-jurisdiction/all-party consent mode · content-capture opt-in with redaction + aggregate/pseudonymized default · data-governance suite (residency + configurable retention/auto-purge + DSAR) · **DPIA + LIA + works-council templates** CCGuard generates.

### Hard "never" lines
Emotion/affect inference from biometrics (EU AI Act) · covert-by-default · webcam/voiceprint · keystroke-dynamics biometric ID · harvesting employee credentials · capturing personal accounts/repos.

### Key legal facts
ECPA business-purpose + consent exception (US federal one-party); **12 all-party states** (CA/CT/DE/FL/IL/MD/MA/MI/MT/NH/OR/PA/WA) — design to CA (CIPA + CCPA employee data + sensitive-PI) and you cover most US. SCA bars personal-account access even on company device. NLRA §7 (GC memo 23-02) limits pervasive surveillance that chills organizing. NY §52-c / CT §31-48d / DE §19-705 = written-notice laws. GDPR: lawful basis = legitimate interest (consent invalid in employment), Art.35 DPIA mandatory, data-minimization. Germany §87 BetrVG works-council co-determination. Microsoft Productivity Score (2020) — forced to strip per-user names → **aggregate is acceptable, individual surveillance is radioactive.**

---

## 7. Competitive landscape & wedge

**Verdict: the wedge is OPEN.** No product does all of: (1) detect which git repo a Claude Code session is in, (2) classify employer-owned vs personal, (3) govern cost by classification, (4) compliance-ready HR/legal/finance workflow, (5) across Claude Code.

| Competitor | What it has | The gap CCGuard exploits |
|---|---|---|
| **Anthropic Console + Analytics API** (closest threat) | per-user token/cost, PR/commit metrics, OTel export, 30-day retention | **no repo context**, no work-vs-personal, no compliance workflow, no cross-tool |
| **Jellyfish AI Impact** | best multi-tool AI analytics, Claude Code dashboard, PR linkage | EM/ROI buyer; no compliance/attribution/enforcement |
| **GitClear** | line-level AI model attribution | backward-looking (post-commit); no live session/compliance |
| **Teramind** | screen-capture AI governance incl. Claude Code, compliance audit trails | surveillance-adversarial, no structured repo data, no cost governance |
| **Portal26** | free Claude-specific governance (June 2026) | shadow-AI discovery/security, not repo-attribution compliance |
| **WitnessAI / Prompt Security / Harmonic / Lasso** | network/MCP AI security + DLP incl. Claude Code | security/DLP framing; no repo/work-attribution/cost |
| **CloudZero / Finout** | per-dev Claude cost (Anthropic Usage API) | finance buyer, billing-level, no repo context/compliance |
| **Copilot / Cursor dashboards** | first-party per-user usage | single-tool silo; no repo attribution |

**Platform-owner risk (real):** Anthropic could expose working-dir in telemetry → close the wedge (6–18 mo). **Mitigation = cross-tool coverage + own the compliance workflow Anthropic won't build.**

### Three defensible angles
1. **Repo-attribution as compliance *infrastructure*, not analytics** — the workflow moat (policy acknowledgment, manager approval, HR/legal evidence export). No analytics tool builds workflows; no compliance tool is session-aware.
2. **Cross-tool token attribution at the work-vs-personal layer** — one view of "company AI spend on personal projects" across Claude Code + Cursor + Copilot, tagged by repo classification. Doesn't exist.
3. **Developer-friendly compliance, not surveillance** — transparency + self-view + flag-misclassification. Unlocks GDPR/eng-culture/remote-first buyers Teramind structurally can't serve.

---

## 8. Recommended capture architecture (for the design phase)

- **Spine:** endpoint agent harvesting Layer-1 on-disk artifacts (richest + attribution + content for free), **MDM-deployable + tamper-resistant**, with **agentless mode** (Layer-3 OTel + hooks via managed-settings) for customers who won't install an agent.
- **Classifier:** Layer-2 repo-allowlist (auto-built from SCM org APIs), verified server-side via Layer-5 push events/hooks.
- **Deep/enterprise tier:** Layer-4 network/cloud content capture (Bedrock/Vertex/gateway/SASE) for cross-tool, full-content customers.
- **Tamper backstops:** Layer-6 process monitoring + Layer-7 DNS/egress (direct hit to api.anthropic.com = bypass signal).
- **Posture:** safe-by-default = metadata + repo-attribution + aggregate; content/individual/screenshot vectors **gated behind the compliance primitives**; the "never" lines hard-excluded. Lead the product with **transparency** (the differentiator), offer maximal capture as consented tiers.

---

*Sources are inline in the per-agent reports; key docs: code.claude.com/docs (monitoring-usage, hooks, server-managed-settings, admin-setup, network-config, amazon-bedrock, settings, analytics), GitHub/GitLab/Bitbucket audit & hooks docs, EU AI Act Art.5, ICO worker-monitoring guidance, vendor pages cited per row.*
