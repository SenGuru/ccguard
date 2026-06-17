-- Which AI coding tool produced each captured session. Added for cross-tool
-- capture (Claude Code / OpenAI Codex CLI / GitHub Copilot CLI). Existing rows
-- and older agents that don't send the field default to 'claude_code'.
alter table captured_sessions
    add column if not exists tool text not null default 'claude_code';

create index if not exists captured_sessions_tenant_tool_idx
    on captured_sessions (tenant_id, tool);
