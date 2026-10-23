"""
Tests for libfec.parser module
"""
import builtins
import errno
import pickle
from collections.abc import Mapping
from datetime import date

import pytest

# `open` here is `libfec.parser.open`, not the builtin — reach the real
# one through `builtins.open`, exactly as `parser.py` itself has to.
from libfec.parser import (
    fec_header,
    Header,
    Record,
    open,
    FecError,
    FecParseError,
    MissingMappingError,
)

# The fixtures (`sample_fec_file`, `sample_fec_bytes`, `all_fixture_files`, …)
# live in conftest.py. The primary one is tests/fixtures/1921705.fec:
# v8.5, F3N, filer C00900860 "Jason Byors for Congress", 20 itemizations
# (1 SA11AI, 13 SA11C, 1 SA11D, 5 SB17) in that file order.


def records(source) -> list[Record]:
    """Every raw record of a fresh `open(source)`."""
    return list(open(source).itemization_records)


class TestFecHeader:
    """Tests for fec_header function"""
    
    def test_fec_header_returns_version(self, sample_fec_bytes):
        """Test that fec_header returns a version string"""
        version = fec_header(sample_fec_bytes)
        assert isinstance(version, str)
        assert len(version) > 0
    
    def test_fec_header_with_invalid_data(self):
        """Test fec_header with invalid data"""
        with pytest.raises(ValueError):
            fec_header(b"invalid fec data")
    
    def test_fec_header_with_empty_bytes(self):
        """Test fec_header with empty bytes"""
        with pytest.raises(ValueError):
            fec_header(b"")

    def test_fec_header_accepts_path_and_file(self, sample_fec_file, sample_fec_bytes):
        """`fec_header()` takes every source `open()` does"""
        assert fec_header(sample_fec_bytes) == "8.5"
        assert fec_header(str(sample_fec_file)) == "8.5"
        assert fec_header(sample_fec_file) == "8.5"
        with builtins.open(sample_fec_file, "rb") as f:
            assert fec_header(f) == "8.5"

    def test_fec_header_ignores_cover(self):
        """Reads only the HDR record: a filing whose cover has no mapping still
        fails `open()` but `fec_header()` reads past it."""
        hdr = b"HDR\x1cFEC\x1c8.5\x1cFECfile\x1c8.5.0.0(f33)\x1c\x1c\n"
        garbage_cover = b"ZZZZ\x1cwhatever\n"
        filing = hdr + garbage_cover

        with pytest.raises(FecParseError):
            open(filing)

        assert fec_header(filing) == "8.5"


class TestHeader:
    """Tests for Header class"""
    
    def test_header_attributes(self, sample_fec_file):
        """Test that Header has expected attributes"""
        filing = open(sample_fec_file)
        header = filing.header
        
        assert isinstance(header, Header)
        assert hasattr(header, 'record_type')
        assert hasattr(header, 'ef_type')
        assert hasattr(header, 'fec_version')
        assert hasattr(header, 'software_name')
        assert hasattr(header, 'software_version')
        assert hasattr(header, 'report_id')
        assert hasattr(header, 'report_number')
        assert hasattr(header, 'comment')

    def test_header_values(self, sample_fec_file):
        """Test Header values for the known fixture 1921705.fec"""
        header = open(sample_fec_file).header

        assert header.fec_version == "8.5"
        assert header.record_type == "HDR"
        assert header.ef_type == "FEC"
        assert header.software_name == "FECfile"

    def test_header_repr(self, sample_fec_file):
        """Test Header __repr__"""
        filing = open(sample_fec_file)
        header = filing.header
        repr_str = repr(header)
        
        assert isinstance(repr_str, str)
        assert 'Header' in repr_str
        assert header.fec_version in repr_str


def _edit_first_row(raw: bytes, edit) -> bytes:
    """Return ``raw`` with ``edit`` applied to its first itemization line (line 3)."""
    lines = raw.split(b"\n")
    assert lines[2].startswith(b"SA11AI"), lines[2][:20]
    lines[2] = edit(lines[2])
    return b"\n".join(lines)


