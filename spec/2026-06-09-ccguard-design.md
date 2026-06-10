# CCGuard — Design Spec

**Status:** Draft for review · **Date:** 2026-06-09 · **Owner:** Senthil
**Companion research:** [`../research/tracking-surface.md`](../research/tracking-surface.md)

---

## 1. Overview

**CCGuard is a multi-tenant B2B SaaS that gives employers visibility and governance over how their employees use company-issued Claude Code (and AI coding tools generally).**

The lead wedge: **work-vs-personal attribution.** CCGuard captures AI-coding activity, attributes each session to a git repository, classifies that repo as *company work* / *personal* / *unknown* against a company allowlist, and reports spend, usage, and policy violations by that classification. The product sentence:

> *"Dev A spent $312 of company Claude Code tokens this month — $48 of it (15%) on repos outside your org."*

This is the one signal no competitor has (Anthropic's own console shows per-user cost but **no repo context**; analytics tools like Jellyfish show usage but no compliance/attribution; Teramind captures screens but is developer-toxic and has no structured repo data). See research §7.

Part of the **altkey + CCGuard + Grove** "AI-usage governance" suite (altkey = control AI-sub access; CCGuard = govern employee AI use; Grove = govern your own focus).

**Targets:** $4k MRR Dec 2026 → $15k MRR May 2027 (seat-based B2B).

### 1.1 Scope decisions (locked with user)
- **Market SaaS from day one** — multi-tenant, signup, billing, org isolation up front.
- **Lead wedge = on-task vs personal** (repo attribution), not raw cost/productivity.
- **Capture all 7 tracking layers, at full depth** — built as pluggable collectors on a shared core (this spec covers all 7).
- **Transparent, tiered capture** — metadata + repo-attribution + aggregate are safe-by-default; content/individual/OS-heavy vectors are built but consent-gated. The product *leads* with transparency (the differentiator) and offers maximal capture as consented tiers.

---

## 2. Goals & non-goals

### Goals
1. Capture AI-coding activity across all 7 layers (endpoint, SCM, OTel, network/cloud, git-server, OS, DNS/egress).
2. Attribute every session/event to a repo and classify work/personal/unknown.
3. Report spend + usage + policy violations, aggregate-first, with role-gated individual drill-down.
4. Be **cross-tool** (Claude Code anchor; Cursor/Copilot/Codex as the same event shape) — the hedge against Anthropic closing the wedge.
5. Ship consent/compliance + data-governance as first-class features (they are the enterprise sales unlock).
6. Self-serve onboarding: signup → connect SCM → deploy agent → invite seats → see the donut.

### Non-goals / hard "never" lines (enforced in product)
- ❌ Emotion / sentiment / "engagement" inference from biometrics — **prohibited by EU AI Act Art. 5**.
- ❌ Covert-by-default monitoring — visible agent + on-screen monitoring indicator always.
- ❌ Webcam / camera / voiceprint capture.
- ❌ Keystroke-*dynamics* / behavioral-biometric identification.
- ❌ Capture of personal accounts / personal repos / personal email content.
- ❌ Harvesting or reusing employee credentials.

---

## 3. Users & personas

| Persona | Needs | Primary views |
|---|---|---|
| **Owner / Admin** (buyer) | Set up org, billing, allowlist, capture tiers, compliance config | Org overview, settings, billing, compliance |
| **Engineering Manager** | See team spend, work/personal split, ROI, alerts | Org overview, repo view, alerts |
| **Auditor / Legal / HR** | Pull evidence reports, manage DSAR/retention/notice | Compliance evidence export, consent tracking |
| **Developer** (monitored, transparency) | See own usage, flag misclassified repos | Developer self-view |

Default reporting is **aggregate/team-level**; individual drill-down is **role-gated + justification-logged + optionally pseudonymized** (NLRA narrow-tailoring; Productivity-Score lesson).

---

## 4. Architecture

```
   7 COLLECTORS (pluggable feeders)        CCGUARD CORE
  1 Endpoint agent  ─┐
  2 SCM repo feed   ─┤      ┌───────────────────────────────────────┐
  3 OTel endpoint   ─┼────▶ │ Ingest API → normalized "CCGuard event"│
  4 Network/cloud   ─┤      │              │                          │
  5 Git/SCM server  ─┤      │   Identity + REPO-ALLOWLIST classifier  │ ◀ moat
  6 OS extras       ─┤      │              │                          │
  7 DNS/egress      ─┘      │   Multi-tenant store (retention/residency)│
                           │              │                          │
                           │   Dashboard + Alerts + Evidence export   │
                           │                                          │
                           │   Cross-cutting: Consent/Compliance,     │
                           │   Tenant admin, Roles/SSO, Seat billing  │
                           └───────────────────────────────────────┘
```

**Principle:** every collector maps raw data into one normalized event. The classifier, store, and dashboard operate only on that event, so a collector can be added or deepened without touching the core. Each unit (collector, classifier, store, dashboard, billing, consent) has one purpose and a defined interface.

### 4.1 Proposed tech stack (recommendation — changeable)
- **Endpoint agent:** **Rust** — single static cross-platform binary, low footprint, filesystem watching (`notify` crate over ReadDirectoryChangesW/FSEvents/inotify), MDM-deployable. (Matches altkey Rust precedent.)
- **Core backend / ingest API:** **Rust (axum)** for the ingest hot-path + classifier; shares event types with the agent. *Alternative if team prefers faster CRUD: Python/FastAPI for the app, Rust only for agent + ingest.*
- **Datastore:** **Postgres** (multi-tenant: tenants, users, repos, allowlist, config, classified event summaries) + **object storage (S3-compatible)** for gated content blobs. **Event time-series** in Postgres partitioned tables to start; ClickHouse/Timescale as a later scale optimization.
- **Queue/stream:** Redis or NATS for ingest → classify → store pipeline.
- **Dashboard:** React/Next.js web app.
- **Billing:** Stripe (seat-based).
- **Auth:** email + SSO (SAML/SCIM) for enterprise tier.

---

## 5. The normalized CCGuard event

```jsonc
{
  "tenant_id":  "acme",
  "user":       { "email": "dev@acme.com", "seat_id": "u_123" },
  "tool":       "claude-code",          // cursor | copilot | codex | …
  "session_id": "abc-…",
  "ts":         "2026-06-09T21:13:00Z",
  "repo":       { "host": "github.com", "org": "acme-corp", "name": "billing-svc",
                  "path": "C:\\work\\billing-svc",
                  "classification": "work|personal|unknown", "confidence": 0.0 },
  "activity":   { "type": "prompt|tool_use|edit|commit|api_request|session_start",
                  "tokens_in": 0, "tokens_out": 0, "cost_usd": 0.0,
                  "model": "claude-opus-4-8", "tool_name": "Bash",
                  "decision": "accept|reject|null" },
  "content_ref": null,                  // S3 pointer to gated content blob, if consented
  "source_layer": "endpoint_agent"      // 1 of 7
}
```

`tool` being a field means cross-tool totals ("company AI spend on personal projects across Claude Code + Cursor + Copilot") fall out for free. `content_ref` is null unless the tenant has enabled content capture; the blob is redacted on write.

---

## 6. Repo-allowlist classifier

**Per-tenant allowlist**, auto-seeded from the SCM org API (collector 2), then editable:
- approved **git hosts** (`github.com`, `gitlab.acme.com`)
- approved **orgs/owners** (`acme-corp`, `acme-labs`)
- approved **local path roots** (`C:\work\…`, `~/acme/…`) for non-git / pre-clone activity

**Decision:**

| Repo signal | Classification |
|---|---|
| host + org both in allowlist | **work** |
| git remote present, host/org not in allowlist | **personal** |
| no remote / unknown host / scratch dir | **unknown** (tenant sets default treatment) |

**Confidence & anti-spoofing:** the agent/OTel report the repo client-side; CCGuard *verifies* it against SCM push events/hooks (collector 5) — a dev can't fake a remote URL at push time. Signed-commit enforcement raises confidence.

**Edge cases (explicit):** personal forks of company repos (upstream-org check), SSH↔HTTPS URL normalization (`git@host:org/repo.git` ↔ `https://host/org/repo`), monorepos (path-prefix sub-rules), non-git directories (path-root rules).

**Headline aggregation:** sum `cost_usd` / tokens grouped by `(user, classification)` and by `(repo)`.

---

## 7. The 7 collectors (all in scope, full depth)

### Collector 1 — Endpoint agent (richest)
Rust agent, MDM-deployable, runs as a service/daemon (SYSTEM/root), tamper-resistant, **visible** (shows a monitoring indicator; never covert).
- **On-disk harvest:** filesystem-watch + byte-offset tail of `~/.claude/projects/**/*.jsonl` (+ `subagents/`, `tool-results/`), `~/.claude/history.jsonl`, `~/.claude.json`, `paste-cache/`, `file-history/`. Yields full prompts/replies/tool calls/stdout/edits/tokens/cost/git-branch — repo from the encoded folder name + per-line `cwd`/`gitBranch`; identity from `oauthAccount.emailAddress`.
- **Process monitor:** detect `claude`/`node` exec + args + cwd (ETW/Sysmon · EndpointSecurity · eBPF/`/proc`) — confirms activity even if transcripts are deleted (tamper signal).
- **Git-on-disk:** repos, branches, commits, remotes.
- **Cross-tool:** harvest Cursor/Copilot local artifacts where present → same event shape.
- **Capture depth obeys the tenant's tier toggle** (metadata-only ships only counts + repo; content tier ships the transcript text to a redacted blob).

### Collector 2 — SCM repo feed (powers the classifier)
OAuth connect to GitHub/GitLab/Bitbucket → enumerate orgs/repos (`GET /orgs/{org}/repos`, GitLab `/groups/{id}/projects`, Bitbucket `/repositories/{ws}`) → auto-build + refresh the allowlist. Admin edits/overrides in the UI.

### Collector 3 — OTel endpoint (clean agentless channel)
Stand up an OTLP receiver. Customer points Claude Code at it + force-enables via managed-settings (`CLAUDE_CODE_ENABLE_TELEMETRY=1`, `OTEL_*_EXPORTER=otlp`, endpoint, `OTEL_RESOURCE_ATTRIBUTES=tenant.id…,enduser.id…`). Parse metrics (`claude_code.token.usage`, `.cost.usage`, `.code_edit_tool.decision`, `.session.count`, `.active_time.total`) + events. Repo attribution still comes from a CCGuard **hooks bundle** (deployed via managed-settings, runs `git remote get-url origin` on `SessionStart`/`PreToolUse`) since OTel lacks repo info. Lock with `allowManagedHooksOnly: true`.

### Collector 4 — Network / cloud capture (cross-tool, deep content)
Ingest connectors for: **Bedrock model-invocation logging** (S3/CloudWatch → full prompt+completion + `identity.arn`), **Vertex request-response logging** (BigQuery), **self-hosted gateway** logs (LiteLLM/Portkey/Cloudflare AI Gateway — `ANTHROPIC_BASE_URL` force-routed), and **SASE/TLS-inspection** exports (Zscaler/Netskope). Captures *all* AI tools' content. Pair gateway routing with an egress-firewall rule (block direct `api.anthropic.com`) so it can't be bypassed; flag direct hits as tamper.

### Collector 5 — Git/SCM server-side (tamper-proof)
- **Commit-trailer scanner:** detect `Co-Authored-By: Claude` + cross-tool markers via `git log --format='%(trailers…)'` / GitHub Search API. **Enforce** trailers via managed settings so they're trustworthy.
- **Audit-log streaming receiver:** ingest GitHub/GitLab audit events (push/clone/fetch) streamed to CCGuard/SIEM (defeats GitHub's 7-day retention).
- **Server-side push-hook receiver:** for Enterprise-Server/GitLab-DC/Bitbucket-DC `pre/post-receive` → author + repo + files + trailer on 100% of pushes.
- **Secret-scan join:** AI-trailered commit ∩ secret-push attempt → risk alert.
- **Signing check:** report whether require-signed-commits is on (confidence modifier).

### Collector 6 — OS extras (scaffolded, consent-gated, off by default)
In the agent: active-window/idle (productivity) and the heavy vectors — **screenshots / clipboard / keystroke-content** — built but **OFF**, each behind its own tenant opt-in toggle + notice banner + redaction. Never keystroke-*dynamics* (biometric). Process-detect (above) is the always-on, low-risk part.

### Collector 7 — DNS / egress (bypass backstop)
Ingest endpoint for DNS-resolver / firewall / NetFlow logs. Rule: a machine resolving/hitting `api.anthropic.com` *not* via the sanctioned path = bypass/tamper alert. Coarse, no content; pure tripwire.

---

## 8. Dashboard & alerts (6 views)

1. **Org overview** — seats, total AI spend, **work/personal/unknown donut**, trend, top spenders, top personal-burners, alerts feed.
2. **Repo view** — spend/activity by repo + **unknown-repo triage queue** (classify → allowlist or personal).
3. **Per-seat drill-down** — one dev's split, repos, tokens/cost over time, accept-reject, active time. Role-gated + justification-logged + pseudonymizable.
4. **Alerts / policy rules** — personal-spend threshold, token-burn anomaly, bypass/tamper, secret-push-on-AI-commit → dashboard/email/Slack/HR.
5. **Compliance evidence export** — audit-ready report (per-user or aggregate, date range, consent/notice status attached). *The workflow moat.*
6. **Developer self-view** — dev sees own classification, flags misclassifications. *The transparency differentiator.*

---

## 9. Consent / compliance layer (cross-cutting, ships with core)

- **Onboarding consent + notice** on install/first-run; acknowledgment logged (timestamped). **Notice-template generator + e-acknowledgment tracking** (NY §52-c / CT §31-48d / DE §19-705).
- **Capture-depth tiers (per tenant):** `metadata-only` (default) → `repo-attribution` → `content-capture` (opt-in, redacted) → `os-extras` (gated, off). Each tier gates the depth a collector ships.
- **Redaction** of secrets/PII/passwords on any content path.
- **Data governance:** per-data-type retention + auto-purge · residency region (EU/US, region-pinned processing) · DSAR workflow (access/delete/correct) · pseudonymization toggle.
- **Generated artifacts (sales accelerators):** DPIA wizard, LIA template, works-council annex.
- **Jurisdiction config:** per-employee work location drives consent mode (incl. all-party-consent-state handling, GDPR legitimate-interest basis, German works-council enablement gate).

Design to the **California + GDPR worst case** and most jurisdictions are covered.

---

## 10. Multi-tenant, auth, roles

- **Tenant isolation** by `tenant_id` across all tables/blobs.
- **Roles:** owner, admin, manager, auditor, member(developer). Individual drill-down + evidence export are role-gated; all admin actions written to an audit log.
- **Auth:** email/password + org SSO (SAML) + SCIM provisioning on the enterprise tier.
- **Onboarding:** signup → create org → connect SCM (OAuth, auto-build allowlist) → deploy agent (download installer / MDM package / managed-settings file generator) → invite seats → first donut.

---

## 11. Billing

- **Seat-based** per monitored developer/month, via Stripe.
- **Tiers** gated by capture depth + compliance suite + retention length + SSO/SCIM. (Indicative: Starter = metadata + repo-attribution + aggregate; Pro = content capture + individual drill-down + alerts; Enterprise = SSO/SCIM + residency + works-council/DPIA artifacts + network/cloud connectors.)
- Trajectory aligns to $4k Dec → $15k May.

---

## 12. Data model (first cut)

- `tenants`, `users`(seats), `roles`, `scm_connections`, `repos`(+classification, allowlist flags), `allowlist_rules`, `events`(partitioned time-series, normalized shape), `content_blobs`(S3 refs, gated), `alerts`, `alert_rules`, `consent_records`, `retention_policies`, `audit_log`, `subscriptions`(Stripe).

---

## 13. Security & platform-owner risk

- **Platform-owner risk:** Anthropic could expose working-dir in telemetry and close the wedge (6–18 mo). **Mitigations:** (a) cross-tool from day one (Cursor/Copilot/Codex), (b) own the compliance/evidence *workflow* Anthropic won't build, (c) the endpoint agent captures more than any first-party API ever will.
- CCGuard handles sensitive data (prompts, code, credentials-in-transcripts) → encryption at rest + in transit, tenant isolation, least-privilege, SOC 2 path on the enterprise tier, redaction before storage, configurable retention/purge.

---

## 14. Build order (all 7 are in scope — this is sequence, not scope-cut)

- **Phase 0 — Core:** event schema + ingest API, multi-tenant Postgres, auth/roles, classifier, minimal dashboard (org overview + repo view), consent skeleton, Stripe skeleton.
- **Phase 1 — Make the wedge real + dogfood:** Collector 1 (endpoint agent) + Collector 2 (SCM repo feed). Run on your own team → first donut + first case study.
- **Phase 2 — Cheap/real channels:** Collector 3 (OTel + hooks bundle) + Collector 5 (git/SCM server-side).
- **Phase 3 — Deep/cross-tool:** Collector 4 (network/cloud) + Collector 6 (OS extras, gated) + Collector 7 (DNS/egress).
- **Phase 4 — Enterprise:** SSO/SCIM, compliance artifact generators (DPIA/LIA/works-council), residency, SOC 2 path.

Each phase ships something usable; collectors deepen independently afterward.

---

## 15. Success metrics
- Activation: org connects SCM + deploys agent + sees first work/personal donut.
- Wedge proof: % of orgs where CCGuard surfaces real personal-project spend in month 1.
- Revenue: seats × price → $4k MRR Dec 2026, $15k MRR May 2027.

---

## 16. Open questions
1. Backend language: Rust-everywhere (shared types, perf) vs Rust-agent + Python/FastAPI-app (faster CRUD)? (Recommend deciding at plan time.)
2. First billing tiers & price points (needs a pricing pass).
3. Which SCM to support first (GitHub almost certainly) and whether Bitbucket Cloud's weak audit is acceptable for v1.
4. Hosted region(s) at launch for residency.
5. Dogfood target: which of your own repos/team is the Phase-1 pilot.
