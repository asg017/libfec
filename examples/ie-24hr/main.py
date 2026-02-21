import argparse
import time
import uuid
import urllib.request
from generated_client import Db


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


def notify(message):
    req = urllib.request.Request(
        "https://ntfy.sh/15B17BCC55AFA3967CCCEB82",
        data=message.encode(),
    )
    urllib.request.urlopen(req)


def process_queue(db, worker_id, alert_threshold):
    while True:
        with db:
            jobs = db.claim_job(worker_id=worker_id, lease_seconds=30)
        if not jobs:
            return False
        job = jobs[0]
        filing_id = job.item_id

        try:
            rows = db.filing_spent_per_candidate(filing_id)
            assert rows
            for r in rows:
                race = format_race(
                    r.candidate_office, r.candidate_state, r.candidate_district
                )
                action = format_support_oppose(r.support_oppose_code)
                msg = f"{r.filer_name} spent ${r.total_spent:,.2f} {action} {r.candidate_first_name} {r.candidate_last_name} ({r.candidate_id_number}) in {race}"
                print(msg)
                if alert_threshold is not None and r.total_spent >= alert_threshold:
                    notify(msg)
            with db:
                db.complete_job(job_id=job.id, worker_id=worker_id)
        except Exception as e:
            with db:
                db.fail_job(
                    job_id=job.id, worker_id=worker_id, retry_delay=5, error=str(e)
                )
            raise
    return True


def main():
    parser = argparse.ArgumentParser(
        description="Process F24 independent expenditure queue"
    )
    parser.add_argument("db", help="Path to SQLite database")
    parser.add_argument(
        "--alert-threshold",
        type=float,
        default=None,
        help="Dollar amount threshold for ntfy alerts",
    )
    args = parser.parse_args()

    db = Db(args.db)
    worker_id = str(uuid.uuid4())

    while True:
        had_work = process_queue(db, worker_id, args.alert_threshold)
        if not had_work:
            time.sleep(5)


if __name__ == "__main__":
    main()
