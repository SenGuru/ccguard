# CCGuard — Total Visibility into Company-Provided Claude Code

> Authorized employer monitoring of **company-owned** Claude Code seats on **company-managed** machines (the company pays for and provides the tooling). Pure technical-capability research, 2026-06-10. Five parallel passes: complete data inventory · native enterprise surface · network/cloud full-content · endpoint agent · complete-visibility data model/UI. Field paths/env vars verified against a live `~/.claude` install (v2.1.156) + official docs.

This is standard enterprise-admin tooling — the same category as Google Vault, M365 Purview eDiscovery, GitHub audit-log streaming + Copilot metrics, and Slack Discovery. CCGuard operationalizes telemetry Anthropic already emits + policy controls Anthropic already provides.

---

## 0. The bottom line

**From a company-provided Claude Code seat you can reconstruct, byte-for-byte:** every prompt (incl. pastes/attachments), every assistant response *including raw extended-thinking*, every tool call with complete arguments, every bash command with full stdout/stderr, every file read/created/edited with full content and reversible before/after diffs, every web fetch/search result, every MCP call, every subagent's full private transcript, all todos, the complete economics ledger (tokens + USD per request/project/day/model), every permission decision, complete identity (email, account/org UUIDs, role), machine/terminal/git fingerprint, PRs/commits produced, and exact timestamps for a turn-by-turn replay — **plus any secrets** that passed through prompts, output, or config. The only thing not recoverable is content the user never sent to the tool.

