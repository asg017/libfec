import uuid
from generated_client import Db

db = Db("tmp.db")
worker_id = str(uuid.uuid4())


def format_race(office, state, district):
    if office == "P":
        return "President"
    if office == "S":
        return f"{state} Senate"
    return f"{state}-{district}"


def format_support_oppose(code):
    if code == "S":
        return "supporting"
    return "opposing"


while True:
    with db:
        jobs = db.claim_job(worker_id=worker_id, lease_seconds=30)
    if not jobs:
        break
    job = jobs[0]
    filing_id = job.item_id

    try:
        rows = db.filing_spent_per_candidate(filing_id)
        assert rows
        for r in rows:
            race = format_race(r.candidate_office, r.candidate_state, r.candidate_district)
            action = format_support_oppose(r.support_oppose_code)
            print(
                f"{r.filer_name} spent ${r.total_spent:,.2f} {action} {r.candidate_first_name} {r.candidate_last_name} ({r.candidate_id_number}) in {race}"
            )
        with db:
            db.complete_job(job_id=job.id, worker_id=worker_id)
    except Exception as e:
        with db:
            db.fail_job(job_id=job.id, worker_id=worker_id, retry_delay=5, error=str(e))
        raise
