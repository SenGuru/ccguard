-- On-task scoring, job roles, per-repo work-definition overrides, and the
-- indicator review queue. Metadata-only; indicators feed a human review queue
-- (open -> reviewed/dismissed), never an automated verdict.

-- Admin per-repo work-definition. Consulted BEFORE the org allowlist so an
-- admin can mark a repo work/personal/unknown with a context note (handles
-- "looks unrelated but is related").
create table if not exists repo_overrides (
    id bigserial primary key,
    tenant_id text not null,
    repo_host text not null,
    repo_org text not null,
    repo_name text not null,
    classification text not null,      -- work | personal | unknown
    note text,
    updated_at timestamptz not null default now(),
    unique (tenant_id, repo_host, repo_org, repo_name)
);

-- Admin-assigned job role per employee.
create table if not exists employee_roles (
    tenant_id text not null,
    user_email text not null,
    job_role text not null,
    note text,
    updated_at timestamptz not null default now(),
    primary key (tenant_id, user_email)
);

-- Latest on-task score per session (recomputed on every capture chunk).
create table if not exists session_scores (
    tenant_id text not null,
    session_id text not null,
    score int not null,
    label text not null,               -- on_task | review | off_task
    reasons text,
    updated_at timestamptz not null default now(),
    primary key (tenant_id, session_id)
);

-- Review-queue indicators raised by scoring / role anomalies.
create table if not exists indicators (
    id bigserial primary key,
    tenant_id text not null,
    user_email text,
    session_id text,
    kind text not null,
    detail text,
    status text not null default 'open',  -- open | reviewed | dismissed
    created_at timestamptz not null default now()
);
create index if not exists indicators_tenant_idx on indicators (tenant_id, status);

-- Idempotency for auto-raised indicators: at most one open auto-indicator of a
-- given kind per session. Used with `on conflict do nothing`.
create unique index if not exists indicators_auto_uq
    on indicators (tenant_id, session_id, kind);

-- Tracked-ticket references detected in a session's captured content.
create table if not exists session_tickets (
    tenant_id text not null,
    session_id text not null,
    ticket text not null,
    primary key (tenant_id, session_id, ticket)
);
