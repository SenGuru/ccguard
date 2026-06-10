alter table captured_sessions add column if not exists on_hold boolean not null default false;
