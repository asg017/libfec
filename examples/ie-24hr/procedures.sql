-- schema: tmp.db

-- name: claim_job
with
  enqueued as (
    select id
    from queue
    where status in ('pending', 'leased')
      and available_at <= unixepoch('subsec')
      and (lease_until is null
        or lease_until < unixepoch('subsec'))
      and attempts < max_attempts
    order by priority desc, created_at
    limit 1
  )
update queue
set
  status = 'leased',
  leased_by = :worker_id,
  lease_until = unixepoch('subsec') + ifnull(:lease_seconds, 3),
  attempts = attempts + 1
where id = (select id from enqueued)
returning *;

-- name: complete_job
update queue
set
  status = 'completed',
  completed_at = unixepoch('subsec')
where id = :job_id
  and leased_by = :worker_id
returning *;

-- name: fail_job
update queue
set
  status = 'pending',
  lease_until = null,
  leased_by = null,
  available_at = unixepoch('subsec') + :retry_delay,
  last_error = :error
where id = :job_id
  and leased_by = :worker_id
returning *;

-- name: filing_spent_per_candidate
select
  f.filer_name,
  e.support_oppose_code,
  e.candidate_first_name,
  e.candidate_last_name,
  e.candidate_office,
  e.candidate_state,
  e.candidate_district,
  e.candidate_id_number,
  sum(e.expenditure_amount) as total_spent
from libfec_schedule_e as e
join libfec_filings as f on e.filing_id = f.filing_id
where e.filing_id = $filing_id
group by e.candidate_id_number, e.support_oppose_code;
