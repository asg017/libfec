"""`libfec_parser.itemizations`: the typed itemization classes behind `Row.itemization`.

Like `test_covers.py`, the main check is differential: for every itemization
fixture `fec-parser`'s tests snapshot as JSON
(`crates/fec-parser/tests/snapshots/itemizations__*.snap`), each row's
`Row.itemization.to_dict()` must equal that snapshot's entry field for field,
and it must be one of the module's classes.
"""

import json
import pickle
import re
import subprocess
import sys
from datetime import date
from pathlib import Path
from typing import Any

import pytest

import libfec_parser as fec
from libfec_parser import itemizations
from libfec_parser.covers import Address, PersonName
from libfec_parser.itemizations import CandidateRef, Entity, Itemization, ScheduleA

FEC_PY = Path(__file__).resolve().parents[1]
PARSER_TESTS = FEC_PY.parent / "fec-parser" / "tests"
FIXTURES = PARSER_TESTS / "fixtures" / "itemizations"


def snapshots() -> list[tuple[Path, list[dict[str, Any] | None]]]:
    """Every `itemizations("…")` JSON snapshot of `tests/itemizations.rs`."""
    found = []
    for snap in sorted((PARSER_TESTS / "snapshots").glob("itemizations__*.snap")):
        _, header, body = snap.read_text(encoding="utf-8").split("---\n", 2)
        m = re.search(r'^expression: "itemizations\(\\"(.+?)\\"\)"$', header, re.M)
        assert m, snap
        found.append((FIXTURES / m.group(1), json.loads(body)))
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


def test_every_itemization_snapshot_is_found():
    assert len(SNAPSHOTS) >= 5


@pytest.mark.parametrize("fixture,expected", SNAPSHOTS, ids=lambda s: getattr(s, "name", ""))
def test_itemization_matches_fec_parser_snapshot(fixture: Path, expected: list[dict[str, Any] | None]):
    with fec.open(fixture) as filing:
        got = [row.itemization for row in filing]
    assert len(got) == len(expected)
    for item, want in zip(got, expected):
        if want is None:
            assert item is None
            continue
        assert item is not None
        assert type(item).__module__ == "libfec_parser.itemizations"
        want = dict(want)
        family = want.pop("family")
        data = as_json(item.to_dict())
        assert list(data) == list(want)  # same fields, same order
        assert data == want, family


def test_typed_fields():
    with fec.open(FIXTURES / "SA_1907925.fec") as filing:
        items = [row.itemization for row in filing.rows("SA")]
    match items[0]:
        case ScheduleA(memo=False) as sa:
            contributor: Entity = sa.contributor
            name: PersonName = contributor.name
            address: Address = contributor.address
            candidate: CandidateRef = sa.donor_candidate
            amount: float = sa.contribution_amount
            assert amount != 0
            assert isinstance(sa.contribution_date, date)
            assert sa.line_number() == sa.form_type[2:]
            assert contributor.display_name()
            assert isinstance(name.last_name, str) and isinstance(address.city, str | None)
            assert candidate.is_empty()
        case other:
            pytest.fail(f"expected a non-memo ScheduleA, got {other!r}")


def test_cover_row_is_not_an_itemization():
    with fec.open(FIXTURES / "SA_1907925.fec") as filing:
        assert filing.cover_row.itemization is None


def test_legacy_combined_name_is_split():
    with fec.open(FIXTURES / "SA_42174.fec") as filing:
        sa = next(iter(filing)).itemization
    assert isinstance(sa, ScheduleA)
    assert (sa.contributor.name.last_name, sa.contributor.name.first_name) == ("Amir", "Michael")


def test_pickled_row_keeps_its_itemization():
    with fec.open(FIXTURES / "SA_42174.fec") as filing:
        row = next(iter(filing))
    assert pickle.loads(pickle.dumps(row)).itemization == row.itemization


def test_classes_are_frozen():
    with fec.open(FIXTURES / "SA_1907925.fec") as filing:
        sa = next(iter(filing)).itemization
    assert isinstance(sa, ScheduleA)
    with pytest.raises(AttributeError):
        sa.contribution_amount = 0.0  # type: ignore[misc]


def test_module_exports_match_stub():
    stub = (Path(itemizations.__file__).with_suffix(".pyi")).read_text(encoding="utf-8")
    for name in itemizations.__all__:
        assert f'"{name}"' in stub, name


def test_stubs_are_current():
    script = FEC_PY / "scripts" / "gen_cover_stubs.py"
    subprocess.run([sys.executable, script, "--check"], check=True)
