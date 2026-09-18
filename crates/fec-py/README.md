# libfec_parser

Python bindings for [libfec](https://github.com/asg017/libfec)'s `.fec` parser. Parse FEC electronic filings from a path, from bytes, or straight from the FEC's website, with the parsing done in Rust.

```python
from libfec_parser import fecfile

filing = fecfile.from_file("1721696.fec")

filing["filing"]["committee_name"]          # 'PFIZER INC. PAC'
filing["filing"]["col_a_total_receipts"]    # '83741.93'

for row in filing["itemizations"]["Schedule A"]:
    print(row["contributor_last_name"], row["contribution_amount"])
```

Two APIs are included:

- [`libfec_parser.fecfile`](#fecfile-api): rows as dicts keyed by column name, modeled on the [`fecfile`](https://pypi.org/project/fecfile/) package.
- [`libfec_parser.parser`](#native-api): a lower-level `Filing` class with positional fields.

For a guided tour, including loading a filing into pandas, see [`examples/quickstart.ipynb`](examples/quickstart.ipynb).

> **Status:** early and unpublished. The package is not on PyPI yet, and the API may change.

## Installation

`libfec_parser` has to be built from source for now. You'll need a [Rust toolchain](https://rustup.rs/) and [uv](https://docs.astral.sh/uv/) (or `pip install maturin`).

```bash
git clone https://github.com/asg017/libfec
cd libfec/crates/fec-py

# Install into the active virtualenv
uvx maturin develop --release

# ...or build a wheel into dist/ and install it wherever you like
uvx maturin build --release --out dist
pip install dist/libfec_parser-*.whl
```

Wheels use the stable ABI (`abi3`), so one build works on Python 3.9 and up.

## `fecfile` API

```python
from libfec_parser import fecfile
```

### Loading a filing

| Function | Input |
| --- | --- |
| `from_file(path, options=None)` | Path to a `.fec` file, as a `str` |
| `loads(content, options=None)` | `bytes`, a `str`, or a list of lines |
| `from_http(filing_id, options=None)` | A filing ID (`int` or `str`), downloaded from `docquery.fec.gov`. Returns `None` if the filing doesn't exist |

All three return a dict with the same shape:

```python
{
    "header": {"record_type": "HDR", "fec_version": "8.4", "software_name": "FECFile", ...},
    "filing": {"form_type": "F3XN", "filer_committee_id_number": "C00016683", "committee_name": ..., ...},
    "itemizations": {
        "Schedule A": [{"form_type": "SA11AI", "contributor_last_name": ..., ...}, ...],
        "Schedule B": [...],
    },
    "text": [...],
}
```

- `header` is the `HDR` record. `report_id`, `report_number` and `comment` are only present when the filing sets them.
- `filing` is the cover page, with one key for every column on the form.
- `itemizations` groups rows by schedule. Row types starting with `S` are keyed `"Schedule A"`, `"Schedule B"`, and so on. Anything else is keyed by its row type, such as `"F1S"`. Each row's exact line number is in its `form_type`.
- `text` holds free-form `TEXT` records.

Column names come from libfec's mappings for the filing's FEC format version. Rows with no known mapping fall back to `field_0`, `field_1`, ….

### Options

`filter_itemizations` takes a list of row-type prefixes. Rows that don't match are skipped, which saves most of the memory on large filings:

```python
# Only Schedule A and Schedule B rows
fecfile.from_file(path, options={"filter_itemizations": ["SA", "SB"]})

# Only line 11(a)(i) of Schedule A
fecfile.from_file(path, options={"filter_itemizations": ["SA11AI"]})

# Header and cover page only
fecfile.from_file(path, options={"filter_itemizations": []})
```

### Parsing single records

```python
header, version, lines_consumed = fecfile.parse_header(first_line)
row = fecfile.parse_line(line, version)
```

`.fec` fields are separated by the ASCII 28 "file separator" character, which `str.splitlines()` treats as a line break. Split on `"\n"` when breaking a filing into lines yourself.

`fecfile.print_example(parsed)` prints the header, cover page, and the first row of each schedule as JSON.

### Differences from `fecfile`

- **Every value is a string.** Amounts are not converted to `float`, and dates stay as `YYYYMMDD`. The `as_strings` option is accepted and ignored.
- Only the ASCII 28-delimited format (FEC version 6 and later) is supported by `parse_header()` and `parse_line()`.
- `from_http()` reads the whole response into memory before parsing, and there is no `iter_file()` / `iter_http()` yet.

## Native API

```python
from libfec_parser.parser import Filing

filing = Filing("1721696.fec")
# Filing(form_type='F3XN', filer_id='C00016683', 1387 itemizations)
```

`Filing(source)` accepts a path (`str`), `bytes`, or any object with a `.read()` method that returns bytes:

```python
import urllib.request

with urllib.request.urlopen("https://docquery.fec.gov/dcdev/posted/1721696.fec") as response:
    filing = Filing(response)
```

The whole filing is parsed up front. A bad path raises `IOError`, an unparseable filing raises `ValueError`.

### `Filing`

| Attribute | Type | |
| --- | --- | --- |
| `header` | `Header` | The `HDR` record |
| `cover` | `Cover` | The cover page |
| `itemizations` | `list[Itemization]` | Every remaining row, in file order |

Each access to `itemizations` copies the list, so bind it to a variable rather than indexing `filing.itemizations` in a loop.

### `Header`

| Attribute | Type | |
| --- | --- | --- |
| `record_type` | `str` | Always `"HDR"` |
| `ef_type` | `str` | Electronic filing type |
| `fec_version` | `str` | FEC format version, such as `"8.4"` |
| `software_name` | `str` | Software that produced the filing |
| `software_version` | `str` | |
| `report_id` | `str \| None` | For amendments, the filing being amended |
| `report_number` | `str \| None` | |
| `comment` | `str \| None` | |

### `Cover`

| Attribute | Type | |
| --- | --- | --- |
| `form_type` | `str` | Such as `"F3XN"` or `"F3PA"` |
| `filer_id` | `str` | Committee ID |
| `filer_name` | `str` | |
| `report_code` | `str \| None` | Such as `"Q1"`, `"M8"`, `"YE"` |
| `coverage_from_date` | `str \| None` | ISO formatted, `"2023-07-01"` |
| `coverage_through_date` | `str \| None` | ISO formatted |

`cover.fields()` returns the same six values as a dict. For the rest of the cover page, use the [`fecfile` API](#fecfile-api).

### `Itemization`

A single row: its `row_type` (such as `"SA11AI"`) plus the raw fields, in file order and without column names.

```python
item = filing.itemizations[0]

item.row_type   # 'SA11AI'
len(item)       # 45
item[0]         # 'SA11AI'
item[-1]        # negative indexes work
item.fields()   # all fields as a list[str]
```

### `fec_header(contents)`

Takes the `bytes` of a filing and returns just its FEC format version.

## Development

Build with `maturin`, not `cargo build`, which can't link a Python extension module on its own.

```bash
make build          # debug wheel into dist/
make build-release  # optimized wheel into dist/
make test-pytest    # run tests/ against the wheel in dist/
make notebook       # build, then open examples/quickstart.ipynb in JupyterLab
```

The tests look for `.fec` files in the repo's `cache/` and `benchmarks/` directories and skip when none are found. See [`tests/README.md`](tests/README.md).

Rebuilt wheels keep the same filename, so `uv` will happily reuse a stale cached copy. Pass `--no-cache` whenever you `uv run --with` a wheel from `dist/`, as the Makefile targets do.

To refresh the notebook's saved outputs after an API change:

```bash
make build
cd examples
uv run --no-cache --no-project --isolated \
  --with "$(ls ../dist/libfec_parser-*.whl | head -1)" \
  --with pandas --with nbconvert --with ipykernel \
  jupyter nbconvert --to notebook --execute --inplace quickstart.ipynb
```
