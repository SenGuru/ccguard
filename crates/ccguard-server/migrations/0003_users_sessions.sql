create table if not exists users (
    id            bigserial primary key,
    tenant_id     text not null references tenants(id),
    email         text not null,
    password_hash text not null,
    role          text not null check (role in ('owner','admin','manager','auditor','member')),
    created_at    timestamptz not null default now(),
    unique (tenant_id, email)
);

create table if not exists sessions (
    id          bigserial primary key,
    user_id     bigint not null references users(id),
    token_hash  text not null unique,
    created_at  timestamptz not null default now(),
    expires_at  timestamptz
);

create index if not exists sessions_hash on sessions (token_hash);
