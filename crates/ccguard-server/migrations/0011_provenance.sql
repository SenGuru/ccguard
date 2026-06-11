-- Provenance Ledger: the deterministic content-free classification cascade.
-- The agent sends content-free structural signals; the server evaluates them
-- against tenant policy (ccguard_core::provenance) and records the verdict here.
-- This SUPERSEDES the old git-allowlist guess as the primary classifier; what it
-- leaves UNCLASSIFIED flows to the LLM triage tier (migration 0010).

-- Per-tenant provenance policy. Corp hosts/orgs reuse the existing allowlist_rules
-- (kind=host/org); these are the additional, comma/newline-separated lists.
create table if not exists provenance_policy (
    tenant_id              text primary key references tenants(id),
    corp_email_domains     text not null default '',   -- e.g. "acme.com, eng.acme.com"
    personal_orgs          text not null default '',   -- explicitly-flagged personal destinations
    personal_email_domains text not null default '',   -- e.g. "gmail.com"
    ticket_prefixes        text not null default '',   -- exact JIRA prefixes, e.g. "ACME, BILL"
    corp_env_name          text not null default 'CCGUARD_CORP',  -- MDM-injected env var (C-MDM-ENV)
    registry_patterns      text not null default '',   -- corp registry/scope substrings
    updated_at             timestamptz not null default now()
);

-- One provenance verdict per session.
create table if not exists session_provenance (
    tenant_id   text not null,
    session_id  text not null,
    class       text not null,                 -- work | work_provisional | unclassified | personal
    confidence  real not null,
    provisional boolean not null default false,
    resolved_by text not null,                 -- tier_g | personal_confirmed | corroborator | unclassified
    reasons     text not null default '',      -- fired signal codes, "; "-joined
    updated_at  timestamptz not null default now(),
    primary key (tenant_id, session_id)
);
create index if not exists session_provenance_tenant_idx on session_provenance (tenant_id, class);
