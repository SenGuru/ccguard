-- Optional admin-set weekly token budget, for the "measured tokens" usage view.
-- This is the ADMIN'S figure for their plan's rough weekly allowance, NOT a
-- scraped account limit (the subscription limit isn't cleanly readable, and
-- transcript tokens aren't the same unit it counts in). 0 / null = unset.
alter table tenant_limit_config add column if not exists weekly_token_budget bigint;
