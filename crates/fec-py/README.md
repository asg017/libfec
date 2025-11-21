# fec-py

Python bindings for the FEC parser library.

## Installation

```bash
# Install from wheel (after building)
pip install dist/fec_py-0.1.0-cp39-abi3-macosx_11_0_arm64.whl
```

## Building

**Note:** Don't use `cargo build` directly - use `maturin` to build PyO3 extension modules.

### Development Build

For local development and testing, use `maturin develop` to build and install in-place:

```bash
cd /Users/alex/projects/libfec
maturin develop -m crates/fec-py/Cargo.toml
```

This installs the package in editable mode in your current Python environment.

### Build Wheel

To build a distributable wheel package:

```bash
cd /Users/alex/projects/libfec
maturin build -m crates/fec-py/Cargo.toml --out dist
```

For a release (optimized) build:

```bash
cd /Users/alex/projects/libfec
maturin build -m crates/fec-py/Cargo.toml --release --out dist
```

## Usage

### fecfile Compatibility API

The `fec_py.fecfile` module provides a compatibility layer that mimics the API of the [fecfile](https://pypi.org/project/fecfile/) PyPI package:

```python
from fec_py.fecfile import loads, from_file, parse_header, parse_line, print_example

# Load and parse a filing from a file
parsed = from_file("./path/to/filing.fec")

# Access parsed data
print(parsed['header']['fec_version'])
print(parsed['filing']['form_type'])
print(parsed['itemizations']['Schedule A'][0])

# Parse from string/bytes
content = open("filing.fec", "rb").read()
parsed = loads(content)

# Filter specific schedules
parsed = loads(content, options={'filter_itemizations': ['SA', 'SB']})

# Parse just the header
header, version, lines_consumed = parse_header(header_line)

# Parse a single line
line_dict = parse_line(line, version)

# Print example (first item of each type)
print_example(parsed)
```

See [demo-fecfile.py](demo-fecfile.py) for a complete demonstration.

### Native Python API

```python
from fec_py.parser import Filing

# Create a Filing from a file path
f = Filing("./path/to/filing.fec")

# Access header information
print(f.header.fec_version)      # FEC file version
print(f.header.software_name)     # Software used to create filing
print(f.header.software_version)  # Software version

# Access cover page information
print(f.cover.form_type)          # Form type (e.g., "F3P")
print(f.cover.filer_id)           # Committee/filer ID
print(f.cover.filer_name)         # Committee/filer name
print(f.cover.report_code)        # Report code (e.g., "Q1", "M10")
print(f.cover.coverage_from_date) # Coverage period start
print(f.cover.coverage_through_date) # Coverage period end

# Iterate through itemizations (schedules)
for itemization in f.itemizations:
    print(itemization.row_type)    # Row type (e.g., "SA11AI", "SB21B")
    print(itemization.fields())    # All fields as a list
    print(itemization[0])          # Access specific field by index

# Alternative: Create from bytes
with open("./path/to/filing.fec", "rb") as file:
    f = Filing(file.read())

# Alternative: Create from file-like object
import urllib.request
with urllib.request.urlopen("https://example.com/filing.fec") as response:
    f = Filing(response)
```

### Legacy Function

The `fec_header` function is also available for quick header parsing:

```python
from fec_py.parser import fec_header
from pathlib import Path

contents = Path("./filing.fec").read_bytes()
version = fec_header(contents)  # Returns FEC version string
print(version)  # e.g., "8.4"
```

## Running the Demo

The `demo.py` file demonstrates the API usage:

```bash
cd /Users/alex/projects/libfec/crates/fec-py

# Run with the built wheel using uv (recommended)
uv run --no-cache --no-project --isolated \
  --with 'fec_py @ file://../../dist/fec_py-0.1.0-cp39-abi3-macosx_11_0_arm64.whl' \
  demo.py ../../cache2/*.fec

# Or with specific files
uv run --no-cache --no-project --isolated \
  --with 'fec_py @ file://../../dist/fec_py-0.1.0-cp39-abi3-macosx_11_0_arm64.whl' \
  demo.py ../../cache2/1461586.fec ../../cache2/1478292.fec

# Or if installed locally
python demo.py path/to/filing1.fec path/to/filing2.fec
```

The demo will:
1. Parse each provided FEC file
2. Extract and print the FEC version for each file

## API Reference

### Classes

#### `Filing`

Main class for parsing FEC filing files.

**Constructor:** `Filing(source)`
- `source`: Can be a file path (str), bytes, or file-like object with a `read()` method

**Attributes:**
- `header`: `Header` object with filing header information
- `cover`: `Cover` object with cover page information
- `itemizations`: List of `Itemization` objects representing all schedule rows

#### `Header`

Contains FEC filing header information.

**Attributes:**
- `record_type`: str - Record type (always "HDR")
- `ef_type`: str - Electronic filing type
- `fec_version`: str - FEC version (e.g., "8.4", "8.5")
- `software_name`: str - Software name used to create filing
- `software_version`: str - Software version
- `report_id`: Optional[str] - Report ID if present
- `report_number`: Optional[str] - Report number if present
- `comment`: Optional[str] - Comment if present

#### `Cover`

Contains FEC filing cover page information.

**Attributes:**
- `form_type`: str - Form type (e.g., "F3P", "F3X")
- `filer_id`: str - Committee or filer ID
- `filer_name`: str - Committee or filer name
- `report_code`: Optional[str] - Report code (e.g., "Q1", "M10", "YE")
- `coverage_from_date`: Optional[str] - Coverage period start date
- `coverage_through_date`: Optional[str] - Coverage period end date

**Methods:**
- `fields()`: Returns a dictionary of all cover record fields

#### `Itemization`

Represents a single itemization (schedule) row in the filing.

**Attributes:**
- `row_type`: str - Row type identifier (e.g., "SA11AI", "SB21B")

**Methods:**
- `fields()`: Returns list of all field values
- `__len__()`: Returns number of fields
- `__getitem__(idx)`: Access field by index (supports negative indexing)

## Development

After making changes to the Rust code:

1. Rebuild the wheel:
   ```bash
   maturin build -m crates/fec-py/Cargo.toml --out dist
   ```

2. Test with the new wheel:
   ```bash
   uv run --no-cache --no-project --isolated \
     --with 'fec_py @ file://../../dist/fec_py-0.1.0-cp39-abi3-macosx_11_0_arm64.whl' \
     demo.py ../../cache2/*.fec
   ```

Note: Use `--no-cache` with `uv` to ensure it uses the newly built wheel and doesn't cache an old version.
