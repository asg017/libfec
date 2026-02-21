# hash: 181c8e3eeaabf1f6a57c4b59df9396c9162b3f2269ca77f32f67e351b1e42d00
import sqlite3

from typing import Any, Optional

from dataclasses import dataclass

@dataclass
class ClaimJobResult:
  id: int
  item_id: str
  status: str
  priority: int
  attempts: int
  max_attempts: int
  lease_until: int
  leased_by: str
  created_at: int
  available_at: int
  completed_at: int
  failed_at: int
  last_error: str

@dataclass
class CompleteJobResult:
  id: int
  item_id: str
  status: str
  priority: int
  attempts: int
  max_attempts: int
  lease_until: int
  leased_by: str
  created_at: int
  available_at: int
  completed_at: int
  failed_at: int
  last_error: str

@dataclass
class FailJobResult:
  id: int
  item_id: str
  status: str
  priority: int
  attempts: int
  max_attempts: int
  lease_until: int
  leased_by: str
  created_at: int
  available_at: int
  completed_at: int
  failed_at: int
  last_error: str

@dataclass
class FilingSpentPerCandidateResult:
  filer_name: str
  support_oppose_code: str
  candidate_first_name: str
  candidate_last_name: str
  candidate_office: str
  candidate_state: str
  candidate_district: str
  candidate_id_number: str
  total_spent: Any

class Db:
  def __init__(self, *kwargs):
    self.connection = sqlite3.connect(*kwargs)
    sql = ''
    self.connection.executescript(sql)

  def __enter__(self):
    self.connection = self.connection.__enter__()
    return self

  def __exit__(self, exc_type, exc_value, traceback):
    return self.connection.__exit__(exc_type, exc_value, traceback)

  def claim_job(self, worker_id: Any, lease_seconds: Any) -> Optional[list[ClaimJobResult]]:
    sql = "with\n  enqueued as (\n    select id\n    from queue\n    where status in ('pending', 'leased')\n      and available_at <= unixepoch('subsec')\n      and (lease_until is null\n        or lease_until < unixepoch('subsec'))\n      and attempts < max_attempts\n    order by priority desc, created_at\n    limit 1\n  )\nupdate queue\nset\n  status = 'leased',\n  leased_by = :worker_id,\n  lease_until = unixepoch('subsec') + ifnull(:lease_seconds, 3),\n  attempts = attempts + 1\nwhere id = (select id from enqueued)\nreturning *;"
    params = {'worker_id': worker_id, 'lease_seconds': lease_seconds}
    result = self.connection.execute(sql, params)
    return [ClaimJobResult(*row) for row in result.fetchall()]

  def complete_job(self, job_id: Any, worker_id: Any) -> Optional[list[CompleteJobResult]]:
    sql = "update queue\nset\n  status = 'completed',\n  completed_at = unixepoch('subsec')\nwhere id = :job_id\n  and leased_by = :worker_id\nreturning *;"
    params = {'job_id': job_id, 'worker_id': worker_id}
    result = self.connection.execute(sql, params)
    return [CompleteJobResult(*row) for row in result.fetchall()]

  def fail_job(self, retry_delay: Any, error: Any, job_id: Any, worker_id: Any) -> Optional[list[FailJobResult]]:
    sql = "update queue\nset\n  status = 'pending',\n  lease_until = null,\n  leased_by = null,\n  available_at = unixepoch('subsec') + :retry_delay,\n  last_error = :error\nwhere id = :job_id\n  and leased_by = :worker_id\nreturning *;"
    params = {'retry_delay': retry_delay, 'error': error, 'job_id': job_id, 'worker_id': worker_id}
    result = self.connection.execute(sql, params)
    return [FailJobResult(*row) for row in result.fetchall()]

  def filing_spent_per_candidate(self, filing_id: Any) -> Optional[list[FilingSpentPerCandidateResult]]:
    sql = 'select\n  f.filer_name,\n  e.support_oppose_code,\n  e.candidate_first_name,\n  e.candidate_last_name,\n  e.candidate_office,\n  e.candidate_state,\n  e.candidate_district,\n  e.candidate_id_number,\n  sum(e.expenditure_amount) as total_spent\nfrom libfec_schedule_e as e\njoin libfec_filings as f on e.filing_id = f.filing_id\nwhere e.filing_id = $filing_id\ngroup by e.candidate_id_number, e.support_oppose_code;'
    params = {'filing_id': filing_id}
    result = self.connection.execute(sql, params)
    return [FilingSpentPerCandidateResult(*row) for row in result.fetchall()]
