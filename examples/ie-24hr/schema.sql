create table if not exists queue(
  id INTEGER primary key autoincrement,
  item_id TEXT not null,
  status TEXT not null default 'pending' check(status in ('pending', 'leased', 'failed', 'completed')),
  priority INTEGER default 0,
  attempts INTEGER default 0,
  max_attempts INTEGER default 5,
  lease_until INTEGER,
  leased_by TEXT,
  created_at INTEGER not null,
  available_at INTEGER not null,
  completed_at INTEGER,
  failed_at INTEGER,
  last_error TEXT
);

create index if not exists idx_queue_fetch on queue(status, available_at, lease_until, priority desc);

create index if not exists idx_queue_lease on queue(lease_until);

create index if not exists idx_queue_item on queue(item_id);

create trigger if not exists f24_enqueue
after insert on libfec_filings
when new.cover_record_form = 'F24'
  and new.cover_record_form_amendment_indicator = 'N'
begin
  insert into queue(item_id, created_at, available_at)
  values
    (new.filing_id, unixepoch('subsec'), unixepoch('subsec'));
end;

