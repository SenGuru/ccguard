# CCGuard Spec Addendum — Total Visibility (2026-06-10)

> Extends `2026-06-09-ccguard-design.md`. Locks the direction after the total-visibility + on-task research (`research/total-visibility.md`, `research/on-task-determination.md`). Authorized employer monitoring of **company-provided** Claude Code on **company-managed** machines.

## Direction (locked with Senthil)
CCGuard's job expands from "work-vs-personal repo donut" to **complete visibility into how employees use company-provided Claude Code** — a fully replayable, per-employee record of everything done with the tool, with the work/personal split as one lens on top. Same category as Google Vault / M365 Purview eDiscovery / GitHub audit + Copilot metrics / Slack Discovery.

## What we capture (the complete record)
From the on-disk transcripts (`~/.claude/projects/**/*.jsonl`, plaintext, no config needed): every prompt (+ pastes/attachments), every assistant response incl. raw thinking, every tool call (full args), every tool result (full output incl. offloaded `tool-results/*.txt`), file edits with before/after diffs (+ `file-history/` source bytes), subagent transcripts, PRs/commits, full token+cost ledger, identity (`.claude.json oauthAccount`), and exact timestamps. Repo attribution (work/personal) stays the core classifier.

## Four capture planes (run together)
1. **Endpoint agent** (primary) — on-disk harvest = the complete record, works telemetry-off.
2. **Native Anthropic** — Claude Code Analytics API (per-user, indefinite retention); Enterprise **Compliance API** content/eDiscovery; Admin/Usage/Cost API.
3. **Network/cloud** — Bedrock/Vertex native logging (full content in company cloud, dev can't disable) / force-routed gateway+firewall / SASE TLS-inspection (captures all AI tools; CC doesn't cert-pin).
4. **Managed-settings enforcement** — force telemetry + CCGuard hook, `allowManagedHooksOnly`, lock login to corporate tenant, block personal accounts; MDM-deployed service + tamper detection.

## Data model (the spine)
`Employee → Device → Session → Turn → Event → ContentBlob / Artifact / Finding`, plus `Repo` (work/personal), `Hold` (legal hold), `ExportJob`.
- **Event** = typed atom + JSON detail blob (Purview pattern): `kind ∈ {user_prompt, assistant_response, thinking, tool_call, tool_result, file_edit, bash_command, web_fetch, mcp_call, pr, session_start/end}`, `target`, `content_ref → ContentBlob` (verbatim, sha256-deduped, full-text indexed), `detail_json`.
- **Finding** = first-class filterable (secret/PII/source-code/credential + policy_action). (Later.)

## Views (later plans)
Employee profile (KPI strip + session list, work/personal donut) · **Session drill-in timeline** (full prompt/response/diff/command, replay scrubber) = the product · global full-text search + eDiscovery (hold/export). Aggregate-first; employee self-view; CCGuard console actions self-audited.

## On-task layer (folds in)
Two-axis: repo-attribution × output-landing+task-alignment. On-task score (metadata-only): repo × session→commit × merged-PR survival × code-churn × ticket-alignment (Jira/Linear/GitHub-Issues connector) × abandoned-session. Role profiles (admin-assigned role → expected-action) + per-user self-baseline → indicators into a review queue. Per-repo work-definition (override + context note) on top of the org allowlist. (Later plans.)

## Build sequence
- **Plan 5 — Complete-capture pipeline** ← next. Agent full-transcript parser (everything, not just tokens) + identity from `.claude.json` + new server schema (sessions/events/content_blobs) + `POST /v1/capture` ingest + `GET /v1/sessions/:id/timeline` retrieval. Proves "we capture everything," retrievable.
- **Plan 6 — Session-replay UI** (askama+htmx): employee profile + session drill-in timeline + work/personal donut.
- **Plan 7 — Search + eDiscovery + findings** (full-text, secret/PII detection, hold/export).
- **Plan 8 — Managed-settings enforcement + MDM packaging + tamper detection.**
- **Plan 9 — On-task score + tracker connector + role profiles.**
- Later: network/cloud collectors (Bedrock/Vertex/gateway/SASE), allowlist-management API, consent layer, Stripe.

Guardrails carried from the base spec: metadata-tier default with content as a configured tier; visible (non-covert) agent; aggregate-first presentation; the hard "never" lines remain (no emotion inference, no webcam/voiceprint, no keystroke-dynamics biometrics, no personal-account capture).
