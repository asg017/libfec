"""
Tests for libfec_parser.parser module
"""
import pytest
from libfec_parser.parser import fec_header, Filing, Header, Cover, Itemization

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


class TestItemization:
    """Tests for Itemization class, against the known fixture 1921705.fec"""

    def test_itemization_attributes(self, sample_fec_file):
        """Test that Itemization has expected attributes"""
        itemization = Filing(str(sample_fec_file)).itemizations[0]

        assert isinstance(itemization, Itemization)
        assert itemization.row_type == "SA11AI"

    def test_itemization_row_type_order(self, sample_fec_file):
        """Test the exact itemization row types, in file order"""
        filing = Filing(str(sample_fec_file))

        assert len(filing.itemizations) == 20
        assert [i.row_type for i in filing.itemizations] == (
            ["SA11AI"] + ["SA11C"] * 13 + ["SA11D"] + ["SB17"] * 5
        )

    def test_itemization_repr(self, sample_fec_file):
        """Test Itemization __repr__"""
        itemization = Filing(str(sample_fec_file)).itemizations[0]

        assert repr(itemization) == "Itemization(row_type='SA11AI', 45 fields)"

    def test_itemization_len(self, sample_fec_file):
        """Test Itemization __len__"""
        itemization = Filing(str(sample_fec_file)).itemizations[0]

        assert len(itemization) == 45

    def test_itemization_getitem_positive_index(self, sample_fec_file):
        """Test Itemization __getitem__ with positive index"""
        itemization = Filing(str(sample_fec_file)).itemizations[0]

        assert itemization[0] == "SA11AI"
        assert itemization[1] == "C00900860"

    def test_itemization_getitem_negative_index(self, sample_fec_file):
        """Test Itemization __getitem__ with negative index"""
        itemization = Filing(str(sample_fec_file)).itemizations[0]

        assert itemization[-1] == itemization[len(itemization) - 1]
        assert isinstance(itemization[-1], str)

    def test_itemization_getitem_out_of_bounds(self, sample_fec_file):
        """Test Itemization __getitem__ with out of bounds index"""
        itemization = Filing(str(sample_fec_file)).itemizations[0]

        with pytest.raises(IndexError):
            _ = itemization[9999]

    def test_itemization_fields_method(self, sample_fec_file):
        """Test Itemization.fields() returns a list"""
        itemization = Filing(str(sample_fec_file)).itemizations[0]
        fields = itemization.fields()

        assert isinstance(fields, list)
        assert len(fields) == 45
        assert all(isinstance(f, str) for f in fields)
        assert fields[0] == "SA11AI"


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
        assert all(isinstance(item, Itemization) for item in itemizations)

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
        with pytest.raises(IOError):
            Filing("/path/that/does/not/exist.fec")
    
    def test_filing_with_invalid_type(self):
        """Test Filing with invalid input type"""
        with pytest.raises(TypeError):
            Filing(12345)  # type: ignore[arg-type]  # invalid type, on purpose
    
    def test_filing_with_invalid_data(self):
        """Test Filing with invalid FEC data"""
        with pytest.raises(ValueError):
            Filing(b"invalid fec data")
