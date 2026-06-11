-- Tier-A hardening + enforcement arming.
--
-- Tier-A judge moves from admin prose to a STRUCTURED policy schema (typed
-- predicates) to remove the prose-injection surface, and its verdicts are gated by
-- a conformal selective threshold (abstain rather than label-force). Enforcement is
-- only ever armable after a build-time precision GO/NO-GO clears on a labeled
-- holdout; until then everything is observation-only.

-- Structured Tier-A predicates (supplement the free-text work_definition).
alter table tenant_triage_config add column if not exists work_domains text not null default '';
alter table tenant_triage_config add column if not exists work_ticket_prefixes text not null default '';
alter table tenant_triage_config add column if not exists approved_langs text not null default '';

-- Independent human labels on triage verdicts. The reviewer can AGREE (confirm) or
-- DISAGREE (relabel) — disagreement is what makes the conformal calibration and the
-- precision gate non-circular ground truth, not a rubber stamp of the model.
alter table session_triage add column if not exists human_reviewed boolean not null default false;
alter table session_triage add column if not exists human_label text;  -- work | personal | unsure

-- The enforcement arming record (tenant-level in v1). Surfaced to the dev's own
-- view as a first-class fact ("no rung armed; observation-only since <date>").
create table if not exists enforcement_arming (
    tenant_id              text primary key references tenants(id),
    armed                  boolean not null default false,   -- stays false until a human arms a passing GO
    precision_go           boolean not null default false,   -- latest build-time GO/NO-GO
    n_labels               integer not null default 0,
    false_personal_rate    real not null default 1.0,
    false_personal_upper   real not null default 1.0,        -- Wilson upper bound (the gated number)
    conformal_threshold    real not null default 1.01,       -- judge abstains below this confidence
    fail_closed_state      text not null default 'observation_only',
    decided_at             timestamptz not null default now()
);
