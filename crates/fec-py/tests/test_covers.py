"""`libfec.covers`: the typed cover classes behind `cover_data`.

The classes are `fec-parser`'s own Rust structs, so the main check is a
differential one: for every cover fixture `fec-parser`'s tests snapshot as JSON
(`crates/fec-parser/tests/snapshots/covers__*.snap`), the Python object's
`to_dict()` must equal that snapshot field for field, and its class must be the
snapshot's variant.  Anything the binding drops, renames or retypes shows up here.
"""

import dataclasses
import json
import re
import subprocess
import sys
from datetime import date
from pathlib import Path
from typing import Any

import pytest

import libfec as fec
from libfec import covers
from libfec.covers import (
    Address,
    CoverData,
    DetailedSummaryRow,
    Form1,
    Form3X,
    Form99,
    PersonName,
)

FEC_PY = Path(__file__).resolve().parents[1]
PARSER_TESTS = FEC_PY.parent / "fec-parser" / "tests"
FIXTURES = {
    "cover": PARSER_TESTS / "fixtures" / "covers",
    "legacy_cover": PARSER_TESTS / "fixtures" / "legacy",
}


@dataclasses.dataclass
class Snapshot:
    fixture: Path
    form: str
    data: dict[str, Any]


def snapshots() -> list[Snapshot]:
    """Every `cover("…")`/`legacy_cover("…")` JSON snapshot of `tests/covers.rs`."""
    found = []
    for snap in sorted((PARSER_TESTS / "snapshots").glob("covers__*.snap")):
        _, header, body = snap.read_text(encoding="utf-8").split("---\n", 2)
        m = re.search(r'^expression: "(cover|legacy_cover)\(\\"(.+?)\\"\)"$', header, re.M)
        if m is None:
            continue  # a TUI render snapshot, not a cover
        cover = json.loads(body)
        found.append(Snapshot(FIXTURES[m.group(1)] / m.group(2), cover["form"], cover["data"]))
    return found


SNAPSHOTS = snapshots()


def as_json(value: Any) -> Any:
    """A `to_dict()` value the way `serde_json` writes it (dates as ISO strings)."""
    if isinstance(value, dict):
        return {k: as_json(v) for k, v in value.items()}
    if isinstance(value, list):
        return [as_json(v) for v in value]
    if isinstance(value, date):
        return value.isoformat()
    return value


def test_every_cover_snapshot_is_found():
    # 37 electronic + 8 legacy/paper at the time of writing; guard against the
    # regex silently matching nothing.
    assert len(SNAPSHOTS) >= 40


@pytest.mark.parametrize("snap", SNAPSHOTS, ids=lambda s: s.fixture.name)
def test_cover_data_matches_fec_parser_snapshot(snap: Snapshot):
    with fec.open(snap.fixture) as filing:
        cover = filing.cover_data
    assert type(cover).__name__ == snap.form
    assert cover is not None
    got = as_json(cover.to_dict())
    assert list(got) == list(snap.data)  # same fields, same order
    assert got == snap.data


def test_cover_data_is_the_same_object_every_time():
    with fec.open(FIXTURES["cover"] / "F3XN_1926068.fec") as filing:
        assert filing.cover_data is filing.cover_data


def test_filing_carries_cover_data():
    filing = fec.read(FIXTURES["cover"] / "F1N_1906351.fec")
    assert isinstance(filing.cover_data, Form1)


def test_typed_fields():
    with fec.open(FIXTURES["cover"] / "F3XN_1926068.fec") as filing:
        cover = filing.cover_data
    # Narrowing with `match` (and `isinstance`) is what the `CoverData` union is
    # for; mypy checks every attribute access below against the generated stubs.
    match cover:
        case Form3X() as f3x:
            receipts: DetailedSummaryRow = f3x.summary.line6c_total_receipts
            total: float = receipts.column_a + receipts.column_b
            assert total > 0
        case _:
            pytest.fail(f"expected Form3X, got {cover!r}")


def test_typed_field_values():
    with fec.open(FIXTURES["cover"] / "F3XN_1926068.fec") as filing:
        cover = filing.cover_data
    assert isinstance(cover, Form3X)
    assert cover.form_type == "F3XN"
    assert cover.filer_committee_id == "C00016899"
    assert cover.coverage_from_date == date(2025, 10, 1)
    assert isinstance(cover.address, Address)
    assert cover.address.state == "OH"
    assert isinstance(cover.treasurer, PersonName)
    assert cover.treasurer.last_name == "Frost-Brooks"
    row = cover.summary.line6c_total_receipts
    assert isinstance(row, DetailedSummaryRow)
    assert isinstance(row.column_a, float)
    assert (row.column_a, row.column_b) == (445072.35, 3484593.19)
    assert cover.is_amendment() is False
    assert cover.report_code_label() == "November Monthly"


def test_form99_text():
    fixture = next(s.fixture for s in SNAPSHOTS if s.form == "Form99")
    with fec.open(fixture) as filing:
        cover = filing.cover_data
    assert isinstance(cover, Form99)
    assert cover.text is None or isinstance(cover.text, str)


def test_frozen():
    with fec.open(FIXTURES["cover"] / "F3XN_1926068.fec") as filing:
        cover = filing.cover_data
    assert isinstance(cover, Form3X)
    with pytest.raises(AttributeError):
        cover.form_type = "F3XA"  # type: ignore[misc]
    with pytest.raises(TypeError):
        Form3X()  # type: ignore[call-arg]


def test_eq_and_repr():
    path = FIXTURES["cover"] / "F3XN_1926068.fec"
    with fec.open(path) as a, fec.open(path) as b:
        assert a.cover_data == b.cover_data
        assert a.cover_data is not b.cover_data
        assert a.cover_data != a.cover_data.treasurer  # type: ignore[union-attr]
        text = repr(a.cover_data)
    assert text.startswith("Form3X(form_type='F3XN', filer_committee_id='C00016899', ")
    assert "summary=Form3XSummary(...)" in text


def test_eq_compares_amounts_as_floats():
    """`==` is field-wise Python equality: NaN never equal, infinities by sign."""
    raw = (FIXTURES["cover"] / "F3XN_1926068.fec").read_bytes()
    assert b"1976082.12" in raw  # Line 6(b), cash on hand at the beginning of the period

    def cover(amount: bytes) -> Any:
        with fec.open(raw.replace(b"1976082.12", amount)) as filing:
            return filing.cover_data

    assert cover(b"inf") == cover(b"inf")
    assert cover(b"inf") != cover(b"-inf")
    assert cover(b"NaN") != cover(b"inf")
    assert cover(b"NaN") != cover(b"NaN")


def test_cover_data_union_is_every_form_class():
    forms = {cls.__name__ for cls in CoverData.__args__}
    assert forms == {s.form for s in SNAPSHOTS}
    assert all(cls.__module__ == "libfec.covers" for cls in CoverData.__args__)
    assert all(getattr(covers, name).__name__ == name for name in forms)


def test_stubs_are_generated_from_the_rust_source():
    script = FEC_PY / "scripts" / "gen_cover_stubs.py"
    result = subprocess.run([sys.executable, str(script), "--check"], capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
