# libfec_parser

> **Alpha.** Wheels are published on [GitHub Releases](https://github.com/asg017/libfec/releases),
> not PyPI. The native `Filing` API will change in upcoming releases (see
> [`plans/python/`](../../plans/python/)); the `fecfile` module is not yet a drop-in
> replacement.

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

## Install

**Python 3.11 or newer.** `libfec_parser` is not on PyPI, so `pip install libfec-parser`
will not find it. Wheels are attached to every
[GitHub Release](https://github.com/asg017/libfec/releases); install one by URL with
[uv](https://docs.astral.sh/uv/):

```bash
VERSION=0.0.33   # the release you want, from the releases page
uv pip install "https://github.com/asg017/libfec/releases/download/$VERSION/libfec_parser-$VERSION-cp311-abi3-macosx_11_0_arm64.whl"
```

Swap the platform tag at the end for your machine:

| Platform | Filename |
| --- | --- |
| Linux x86_64 (glibc 2.17+) | `libfec_parser-$VERSION-cp311-abi3-manylinux_2_17_x86_64.manylinux2014_x86_64.whl` |
| Linux aarch64 (glibc 2.17+) | `libfec_parser-$VERSION-cp311-abi3-manylinux_2_17_aarch64.manylinux2014_aarch64.whl` |
| macOS Apple silicon | `libfec_parser-$VERSION-cp311-abi3-macosx_11_0_arm64.whl` |
| macOS Intel | `libfec_parser-$VERSION-cp311-abi3-macosx_10_12_x86_64.whl` |
| Windows x64 | `libfec_parser-$VERSION-cp311-abi3-win_amd64.whl` |

One wheel per platform covers every Python from 3.11 up: they are built against the stable
ABI (`cp311-abi3`). On Python 3.10 and older the install is refused.

Other platforms — musl Linux, armv7, 32-bit, BSD — build from the source distribution on the
same release page, which needs a [Rust toolchain](https://rustup.rs/):

```bash
uv pip install "https://github.com/asg017/libfec/releases/download/$VERSION/libfec_parser-$VERSION.tar.gz"
```

`pip install` works in place of `uv pip install` throughout. To build from a git checkout
instead, see [Development](#development).

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

## Known issues

Three known gaps come from the underlying Rust parser (`fec-parser`), not the bindings. They
are tracked as parser bugs and deliberately **not** worked around here, so `fecfile` output
differs from the `fecfile` package on exactly these points:

- **`[BEGINTEXT]…[ENDTEXT]` bodies are dropped.** F99 filings parse, but their free-form text
  never reaches `filing["text"]`.
- **`_TODO_DUP` cover column names.** A few Form 3P cover-page columns come back with
  placeholder names such as `_TODO_DUP1` instead of a real column name.
- **Non-UTF-8 bytes become U+FFFD.** Filings written in cp1252 (curly quotes, en dashes) decode
  lossily: the offending bytes are replaced with `�` rather than transcoded.

Background and the intended fixes are in [`plans/python/`](../../plans/python/) — see
[`00-decisions.md`](../../plans/python/00-decisions.md), "Deferred to `fec-parser`".

## Development

Build with `maturin`, not `cargo build`, which can't link a Python extension module on its own.

```bash
cd crates/fec-py
uv venv && uv sync --group dev   # once
make develop                     # after Rust changes
make test
make notebook
```

`make build` produces a wheel into `dist/`, which is what CI and releases install.

Tests run against the committed fixtures in [`tests/fixtures/`](tests/fixtures/) — see [`tests/README.md`](tests/README.md).

**Releasing:** the version is read from `Cargo.toml` (not `pyproject.toml`) — bump it together with `crates/fec-cli/Cargo.toml` so the bindings stay in lockstep with the CLI.

To refresh the notebook's saved outputs after an API change, run `make notebook-check`.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
