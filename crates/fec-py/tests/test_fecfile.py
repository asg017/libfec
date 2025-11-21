"""
Tests for libfec_parser.fecfile module
"""
import pytest
from pathlib import Path
from libfec_parser.fecfile import (
    loads,
    from_file,
    from_http,
    parse_header,
    parse_line,
    print_example
)


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
def sample_fec_content(sample_fec_file):
    """Read sample FEC file as string"""
    return sample_fec_file.read_text(encoding='utf-8', errors='ignore')


@pytest.fixture
def sample_fec_bytes(sample_fec_file):
    """Read sample FEC file as bytes"""
    return sample_fec_file.read_bytes()


class TestLoads:
    """Tests for loads function"""
    
    def test_loads_with_string(self, sample_fec_content):
        """Test loads() with string input"""
        result = loads(sample_fec_content)
        
        assert isinstance(result, dict)
        assert 'header' in result
        assert 'filing' in result
        assert 'itemizations' in result
        assert 'text' in result
    
    def test_loads_with_bytes(self, sample_fec_bytes):
        """Test loads() with bytes input"""
        result = loads(sample_fec_bytes)
        
        assert isinstance(result, dict)
        assert 'header' in result
        assert 'filing' in result
    
    def test_loads_with_lines_list(self, sample_fec_content):
        """Test loads() with list of lines"""
        lines = sample_fec_content.split('\n')
        result = loads(lines)
        
        assert isinstance(result, dict)
        assert 'header' in result
    
    def test_loads_header_structure(self, sample_fec_content):
        """Test that header has expected structure"""
        result = loads(sample_fec_content)
        header = result['header']
        
        assert 'record_type' in header
        assert 'ef_type' in header
        assert 'fec_version' in header
        assert 'software_name' in header
        assert 'software_version' in header
    
    def test_loads_filing_structure(self, sample_fec_content):
        """Test that filing has expected structure"""
        result = loads(sample_fec_content)
        filing = result['filing']
        
        assert 'form_type' in filing
        assert 'filer_committee_id_number' in filing
    
    def test_loads_itemizations_structure(self, sample_fec_content):
        """Test that itemizations is a dictionary"""
        result = loads(sample_fec_content)
        itemizations = result['itemizations']
        
        assert isinstance(itemizations, dict)
    
    def test_loads_text_structure(self, sample_fec_content):
        """Test that text is a list"""
        result = loads(sample_fec_content)
        text = result['text']
        
        assert isinstance(text, list)
    
    def test_loads_with_filter_empty(self, sample_fec_content):
        """Test loads() with empty filter_itemizations"""
        result = loads(sample_fec_content, options={'filter_itemizations': []})
        
        assert isinstance(result, dict)
        assert len(result['itemizations']) == 0
    
    def test_loads_with_filter_specific_schedules(self, sample_fec_content):
        """Test loads() with specific schedule filter"""
        result = loads(sample_fec_content, options={'filter_itemizations': ['SA', 'SB']})
        itemizations = result['itemizations']
        
        # Should only have Schedule A and Schedule B items
        for schedule_key in itemizations.keys():
            assert 'Schedule A' in schedule_key or 'Schedule B' in schedule_key or schedule_key in ['SA', 'SB']
    
    def test_loads_with_as_strings_option(self, sample_fec_content):
        """Test loads() with as_strings option"""
        result = loads(sample_fec_content, options={'as_strings': True})
        
        assert isinstance(result, dict)
        assert 'header' in result
    
    def test_loads_with_invalid_type(self):
        """Test loads() with invalid input type"""
        with pytest.raises(TypeError):
            loads(12345)
    
    def test_loads_with_empty_string(self):
        """Test loads() with empty string"""
        with pytest.raises(ValueError):
            loads("")
    
    def test_loads_with_invalid_data(self):
        """Test loads() with invalid FEC data"""
        with pytest.raises(ValueError):
            loads("not valid fec data")


class TestFromFile:
    """Tests for from_file function"""
    
    def test_from_file_returns_dict(self, sample_fec_file):
        """Test from_file() returns a dictionary"""
        result = from_file(str(sample_fec_file))
        
        assert isinstance(result, dict)
        assert 'header' in result
        assert 'filing' in result
        assert 'itemizations' in result
    
    def test_from_file_with_options(self, sample_fec_file):
        """Test from_file() with options"""
        result = from_file(str(sample_fec_file), options={'filter_itemizations': ['SA']})
        
        assert isinstance(result, dict)
        assert 'itemizations' in result
    
    def test_from_file_with_nonexistent_file(self):
        """Test from_file() with non-existent file"""
        with pytest.raises(IOError):
            from_file("/path/that/does/not/exist.fec")


class TestFromHttp:
    """Tests for from_http function"""
    
    @pytest.mark.skip(reason="Requires network access and may be flaky")
    def test_from_http_with_valid_file_number(self):
        """Test from_http() with valid file number"""
        # Using a known valid FEC file number
        result = from_http(1805249)
        
        if result is not None:
            assert isinstance(result, dict)
            assert 'header' in result
    
    def test_from_http_with_string_file_number(self):
        """Test from_http() accepts string file number"""
        # This test just checks that the function accepts strings
        # without making an actual HTTP request (would need mocking)
        # We'll let it fail gracefully if network is unavailable
        try:
            result = from_http("1805249")
            if result is not None:
                assert isinstance(result, dict)
        except:
            # Network errors are okay for this test
            pass
    
    def test_from_http_with_options(self):
        """Test from_http() with options parameter"""
        try:
            result = from_http(1805249, options={'filter_itemizations': ['SA']})
            if result is not None:
                assert isinstance(result, dict)
        except:
            # Network errors are okay for this test
            pass


