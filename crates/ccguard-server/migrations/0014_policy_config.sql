-- AI-primary: the admin's plain-English business description becomes the classifier.
-- Two authored fields (+ optional contrast examples) compose into the prompt's
-- work_definition slot; policy_version stamps every verdict so calibration/labels
-- bind to the policy that produced them.
alter table tenant_triage_config add column if not exists business_desc     text not null default '';
alter table tenant_triage_config add column if not exists work_allowed      text not null default '';
alter table tenant_triage_config add column if not exists personal_examples text not null default '';
alter table tenant_triage_config add column if not exists template_key      text;
alter table tenant_triage_config add column if not exists policy_version    integer not null default 1;

-- AI-primary classification is the product — default the judge on for new tenants.
alter table tenant_triage_config alter column enabled set default true;