class TestRecord:
    """Tests for Record, against the known fixture 1921705.fec.

    Its first itemization is line 3, an ``SA11AI`` with 45 fields:
    ``SA11AI|C00900860|SA11AI.4264|||IND||Weed|Richard||||14 Stacey St||…``
    """

    def test_attributes(self, sample_fec_file):
        """A Record exposes row_type and is a Mapping"""
        row = records(sample_fec_file)[0]

        assert isinstance(row, Record)
        assert isinstance(row, Mapping)
        assert row.row_type == "SA11AI"

    def test_row_type_order(self, sample_fec_file):
        """Test the exact row types, in file order"""
        rows = records(sample_fec_file)

        assert len(rows) == 20
        assert [r.row_type for r in rows] == (
            ["SA11AI"] + ["SA11C"] * 13 + ["SA11D"] + ["SB17"] * 5
        )

    def test_getitem_by_name_is_typed(self, sample_fec_file):
        """By name: amounts are float, dates are date, text stays str (`""` if empty)"""
        row = records(sample_fec_file)[0]

        assert row["contribution_amount"] == 500.0
        assert isinstance(row["contribution_amount"], float)
        assert row["contribution_date"] == date(2025, 7, 7)
        assert row["contributor_last_name"] == "Weed"
        assert row["contributor_middle_name"] == ""
        assert row["contribution_purpose_descrip"] == ""

    def test_getitem_unknown_name_is_key_error(self, sample_fec_file):
        """An unmapped column name raises KeyError"""
        row = records(sample_fec_file)[0]

        with pytest.raises(KeyError):
            _ = row["not_a_column"]

    def test_getitem_by_position_is_raw(self, sample_fec_file):
        """By position: the raw field, negative indexes allowed"""
        row = records(sample_fec_file)[0]

        assert row[0] == "SA11AI"
        assert row[1] == "C00900860"
        assert row[20] == "500.00"
        assert row[-1] == row[len(row.fields()) - 1]

    def test_getitem_out_of_bounds(self, sample_fec_file):
        """An out-of-range position raises IndexError"""
        row = records(sample_fec_file)[0]

        with pytest.raises(IndexError):
            _ = row[9999]

    def test_getitem_bad_key_type(self, sample_fec_file):
        """A key that is neither str, int nor slice raises TypeError"""
        row = records(sample_fec_file)[0]

        with pytest.raises(TypeError):
            _ = row[object()]  # type: ignore[call-overload]  # invalid key, on purpose

    def test_empty_amount_is_none(self, sample_fec_file):
        """An empty amount column reads as None, not `""` or 0.0"""
        row = next(r for r in records(sample_fec_file) if r.row_type == "SB17")

        assert row.fields()[21] == ""
        assert row["semi_annual_refunded_bundled_amt"] is None

    def test_garbage_amount_is_raw_str(self, sample_fec_bytes):
        """An amount that does not parse comes back as the raw string"""
        raw = _edit_first_row(
            sample_fec_bytes, lambda line: line.replace(b"\x1c500.00\x1c", b"\x1cN/A\x1c", 1)
        )
        row = records(raw)[0]

        assert row["contribution_amount"] == "N/A"
        assert row["contribution_aggregate"] == 500.0

    def test_garbage_date_is_raw_str(self, sample_fec_bytes):
        """A date that does not parse comes back as the raw string"""
        raw = _edit_first_row(
            sample_fec_bytes, lambda line: line.replace(b"\x1c20250707\x1c", b"\x1cnotadate\x1c", 1)
        )
        row = records(raw)[0]

        assert row["contribution_date"] == "notadate"

    def test_slice(self, sample_fec_file):
        """A slice yields raw fields as a list"""
        row = records(sample_fec_file)[0]

        assert row[0:3] == ["SA11AI", "C00900860", "SA11AI.4264"]
        assert row[:2] == ["SA11AI", "C00900860"]
        assert row[-2:] == row.fields()[-2:]
        assert row[0:6:2] == ["SA11AI", "SA11AI.4264", ""]

    def test_len_is_column_count(self, sample_fec_file):
        """len(row) counts columns, not raw fields"""
        row = records(sample_fec_file)[0]

        assert len(row) == 45
        assert len(row) == len(row.keys())

    def test_iter_yields_names(self, sample_fec_file):
        """Iterating a row yields column names, in column order"""
        row = records(sample_fec_file)[0]

        assert list(row) == row.keys()
        assert list(row)[:3] == [
            "form_type",
            "filer_committee_id_number",
            "transaction_id",
        ]

    def test_values_and_items(self, sample_fec_file):
        """values() and items() are lists of typed values"""
        row = records(sample_fec_file)[0]

        assert isinstance(row.values(), list)
        assert isinstance(row.items(), list)
        assert row.items() == list(zip(row.keys(), row.values()))
        assert row.values()[20] == 500.0

    def test_dict_roundtrip(self, sample_fec_file):
        """dict(row) maps every column name to its typed value"""
        row = records(sample_fec_file)[0]
        as_dict = dict(row)

        assert len(as_dict) == len(row)
        assert as_dict["contributor_state"] == "MA"
        assert as_dict["contribution_amount"] == 500.0

    def test_contains(self, sample_fec_file):
        """Membership is over column names; an int key is never a column"""
        row = records(sample_fec_file)[0]

        assert "contribution_amount" in row
        assert "not_a_column" not in row
        assert 0 not in row

    def test_get_default(self, sample_fec_file):
        """get() behaves like dict.get"""
        row = records(sample_fec_file)[0]

        assert row.get("contribution_amount") == 500.0
        assert row.get("not_a_column") is None
        assert row.get("not_a_column", "fallback") == "fallback"

    def test_fields_method(self, sample_fec_file):
        """fields() returns every raw field, in file order"""
        row = records(sample_fec_file)[0]
        fields = row.fields()

        assert isinstance(fields, list)
        assert len(fields) == 45
        assert all(isinstance(f, str) for f in fields)
        assert fields[0] == "SA11AI"

    def test_extra_fields_trailing_empty_ignored(self, sample_fec_bytes):
        """A stray trailing delimiter is not data"""
        raw = _edit_first_row(sample_fec_bytes, lambda line: line + b"\x1c")
        row = records(raw)[0]

        assert len(row.fields()) == 46
        assert row.extra_fields == []
        assert len(row) == 45

    def test_extra_fields_non_empty_kept(self, sample_fec_bytes):
        """A non-empty extra field is kept, positionally, and never in keys()"""
        raw = _edit_first_row(sample_fec_bytes, lambda line: line + b"\x1cEXTRA")
        row = records(raw)[0]

        assert row.extra_fields == ["EXTRA"]
        assert row[45] == "EXTRA"
        assert "EXTRA" not in row.keys()
        assert len(row) == 45

    def test_short_row_missing_is_none(self, sample_fec_bytes):
        """Columns past the end of a short row read as None"""
        raw = _edit_first_row(
            sample_fec_bytes, lambda line: b"\x1c".join(line.split(b"\x1c")[:11])
        )
        row = records(raw)[0]

        assert len(row.fields()) == 11
        assert len(row) == 45
        assert row["contributor_prefix"] == ""  # column 10, the last one present
        assert row["contributor_suffix"] is None  # column 11, past the end
        assert row["contribution_amount"] is None
        assert row["contributor_first_name"] == "Richard"

    def test_eq_hash(self, sample_fec_file):
        """Records compare and hash by (row_type, version, raw fields)"""
        rows = records(sample_fec_file)

        assert rows[0] == rows[0]
        assert rows[0] != rows[1]
        assert rows[0] != "not a row"
        assert len({rows[0], rows[0]}) == 1
        assert hash(rows[0]) == hash(records(sample_fec_file)[0])

    def test_pickle_roundtrip(self, sample_fec_file):
        """A Record survives pickling, line number included"""
        row = records(sample_fec_file)[0]
        restored = pickle.loads(pickle.dumps(row))

        assert restored == row
        assert restored.line == row.line
        assert restored["contribution_amount"] == 500.0

    def test_repr(self, sample_fec_file):
        """Test Record __repr__"""
        row = records(sample_fec_file)[0]

        assert repr(row) == "Record(row_type='SA11AI', line=3, 45 fields)"

    def test_line(self, sample_fec_file):
        """The first itemization is on line 3; the rest follow one per line"""
        rows = records(sample_fec_file)

        assert rows[0].line == 3
        assert [r.line for r in rows] == list(range(3, 23))

    def test_keys_are_interned(self, sample_fec_file):
        """Column names are one object per (row_type, version), not per row"""
        rows = records(sample_fec_file)

        assert rows[0].keys()[0] is rows[1].keys()[0]
        assert rows[0].keys()[7] is rows[13].keys()[7]

    def test_unicode_replacement_row_has_line(self, fec_fixture):
        """Every row of every fixture knows its line, even after lossy decoding"""
        for row in open(fec_fixture).itemization_records:
            assert row.line >= 3


class TestErrors:
    """Tests for the FecError/FecParseError/MissingMappingError hierarchy"""

    def test_missing_file_is_file_not_found(self, tmp_path):
        missing = tmp_path / "nope.fec"
        with pytest.raises(FileNotFoundError) as ei:
            open(str(missing))
        assert ei.value.errno == errno.ENOENT
        assert ei.value.filename == str(missing)

    def test_invalid_source_type_is_type_error(self):
        with pytest.raises(TypeError):
            open(12345)  # type: ignore[arg-type]  # invalid type, on purpose

    def test_garbage_is_parse_error(self):
        with pytest.raises(FecParseError):
            open(b"invalid fec data")

    def test_hierarchy(self):
        assert issubclass(FecParseError, FecError) and issubclass(FecError, ValueError)
        assert issubclass(MissingMappingError, FecError)
        assert FecError.__module__ == "libfec.parser"

    def test_missing_mapping_error_attributes(self):
        e = MissingMappingError("ZZZ", "8.4", 7)
        assert (e.row_type, e.version, e.line) == ("ZZZ", "8.4", 7)
        assert "ZZZ" in str(e) and "8.4" in str(e)
