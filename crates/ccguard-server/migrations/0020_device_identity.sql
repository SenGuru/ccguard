-- The real seat identity is the MACHINE, not the Claude Code subscription email
-- (a subscription can be shared across people/machines). Attribute sessions to a
-- device; keep the subscription email + plan as side info.
alter table captured_sessions add column if not exists device_id text;
alter table captured_sessions add column if not exists hostname text;
alter table captured_sessions add column if not exists plan text;

create index if not exists captured_sessions_hostname on captured_sessions (tenant_id, hostname);
