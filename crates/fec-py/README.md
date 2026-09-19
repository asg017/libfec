# libfec_parser

> **Alpha.** Wheels are published on [GitHub Releases](https://github.com/asg017/libfec/releases),
> not PyPI.

Python bindings for [libfec](https://github.com/asg017/libfec)'s `.fec` parser. Parse FEC electronic filings from a path, from bytes, or straight from the FEC's website, with the parsing done in Rust.

```python
import libfec_parser

with libfec_parser.open("1721696.fec") as filing:
    filing.cover.filer_name                    # 'PFIZER INC. PAC'
    filing.cover_row["col_a_total_receipts"]   # 83741.93
    for row in filing.rows("SA"):
        print(row["contributor_last_name"], row["contribution_amount"], row["contribution_date"])
```

Two APIs are included:

- [`libfec_parser.parser`](#native-api) (also exported at the top level as `open`/`read`): the
  primary API — a streaming `FilingReader` or an eager `Filing`, both with typed values.
- [`libfec_parser.fecfile`](#fecfile-api): rows as dicts keyed by column name, a drop-in for the
  [`fecfile`](https://pypi.org/project/fecfile/) package (0.9.1), typed values included.

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

`libfec_parser.fecfile` is a **drop-in replacement for [`fecfile`](https://pypi.org/project/fecfile/) 0.9.1**'s public API: the same keys, in the same order, with the same values *and* the same types. This is enforced by a differential test that compares every value against the real package — on every committed fixture, and on a 408,162-item, 91 MB filing (`tests/test_fecfile_differential.py`, `tests/test_perf.py::test_compat_differential_benchmark_filing`). The handful of differences that survive are listed in [Where it differs](#where-it-differs), below.

Public names: `loads`, `from_file`, `from_http`, `iter_file`, `iter_http`, `iter_lines`, `parse_header`, `parse_line`, `print_example`, `FecItem`, `FecParserMissingMappingError`, `FilingUnavailableError`, `FecParserTypeWarning`.

### Loading a filing

| Function | Input |
| --- | --- |
| `from_file(path, options=None)` | Path to a `.fec` file, as a `str` or `os.PathLike` |
| `loads(content, options=None)` | `bytes`, a `str`, or any iterable of lines |
| `from_http(filing_id, options=None)` | A filing ID (`int` or `str`), downloaded from `docquery.fec.gov`. `None` if neither the electronic nor the paper URL exists |
| `iter_file(path, options=None)` | Like `from_file`, streamed as `FecItem`s instead of built into one dict |
| `iter_http(filing_id, options=None)` | Like `from_http`, streamed; raises instead of returning `None` on a 404 (see the HTTP section below) |
| `iter_lines(lines, options=None)` | Streamed, from an iterable of `str`/`bytes` lines |

`from_file`, `loads` and `from_http` return a dict with the same shape:

```python
{
    "header": {"record_type": "HDR", "fec_version": "8.5", "soft_name": "FECfile", ...},
    "filing": {"form_type": "F3N", "filer_committee_id_number": "C00900860", "committee_name": ..., ...},
    "itemizations": {
        "Schedule A": [{"form_type": "SA11AI", "contributor_last_name": ..., ...}, ...],
        "Schedule B": [...],
    },
    "text": [...],
}
```

(Keys shown are real output from `fecfile.from_file("tests/fixtures/1921705.fec")`.)

- `header` is the `HDR` record. `report_id`, `report_number` and `comment` are `''` unless the filing sets them.
- `filing` is the cover page, with one key for every column on the form.
- `itemizations` groups rows by schedule, in file order. Row types starting with `S` are keyed `"Schedule A"`, `"Schedule B"`, and so on. Anything else is keyed by its row type, such as `"F1S"`.
- `text` holds free-form `TEXT` records.

Column names come from libfec's mappings for the filing's FEC format version, the same names real `fecfile` uses.

### Values

Every value is typed from `fecfile`'s own (vendored) type table, exactly as real `fecfile` types it:

- amount columns parse to `float`;
- date columns parse to a **tz-aware `datetime` in US/Eastern** (via `zoneinfo`, not `pytz` — same instant, same UTC offset as real);
- an empty amount or date column is `None`;
- a handful of columns parse to `int`;
- anything else — or a value the table types but that doesn't parse — comes back as the raw `str` (the latter also raises a `FecParserTypeWarning`);
- `options={"as_strings": True}` turns all of the above off: every value is the raw `str`.

```python
>>> row = fecfile.from_file("tests/fixtures/1921705.fec")["itemizations"]["Schedule A"][0]
>>> row["contribution_amount"], row["contribution_date"]
(500.0, datetime.datetime(2025, 7, 7, 0, 0, tzinfo=zoneinfo.ZoneInfo(key='America/New_York')))
```

**Windows:** `zoneinfo` needs the `tzdata` package there (macOS and Linux ship a system tz database). A filing with a date raises `ImportError: libfec_parser.fecfile needs the 'tzdata' package on this platform: pip install tzdata` if it's missing.

### Streaming large filings

`iter_file`/`iter_http`/`iter_lines` yield one `FecItem` (`.data_type`, `.data`) at a time — the header, then the summary (cover page), then one itemization/text item per row — instead of building the whole dict, and never hold more than a batch of rows in memory. On the 91 MB, 408,160-row benchmark filing (see [Performance](#performance)), `fecfile.iter_file` peaks at 32.6 MB against 1,106 MB for `from_file`:

```python
for item in fecfile.iter_file(path):
    if item.data_type == "itemization":
        ...
```

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

Unlike real `fecfile`, `options` is validated up front: an unknown key raises `ValueError`, and a wrong type (a bare `"SA"` instead of `["SA"]`, say) raises `TypeError` rather than silently doing something unintended.

### HTTP: `from_http` / `iter_http`

Requires the `[http]` extra (`httpx2`), not installed by default. Since this package isn't on PyPI, install it by wheel URL with the extra appended, the same way as [Install](#install) above:

```bash
VERSION=0.0.33   # the release you want, from the releases page
uv pip install "libfec-parser[http] @ https://github.com/asg017/libfec/releases/download/$VERSION/libfec_parser-$VERSION-cp311-abi3-macosx_11_0_arm64.whl"
```

(Swap the wheel filename for your platform — see the table in [Install](#install).) Without `httpx2` installed, `from_http`/`iter_http` raise `ImportError` naming the extra.

`from_http(filing_id)` tries the electronic ("dcdev") URL first and the paper URL on a 404, and returns `None` if both are 404 — matching real `fecfile` (the idiom in the wild is `if fecfile.from_http(n) is None: ...`). Any other non-200 status raises `FilingUnavailableError` instead of trying to parse an error page as a filing, which is what real `fecfile` attempts. `iter_http` streams the same way `iter_file` does — the first item can arrive before the download finishes — and raises `FilingUnavailableError` on *any* non-200, including a 404, since there's no dict to return `None` in its place.

### Parsing single records

```python
header, version, lines_consumed = fecfile.parse_header(first_line)
row = fecfile.parse_line(line, version)
```

`.fec` fields are separated by the ASCII 28 "file separator" character, which `str.splitlines()` treats as a line break. Split on `"\n"` when breaking a filing into lines yourself.

`fecfile.print_example(parsed)` prints the header, cover page, and the first row of each schedule as JSON.

### Where it differs

Three differences survive the differential test; nothing is excluded from a comparison without appearing here:

- **`_TODO_DUP` cover columns.** An F3X/F3P cover mapping names a few columns twice; `fec-parser` keeps both copies and suffixes the second `_TODO_DUP` (deferred parser item). Real `fecfile` builds its dict by name, so the second copy just overwrites the first — which is also the value this package has under the unsuffixed name, so dropping the `_TODO_DUP` extras is enough to make the two comparable.
- **`F99_text` / `[BEGINTEXT]…[ENDTEXT]` bodies are not surfaced.** A form 99's free-form narrative sits between `[BEGINTEXT]` and `[ENDTEXT]` markers that `fec-parser` doesn't parse out (deferred parser item N14), so this package never has an `F99_text` key or item to offer.
- **Real's own `iter_file` leaves a trailing `\n` in the last field of every row.** It reads a file object a line at a time and never strips the newline (`'20006\n'`, or just `'\n'` for an empty field); `from_file`/`loads` split on `"\n"` first and are clean. This is a bug in the real package, not a difference of this one, and it isn't reproduced here.

**Known limitation, not (so far) an observed difference:** a filing with non-UTF-8 bytes (cp1252 curly quotes, en dashes, say) decodes lossily here — the offending bytes become `�` rather than being transcoded the way real `fecfile`'s `ISO-8859-1` fallback would render them. No committed fixture, and not the 408,160-row benchmark filing either, has ever exercised this, so it isn't part of the allowlist above — but a filing that does contain such bytes could show a difference here.

Deliberate supersets — things this package does that real `fecfile` doesn't:

- `options` is validated: an unknown key raises `ValueError`; a wrong-typed value (a bare `str` where a list is expected) raises `TypeError`.
- `filter_itemizations` prefixes are matched case-insensitively (real is case-sensitive).
- `loads` accepts a bytes-like object or any iterable of lines, not just `str`.
- `from_file` accepts `os.PathLike`, not just `str`.
- `FecParserMissingMappingError` and `FilingUnavailableError` derive from `FecError` (a `ValueError`), so they're catchable alongside the native API's exceptions.
- `parse_header` on a line that isn't a header raises `ValueError`; real raises `IndexError`.
- `from_http` raises `FilingUnavailableError` for a non-404 HTTP error rather than trying to parse the error page as a filing.

Two further, honest gaps, not allowlisted because no test currently exercises them:

- Multi-line `/* ... */` headers (FEC format versions 1 and 2) aren't supported: `parse_header` raises `FecParserMissingMappingError`, and a file that starts with one raises `FecParseError` from `from_file`/`loads`/`iter_file`.
- The HDR `name_delim` column (present in format versions 3.x–5.x) always reads `''` — the native `Header` type doesn't carry it.

### Use the native API instead when…

…matching `fecfile`'s exact shape isn't the point. `open()`/`read()` are roughly 9–14× faster on this package's own benchmark filing (see [Performance](#performance) — `read()` vs. `fecfile.from_file`, `open()` vs. `fecfile.iter_file`), hand back a typed `Row` instead of building a `dict` per row, and never consult the vendored type table at all.

## Native API

```python
from libfec_parser import open, read
```

(`open`/`read` are also reachable as `libfec_parser.parser.open`/`.read`; `open` shadows the
builtin, so reach the real one through `builtins.open` if you need both in the same scope.)

### `open()` vs `read()`

`open(source)` returns a `FilingReader`: `header`, `cover` and `cover_row` are parsed eagerly, so
they're available immediately, and the itemization rows are pulled lazily as you iterate. Use it
for large filings, or when `rows(*prefixes)` lets you skip most of the file.

`read(source)` (a thin function wrapper over the `Filing` class) parses the whole filing up front
into a `Filing`, whose `rows` is a plain `list`. `Filing(source)` is exactly `read(source)` — pick
whichever reads better. Use `read`/`Filing` when you want everything in memory at once, such as to
build a `pandas.DataFrame`.

```python
from libfec_parser import open, read

reader = open("1721696.fec")
next(reader).row_type          # 'SA11AI'

filing = read("1721696.fec")   # same as Filing("1721696.fec")
len(filing.rows)                # 1387
```

### Sources

Both `open()` and `read()` (and `Filing()` and `fec_header()`) accept the same kinds of source:

- a filesystem path — `str` or `os.PathLike`. **A `str` is always a path**, never the filing's
  contents — pass `bytes` if you already have the data in memory.
- a bytes-like object — `bytes`, `bytearray`, `memoryview`, `mmap.mmap` — read without copying.
- a binary file object: anything with a `read(n) -> bytes` method (an open file, `io.BytesIO`, an
  `urlopen()` response, …), pulled a chunk at a time rather than read whole.

A text-mode file (or anything else whose `read()` returns `str`) raises `TypeError`:

```python
import io
import libfec_parser

with open("1721696.fec", "rb") as f:      # builtin `open`, binary mode
    libfec_parser.open(f)

libfec_parser.open(io.StringIO("not bytes"))
# TypeError: file must be opened in binary mode, e.g. open(path, 'rb')
```

### `FilingReader`

```python
with libfec_parser.open("1721696.fec") as filing:
    filing.id            # '1721696' — the file stem, or a file object's `name`
    filing.fec_version    # '8.4' — shortcut for filing.header.fec_version

    for row in filing.rows("SA11AI"):   # filters before a Row is even built
        ...
```

- Iterating a `FilingReader` is single-pass: once exhausted, a second `for` yields nothing.
- `rows(*prefixes)` keeps only rows whose type starts with one of `prefixes`
  (case-insensitive) and returns the reader itself, so it chains into a `for`. A second call
  replaces the filter; `rows()` with no arguments clears it.
- `close()` drops the source; it's idempotent, and iterating afterwards raises `ValueError`.
  Use it as a context manager (as above) rather than calling it directly.
- There is no `len()` — the row count isn't known without a full pass.

### `Filing`

```python
from libfec_parser import read

filing = read("1721696.fec")
filing              # Filing(id='1721696', form_type='F3XN', filer_id='C00016683', 1387 rows)
len(filing)          # 1387, same as len(filing.rows)
list(filing)[0] is filing.rows[0]   # True — iterating a Filing iterates its rows
```

`filing.itemizations` still works as an alias for `filing.rows`, but warns with
`DeprecationWarning` — use `rows`.

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
| `coverage_from_date` | `date \| None` | |
| `coverage_through_date` | `date \| None` | |

`cover.fields()` returns those same six values as a `dict`, dates included as `datetime.date`.
For every other column on the cover page, use `cover_row` (below).

```python
cover = filing.cover
cover.coverage_from_date   # datetime.date(2023, 7, 1)
cover.fields()
# {'form_type': 'F3XN', 'filer_id': 'C00016683', 'filer_name': 'PFIZER INC. PAC',
#  'report_code': 'M8', 'coverage_from_date': datetime.date(2023, 7, 1),
#  'coverage_through_date': datetime.date(2023, 7, 31)}
```

### `cover_row`

`filing.cover_row` is the cover line itself, as a full `Row` — every column the form defines, not
just the six normalized ones above:

```python
filing.cover_row["col_a_total_receipts"]   # 83741.93 (float)
len(filing.cover_row)                       # 123 mapped columns, for an F3XN 8.4 cover
```

It follows the same rules as any other `Row` (below). Note that an unparseable cover date comes
back as `None` from `Cover.coverage_from_date`, but as its raw `str` from
`cover_row["coverage_from_date"]` — the two are not reconciled.

### `Row`

A `Row` is a mapping from column name to a typed value, plus positional access to the raw fields.
`len(row)` is the number of *mapped columns* — `len(row.fields())` is the raw field count, which
can differ for a short or an over-long line.

| Access | Result |
| --- | --- |
| `row["name"]`, amount/date column, parses | typed: `float` or `datetime.date` |
| `row["name"]`, amount/date column, empty | `None` |
| `row["name"]`, amount/date column, garbage (doesn't parse) | the raw `str` |
| `row["name"]`, text column | the raw `str` (`""` if empty — text never becomes `None`) |
| `row["name"]`, any column, past the end of a short row | `None` |
| `row[i]` / `row[a:b]` (by position) | always the raw `str` / `list[str]`, whatever the column |

```python
row = filing.rows[0]
row["contribution_amount"]      # 104.17 (float)
row[20]                          # '104.17' (str) — same field, by position
dict(row)["contributor_last_name"]   # 'Aaronson'
```

Because a garbage value comes back as `str` in place of the expected type, guard with
`isinstance` rather than assuming every amount parsed:

```python
value = row["contribution_amount"]
if isinstance(value, str):
    ...  # the parser couldn't convert it; handle the raw text explicitly
```

`row.extra_fields` holds any fields past the last mapped column (`[]` unless one is non-empty),
and `row.line` is the row's 1-based physical line in the file. Two `Row`s compare and hash equal
when their `(row_type, fec_version, raw fields)` match, and a `Row` pickles and unpickles cleanly.

### Errors

| Exception | Raised when |
| --- | --- |
| `FileNotFoundError` | the source is a path that doesn't exist (`errno`/`filename` set, like `open()`) |
| `FecParseError` (a `FecError`, a `ValueError`) | the input isn't a parseable `.fec` filing |
| `MissingMappingError` (a `FecError`) | a row's `(row_type, fec_version)` has no column mapping — has `.row_type`, `.version`, `.line` |

`read()`/`Filing()` are strict: a missing mapping anywhere raises out of the call. `open()` raises
it from the specific `next()` that reached that row, and the reader stays usable — but only a
`while`/`next()` loop can catch it and keep going; a `for` loop can't, because the exception comes
out of the `for` statement's own call to `__next__` before your loop body ever runs:

```python
from libfec_parser import MissingMappingError, open

reader = open("1721696.fec")
rows = []
while True:
    try:
        rows.append(next(reader))
    except StopIteration:
        break
    except MissingMappingError:
        continue   # skip this row; the reader is still usable
```

### `fec_header(source, /)`

Returns just the `fec_version` of a filing, from anything `open()` accepts. It reads only the
`HDR` record — the first line — so it works even when the rest of the file (say, an unmapped
cover form) would make `open()` raise:

```python
libfec_parser.fec_header("1721696.fec")   # '8.4'
```

### pandas

`Row` registers as a `collections.abc.Mapping`, so a list of them is exactly what `pd.DataFrame`
wants — and because values are already typed, no per-column conversion is needed:

```python
import pandas as pd
from libfec_parser import read

df = pd.DataFrame(read("1721696.fec").rows)
df["contribution_amount"].dtype   # dtype('float64')
```

Dates land as `datetime.date` objects in an `object`-dtype column — pandas does not auto-convert
`date` to `datetime64`. That's fine for most uses; if you need `datetime64` semantics (`.dt`
accessors, resampling), convert explicitly:

```python
pd.to_datetime(df["contribution_date"]).dt.year.min()   # 2023
```

## Known issues

Two gaps come from the underlying Rust parser (`fec-parser`), not the bindings, and are tracked
as parser bugs, deliberately **not** worked around here. Both show up in the `fecfile` API's
[differential allowlist](#where-it-differs) too, since `fecfile` inherits whatever the parser
gives it:

- **`_TODO_DUP` cover column names.** A few Form 3X and Form 3P cover-page columns come back
  with placeholder names such as `col_a_total_receipts_TODO_DUP`. Shows up in
  `cover_row.keys()`/`dict(cover_row)` on the native API, and in `filing["filing"]` on the
  `fecfile` API.
- **`[BEGINTEXT]…[ENDTEXT]` bodies are dropped.** F99 filings parse, but their free-form text
  never reaches a `Row`, and the `fecfile` API never gets an `F99_text` key to offer.

One more is a true limitation of both APIs rather than a parser bug as such, and isn't
allowlisted because no fixture or benchmark filing has triggered it: **non-UTF-8 bytes become
U+FFFD.** Filings written in cp1252 (curly quotes, en dashes) decode lossily — the offending
bytes are replaced with `�` rather than transcoded.

Background and the intended fixes are in [`plans/python/`](../../plans/python/) — see
[`00-decisions.md`](../../plans/python/00-decisions.md), "Deferred to `fec-parser`".

## Performance

Measured 2026-09-18 on an Apple M4 Pro, CPython 3.13, release build, parsing the 91 MB, 408,160-row
`1805248.fec` filing (gitignored; not one of the committed fixtures). Reproduce with `make bench`
([`benchmarks/python/bench.py`](../../benchmarks/python/bench.py)).

| Operation | Time | Peak RSS |
| --- | --- | --- |
| `open()`, streamed | 0.18 s | 31.6 MB |
| `read()` (eager `Filing`) | 0.30 s | 1,007 MB |
| `pd.DataFrame(read(p).rows)` | 5.32 s | 2,389 MB |
| `fecfile.from_file` (this package) | 2.70 s | 1,106 MB |
| `fecfile.iter_file` (this package) | 2.53 s | 32.6 MB |
| PyPI `fecfile.from_file` | 7.14 s | 1,403 MB |
| PyPI `fecfile.iter_file` | 6.87 s | 36.4 MB |

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
