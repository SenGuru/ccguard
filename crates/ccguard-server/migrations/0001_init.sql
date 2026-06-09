create table if not exists tenants (
    id         text primary key,
    name       text not null,
    created_at timestamptz not null default now()
);

create table if not exists allowlist_rules (
    id         bigserial primary key,
    tenant_id  text not null references tenants(id),
    kind       text not null check (kind in ('host', 'org', 'path_root')),
    value      text not null,
    created_at timestamptz not null default now()
);

create table if not exists events (
    id            bigserial primary key,
    tenant_id     text not null references tenants(id),
    user_email    text not null,
    seat_id       text,
    tool          text not null,
    session_id    text not null,
    ts            timestamptz not null,
    repo_host     text,
    repo_org      text,
    repo_name     text,
    repo_path     text,
    classification text not null,
    confidence    real not null default 0,
    activity_type text not null,
    tokens_in     bigint not null default 0,
    tokens_out    bigint not null default 0,
    cost_usd      double precision not null default 0,
    model         text,
    tool_name     text,
    content_ref   text,
    source_layer  text not null,
    created_at    timestamptz not null default now()
);

create index if not exists events_tenant_ts on events (tenant_id, ts);
create index if not exists events_tenant_class on events (tenant_id, classification);
