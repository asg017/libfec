"""
Tests for libfec_parser.parser module
"""
import errno
import pickle
from collections.abc import Mapping
from datetime import date

import pytest
from libfec_parser.parser import (
    fec_header,
    Filing,
    Header,
    Cover,
    Row,
    FecError,
    FecParseError,
    MissingMappingError,
)

# The fixtures (`sample_fec_file`, `sample_fec_bytes`, `all_fixture_files`, …)
# live in conftest.py. The primary one is tests/fixtures/1921705.fec:
# v8.5, F3N, filer C00900860 "Jason Byors for Congress", 20 itemizations
# (1 SA11AI, 13 SA11C, 1 SA11D, 5 SB17) in that file order.


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


class TestHeader:
    """Tests for Header class"""
    
    def test_header_attributes(self, sample_fec_file):
        """Test that Header has expected attributes"""
        filing = Filing(str(sample_fec_file))
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
        header = Filing(str(sample_fec_file)).header

        assert header.fec_version == "8.5"
        assert header.record_type == "HDR"
        assert header.ef_type == "FEC"
        assert header.software_name == "FECfile"

    def test_header_repr(self, sample_fec_file):
        """Test Header __repr__"""
        filing = Filing(str(sample_fec_file))
        header = filing.header
        repr_str = repr(header)
        
        assert isinstance(repr_str, str)
        assert 'Header' in repr_str
        assert header.fec_version in repr_str


class TestCover:
    """Tests for Cover class"""
    
    def test_cover_attributes(self, sample_fec_file):
        """Test that Cover has expected attributes"""
        filing = Filing(str(sample_fec_file))
        cover = filing.cover
        
        assert isinstance(cover, Cover)
        assert hasattr(cover, 'form_type')
        assert hasattr(cover, 'filer_id')
        assert hasattr(cover, 'filer_name')
        assert hasattr(cover, 'report_code')
        assert hasattr(cover, 'coverage_from_date')
        assert hasattr(cover, 'coverage_through_date')

    def test_cover_values(self, sample_fec_file):
        """Test Cover values for the known fixture 1921705.fec"""
        cover = Filing(str(sample_fec_file)).cover

        assert cover.form_type == "F3N"
        assert cover.filer_id == "C00900860"
        assert cover.filer_name == "Jason Byors for Congress"

    def test_cover_repr(self, sample_fec_file):
        """Test Cover __repr__"""
        filing = Filing(str(sample_fec_file))
        cover = filing.cover
        repr_str = repr(cover)
        
        assert isinstance(repr_str, str)
        assert 'Cover' in repr_str
        assert cover.form_type in repr_str
    
    def test_cover_fields_method(self, sample_fec_file):
        """Test Cover.fields() returns a dictionary"""
        filing = Filing(str(sample_fec_file))
        cover = filing.cover
        fields = cover.fields()
        
        assert isinstance(fields, dict)
        assert 'form_type' in fields
        assert 'filer_id' in fields
        assert 'filer_name' in fields


def _edit_first_row(raw: bytes, edit) -> bytes:
    """Return ``raw`` with ``edit`` applied to its first itemization line (line 3)."""
    lines = raw.split(b"\n")
    assert lines[2].startswith(b"SA11AI"), lines[2][:20]
    lines[2] = edit(lines[2])
    return b"\n".join(lines)


