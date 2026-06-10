create table if not exists findings (
    id         bigserial primary key,
    tenant_id  text not null,
    session_id text not null,
    seq        bigint not null,
    kind       text not null,
    rule       text not null,
    severity   text not null,
    redacted   text not null,
    created_at timestamptz not null default now(),
    unique (tenant_id, session_id, seq, rule, redacted)
);
create index if not exists findings_tenant_idx on findings (tenant_id, severity);
create index if not exists findings_session_idx on findings (tenant_id, session_id);
