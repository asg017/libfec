"""Writes the synthetic .fec fixtures in this directory (FS-delimited, v8.4).

    python3 make_fixtures.py

Column layouts come from crates/fec-parser-macros/src/mappings2.json. Every
text column defaults to "<PREFIX>.<column>", so each value is distinct and a
swapped pair of columns (or fields) fails a test; dates and amounts are set
explicitly, also all distinct. The tests in ../ assert these values.
"""

import json
import re
from pathlib import Path

HERE = Path(__file__).parent
MAPPINGS = json.loads(
    (HERE / "../../../../fec-parser-macros/src/mappings2.json").read_text()
)
FS = "\x1c"
VERSION = "8.4"


def columns(row_type):
    for pattern, versions in MAPPINGS.items():
        if re.match(pattern, row_type.lower()):
            for vpattern, cols in versions.items():
                if re.match(vpattern, VERSION):
                    return cols
    raise KeyError(row_type)


def row(row_type, prefix, values=None, upto=None, extra=()):
    """One FS-delimited line: text columns default to f"{prefix}.{column}"."""
    values = values or {}
    cols = columns(row_type)
    fields = [values.get(c, f"{prefix}.{c}") for c in cols]
    fields[0] = values.get(cols[0], row_type)
    if upto is not None:
        fields = fields[:upto]
    return FS.join(list(fields) + list(extra))


HDR = FS.join(
    ["HDR", "FEC", VERSION, "SoftName", "1.2.3", "FEC-1111", "7", "Header comment"]
)


def f3x_cover(prefix, overrides=None):
    cols = columns("F3XN")
    values = {
        "filer_committee_id_number": "C00424242",
        "committee_name": "Distinct PAC",
        "change_of_address": "X",
        "report_code": "Q1",
        "date_of_election": "20241105",
        "coverage_from_date": "20240101",
        "coverage_through_date": "20240331",
        "qualified_committee": "",
        "date_signed": "20240415",
        "col_b_year": "2024",
        # Non-finite: the typed `f64` reads it as 0.0, never `badarg`.
        "col_a_debts_to": "inf",
    }
    # Every summary amount distinct: 1000.5, 1001.5, ...
    n = 0
    for c in cols:
        if c.startswith(("col_a_", "col_b_")) and c not in values:
            values[c] = f"{1000 + n}.5"
            n += 1
    values.update(overrides or {})
    return row("F3XN", prefix, values)


def write(name, lines):
    # LF, not CRLF: fec-parser reports FS-delimited CRLF rows one line early
    # (row 3 as line 2), a pre-existing parser quirk the line tests would trip.
    (HERE / name).write_bytes(("\n".join(lines) + "\n").encode())


# The round-trip fixture: header, F3XN cover, one row each of SA, SB, SE and
# TEXT with distinct values, then value-rule edge cases.
write(
    "FEC-424242.fec",
    [
        HDR,
        f3x_cover("F3X"),
        # line 3
        row(
            "SA11AI",
            "SA",
            {
                "filer_committee_id_number": "C00424242",
                "entity_type": "IND",
                "contribution_date": "20240115",
                "contribution_amount": "250.75",
                "contribution_aggregate": "1000.25",
                "memo_code": "X",
            },
            extra=["EXTRA1", "EXTRA2"],
        ),
        # line 4
        row(
            "SB23",
            "SB",
            {
                "filer_committee_id_number": "C00424242",
                "entity_type": "CCM",
                "expenditure_date": "20240220",
                "expenditure_amount": "99.25",
                "semi_annual_refunded_bundled_amt": "",
                "memo_code": "",
            },
        ),
        # line 5
        row(
            "SE",
            "SE",
            {
                "filer_committee_id_number": "C00424242",
                "entity_type": "ORG",
                "dissemination_date": "20240305",
                "disbursement_date": "20240306",
                "expenditure_amount": "5000.5",
                "calendar_y_t_d_per_election_office": "12000.25",
                "support_oppose_code": "S",
                "date_signed": "20240307",
                "memo_code": "",
            },
        ),
        # line 6
        row("TEXT", "TX", {"text": "Hello text"}),
        # line 7: garbage date and amount stay Text(raw); blank -> Empty;
        # text "" -> Empty, text "  " -> Text("  ")
        row(
            "SA11AI",
            "SA2",
            {
                "contribution_date": "2024-01-15",
                "contribution_amount": " 12abc ",
                "contribution_aggregate": "   ",
                "contributor_street_1": "  ",
                "contributor_street_2": "",
            },
        ),
        # line 8: short row (21 fields), padded date and amount parse
        row(
            "SB23",
            "SB2",
            {"expenditure_date": " 20240221 ", "expenditure_amount": " 42 "},
            upto=21,
        ),
        # line 9-10: non-finite floats are Text(raw); typed f64 0.0 / None
        row(
            "SA11AI",
            "SA3",
            {"contribution_amount": "nan", "contribution_aggregate": "inf"},
        ),
        row(
            "SA11AI",
            "SA4",
            {"contribution_amount": "1e999", "contribution_aggregate": "-inf"},
        ),
    ],
)

# G11: a row type with no mapping between mapped rows.
write(
    "unknown_rows.fec",
    [
        HDR,
        f3x_cover("U"),
        row("SA11AI", "U1", {"contribution_amount": "1"}),  # line 3
        row("SA11AI", "U2", {"contribution_amount": "2"}),  # line 4
        FS.join(["ZZZ9", "C00424242", "mystery"]),  # line 5
        row("SA11AI", "U3", {"contribution_amount": "3"}),  # line 6
    ],
)

# Form 9 line items (no real fixture has F91-F94).
write(
    "form9_items.fec",
    [
        HDR,
        row(
            "F9N",
            "F9",
            {
                "filer_committee_id_number": "C90009999",
                "coverage_from_date": "20240101",
                "coverage_through_date": "20240131",
                "total_donations": "10",
                "total_disbursements": "20",
            },
        ),
        row("F91", "F91", {"filer_committee_id_number": "C90009999"}),
        row(
            "F92",
            "F92",
            {
                "filer_committee_id_number": "C90009999",
                "contribution_date": "20240102",
                "contribution_amount": "11",
            },
        ),
        row(
            "F93",
            "F93",
            {
                "filer_committee_id_number": "C90009999",
                "expenditure_date": "20240103",
                "expenditure_amount": "12",
                "communication_date": "20240104",
            },
        ),
        row("F94", "F94", {"filer_committee_id_number": "C90009999"}),
    ],
)

write(
    "unsupported_version.fec",
    [FS.join(["HDR", "FEC", "9.9", "SoftName", "1.2.3"]), "F3XN" + FS + "C00424242"],
)
write("not_fec.fec", ["hello, world", "not a filing"])
