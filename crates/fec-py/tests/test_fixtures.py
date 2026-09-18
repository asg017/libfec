"""
Smoke tests over every committed fixture, through both public APIs.

``fec_fixture`` is parametrized in conftest.py over ``tests/fixtures/*.fec``,
so dropping a new filing into that directory automatically extends this suite.
"""
from pathlib import Path

from libfec_parser import fecfile
from libfec_parser.parser import open

# name -> (fec_version, form_type, filer_id, itemization row count)
EXPECTED = {
    "1721696.fec": ("8.4", "F3XN", "C00016683", 1387),
    "1913493.fec": ("8.4", "F99", "C00917203", 0),  # [BEGINTEXT], zero rows
    "1913562.fec": ("8.4", "F1A", "C00677898", 3),
    "1921705.fec": ("8.5", "F3N", "C00900860", 20),
    "1923816.fec": ("8.5", "F1N", "C00925099", 3),
}


def test_all_fixtures_present(all_fixture_files):
    """All five fixtures are committed and discovered"""
    assert [p.name for p in all_fixture_files] == sorted(EXPECTED)


def test_parser_api_parses_fixture(fec_fixture: Path):
    """open() parses every fixture with the documented shape"""
    version, form_type, filer_id, n_rows = EXPECTED[fec_fixture.name]
    reader = open(fec_fixture)

    assert reader.header.fec_version == version
    assert reader.cover.form_type == form_type
    assert reader.cover.filer_id == filer_id
    assert reader.id == fec_fixture.stem
    assert sum(1 for _ in reader) == n_rows


def test_fecfile_api_parses_fixture(fec_fixture: Path):
    """fecfile.from_file() parses every fixture with the documented shape"""
    version, form_type, filer_id, n_rows = EXPECTED[fec_fixture.name]
    result = fecfile.from_file(str(fec_fixture))

    assert set(result) == {"header", "filing", "itemizations", "text"}
    assert result["header"]["fec_version"] == version
    assert result["filing"]["form_type"] == form_type
    assert result["filing"]["filer_committee_id_number"] == filer_id
    assert sum(len(v) for v in result["itemizations"].values()) == n_rows


def test_both_apis_agree_on_row_count(fec_fixture: Path):
    """The parser and fecfile layers see the same number of itemizations"""
    n_rows = sum(1 for _ in open(fec_fixture))
    result = fecfile.from_file(str(fec_fixture))

    assert n_rows == sum(len(v) for v in result["itemizations"].values())


def test_bytes_and_path_agree(fec_fixture: Path):
    """open(bytes) and open(path) read the same filing (`id` aside)"""
    from_path = open(fec_fixture)
    from_bytes = open(fec_fixture.read_bytes())

    assert from_path.cover.fields() == from_bytes.cover.fields()
    assert from_path.cover_row.fields() == from_bytes.cover_row.fields()
    assert [r.fields() for r in from_path] == [r.fields() for r in from_bytes]
