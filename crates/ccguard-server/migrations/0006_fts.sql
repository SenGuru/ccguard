-- Full-text search over captured content.
-- Generated tsvector kept in-sync on every insert/update of content_blobs.
-- left(content, 800000) truncates so we never exceed Postgres's ~1MB tsvector cap.
alter table content_blobs
  add column if not exists content_tsv tsvector
  generated always as (to_tsvector('english', left(content, 800000))) stored;

create index if not exists content_blobs_tsv_idx on content_blobs using gin (content_tsv);