class TestRow:
    """Tests for Row, against the known fixture 1921705.fec.

    Its first itemization is line 3, an ``SA11AI`` with 45 fields:
    ``SA11AI|C00900860|SA11AI.4264|||IND||Weed|Richard||||14 Stacey St||…``
    """

    def test_attributes(self, sample_fec_file):
        """A Row exposes row_type and is a Mapping"""
        row = Filing(str(sample_fec_file)).itemizations[0]

        assert isinstance(row, Row)
        assert isinstance(row, Mapping)
        assert row.row_type == "SA11AI"

    def test_row_type_order(self, sample_fec_file):
        """Test the exact row types, in file order"""
        filing = Filing(str(sample_fec_file))

        assert len(filing.itemizations) == 20
        assert [r.row_type for r in filing.itemizations] == (
            ["SA11AI"] + ["SA11C"] * 13 + ["SA11D"] + ["SB17"] * 5
        )

    def test_getitem_by_name_is_typed(self, sample_fec_file):
        """By name: amounts are float, dates are date, text stays str (`""` if empty)"""
        row = Filing(str(sample_fec_file)).itemizations[0]

        assert row["contribution_amount"] == 500.0
        assert isinstance(row["contribution_amount"], float)
        assert row["contribution_date"] == date(2025, 7, 7)
        assert row["contributor_last_name"] == "Weed"
        assert row["contributor_middle_name"] == ""
        assert row["contribution_purpose_descrip"] == ""

    def test_getitem_unknown_name_is_key_error(self, sample_fec_file):
        """An unmapped column name raises KeyError"""
        row = Filing(str(sample_fec_file)).itemizations[0]

        with pytest.raises(KeyError):
            _ = row["not_a_column"]

    def test_getitem_by_position_is_raw(self, sample_fec_file):
        """By position: the raw field, negative indexes allowed"""
        row = Filing(str(sample_fec_file)).itemizations[0]

        assert row[0] == "SA11AI"
        assert row[1] == "C00900860"
        assert row[20] == "500.00"
        assert row[-1] == row[len(row.fields()) - 1]

    def test_getitem_out_of_bounds(self, sample_fec_file):
        """An out-of-range position raises IndexError"""
        row = Filing(str(sample_fec_file)).itemizations[0]

        with pytest.raises(IndexError):
            _ = row[9999]

    def test_getitem_bad_key_type(self, sample_fec_file):
        """A key that is neither str, int nor slice raises TypeError"""
        row = Filing(str(sample_fec_file)).itemizations[0]

        with pytest.raises(TypeError):
            _ = row[object()]  # type: ignore[call-overload]  # invalid key, on purpose

    def test_empty_amount_is_none(self, sample_fec_file):
        """An empty amount column reads as None, not `""` or 0.0"""
        row = [
            r for r in Filing(str(sample_fec_file)).itemizations if r.row_type == "SB17"
        ][0]

        assert row.fields()[21] == ""
        assert row["semi_annual_refunded_bundled_amt"] is None

    def test_garbage_amount_is_raw_str(self, sample_fec_bytes):
        """An amount that does not parse comes back as the raw string"""
        raw = _edit_first_row(
            sample_fec_bytes, lambda line: line.replace(b"\x1c500.00\x1c", b"\x1cN/A\x1c", 1)
        )
        row = Filing(raw).itemizations[0]

        assert row["contribution_amount"] == "N/A"
        assert row["contribution_aggregate"] == 500.0

    def test_garbage_date_is_raw_str(self, sample_fec_bytes):
        """A date that does not parse comes back as the raw string"""
        raw = _edit_first_row(
            sample_fec_bytes, lambda line: line.replace(b"\x1c20250707\x1c", b"\x1cnotadate\x1c", 1)
        )
        row = Filing(raw).itemizations[0]

        assert row["contribution_date"] == "notadate"

    def test_slice(self, sample_fec_file):
        """A slice yields raw fields as a list"""
        row = Filing(str(sample_fec_file)).itemizations[0]

        assert row[0:3] == ["SA11AI", "C00900860", "SA11AI.4264"]
        assert row[:2] == ["SA11AI", "C00900860"]
        assert row[-2:] == row.fields()[-2:]
        assert row[0:6:2] == ["SA11AI", "SA11AI.4264", ""]

    def test_len_is_column_count(self, sample_fec_file):
        """len(row) counts columns, not raw fields"""
        row = Filing(str(sample_fec_file)).itemizations[0]

        assert len(row) == 45
        assert len(row) == len(row.keys())

    def test_iter_yields_names(self, sample_fec_file):
        """Iterating a row yields column names, in column order"""
        row = Filing(str(sample_fec_file)).itemizations[0]

        assert list(row) == row.keys()
        assert list(row)[:3] == [
            "form_type",
            "filer_committee_id_number",
            "transaction_id",
        ]

    def test_values_and_items(self, sample_fec_file):
        """values() and items() are lists of typed values"""
        row = Filing(str(sample_fec_file)).itemizations[0]

        assert isinstance(row.values(), list)
        assert isinstance(row.items(), list)
        assert row.items() == list(zip(row.keys(), row.values()))
        assert row.values()[20] == 500.0

    def test_dict_roundtrip(self, sample_fec_file):
        """dict(row) maps every column name to its typed value"""
        row = Filing(str(sample_fec_file)).itemizations[0]
        as_dict = dict(row)

        assert len(as_dict) == len(row)
        assert as_dict["contributor_state"] == "MA"
        assert as_dict["contribution_amount"] == 500.0

    def test_contains(self, sample_fec_file):
        """Membership is over column names; an int key is never a column"""
        row = Filing(str(sample_fec_file)).itemizations[0]

        assert "contribution_amount" in row
        assert "not_a_column" not in row
        assert 0 not in row

    def test_get_default(self, sample_fec_file):
        """get() behaves like dict.get"""
        row = Filing(str(sample_fec_file)).itemizations[0]

        assert row.get("contribution_amount") == 500.0
        assert row.get("not_a_column") is None
        assert row.get("not_a_column", "fallback") == "fallback"

    def test_fields_method(self, sample_fec_file):
        """fields() returns every raw field, in file order"""
        row = Filing(str(sample_fec_file)).itemizations[0]
        fields = row.fields()

        assert isinstance(fields, list)
        assert len(fields) == 45
        assert all(isinstance(f, str) for f in fields)
        assert fields[0] == "SA11AI"

    def test_extra_fields_trailing_empty_ignored(self, sample_fec_bytes):
        """A stray trailing delimiter is not data"""
        raw = _edit_first_row(sample_fec_bytes, lambda line: line + b"\x1c")
        row = Filing(raw).itemizations[0]

        assert len(row.fields()) == 46
        assert row.extra_fields == []
        assert len(row) == 45

    def test_extra_fields_non_empty_kept(self, sample_fec_bytes):
        """A non-empty extra field is kept, positionally, and never in keys()"""
        raw = _edit_first_row(sample_fec_bytes, lambda line: line + b"\x1cEXTRA")
        row = Filing(raw).itemizations[0]

        assert row.extra_fields == ["EXTRA"]
        assert row[45] == "EXTRA"
        assert "EXTRA" not in row.keys()
        assert len(row) == 45

    def test_short_row_missing_is_none(self, sample_fec_bytes):
        """Columns past the end of a short row read as None"""
        raw = _edit_first_row(
            sample_fec_bytes, lambda line: b"\x1c".join(line.split(b"\x1c")[:11])
        )
        row = Filing(raw).itemizations[0]

        assert len(row.fields()) == 11
        assert len(row) == 45
        assert row["contributor_prefix"] == ""  # column 10, the last one present
        assert row["contributor_suffix"] is None  # column 11, past the end
        assert row["contribution_amount"] is None
        assert row["contributor_first_name"] == "Richard"

    def test_eq_hash(self, sample_fec_file):
        """Rows compare and hash by (row_type, version, raw fields)"""
        rows = Filing(str(sample_fec_file)).itemizations

        assert rows[0] == rows[0]
        assert rows[0] != rows[1]
        assert rows[0] != "not a row"
        assert len({rows[0], rows[0]}) == 1
        assert hash(rows[0]) == hash(Filing(str(sample_fec_file)).itemizations[0])

    def test_pickle_roundtrip(self, sample_fec_file):
        """A Row survives pickling, line number included"""
        row = Filing(str(sample_fec_file)).itemizations[0]
        restored = pickle.loads(pickle.dumps(row))

        assert restored == row
        assert restored.line == row.line
        assert restored["contribution_amount"] == 500.0

    def test_repr(self, sample_fec_file):
        """Test Row __repr__"""
        row = Filing(str(sample_fec_file)).itemizations[0]

        assert repr(row) == "Row(row_type='SA11AI', line=3, 45 fields)"

    def test_line(self, sample_fec_file):
        """The first itemization is on line 3; the rest follow one per line"""
        rows = Filing(str(sample_fec_file)).itemizations

        assert rows[0].line == 3
        assert [r.line for r in rows] == list(range(3, 23))

    def test_keys_are_interned(self, sample_fec_file):
        """Column names are one object per (row_type, version), not per row"""
        rows = Filing(str(sample_fec_file)).itemizations

        assert rows[0].keys()[0] is rows[1].keys()[0]
        assert rows[0].keys()[7] is rows[13].keys()[7]

    def test_unicode_replacement_row_has_line(self, fec_fixture):
        """Every row of every fixture knows its line, even after lossy decoding"""
        for row in Filing(str(fec_fixture)).itemizations:
            assert row.line >= 3


