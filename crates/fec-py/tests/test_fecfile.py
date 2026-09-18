"""
Tests for libfec_parser.fecfile module
"""
import io
import urllib.error
import urllib.request

import pytest
from libfec_parser.fecfile import (
    loads,
    from_file,
    from_http,
    parse_header,
    parse_line,
    print_example
)

# The fixtures (`sample_fec_file`, `sample_fec_content`, `sample_fec_bytes`, …)
# live in conftest.py. The primary one is tests/fixtures/1921705.fec:
# v8.5, F3N, filer C00900860, 15 Schedule A + 5 Schedule B rows.


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
        """Test that itemizations is a dictionary, keyed by schedule"""
        result = loads(sample_fec_content)
        itemizations = result['itemizations']

        assert isinstance(itemizations, dict)
        assert {k: len(v) for k, v in itemizations.items()} == {
            'Schedule A': 15,
            'Schedule B': 5,
        }

    def test_loads_known_values(self, sample_fec_content):
        """Test the parsed values for the known fixture 1921705.fec"""
        result = loads(sample_fec_content)

        assert result['header']['fec_version'] == '8.5'
        assert result['filing']['form_type'] == 'F3N'
        assert result['filing']['filer_committee_id_number'] == 'C00900860'
        assert result['text'] == []

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
    """Tests for from_http function.

    The Rust side resolves ``urlopen`` at call time via
    ``py.import("urllib.request").getattr("urlopen")`` (src/fecfile.rs), so
    monkeypatching ``urllib.request.urlopen`` is enough to intercept the
    download.
    """

    def test_from_http_parses_downloaded_bytes(self, monkeypatch, sample_fec_bytes):
        """Test from_http() parses whatever urlopen() hands back"""
        urls = []

        def fake_urlopen(url):  # fecfile.rs calls urlopen(url) with one positional arg
            urls.append(url)
            return io.BytesIO(sample_fec_bytes)

        monkeypatch.setattr(urllib.request, "urlopen", fake_urlopen)
        result = from_http(1921705)

        assert urls == ["https://docquery.fec.gov/dcdev/posted/1921705.fec"]
        assert result["header"]["fec_version"] == "8.5"
        assert result["filing"]["filer_committee_id_number"] == "C00900860"
        assert len(result["itemizations"]["Schedule A"]) == 15

    def test_from_http_accepts_string_file_number(self, monkeypatch, sample_fec_bytes):
        """Test from_http() accepts a string file number"""
        urls = []

        def fake_urlopen(url):
            urls.append(url)
            return io.BytesIO(sample_fec_bytes)

        monkeypatch.setattr(urllib.request, "urlopen", fake_urlopen)
        result = from_http("1921705", options={'filter_itemizations': ['SA']})

        assert urls == ["https://docquery.fec.gov/dcdev/posted/1921705.fec"]
        assert set(result["itemizations"]) == {"Schedule A"}

    def test_from_http_falls_back_then_returns_none(self, monkeypatch):
        """Test from_http() tries the paper URL, then returns None"""
        urls = []

        def failing(url):
            urls.append(url)
            raise urllib.error.URLError("nope")

        monkeypatch.setattr(urllib.request, "urlopen", failing)

        assert from_http(1) is None
        assert urls == [
            "https://docquery.fec.gov/dcdev/posted/1.fec",
            "https://docquery.fec.gov/paper/posted/1.fec",
        ]

    @pytest.mark.network
    def test_from_http_live(self):
        """Test from_http() against the real docquery.fec.gov (opt in: -m network)"""
        result = from_http(1921705)

        assert result is not None
        assert result["filing"]["filer_committee_id_number"] == "C00900860"


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
        # Get header first to extract version
        _, version, _ = parse_header(lines[0])

        # Parse second line (cover record)
        result = parse_line(lines[1], version)

        assert isinstance(result, dict)
        assert result['form_type'] == 'F3N'
        assert result['filer_committee_id_number'] == 'C00900860'

    def test_parse_line_with_line_number(self, sample_fec_content):
        """Test parse_line() with line_num parameter"""
        lines = sample_fec_content.split('\n')
        _, version, _ = parse_header(lines[0])
        result = parse_line(lines[1], version, _line_num=1)

        assert isinstance(result, dict)
        assert result['form_type'] == 'F3N'

    def test_parse_line_has_form_type(self, sample_fec_content):
        """Test parse_line() result has form/record type field"""
        lines = sample_fec_content.split('\n')
        _, version, _ = parse_header(lines[0])
        result = parse_line(lines[1], version)

        # The first field should be the form type
        assert len(result) > 0
        assert 'form_type' in result

    def test_parse_line_with_empty_line(self):
        """Test parse_line() with empty line"""
        with pytest.raises(ValueError):
            parse_line("", "8.0")

    def test_parse_line_with_invalid_version(self, sample_fec_content):
        """Test parse_line() with various versions"""
        lines = sample_fec_content.split('\n')
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
