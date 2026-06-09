create table if not exists api_tokens (
    id          bigserial primary key,
    tenant_id   text not null references tenants(id),
    token_hash  text not null unique,
    name        text not null default 'ingest',
    created_at  timestamptz not null default now(),
    revoked_at  timestamptz
);

create index if not exists api_tokens_hash on api_tokens (token_hash);
