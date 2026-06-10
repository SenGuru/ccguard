create table if not exists captured_sessions (
    id             bigserial primary key,
    tenant_id      text not null references tenants(id),
    session_id     text not null,
    user_email     text not null,
    repo_host      text, repo_org text, repo_name text, repo_path text,
    classification text not null,
    title          text,
    cwd            text,
    first_ts       timestamptz,
    last_ts        timestamptz,
    event_count    integer not null default 0,
    created_at     timestamptz not null default now(),
    unique (tenant_id, session_id)
);

create table if not exists content_blobs (
    id         bigserial primary key,
    tenant_id  text not null references tenants(id),
    sha256     text not null,
    content    text not null,
    bytes      integer not null,
    created_at timestamptz not null default now(),
    unique (tenant_id, sha256)
);

create table if not exists captured_events (
    id           bigserial primary key,
    tenant_id    text not null references tenants(id),
    session_id   text not null,
    seq          bigint not null,
    ts           timestamptz not null,
    kind         text not null,
    model        text,
    tool_name    text,
    target       text,
    content_sha  text,
    tokens_in    bigint not null default 0,
    tokens_out   bigint not null default 0,
    is_sidechain boolean not null default false,
    unique (tenant_id, session_id, seq)
);
create index if not exists captured_events_session on captured_events (tenant_id, session_id, seq);
create index if not exists captured_sessions_tenant_ts on captured_sessions (tenant_id, last_ts);