There are **four independent capture planes; you can run all four at once:**
1. **Endpoint agent** — on-disk harvest. The complete record, works even with all telemetry off. (CCGuard's path.)
2. **Native enterprise (Anthropic)** — Analytics API + Admin/Usage/Cost API + (Enterprise) **Compliance API content/eDiscovery** + managed-settings enforcement.
3. **Network/cloud** — route through company cloud/gateway/SASE to centralize 100% of content, tamper-proof.
4. **Managed-settings enforcement** — force telemetry + lock hooks + lock login to corporate tenant; the teeth that make it un-bypassable.

---

## 1. The complete reconstructable record (what's knowable, where it lives)

**Source of truth = the on-disk session transcript JSONL**, `~/.claude/projects/<encoded-cwd>/<sessionId>.jsonl` — append-only, plaintext, NO config required. Every line carries join keys: `sessionId`, `uuid`, `parentUuid` (turn DAG), `timestamp`, `cwd`, `gitBranch`, `version`, `userType`, `isSidechain`, `requestId`.

| What | Where (on disk) | Where (OTel, if forced) |
|---|---|---|
| **Verbatim prompt** + pastes + attachments | `user` line `message.content[].text`; `history.jsonl` (flat all-prompts log); `paste-cache/<hash>.txt` | `user_prompt` event, `prompt` field **iff `OTEL_LOG_USER_PROMPTS=1`** |
| **Full assistant response + raw thinking** | `assistant` line `message.content[]` `text`/`thinking` blocks; `message.model`, `stop_reason` | `llm_request` span; full body iff `OTEL_LOG_RAW_API_BODIES` |
| **Every tool call (full args)** | `assistant` `{tool_use, name, input}` (Bash command, Edit file_path/old/new, etc.) | `tool_result` event; args iff `OTEL_LOG_TOOL_DETAILS=1` |
| **Every tool result (full output)** | next `user` line `toolUseResult`: Bash `{stdout,stderr}`, Read `{file content}`, Edit `{originalFile,structuredPatch}`, Grep/WebFetch results; big outputs in `<session>/tool-results/<toolu_id>.txt` | bodies iff `OTEL_LOG_TOOL_CONTENT=1` |
| **File edits + before/after diffs** | in-transcript `structuredPatch`; `file-history/<sessionId>/<hash>@vN` = **pristine pre-edit source bytes** (full version chain, survives revert/no-commit) | `lines_of_code.count`, `code_edit_tool.decision` |
| **Subagent (Task) full transcripts** | `<session>/subagents/agent-<id>.jsonl` + `.meta.json` (type+description) | nested spans |
| **PRs/commits produced** | `pr-link` lines (`prNumber/prUrl/prRepository`); git-on-disk | `commit.count`, `pull_request.count`, `git_commit_id` on tool_result |
| **Identity + spend ledger** | `~/.claude.json` `oauthAccount` (email, accountUuid, orgUuid, role, seatTier) + per-project `lastCost`/lines/tokens + `githubRepoPaths` + `skillUsage` | every event: `user.email`, `user.account_id`, `organization.id`, `session.id` |
| **Per-request economics** | `assistant` `message.usage` (input/output/cache tokens, service_tier); `stats-cache.json` lifetime rollups | `token.usage`, `cost.usage`, `active_time.total` |
| **Permission/policy + config** | `permission-mode` lines; `settings.json`/`settings.local.json` allow/deny/ask | `tool_decision` (accept/reject + source), `permission_mode_changed`, `auth` |
| **Todos / intent / session title** | `todos/<sessionId>`; `ai-title` lines (Claude's own session summary) | — |
| **Secrets** | transcripts/history/paste-cache/tool-results capture any secret pasted or printed; `.credentials.json` (OAuth tokens); `settings.json` MCP keys in cleartext | raw API bodies |

**Minimum source set for ~100% (pure unprivileged file reads, no hooks/OTel):** `projects/**/*.jsonl` + `**/subagents/` + deref `**/tool-results/` ; `file-history/` ; `history.jsonl`+`paste-cache/` ; `.claude.json`+`settings*.json`. Add `git -C <cwd>` for commit ground-truth and OS process/idle for true session bounds.

---

## 2. Native enterprise surface (the official, buy-the-plan path)

Five independent native surfaces, all runnable at once:

- **Claude Code Analytics** — dashboard (`claude.ai/analytics/claude-code`) + **Admin API** `GET /v1/organizations/usage_report/claude_code` (admin key, free, **per-user**: sessions, lines, commits, PRs, tool accept/reject, per-model tokens+cost; **retention indefinite**; "Export all users" CSV; leaderboard).
- **Admin Usage & Cost API** — `/v1/organizations/usage_report/messages` + `/cost_report`, group_by api_key/workspace/model.
- **Compliance API (Enterprise)** — the deepest: `GET /v1/compliance/activities` (6-year, SIEM-streamable activity feed with actor email/IP/user-agent) + content endpoints (`/compliance/apps/chats/{id}/messages` = **full message text + uploaded/generated files + artifacts**, retrievable per-user, incl. soft-deleted) + **hard-delete** for DLP. eDiscovery-grade. (Console orgs = activity feed only; full content = Enterprise; **ZDR neutralizes content capture**.)
- **Audit log export** (Enterprise, 180-day CSV) — IDs/events, no content.
- **OpenTelemetry** — real-time per-user stream; **content toggles** `OTEL_LOG_USER_PROMPTS / _TOOL_DETAILS / _TOOL_CONTENT / _RAW_API_BODIES` (last = entire conversation incl. system prompt).
- **Identity control** — SSO + **Domain Capture "restrict org creation"** (blocks personal accounts on your domains) + SCIM deprovisioning.

**Plan tiers:** Team gets analytics + managed policy + OTel + spend caps; **Enterprise adds SCIM + audit export + Compliance API content/eDiscovery**. (Pro/Max individual = no org admin surface.)

**Precedents (same category):** Google Vault eDiscovery + Admin Reports API · M365 Purview Audit/eDiscovery + Defender CASB · GitHub audit-log streaming + Copilot Metrics API (team-level) · Slack Discovery API (incl. deleted messages) · Okta/Entra sign-in logs.

---

## 3. Managed-settings enforcement (the teeth — un-bypassable)

`managed-settings.json` is the **top of the precedence chain** (managed > CLI > local > project > user), admin-only, user cannot edit. Paths: Windows `C:\ProgramData\ClaudeCode\managed-settings.json` (also registry `HKLM\SOFTWARE\Policies\ClaudeCode`), macOS `/Library/Application Support/ClaudeCode/managed-settings.json` (+ plist `com.anthropic.claudecode`), Linux `/etc/claude-code/managed-settings.json`. Push via MDM (Jamf/Intune/Kandji).

Force + lock: `CLAUDE_CODE_ENABLE_TELEMETRY=1` + pinned OTLP endpoint; a SessionStart/PreToolUse/Stop **hook** that POSTs each event to CCGuard ingest + **`allowManagedHooksOnly: true`** (user can't add/disable hooks); `forceLoginMethod`+`forceLoginOrgUUID` (lock to corporate tenant); `permissions` + `disableBypassPermissionsMode`; `allowedMcpServers`/`allowManagedMcpServersOnly`. Caveat: a user *shell* env var can shadow the managed `env` block in some versions — so don't rely on env alone for tamper-proofing; pair with the network controls below.

---

## 4. Network/cloud full-content capture (centralized, tamper-proof)

Foundational facts: **Claude Code does NOT cert-pin** (trusts bundled+OS CA store by default) → SASE MITM works zero-config. And `managed-settings` env can force routing.

Ranked by completeness × tamper-resistance:
1. **Bedrock/Vertex native logging + egress firewall** — `CLAUDE_CODE_USE_BEDROCK/VERTEX` (managed settings) routes to YOUR cloud; **Bedrock model-invocation logging** captures `inputBodyJson`+`outputBodyJson`+`identity.arn` (per-user) to S3/CloudWatch; **Vertex request-response logging** to BigQuery at `samplingRate=1`. Admin-enabled, dev IAM can't disable, data stays in company cloud, captures tool traffic (it's in the message bodies). Block direct `api.anthropic.com` at the firewall so it can't be bypassed.
2. **Force-routed self-hosted gateway** (LiteLLM/Kong) `ANTHROPIC_BASE_URL` + per-user JWT via `apiKeyHelper` + **egress firewall** + **mTLS** (`CLAUDE_CODE_CLIENT_CERT`) — full bodies, best per-user/per-subagent attribution (Claude Code sends `X-Claude-Code-Session-Id`/`Agent-Id` headers).
3. **SASE / TLS-inspection** (Zscaler/Netskope/Palo Alto/CrowdStrike) — corporate root CA in OS store decrypts all traffic; **captures ALL AI tools on the box** (Cursor, ChatGPT, browser LLMs), not just Claude Code; + DLP.
4. **mTLS / custom CA** — hardener that locks the gateway to corporate clients.
5. **DNS/egress logging** — the deny-rule (block api.anthropic.com/claude.ai, allow only gateway/provider) is what makes 1/2 tamper-proof; also bypass detection.

Recommended stack: Bedrock/Vertex native logging (or self-hosted gateway) **+ egress firewall + SASE backstop + mTLS**.

---

## 5. The "see EVERYTHING an employee did" data model + UI

Every incumbent (Teramind, Veriato, DTEX, ActivTrak, Purview, and the AI gateways WitnessAI/Prompt Security/Portal26/Harmonic/Lasso) converges on the same shape; CCGuard's atoms are *richer* because it owns the native transcript.

**Entities:** `Employee → Device → Session → Turn → Event → ContentBlob / Artifact / Finding`, plus `Repo` (work/personal classification), `Hold` (legal hold), `ExportJob`.
- **Event** = typed atom + JSON detail blob (the Purview `AuditData` pattern): `event_type ∈ {user_prompt, assistant_response, tool_call, file_edit, bash_command, web_fetch, mcp_call, session_start/end}`, `target`, `content_ref` → ContentBlob (full verbatim text, sha256-deduped, **full-text indexed**), `detail_json`.
- **Finding** = first-class filterable: secret/PII/source-code/credential hit + policy_action (logged/flagged/would-block).

**Views:**
- **A. Employee profile** — KPI strip (sessions, work-vs-personal cost donut, hours, findings) + session-list table (date, repo+work/personal badge, duration, tokens/cost, finding-count, hold flag).
- **B. Session drill-in (the core "see everything")** — vertical turn-by-turn timeline; each turn expands to full prompt + full response + nested tool calls/file diffs/bash commands rendered inline; finding chips on offending events; search-within; a **replay scrubber** (the AI-transcript analog of Veriato/Teramind session video). Higher-fidelity than screenshot replay.
- **C. Global search / eDiscovery** — full-text across all prompts/responses/diffs/commands; filters (employee, date, repo, classification, event_type, finding_type, keyword/regex); legal hold + review set + export.
- **D. Reporting/export** — per-employee PDF; manager-scoped view; CSV/JSONL raw export; export jobs themselves audit-logged.

**Build priority:** (1) lock the Employee→Session→Turn→Event→ContentBlob/Finding schema (typed-event + JSON-detail-blob); (2) ship the session drill-in timeline = the product; (3) add full-text search + finding filters, then legal hold + export.

AI-gateway pricing precedent (positioning): Prompt Security ≈ $120/employee/yr, $300/developer/yr.

---

## 6. What CCGuard needs to build (gap from v1 agent)

The v1 agent (`crates/ccguard-agent/`) only extracts billing `usage` from `assistant` lines and **skips subagents + offloaded results + file-history + identity**. To reach the complete record (all additive to the existing incremental byte-offset tailer in `state.rs`):
1. **Full transcript parse** — all line types (prompt/thinking/tool_use/tool_result/pr-link/ai-title), not just usage.
2. **Follow `subagents/` + deref `tool-results/*.txt`** (currently skipped).
3. **Read `file-history/`** for actual pre/post source bytes.
4. **Read `.claude.json`** for identity (`oauthAccount.emailAddress`/orgUuid) + per-project cost ledger + repo paths.
5. **Extend `CcEvent.Activity`** to carry `prompt`/`tool_call`/`edit`/`pr`/`subagent` types with `content_ref` → harvested blob (the `content_ref`/`source_layer` fields already exist).
6. **Server**: the Employee→Session→Turn→Event→ContentBlob/Finding schema + session-timeline view + search/eDiscovery.
7. **Enforcement**: managed-settings generator (force telemetry + CCGuard hook + `allowManagedHooksOnly` + lock login) and the OTel collector as a redundant real-time channel; MDM-deployed service (SYSTEM/root) + watchdog + tamper-detection (process-without-record, heartbeat gap, config drift).

Disk harvest is the floor (complete, needs no cooperation); managed hooks + forced telemetry + network logging are the tamper-proof ceiling. Use all.

*Sources inline; key docs: code.claude.com/docs (monitoring-usage, settings, network-config, llm-gateway, amazon-bedrock, analytics) · platform.claude.com/docs (claude-code-analytics-api, usage-cost-api, compliance-api) · AWS Bedrock invocation logging · Vertex request-response logging · Purview audit/eDiscovery · GitHub audit + Copilot metrics · vendor docs for Teramind/Veriato/DTEX/ActivTrak/WitnessAI/Prompt Security/Portal26.*
