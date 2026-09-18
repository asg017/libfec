# Tests for libfec_parser

This directory contains pytest-based tests for the `libfec_parser` Python package.

## Structure

- `test_parser.py` - Tests for the `libfec_parser.parser` module
  - Tests for `fec_header()` function
  - Tests for `Filing` class
  - Tests for `Header`, `Cover`, and `Itemization` classes

- `test_fecfile.py` - Tests for the `libfec_parser.fecfile` module
  - Tests for `loads()` function
  - Tests for `from_file()` function
  - Tests for `from_http()` function
  - Tests for `parse_header()` function
  - Tests for `parse_line()` function
  - Tests for `print_example()` function
  - Integration tests

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
- ✅ Parser module (Filing, Header, Cover, Itemization classes)
- ✅ Fecfile module (fecfile compatibility layer)
- ✅ Error handling and edge cases
- ✅ Integration tests
- ✅ Every committed fixture, through both APIs (`test_fixtures.py`)

## CI/CD

These tests run in CI from the built wheel — see
[`.github/workflows/test-python.yml`](../../../.github/workflows/test-python.yml).
The `network` and `slow` markers are not selected there, so CI runs the offline
suite only.