class TestFiling:
    """Tests for Filing class"""
    
    def test_filing_from_path_string(self, sample_fec_file):
        """Test Filing initialization with file path string"""
        filing = Filing(str(sample_fec_file))
        
        assert isinstance(filing, Filing)
        assert isinstance(filing.header, Header)
        assert isinstance(filing.cover, Cover)
        assert isinstance(filing.itemizations, list)
    
    def test_filing_from_bytes(self, sample_fec_bytes):
        """Test Filing initialization with bytes"""
        filing = Filing(sample_fec_bytes)
        
        assert isinstance(filing, Filing)
        assert isinstance(filing.header, Header)
        assert isinstance(filing.cover, Cover)
    
    def test_filing_from_file_object(self, sample_fec_file):
        """Test Filing initialization with file-like object"""
        with open(sample_fec_file, 'rb') as f:
            filing = Filing(f)
            
            assert isinstance(filing, Filing)
            assert isinstance(filing.header, Header)
            assert isinstance(filing.cover, Cover)
    
    def test_filing_repr(self, sample_fec_file):
        """Test Filing __repr__"""
        filing = Filing(str(sample_fec_file))

        assert repr(filing) == (
            "Filing(form_type='F3N', filer_id='C00900860', 20 itemizations)"
        )

    def test_filing_header_property(self, sample_fec_file):
        """Test Filing.header property"""
        filing = Filing(str(sample_fec_file))
        header = filing.header
        
        assert isinstance(header, Header)
        assert header.fec_version
    
    def test_filing_cover_property(self, sample_fec_file):
        """Test Filing.cover property"""
        filing = Filing(str(sample_fec_file))
        cover = filing.cover
        
        assert isinstance(cover, Cover)
        assert cover.form_type
        assert cover.filer_id
    
    def test_filing_itemizations_property(self, sample_fec_file):
        """Test Filing.itemizations property"""
        filing = Filing(str(sample_fec_file))
        itemizations = filing.itemizations

        assert isinstance(itemizations, list)
        assert len(itemizations) == 20
        assert all(isinstance(item, Row) for item in itemizations)

    def test_filing_many_itemizations(self, pac_fec_file):
        """Test a filing with many rows: 1721696.fec, v8.4 F3XN, 1,387 rows"""
        filing = Filing(str(pac_fec_file))

        assert filing.header.fec_version == "8.4"
        assert filing.cover.form_type == "F3XN"
        assert filing.cover.filer_id == "C00016683"
        assert len(filing.itemizations) == 1387

    def test_filing_with_no_itemizations(self, f99_fec_file):
        """Test the F99 fixture: a [BEGINTEXT] filing with zero rows"""
        filing = Filing(str(f99_fec_file))

        assert filing.cover.form_type == "F99"
        assert filing.itemizations == []

    def test_filing_with_invalid_path(self):
        """Test Filing with non-existent file path"""
        with pytest.raises(FileNotFoundError):
            Filing("/path/that/does/not/exist.fec")

    def test_filing_with_invalid_type(self):
        """Test Filing with invalid input type"""
        with pytest.raises(TypeError):
            Filing(12345)  # type: ignore[arg-type]  # invalid type, on purpose

    def test_filing_with_invalid_data(self):
        """Test Filing with invalid FEC data"""
        with pytest.raises(ValueError):
            Filing(b"invalid fec data")


class TestErrors:
    """Tests for the FecError/FecParseError/MissingMappingError hierarchy"""

    def test_missing_file_is_file_not_found(self, tmp_path):
        missing = tmp_path / "nope.fec"
        with pytest.raises(FileNotFoundError) as ei:
            Filing(str(missing))
        assert ei.value.errno == errno.ENOENT
        assert ei.value.filename == str(missing)

    def test_garbage_is_parse_error(self):
        with pytest.raises(FecParseError):
            Filing(b"invalid fec data")

    def test_hierarchy(self):
        assert issubclass(FecParseError, FecError) and issubclass(FecError, ValueError)
        assert issubclass(MissingMappingError, FecError)
        assert FecError.__module__ == "libfec_parser.parser"

    def test_missing_mapping_error_attributes(self):
        e = MissingMappingError("ZZZ", "8.4", 7)
        assert (e.row_type, e.version, e.line) == ("ZZZ", "8.4", 7)
        assert "ZZZ" in str(e) and "8.4" in str(e)
