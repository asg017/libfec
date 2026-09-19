# Tests for libfec_parser

This directory contains pytest-based tests for the `libfec_parser` Python package.

## Structure

- `test_parser.py` - Tests for the `libfec_parser.parser` module
  - Tests for `fec_header()` function
  - Tests for `Header`, `Cover`, `Row` and the eager `Filing` class
  - Tests for the `FecError`/`FecParseError`/`MissingMappingError` hierarchy

- `test_reader.py` - Tests for `open()` and the `FilingReader` it returns
  - Accepted sources (paths, bytes-like objects, binary file objects) and rejected ones
  - `rows(*prefixes)` filtering, `close()`/context manager, `id`, `fec_version`
  - Threading: the GIL released during a pull, one reader shared across threads

- `test_pandas.py` - `pd.DataFrame(read(p).rows)` gets typed (`float64`/`date`) columns

- `test_perf.py` - `@pytest.mark.slow` tests for the Phase 2 and Phase 3 done-when numbers
  against the gitignored 91 MB filing: peak RSS and GIL-released thread progress for the
  native API, plus `fecfile.iter_file` peak RSS and a `fecfile` vs. real `fecfile` differential
  over all 408,162 items (the fixture-scale differential lives in
  `test_fecfile_differential.py`; this is the same comparison at scale)

- `test_fecfile.py` - Tests for the `libfec_parser.fecfile` module
  - Tests for `loads()`, `from_file()`, `iter_file()`, `iter_lines()` functions
  - Tests for `from_http()`/`iter_http()` functions, mocked with `httpx2.MockTransport`
  - Tests for `parse_header()` function
  - Tests for `parse_line()` function
  - Tests for `print_example()` function
  - Integration tests

