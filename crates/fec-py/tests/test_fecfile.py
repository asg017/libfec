"""
Tests for libfec_parser.fecfile module

Exactness against the real `fecfile` package is `test_fecfile_differential.py`'s
job; these are the unit tests for the shapes, the options and the edges.
"""
import io
import urllib.error
import urllib.request
import zoneinfo
from datetime import datetime

import pytest
from libfec_parser import fecfile
from libfec_parser.parser import open as open_filing
from libfec_parser.fecfile import (
    FecItem,
    FecParserMissingMappingError,
    FecParserTypeWarning,
    loads,
    from_file,
    from_http,
    iter_file,
    iter_lines,
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

    def test_loads_key_order(self, sample_fec_content):
        """Test the top-level keys come back in fecfile's order"""
        assert list(loads(sample_fec_content)) == [
            'itemizations', 'text', 'header', 'filing'
        ]

    def test_loads_header_structure(self, sample_fec_content):
        """Test that header has expected structure"""
        result = loads(sample_fec_content)
        header = result['header']

        # `soft_name`/`soft_ver`, not `software_*`: these are fecfile's names.
        assert list(header) == [
            'record_type', 'ef_type', 'fec_version',
            'soft_name', 'soft_ver', 'report_id', 'report_number', 'comment',
        ]
        # A column the filing leaves empty is '', never None.
        assert header['comment'] == ''

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
        # Groups are in file order: Schedule A rows come first in this filing.
        assert list(itemizations) == ['Schedule A', 'Schedule B']

    def test_loads_known_values(self, sample_fec_content):
        """Test the parsed values for the known fixture 1921705.fec"""
        result = loads(sample_fec_content)

        assert result['header']['fec_version'] == '8.5'
        assert result['filing']['form_type'] == 'F3N'
        assert result['filing']['filer_committee_id_number'] == 'C00900860'
        assert result['text'] == []

    def test_loads_coerces_types(self, sample_fec_content):
        """Test that money is a float and dates are tz-aware US/Eastern datetimes"""
        result = loads(sample_fec_content)

        total = result['filing']['col_a_total_receipts']
        assert type(total) is float
        assert total == 720.88

        coverage = result['filing']['coverage_from_date']
        assert isinstance(coverage, datetime)
        offset = coverage.utcoffset()
        assert offset is not None
        # US/Eastern is -05:00 in winter, -04:00 in summer; either, never UTC.
        assert offset.total_seconds() in (-5 * 3600, -4 * 3600)

        row = result['itemizations']['Schedule A'][0]
        assert type(row['contribution_amount']) is float
        assert isinstance(row['contribution_date'], datetime)

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
        # The header and the filing still come back.
        assert result['filing']['filer_committee_id_number'] == 'C00900860'

    def test_loads_with_filter_specific_schedules(self, sample_fec_content):
        """Test loads() with specific schedule filter"""
        result = loads(sample_fec_content, options={'filter_itemizations': ['SA', 'SB']})

        assert set(result['itemizations']) == {'Schedule A', 'Schedule B'}

    def test_loads_with_lowercase_filter(self, sample_fec_content):
        """Test that lowercase prefixes work (real fecfile is case-sensitive)"""
        result = loads(sample_fec_content, options={'filter_itemizations': ['sb']})

        assert set(result['itemizations']) == {'Schedule B'}

    def test_loads_with_as_strings_option(self, sample_fec_content):
        """Test loads() with as_strings option"""
        result = loads(sample_fec_content, options={'as_strings': True})

        assert type(result['filing']['col_a_total_receipts']) is str
        assert result['filing']['col_a_total_receipts'] == '720.88'
        assert all(isinstance(v, str) for v in result['header'].values())

    def test_loads_with_invalid_type(self):
        """Test loads() with invalid input type"""
        with pytest.raises(TypeError):
            loads(12345)  # type: ignore[arg-type]  # invalid type, on purpose

    def test_loads_with_empty_string(self):
        """Test loads() with empty string"""
        with pytest.raises(ValueError):
            loads("")

    def test_loads_with_invalid_data(self):
        """Test loads() with invalid FEC data"""
        with pytest.raises(ValueError):
            loads("not valid fec data")


class TestOptions:
    """Tests for options validation — fecfile silently ignores what it doesn't know"""

    def test_unknown_option_names_itself_and_the_valid_keys(self, sample_fec_content):
        with pytest.raises(ValueError, match="unknown option 'filter_itemisations'"):
            loads(sample_fec_content, options={'filter_itemisations': ['SA']})

    def test_options_must_be_a_mapping(self, sample_fec_content):
        with pytest.raises(TypeError, match="options must be a dict"):
            loads(sample_fec_content, options=['SA'])  # type: ignore[arg-type]

    def test_bare_string_filter_is_rejected(self, sample_fec_content):
        """A bare 'SA' would filter on 'S' and 'A' if it were iterated"""
        with pytest.raises(TypeError, match="filter_itemizations must be a list of str"):
            loads(sample_fec_content, options={'filter_itemizations': 'SA'})

    def test_non_string_prefix_is_rejected(self, sample_fec_content):
        with pytest.raises(TypeError, match="filter_itemizations must be a list of str"):
            loads(sample_fec_content, options={'filter_itemizations': ['SA', 3]})

    def test_as_strings_must_be_a_bool(self, sample_fec_content):
        with pytest.raises(TypeError, match="as_strings must be a bool"):
            loads(sample_fec_content, options={'as_strings': 'yes'})

    def test_options_positional(self, sample_fec_content):
        """fecfile's signature takes options positionally as well"""
        result = loads(sample_fec_content, {'filter_itemizations': ['SA']})

        assert set(result['itemizations']) == {'Schedule A'}


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

        assert set(result['itemizations']) == {'Schedule A'}

    def test_from_file_accepts_a_path(self, sample_fec_file):
        """Test from_file() takes os.PathLike too, which real fecfile does not"""
        assert from_file(sample_fec_file) == from_file(str(sample_fec_file))

    def test_from_file_with_nonexistent_file(self):
        """Test from_file() with non-existent file"""
        with pytest.raises(IOError):
            from_file("/path/that/does/not/exist.fec")


class TestIterFile:
    """Tests for iter_file function"""

    def test_iter_file_yields_fec_items(self, sample_fec_file):
        """Test iter_file() yields header, summary, then one item per row"""
        items = list(iter_file(sample_fec_file))

        assert all(isinstance(item, FecItem) for item in items)
        assert [item.data_type for item in items[:2]] == ['header', 'summary']
        assert {item.data_type for item in items[2:]} == {'itemization'}
        assert len(items) == 22  # header + summary + 20 rows

    def test_iter_file_agrees_with_from_file(self, sample_fec_file):
        """Test iter_file() and from_file() parse rows identically"""
        items = list(iter_file(sample_fec_file))
        parsed = from_file(sample_fec_file)

        assert items[0].data == parsed['header']
        assert items[1].data == parsed['filing']
        assert items[2].data == parsed['itemizations']['Schedule A'][0]

    def test_iter_file_with_options(self, sample_fec_file):
        """Test iter_file() honours filter_itemizations"""
        items = list(iter_file(sample_fec_file, options={'filter_itemizations': []}))

        assert [item.data_type for item in items] == ['header', 'summary']

    def test_closing_the_generator_closes_the_reader(self, monkeypatch, sample_fec_file):
        """Test iter_file() does not leave the file open when abandoned"""
        readers = []

        def spy(source):
            reader = open_filing(source)
            readers.append(reader)
            return reader

        monkeypatch.setattr(fecfile, "_open", spy)

        generator = iter_file(sample_fec_file)
        next(generator)
        assert readers and not readers[0].closed

        generator.close()
        assert readers[0].closed


class TestIterLines:
    """Tests for iter_lines function"""

    def test_iter_lines_over_str_lines_without_newlines(self, sample_fec_content):
        """Test iter_lines() with str lines the caller has already split"""
        items = list(iter_lines(sample_fec_content.split('\n')))

        assert [item.data_type for item in items[:2]] == ['header', 'summary']
        assert items[1].data['filer_committee_id_number'] == 'C00900860'

    def test_iter_lines_over_a_generator_of_bytes(self, sample_fec_bytes):
        """Test iter_lines() with a generator of bytes lines, as an HTTP body gives"""
        def body():
            for line in sample_fec_bytes.split(b'\n'):
                yield line + b'\n'

        items = list(iter_lines(body()))

        assert [item.data_type for item in items[:2]] == ['header', 'summary']
        assert len(items) == 22

    def test_iter_lines_matches_loads(self, sample_fec_content):
        """Test iter_lines() and loads() see the same rows"""
        rows = [i.data for i in iter_lines(sample_fec_content.split('\n'))
                if i.data_type == 'itemization']

        assert rows == (
            loads(sample_fec_content)['itemizations']['Schedule A']
            + loads(sample_fec_content)['itemizations']['Schedule B']
        )

    def test_iter_lines_rejects_non_text_lines(self, sample_fec_content):
        """Test iter_lines() says what is wrong with a list of, say, ints"""
        with pytest.raises(TypeError, match="lines must be str or bytes"):
            list(iter_lines([sample_fec_content.split('\n')[0], 12345]))  # type: ignore[list-item]


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
        assert result is not None

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
        assert result is not None

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
        assert lines_consumed == 1

    def test_parse_header_dict_keys(self, sample_fec_content):
        """Test parse_header() dictionary has fecfile's HDR column names"""
        first_line = sample_fec_content.split('\n')[0]
        header_dict, version, lines_consumed = parse_header(first_line)

        assert header_dict is not None
        assert list(header_dict) == [
            'record_type', 'ef_type', 'fec_version',
            'soft_name', 'soft_ver', 'report_id', 'report_number', 'comment',
        ]

    def test_parse_header_matches_the_streaming_header(self, sample_fec_content):
        """Test parse_header() and loads() build the same header dict"""
        first_line = sample_fec_content.split('\n')[0]
        header_dict, _, _ = parse_header(first_line)

        assert header_dict == loads(sample_fec_content)['header']

    def test_parse_header_version_matches(self, sample_fec_content):
        """Test parse_header() version matches header dict"""
        first_line = sample_fec_content.split('\n')[0]
        header_dict, version, _ = parse_header(first_line)

        assert header_dict is not None
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
            parse_header(12345)  # type: ignore[arg-type]  # invalid type, on purpose

    def test_parse_header_rejects_the_v1_v2_block_header(self):
        """Test parse_header() on the multi-line '/* ... /*' header of versions 1-2

        fec-parser does not read that form at all, so there is nothing to build a
        header out of; say so rather than guessing.
        """
        with pytest.raises(FecParserMissingMappingError, match="versions 1 and 2"):
            parse_header(["/* Header", "FEC_VER_# = 2.02", "/* End Header"])


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

    def test_parse_line_coerces_types(self, sample_fec_content):
        """Test parse_line() types values the same way loads() does"""
        lines = sample_fec_content.split('\n')
        _, version, _ = parse_header(lines[0])
        result = parse_line(lines[1], version)

        assert result is not None
        assert type(result['col_a_total_receipts']) is float
        assert isinstance(result['coverage_from_date'], datetime)

    def test_parse_line_with_line_number(self, sample_fec_content):
        """Test parse_line() with line_num parameter"""
        lines = sample_fec_content.split('\n')
        _, version, _ = parse_header(lines[0])
        result = parse_line(lines[1], version, line_num=1)

        assert isinstance(result, dict)
        assert result['form_type'] == 'F3N'

    def test_parse_line_has_form_type(self, sample_fec_content):
        """Test parse_line() result has form/record type field"""
        lines = sample_fec_content.split('\n')
        _, version, _ = parse_header(lines[0])
        result = parse_line(lines[1], version)

        # The first field should be the form type
        assert result is not None
        assert len(result) > 0
        assert 'form_type' in result

    def test_parse_line_with_empty_line(self):
        """Test parse_line() returns None for a line that is not a record"""
        assert parse_line("", "8.0") is None
        assert parse_line("SA11AI", "8.0") is None

    def test_parse_line_pads_a_short_row(self, sample_fec_content):
        """Test parse_line() reads the columns a short line omits as ''"""
        lines = sample_fec_content.split('\n')
        _, version, _ = parse_header(lines[0])
        short = '\x1c'.join(lines[2].split('\x1c')[:6])
        result = parse_line(short, version)
        full = parse_line(lines[2], version)

        # Every column of the SA mapping is there, whatever the line carries.
        assert result is not None and full is not None
        assert len(result) == len(full)
        assert result['contributor_city'] == ''
        # A missing float column is typed like any other value, so it is None.
        assert result['contribution_amount'] is None

    def test_parse_line_with_unmapped_version(self, sample_fec_content):
        """Test parse_line() raises rather than inventing field_N column names"""
        lines = sample_fec_content.split('\n')

        with pytest.raises(FecParserMissingMappingError, match="version 99 of form SA11AI"):
            parse_line(lines[2], "99")

    def test_parse_line_warns_on_an_unparseable_value(self):
        """Test parse_line() warns and yields None when a typed column is garbage"""
        line = '\x1c'.join(['SA11AI'] + [''] * 19 + ['not-a-number'])

        with pytest.warns(FecParserTypeWarning, match="cannot parse value: not-a-number"):
            result = parse_line(line, "8.5")

        assert result is not None
        assert result['contribution_amount'] is None


class TestPrintExample:
    """Tests for print_example function"""

    def test_print_example_is_capturable(self, sample_fec_content, capsys):
        """Test print_example() writes to sys.stdout, so capsys sees it"""
        parsed = loads(sample_fec_content)

        print_example(parsed)

        printed = capsys.readouterr().out
        assert '"form_type": "F3N"' in printed
        # One example row per schedule, not the whole filing.
        assert printed.count('"contribution_amount"') == 1

    def test_print_example_with_minimal_data(self, sample_fec_content, capsys):
        """Test print_example() with filtered data"""
        parsed = loads(sample_fec_content, options={'filter_itemizations': []})

        print_example(parsed)

        assert '"itemizations": {}' in capsys.readouterr().out

    def test_print_example_with_missing_key(self):
        """Test print_example() with invalid dict"""
        invalid_dict: dict[str, dict[str, str]] = {'header': {}}

        with pytest.raises(KeyError):
            print_example(invalid_dict)  # type: ignore[arg-type]  # missing keys, on purpose


class TestTimezone:
    """Tests for the one platform-dependent thing in the module"""

    def test_missing_tzdata_names_the_package_to_install(self, monkeypatch, sample_fec_content):
        """Test the zoneinfo error is turned into actionable advice

        Windows and stripped containers ship no tz database; `zoneinfo` raises
        there and `pytz` (what real fecfile uses) would not, so say what to do
        rather than letting a KeyError out.
        """
        def no_tzdata(key):
            raise zoneinfo.ZoneInfoNotFoundError(key)

        monkeypatch.setattr(fecfile, "_EASTERN", None)
        monkeypatch.setattr(zoneinfo, "ZoneInfo", no_tzdata)

        with pytest.raises(ImportError, match="pip install tzdata"):
            loads(sample_fec_content)


class TestIntegration:
    """Integration tests using multiple functions together"""

    def test_load_parse_and_print(self, sample_fec_file, capsys):
        """Test loading, parsing and printing a file"""
        # Load file
        result = from_file(str(sample_fec_file))

        # Check structure
        assert 'header' in result
        assert 'filing' in result

        # Print example
        print_example(result)
        assert capsys.readouterr().out

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

        # Verify only Schedule A survived
        assert set(result['itemizations']) == {'Schedule A'}
