-- Co-Owned Ledger: the humane personal-usage split (v1 = transparency only).
-- Denominator is SESSION-COUNT, never dollars (the JSONL token fields undercount
-- 100-174x, so a dollar meter fires on fiction). UNCLASSIFIED is excluded from
-- both numerator and denominator. v1 ships Rungs 0-1 only: nothing is armed; the
-- meter is observation-only and reciprocal.

create table if not exists tenant_limit_config (
    tenant_id              text primary key references tenants(id),
    -- "personal <= N% of SESSIONS" — labeled estimated split, not billed dollars.
    personal_allowance_pct integer not null default 20,
    -- No enforcement rung is armed in v1; this stays false (transparency only).
    armed                  boolean not null default false,
    -- "observation-only since X" surfaced to the dev's own view as a first-class fact.
    observation_since      timestamptz not null default now(),
    updated_at             timestamptz not null default now()
);
