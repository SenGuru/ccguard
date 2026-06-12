-- Speed up the triage evidence "continuity" channel. The related-sessions file
-- self-join (and its hub-limit subquery) match captured_events by target where
-- kind='file_edit'; with no index that was an O(file_edits^2) seq-scan per row
-- (~2s on a session with hundreds of edits). A partial index on the file-edit
-- targets turns it into an index lookup.
create index if not exists captured_events_file_target
    on captured_events (tenant_id, target)
    where kind = 'file_edit';