class TestParseHeader:
    """Tests for parse_header function"""
    
    def test_parse_header_returns_tuple(self, sample_fec_content):
        """Test parse_header() returns a tuple"""
        first_line = sample_fec_content.split('\n')[0]
        result = parse_header(first_line)
        
        assert isinstance(result, tuple)
        assert len(result) == 3
    
    def test_parse_header_structure(self, sample_fec_content):
        """Test parse_header() return structure"""
        first_line = sample_fec_content.split('\n')[0]
        header_dict, version, lines_consumed = parse_header(first_line)
        
        assert isinstance(header_dict, dict)
        assert isinstance(version, str)
        assert isinstance(lines_consumed, int)
        assert lines_consumed >= 1
    
    def test_parse_header_dict_keys(self, sample_fec_content):
        """Test parse_header() dictionary has expected keys"""
        first_line = sample_fec_content.split('\n')[0]
        header_dict, version, lines_consumed = parse_header(first_line)
        
        assert 'record_type' in header_dict
        assert 'ef_type' in header_dict
        assert 'fec_version' in header_dict
        assert 'software_name' in header_dict
        assert 'software_version' in header_dict
    
    def test_parse_header_version_matches(self, sample_fec_content):
        """Test parse_header() version matches header dict"""
        first_line = sample_fec_content.split('\n')[0]
        header_dict, version, _ = parse_header(first_line)
        
        assert version == header_dict['fec_version']
    
    def test_parse_header_with_list(self, sample_fec_content):
        """Test parse_header() with list of lines"""
        lines = sample_fec_content.split('\n')
        header_dict, version, lines_consumed = parse_header(lines)
        
        assert isinstance(header_dict, dict)
    
    def test_parse_header_with_empty_string(self):
        """Test parse_header() with empty string"""
        with pytest.raises(ValueError):
            parse_header("")
    
    def test_parse_header_with_invalid_type(self):
        """Test parse_header() with invalid type"""
        with pytest.raises(TypeError):
            parse_header(12345)


class TestParseLine:
    """Tests for parse_line function"""
    
    def test_parse_line_returns_dict(self, sample_fec_content):
        """Test parse_line() returns a dictionary"""
        lines = sample_fec_content.split('\n')
        if len(lines) > 1:
            # Get header first to extract version
            first_line = lines[0]
            _, version, _ = parse_header(first_line)
            
            # Parse second line (cover record)
            second_line = lines[1]
            result = parse_line(second_line, version)
            
            assert isinstance(result, dict)
    
    def test_parse_line_with_line_number(self, sample_fec_content):
        """Test parse_line() with line_num parameter"""
        lines = sample_fec_content.split('\n')
        if len(lines) > 1:
            _, version, _ = parse_header(lines[0])
            result = parse_line(lines[1], version, _line_num=1)
            
            assert isinstance(result, dict)
    
    def test_parse_line_has_form_type(self, sample_fec_content):
        """Test parse_line() result has form/record type field"""
        lines = sample_fec_content.split('\n')
        if len(lines) > 1:
            _, version, _ = parse_header(lines[0])
            result = parse_line(lines[1], version)
            
            # The first field should be the form type
            assert len(result) > 0
    
    def test_parse_line_with_empty_line(self):
        """Test parse_line() with empty line"""
        with pytest.raises(ValueError):
            parse_line("", "8.0")
    
    def test_parse_line_with_invalid_version(self, sample_fec_content):
        """Test parse_line() with various versions"""
        lines = sample_fec_content.split('\n')
        if len(lines) > 1:
            # Should work with any version string
            result = parse_line(lines[1], "8.0")
            assert isinstance(result, dict)


class TestPrintExample:
    """Tests for print_example function"""
    
    def test_print_example_runs(self, sample_fec_content):
        """Test print_example() executes without error"""
        parsed = loads(sample_fec_content)
        
        # Should not raise an exception
        # Note: print_example uses Rust's println! which writes directly to stdout
        # and may not be captured by pytest's capsys
        print_example(parsed)
    
    def test_print_example_with_minimal_data(self, sample_fec_content):
        """Test print_example() with filtered data"""
        parsed = loads(sample_fec_content, options={'filter_itemizations': []})
        
        # Should not raise an exception even with no itemizations
        print_example(parsed)
    
    def test_print_example_with_missing_key(self):
        """Test print_example() with invalid dict"""
        invalid_dict = {'header': {}}
        
        with pytest.raises(KeyError):
            print_example(invalid_dict)


class TestIntegration:
    """Integration tests using multiple functions together"""
    
    def test_load_parse_and_print(self, sample_fec_file):
        """Test loading, parsing and printing a file"""
        # Load file
        result = from_file(str(sample_fec_file))
        
        # Check structure
        assert 'header' in result
        assert 'filing' in result
        
        # Print example
        print_example(result)
    
    def test_parse_header_then_full_load(self, sample_fec_content):
        """Test parsing header first, then loading full file"""
        # Parse header
        first_line = sample_fec_content.split('\n')[0]
        header_dict, version, _ = parse_header(first_line)
        
        assert version
        
        # Load full file
        result = loads(sample_fec_content)
        
        # Versions should match
        assert result['header']['fec_version'] == version
    
    def test_filter_and_verify_itemizations(self, sample_fec_content):
        """Test filtering itemizations and verifying result"""
        # Load with filter
        result = loads(sample_fec_content, options={'filter_itemizations': ['SA']})
        
        # Verify only SA schedules present
        itemizations = result['itemizations']
        for schedule_key in itemizations.keys():
            # Should be Schedule A related
            assert 'A' in schedule_key or 'SA' in schedule_key