- `test_fecfile_differential.py` - `libfec_parser.fecfile` against the real `fecfile` package,
  fixture by fixture: every value compared by key, order, value *and* type. Skipped (via
  `pytest.importorskip`) if the real `fecfile` package isn't installed. Four sections beyond the
  per-fixture comparison: the whole mapping space (every form in real's `mappings.json` x twelve
  versions, names and order), the scope boundary (a pre-8.0 filing raises where real parses),
  time zones (`zoneinfo` against `pytz`, in and out of the range where they agree), and malformed
  input (the handful of places the two packages treat a broken filing differently). The
  two-entry allowlist is documented in the module itself and in the README's
  [`fecfile` API § Where it differs](../README.md#where-it-differs); everything else is compared
  with nothing excluded.

- `test_fixtures.py` - Smoke tests over every file in `fixtures/`, through both APIs

- `conftest.py` - Shared pytest configuration and fixtures (the only place test
  fixtures are defined)

- `fixtures/` - Five small, committed `.fec` filings (see [Test Data](#test-data))

## Running Tests

Make sure you have pytest installed:

```bash
# Using uv (recommended)
uv pip install pytest

# Or using pip
pip install pytest
```

`uv sync --group dev` (the one-time setup in the crate's [README](../README.md#development))
installs everything the suite needs, including two deps that exist only to test *against*:
the real `fecfile` package (for `test_fecfile_differential.py` and the differential half of
`test_perf.py`) and `httpx2` (for `from_http`/`iter_http`, mocked with `httpx2.MockTransport`
in `test_fecfile.py`, and for the `[http]` extra's own dependency). Neither is a runtime
dependency of `libfec_parser` itself. Tests that need one of them and don't find it installed
skip via `pytest.importorskip`, rather than failing.

Run all tests:

```bash
# From the crates/fec-py directory
pytest tests/

# Or with verbose output
pytest -v tests/

# Run specific test file
pytest tests/test_parser.py

# Run specific test class
pytest tests/test_parser.py::TestFiling

# Run specific test
pytest tests/test_parser.py::TestFiling::test_filing_from_path_string
```

## Markers

Two markers are registered in `conftest.py`, and both are **off by default** —
`pytest_collection_modifyitems` skips them unless you select them with `-m`:

| Marker | Meaning | Run it with |
|---|---|---|
| `network` | Hits `docquery.fec.gov` | `pytest -m network tests/` |
| `slow` | Parses the 91 MB `benchmarks/1805248.fec` | `pytest -m slow tests/` |

So a plain `pytest tests/` runs every offline test and skips nothing else. Use
`-rs` to see the skip reasons:

```bash
pytest tests/ -rs        # the only skip should be the one `network` test
```

Run with coverage:
```bash
pytest --cov=libfec_parser --cov-report=html tests/
```

## Test Data

All test data lives in `tests/fixtures/` and is **committed to the repository**
(~271 KB total). The root `.gitignore` ignores `*.fec`, with a negation rule
`!crates/fec-py/tests/fixtures/*.fec` so these five stay tracked.

| File | Bytes | What it is |
|---|---|---|
| `1921705.fec` | 4,022 | **Primary.** v8.5, F3N, filer `C00900860` "Jason Byors for Congress"; 20 rows (1 SA11AI, 13 SA11C, 1 SA11D, 5 SB17) |
| `1721696.fec` | 263,105 | v8.4, F3XN, filer `C00016683` "PFIZER INC. PAC"; 1,387 rows (1,354 SA11AI, 24 SB23, 9 SB29) |
| `1913562.fec` | 1,019 | v8.4, F1A with 3 `F1S` rows: a non-`S` itemization key, `report_id` present in the HDR |
| `1913493.fec` | 1,609 | v8.4, F99 with a `[BEGINTEXT]` block and U+FFFD replacement characters; zero rows |
| `1923816.fec` | 810 | v8.5, F1N with 3 `F1S` rows |

Fixtures defined in `conftest.py`:

| Fixture | Gives you |
|---|---|
| `sample_fec_file` | `Path` to `1921705.fec` |
| `sample_fec_bytes` | its raw `bytes` |
| `sample_fec_content` | its decoded `str` (`parse_header`/`parse_line` take text) |
| `pac_fec_file` | `Path` to `1721696.fec` (many rows) |
| `f99_fec_file` | `Path` to `1913493.fec` (zero rows) |
| `all_fixture_files` | sorted `list[Path]` of every fixture |
| `fec_fixture` | parametrized: the test runs once per fixture file |
| `benchmark_fec_file` | `Path` to the gitignored 91 MB filing; use with `@pytest.mark.slow` |

A **missing fixture is a hard failure, not a skip** — `_fixture()` asserts. Tests
that silently skip are tests that never run in CI.

To add a filing, drop it in `fixtures/`, add its expected values to `EXPECTED`
in `test_fixtures.py`, and `git add` it (no `-f` needed).

## Adding New Tests

When adding new functionality to `libfec_parser`:

1. Add tests to the appropriate `test_*.py` file
2. Use descriptive test names starting with `test_`
3. Add docstrings to explain what each test does
4. Use fixtures from `conftest.py` for common setup — never re-define fixtures
   in a test module, and never `pytest.skip()` for missing test data
5. Assert exact values against the fixtures rather than guarding with
   `if len(...) > 0:` — the data is committed, so the numbers are knowable
6. Add markers for slow or network-dependent tests

Example:
```python
@pytest.mark.slow
def test_large_file_parsing(benchmark_fec_file):
    """Test parsing a very large FEC file"""
    # Test implementation
    pass
```

## Test Coverage

Current coverage includes:
- ✅ Native `open()`/`FilingReader` streaming API: sources, filtering, threading (`test_reader.py`)
- ✅ Native `Header`, `Cover`, `Row` and eager `Filing`/`read()` (`test_parser.py`)
- ✅ pandas interop: typed `float64`/`date` columns from `Filing.rows` (`test_pandas.py`)
- ✅ Fecfile module (fecfile compatibility layer), including `from_http`/`iter_http` over a
  mocked `httpx2` (`test_fecfile.py`)
- ✅ `fecfile` drop-in claim (FEC 8.0–8.5): exact match against the real package on every fixture
  (`test_fecfile_differential.py`) and over a 408,162-item, 91 MB filing
  (`test_perf.py::test_compat_differential_benchmark_filing`), plus column names across the whole
  mapping space, the scope boundary, time zones and malformed input
- ✅ `FecError`/`FecParseError`/`MissingMappingError` hierarchy and edge cases
- ✅ Integration tests
- ✅ Every committed fixture, through both APIs (`test_fixtures.py`)
- ✅ Phase 2 and Phase 3 done-when perf numbers, opt-in via `-m slow` (`test_perf.py`)

## CI/CD

These tests run in CI from the built wheel — see
[`.github/workflows/test-python.yml`](../../../.github/workflows/test-python.yml).
The `network` and `slow` markers are not selected there, so CI runs the offline
suite only.
