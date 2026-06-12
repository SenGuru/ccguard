-- Re-judge-on-growth: remember how many events a session had WHEN it was judged.
-- When it later grows materially (a continued conversation) and settles, the
-- triage sweep re-judges it; a human-confirmed label that drifts raises a review
-- flag instead of being overwritten.
alter table session_triage add column if not exists judged_event_count int not null default 0;

-- Backfill existing rows to the session's current event_count, so nothing re-judges
-- immediately on rollout — only FUTURE growth past this baseline triggers a re-judge.
update session_triage t
set judged_event_count = s.event_count
from captured_sessions s
where s.tenant_id = t.tenant_id and s.session_id = t.session_id
  and t.judged_event_count = 0;
