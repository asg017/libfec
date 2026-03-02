import argparse
import logging
import os
import sqlite3
import time
import urllib.request
from dataclasses import dataclass
from typing import Literal
from dotenv import load_dotenv
from atproto import Client, client_utils

load_dotenv()
log = logging.getLogger(__name__)


SCHEMA_SQL = """
CREATE TABLE IF NOT EXISTS queue(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'completed')),
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_queue_status ON queue(status, created_at);

CREATE TRIGGER IF NOT EXISTS f24_enqueue
AFTER INSERT ON libfec_filings
WHEN new.cover_record_form = 'F24'
    AND new.cover_record_form_amendment_indicator = 'N'
BEGIN
    INSERT INTO queue(item_id, created_at)
    VALUES (new.filing_id, unixepoch('subsec'));
END;
"""

CLAIM_SQL = """
UPDATE queue
SET status = 'completed'
WHERE id = (
    SELECT id FROM queue
    WHERE status = 'pending'
    ORDER BY created_at
    LIMIT 1
)
RETURNING id, item_id;
"""

FILING_SPENT_PER_CANDIDATE_SQL = """
SELECT
    f.filer_name,
    e.filer_committee_id_number,
    e.support_oppose_code,
    e.candidate_first_name,
    e.candidate_last_name,
    c.party_affiliation,
    e.candidate_office,
    e.candidate_state,
    e.candidate_district,
    e.candidate_id_number,
    sum(e.expenditure_amount) AS total_spent,
    group_concat(DISTINCT e.expenditure_purpose_descrip) AS purposes
FROM libfec_schedule_e AS e
JOIN libfec_filings AS f ON e.filing_id = f.filing_id
LEFT JOIN libfec_candidates AS c ON e.candidate_id_number = c.candidate_id
WHERE e.filing_id = :filing_id
GROUP BY e.candidate_id_number, e.support_oppose_code;
"""

OutputMode = Literal["stdout", "ntfy", "bsky"]

STATE_NAMES: dict[str, str] = {
    "AL": "Alabama", "AK": "Alaska", "AZ": "Arizona", "AR": "Arkansas",
    "CA": "California", "CO": "Colorado", "CT": "Connecticut", "DE": "Delaware",
    "DC": "District of Columbia", "FL": "Florida", "GA": "Georgia", "HI": "Hawaii",
    "ID": "Idaho", "IL": "Illinois", "IN": "Indiana", "IA": "Iowa",
    "KS": "Kansas", "KY": "Kentucky", "LA": "Louisiana", "ME": "Maine",
    "MD": "Maryland", "MA": "Massachusetts", "MI": "Michigan", "MN": "Minnesota",
    "MS": "Mississippi", "MO": "Missouri", "MT": "Montana", "NE": "Nebraska",
    "NV": "Nevada", "NH": "New Hampshire", "NJ": "New Jersey", "NM": "New Mexico",
    "NY": "New York", "NC": "North Carolina", "ND": "North Dakota", "OH": "Ohio",
    "OK": "Oklahoma", "OR": "Oregon", "PA": "Pennsylvania", "PR": "Puerto Rico",
    "RI": "Rhode Island", "SC": "South Carolina", "SD": "South Dakota",
    "TN": "Tennessee", "TX": "Texas", "UT": "Utah", "VT": "Vermont",
    "VA": "Virginia", "WA": "Washington", "WV": "West Virginia", "WI": "Wisconsin",
    "WY": "Wyoming",
}

PARTY_NAMES: dict[str, str] = {
    "REP": "Republican", "GOP": "Republican",
    "DEM": "Democrat", "DFL": "Democrat",
    "IND": "Independent", "IDP": "Independent",
    "LIB": "Libertarian",
    "GRE": "Green",
    "NPA": "No Party Affiliation", "NON": "Nonpartisan", "NNE": "None",
    "NOP": "No Party", "UN": "Unaffiliated",
}

@dataclass
class QueueJob:
    id: int
    item_id: str


@dataclass
class CandidateSpending:
    filer_name: str
    filer_committee_id_number: str
    support_oppose_code: str
    candidate_first_name: str
    candidate_last_name: str
    party_affiliation: str | None
    candidate_office: str
    candidate_state: str
    candidate_district: str
    candidate_id_number: str
    total_spent: float
    purposes: str | None


class Db:
    conn: sqlite3.Connection

    def __init__(self, path: str) -> None:
        self.conn = sqlite3.connect(path)
        self.conn.execute("PRAGMA journal_mode=WAL")
        self.conn.executescript(SCHEMA_SQL)

    def claim(self) -> QueueJob | None:
        row = self.conn.execute(CLAIM_SQL).fetchone()
        self.conn.commit()
        if not row:
            return None
        return QueueJob(*row)

    def filing_spent_per_candidate(self, filing_id: str) -> list[CandidateSpending]:
        rows = self.conn.execute(FILING_SPENT_PER_CANDIDATE_SQL, {"filing_id": filing_id}).fetchall()
        return [CandidateSpending(*row) for row in rows]


