-- LLM-tier triage: a Claude judge resolves sessions the deterministic signal
-- cascade left UNCLASSIFIED (classification='unknown') into work / personal /
-- unsure, with a confidence and a one-line reason.
--
-- Two-track rule (deliberate):
--   * Dashboard VISIBILITY trusts the LLM label freely (and we mirror it onto
--     captured_sessions.classification so the existing views light up).
--   * USAGE-LIMITING / ENFORCEMENT only counts a verdict when `enforceable` is
--     true — i.e. an independent structural signal agreed, or a human confirmed.
--     Content is model-judged and gameable, so a wrong "personal" must not be
--     able to throttle someone on the strength of the LLM alone.

-- Per-tenant config: whether triage is on, the org's own definition of "work"
-- (fed to the judge), and which model to use.
create table if not exists tenant_triage_config (
    tenant_id       text primary key references tenants(id),
    enabled         boolean not null default false,
    work_definition text not null default '',
    model           text not null default 'claude-haiku-4-5',
    updated_at      timestamptz not null default now()
);

-- One verdict per session.
create table if not exists session_triage (
    tenant_id    text not null,
    session_id   text not null,
    label        text not null,                 -- work | personal | unsure
    confidence   real not null,
    reason       text not null,
    model        text not null,
    resolved_by  text not null default 'llm',   -- llm | human
    -- Deterministic corroboration captured at triage time (for transparency):
    -- the structural classifier's own label for this session, or 'none'.
    structural   text not null default 'none',
    -- True only when a structural signal agreed OR a human confirmed — the gate
    -- for letting this verdict count toward enforcement / usage limits.
    enforceable  boolean not null default false,
    created_at   timestamptz not null default now(),
    updated_at   timestamptz not null default now(),
    primary key (tenant_id, session_id)
);
create index if not exists session_triage_tenant_idx on session_triage (tenant_id, label);
