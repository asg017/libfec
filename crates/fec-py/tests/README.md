# Tests for libfec_parser

This directory contains pytest-based tests for the `libfec_parser` Python package.

## Structure

- `test_parser.py` - Tests for the `libfec_parser.parser` module
  - Tests for `fec_header()` function
  - Tests for `Filing` class
  - Tests for `Header`, `Cover`, and `Itemization` classes

- `test_foo.py` - Tests for the `libfec_parser.foo` module
  - Tests for the example `bar()` function

- `test_fecfile.py` - Tests for the `libfec_parser.fecfile` module
  - Tests for `loads()` function
  - Tests for `from_file()` function
  - Tests for `from_http()` function
  - Tests for `parse_header()` function
  - Tests for `parse_line()` function
  - Tests for `print_example()` function
  - Integration tests

- `conftest.py` - Shared pytest configuration and fixtures

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

## Test Options

Skip slow tests:
```bash
pytest -m "not slow" tests/
```

Skip network tests:
```bash
pytest -m "not network" tests/
```

Run with coverage:
```bash
pytest --cov=libfec_parser --cov-report=html tests/
```

## Test Data

The tests look for sample FEC files in:
- `../../cache/` - Cached FEC files
- `../../benchmarks/` - Benchmark FEC files

If no sample files are found, tests that require them will be skipped.

## Adding New Tests

When adding new functionality to `libfec_parser`:

1. Add tests to the appropriate `test_*.py` file
2. Use descriptive test names starting with `test_`
3. Add docstrings to explain what each test does
4. Use fixtures from `conftest.py` for common setup
5. Add markers for slow or network-dependent tests

Example:
```python
@pytest.mark.slow
def test_large_file_parsing(first_fec_file):
    """Test parsing a very large FEC file"""
    # Test implementation
    pass
```

## Test Coverage

Current coverage includes:
- ✅ Parser module (Filing, Header, Cover, Itemization classes)
- ✅ Foo module (example module)
- ✅ Fecfile module (fecfile compatibility layer)
- ✅ Error handling and edge cases
- ✅ Integration tests

## CI/CD

These tests can be integrated into CI/CD pipelines:

```bash
# Example GitHub Actions workflow
- name: Run tests
  run: |
    uv pip install pytest pytest-cov
    pytest tests/ --cov=libfec_parser --cov-report=xml
```
