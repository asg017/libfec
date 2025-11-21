"""
Tests for libfec_parser.parser module
"""
import pytest
from pathlib import Path
from libfec_parser.parser import fec_header, Filing, Header, Cover, Itemization


@pytest.fixture
def sample_fec_file():
    """Get path to a sample FEC file"""
    # Look for a sample file in the cache or benchmarks directory
    cache_dir = Path(__file__).parent.parent.parent.parent / "cache"
    if cache_dir.exists():
        fec_files = list(cache_dir.glob("*.fec"))
        if fec_files:
            return fec_files[0]
    
    # Try benchmarks
    bench_dir = Path(__file__).parent.parent.parent.parent / "benchmarks"
    if bench_dir.exists():
        fec_files = list(bench_dir.glob("*.fec"))
        if fec_files:
            return fec_files[0]
    
    pytest.skip("No sample FEC files found")


@pytest.fixture
def sample_fec_bytes(sample_fec_file):
    """Read sample FEC file as bytes"""
    return sample_fec_file.read_bytes()


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
    """Tests for Itemization class"""
    
    def test_itemization_attributes(self, sample_fec_file):
        """Test that Itemization has expected attributes"""
        filing = Filing(str(sample_fec_file))
        
        if len(filing.itemizations) > 0:
            itemization = filing.itemizations[0]
            assert isinstance(itemization, Itemization)
            assert hasattr(itemization, 'row_type')
            assert isinstance(itemization.row_type, str)
    
    def test_itemization_repr(self, sample_fec_file):
        """Test Itemization __repr__"""
        filing = Filing(str(sample_fec_file))
        
        if len(filing.itemizations) > 0:
            itemization = filing.itemizations[0]
            repr_str = repr(itemization)
            
            assert isinstance(repr_str, str)
            assert 'Itemization' in repr_str
            assert itemization.row_type in repr_str
    
    def test_itemization_len(self, sample_fec_file):
        """Test Itemization __len__"""
        filing = Filing(str(sample_fec_file))
        
        if len(filing.itemizations) > 0:
            itemization = filing.itemizations[0]
            length = len(itemization)
            
            assert isinstance(length, int)
            assert length >= 0
    
    def test_itemization_getitem_positive_index(self, sample_fec_file):
        """Test Itemization __getitem__ with positive index"""
        filing = Filing(str(sample_fec_file))
        
        if len(filing.itemizations) > 0:
            itemization = filing.itemizations[0]
            if len(itemization) > 0:
                field = itemization[0]
                assert isinstance(field, str)
    
    def test_itemization_getitem_negative_index(self, sample_fec_file):
        """Test Itemization __getitem__ with negative index"""
        filing = Filing(str(sample_fec_file))
        
        if len(filing.itemizations) > 0:
            itemization = filing.itemizations[0]
            if len(itemization) > 0:
                field = itemization[-1]
                assert isinstance(field, str)
    
    def test_itemization_getitem_out_of_bounds(self, sample_fec_file):
        """Test Itemization __getitem__ with out of bounds index"""
        filing = Filing(str(sample_fec_file))
        
        if len(filing.itemizations) > 0:
            itemization = filing.itemizations[0]
            with pytest.raises(IndexError):
                _ = itemization[9999]
    
    def test_itemization_fields_method(self, sample_fec_file):
        """Test Itemization.fields() returns a list"""
        filing = Filing(str(sample_fec_file))
        
        if len(filing.itemizations) > 0:
            itemization = filing.itemizations[0]
            fields = itemization.fields()
            
            assert isinstance(fields, list)
            assert all(isinstance(f, str) for f in fields)


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
        repr_str = repr(filing)
        
        assert isinstance(repr_str, str)
        assert 'Filing' in repr_str
        assert filing.cover.form_type in repr_str
    
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
        assert all(isinstance(item, Itemization) for item in itemizations)
    
    def test_filing_with_invalid_path(self):
        """Test Filing with non-existent file path"""
        with pytest.raises(IOError):
            Filing("/path/that/does/not/exist.fec")
    
    def test_filing_with_invalid_type(self):
        """Test Filing with invalid input type"""
        with pytest.raises(TypeError):
            Filing(12345)  # Invalid type
    
    def test_filing_with_invalid_data(self):
        """Test Filing with invalid FEC data"""
        with pytest.raises(ValueError):
            Filing(b"invalid fec data")