def format_race(office: str, state: str, district: str) -> str:
    if office == "P":
        return "President"
    if office == "S":
        return f"{STATE_NAMES.get(state, state)} Senate"
    return f"{state}-{district}"


def format_support_oppose(code: str) -> str:
    if code == "S":
        return "supporting"
    return "opposing"


def filing_url(committee_id: str, filing_id: str) -> str:
    return f"https://docquery.fec.gov/cgi-bin/forms/{committee_id}/{filing_id}/se"


def format_message(r: CandidateSpending, filing_id: str) -> str:
    race = format_race(r.candidate_office, r.candidate_state, r.candidate_district)
    action = format_support_oppose(r.support_oppose_code)
    name = f"{r.candidate_first_name} {r.candidate_last_name}"
    party = PARTY_NAMES.get(r.party_affiliation, r.party_affiliation) if r.party_affiliation else None
    if party:
        msg = f"{r.filer_name} spent ${r.total_spent:,.2f} {action} {party} {name} in {race}"
    else:
        msg = f"{r.filer_name} spent ${r.total_spent:,.2f} {action} {name} in {race}"
    if r.purposes:
        msg += f" ({r.purposes})"
    msg += f"\n{filing_url(r.filer_committee_id_number, filing_id)}"
    return msg


def post_ntfy(message: str) -> None:
    log.info("sending ntfy notification")
    req = urllib.request.Request(
        "https://ntfy.sh/15B17BCC55AFA3967CCCEB82",
        data=message.encode(),
    )
    urllib.request.urlopen(req)
    log.info("ntfy notification sent")


def post_bsky(client: Client, r: CandidateSpending, filing_id: str) -> None:
    log.info("posting to bluesky")
    race = format_race(r.candidate_office, r.candidate_state, r.candidate_district)
    action = format_support_oppose(r.support_oppose_code)
    name = f"{r.candidate_first_name} {r.candidate_last_name}"
    party = PARTY_NAMES.get(r.party_affiliation, r.party_affiliation) if r.party_affiliation else None
    if party:
        line = f"{r.filer_name} spent ${r.total_spent:,.2f} {action} {party} {name} in {race}"
    else:
        line = f"{r.filer_name} spent ${r.total_spent:,.2f} {action} {name} in {race}"
    url = filing_url(r.filer_committee_id_number, filing_id)
    text = client_utils.TextBuilder().text(line)
    if r.purposes:
        text = text.text(f" ({r.purposes})")
    text = text.text("\n").link(f"FEC-{filing_id}", url)
    client.send_post(text)
    log.info("posted to bluesky")


def process_queue(db: Db, output: OutputMode, bsky_client: Client | None) -> bool:
    log.debug("checking queue for pending jobs")
    job = db.claim()
    if not job:
        log.debug("no pending jobs")
        return False

    log.info("claimed job %d, filing %s", job.id, job.item_id)
    rows = db.filing_spent_per_candidate(job.item_id)
    assert rows
    log.info("filing %s has %d candidate spending rows", job.item_id, len(rows))

    for r in rows:
        msg = format_message(r, job.item_id)
        print(msg)
        if output == "ntfy":
            post_ntfy(msg)
        elif output == "bsky":
            post_bsky(bsky_client, r, job.item_id)

    log.info("finished processing job %d", job.id)
    return True


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Process F24 independent expenditure queue"
    )
    parser.add_argument("db", help="Path to SQLite database")
    parser.add_argument(
        "--output",
        choices=["stdout", "ntfy", "bsky"],
        default="stdout",
        help="Where to send notifications (default: stdout)",
    )
    args = parser.parse_args()

    handler = logging.StreamHandler()
    handler.setFormatter(logging.Formatter("%(asctime)s %(levelname)s %(message)s"))
    logging.root.handlers = [handler]
    logging.root.setLevel(logging.DEBUG)

    db = Db(args.db)
    log.info("connected to %s, output=%s", args.db, args.output)

    bsky_client: Client | None = None
    if args.output == "bsky":
        log.info("logging in to bluesky")
        bsky_client = Client()
        bsky_client.login(os.environ["BSKY_IDENTIFIER"], os.environ["BSKY_PASSWORD"])
        log.info("bluesky login successful")

    while True:
        had_work = process_queue(db, args.output, bsky_client)
        if not had_work:
            time.sleep(5)


if __name__ == "__main__":
    main()
