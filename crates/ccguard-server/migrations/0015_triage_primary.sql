-- session_triage is now the PRIMARY classification record (not a fallback): richer
-- verdict contract + durable retry, so the agent-driven per-session sweep can drain
-- 'pending' sessions reliably without a separate jobs table.
alter table session_triage add column if not exists mixed          boolean     not null default false;
alter table session_triage add column if not exists matched_clause text;
alter table session_triage add column if not exists policy_version integer     not null default 1;
alter table session_triage add column if not exists gaming_flags   text[]      not null default '{}';
alter table session_triage add column if not exists relabel_reason text;
alter table session_triage add column if not exists attempts       integer     not null default 1;
alter table session_triage add column if not exists next_retry_at  timestamptz;
alter table session_triage add column if not exists input_digest   text;
-- resolved_by (text) gains values 'shortcut' | 'admin_override' | 'server_api'
-- alongside the existing 'llm' | 'human'. No enum to migrate (it is plain text).

create index if not exists session_triage_retry_idx
    on session_triage (tenant_id, next_retry_at) where next_retry_at is not null;
