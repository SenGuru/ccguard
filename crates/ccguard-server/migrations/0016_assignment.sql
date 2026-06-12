-- Per-person assignment: what THIS employee is currently supposed to be working
-- on (admin-set, plain English). Distinct from job_role (their function) and from
-- the tenant-wide work definition (what counts as company work at all). Feeds the
-- AI judge as context so it can flag company work that is off this person's lane
-- (the off_assignment signal) without changing work-vs-personal.
alter table employee_roles add column if not exists assignment text;

-- The judge's off-assignment verdict bit, persisted alongside the triage row.
alter table session_triage add column if not exists off_assignment boolean not null default false;
