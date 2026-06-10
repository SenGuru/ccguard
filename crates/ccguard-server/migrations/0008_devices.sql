create table if not exists devices (
    id            bigserial primary key,
    tenant_id     text not null,
    device_id     text not null,         -- stable per-machine id (hostname + machine guid hash)
    hostname      text,
    os            text,
    agent_version text,
    user_email    text,
    -- latest attestation snapshot:
    policy_present   boolean not null default false,
    policy_match     boolean not null default false,
    telemetry_on     boolean not null default false,
    hook_present     boolean not null default false,
    login_locked     boolean not null default false,
    personal_account boolean not null default false,
    compliance    text not null default 'unknown',  -- compliant|drifted|tampered|noncompliant_account|stale|unknown
    reasons       text,                              -- comma-joined drift reasons
    last_seen     timestamptz,
    created_at    timestamptz not null default now(),
    unique (tenant_id, device_id)
);
create index if not exists devices_tenant_idx on devices (tenant_id, compliance);

create table if not exists tenant_policy (
    tenant_id     text primary key,
    server_url    text not null,
    org_uuid      text not null,
    otel_endpoint text not null,
    min_version   text not null,
    token_env     text not null default 'CCGUARD_TOKEN',
    updated_at    timestamptz not null default now()
);
