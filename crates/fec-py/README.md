# libfec

> **Alpha.** The `fecfile` module is not yet a drop-in replacement for the
> [`fecfile`](https://pypi.org/project/fecfile/) package (Phase 3).

The [libfec](https://github.com/asg017/libfec) command-line tool and Python bindings for its `.fec` parser, in one package. Parse FEC electronic filings from a path, from bytes, or straight from the FEC's website, with the parsing done in Rust.

```python
import libfec

from libfec.itemizations import ScheduleA

with libfec.open("1721696.fec") as filing:
    filing.filer_name                          # 'PFIZER INC. PAC'
    filing.cover.summary.line6c_total_receipts.column_a   # 83741.93
    for item in filing.itemizations:
        if isinstance(item, ScheduleA):
            print(item.contributor.display_name(), item.contribution_amount, item.contribution_date)
```

Two APIs are included:

- [`libfec.parser`](#native-api) (also exported at the top level as `libfec.open`): the
  primary API — a streaming `FilingReader` with typed covers and itemizations.
- [`libfec.fecfile`](#fecfile-api): rows as dicts keyed by column name, modeled on the
  [`fecfile`](https://pypi.org/project/fecfile/) package; strings only until Phase 3.

For a guided tour, including loading a filing into pandas, see [`examples/quickstart.ipynb`](examples/quickstart.ipynb).

## Install

**Python 3.11 or newer.**

```bash
uv add libfec        # or: pip install libfec
```

The same package installs the `libfec` command-line tool, so you can also run it without
installing anything:

```bash
uvx libfec export FEC-1949543 -o filing.db
```

Wheels cover Linux (glibc 2.17+: x86_64, aarch64, armv7), macOS (Apple silicon and Intel) and
Windows x64. One wheel per platform covers every Python from 3.11 up: they are built against the
stable ABI (`cp311-abi3`). Other platforms (musl Linux, 32-bit x86, BSD) build from the source
distribution, which needs a [Rust toolchain](https://rustup.rs/). To build from a git checkout
instead, see [Development](#development).

## `fecfile` API

```python
from libfec import fecfile
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

- **Every value is a string.** Amounts are not converted to `float`, and dates stay as `YYYYMMDD`. The `as_strings` option is accepted and ignored — the native API returns typed values.
- Only the ASCII 28-delimited format (FEC version 6 and later) is supported by `parse_header()` and `parse_line()`.
- `from_http()` reads the whole response into memory before parsing, and there is no `iter_file()` / `iter_http()` yet.

## Native API

```python
from libfec import open
```

(`open` is also reachable as `libfec.parser.open`; it shadows the builtin, so reach the real one
through `builtins.open` if you need both in the same scope.)

`open(source)` returns a `FilingReader`. The `HDR` and cover records are parsed eagerly, so
`header`, `cover` and the filer attributes are available immediately; everything after the cover
is streamed, a batch at a time with the GIL released, by one of two views of the same records:

| | Typed | Raw |
| --- | --- | --- |
| Cover | `filing.cover` — `Form3X`, `Form1`, … | `filing.cover_record` — a `Record` |
| Everything after it | `filing.itemizations` — `ScheduleA`, `ScheduleB`, … | `filing.itemization_records` — `Record`s |

Anything without a typed class (a summary record such as `F3PS`, a cover form with no class)
comes back as an [`Unknown`](#unknown) rather than `None`, so the typed views are total and line up
one-to-one with the raw ones.

```python
from libfec import open

filing = open("1721696.fec")
next(filing.itemizations)           # ScheduleA(form_type='SA11AI', …)
next(filing.itemization_records)    # Record(row_type='SA11AI', line=3, 45 fields)
```

### Sources

`open()` and `fec_header()` accept the same kinds of source:

- a filesystem path — `str` or `os.PathLike`. **A `str` is always a path**, never the filing's
  contents — pass `bytes` if you already have the data in memory.
- a bytes-like object — `bytes`, `bytearray`, `memoryview`, `mmap.mmap` — read without copying.
- a binary file object: anything with a `read(n) -> bytes` method (an open file, `io.BytesIO`, an
  `urlopen()` response, …), pulled a chunk at a time rather than read whole.

A text-mode file (or anything else whose `read()` returns `str`) raises `TypeError`:

```python
import io
import libfec

with open("1721696.fec", "rb") as f:      # builtin `open`, binary mode
    libfec.open(f)

libfec.open(io.StringIO("not bytes"))
# TypeError: file must be opened in binary mode, e.g. open(path, 'rb')
```

### `FilingReader`

```python
with libfec.open("1721696.fec") as filing:
    filing.id              # '1721696' — the file stem, or a file object's `name`
    filing.fec_version     # '8.4' — shortcut for filing.header.fec_version
    filing.form_type       # 'F3XN'
    filing.filer_id        # 'C00016683'
    filing.filer_name      # 'PFIZER INC. PAC'
    filing.report_code     # 'M8'
    filing.coverage_from_date      # datetime.date(2023, 7, 1)
    filing.coverage_through_date   # datetime.date(2023, 7, 31)
```

The six filer attributes are normalized across every form type, whatever the form itself calls
them (`committee_name`, `filer_name`, `organization_name`, …). `report_code` and the coverage
dates are `None` for forms that are not periodic reports (F1, F2, F24, F99, …).

- **Each access of `itemizations` or `itemization_records` starts a new pass** from the top of the
  filing, so you can iterate as many times as you like. A single pass is an ordinary iterator:
  once exhausted, it stays exhausted.
- **A file object can be read only once**: it is consumed as it is read, so a second pass over a
  filing opened from one raises `ValueError`. Paths and bytes-like sources are reopened for every
  pass.
- `close()` drops the source and stops every pass; it's idempotent, and iterating afterwards raises
  `ValueError`. Use it as a context manager (as above) rather than calling it directly.
- There is no `len()` — the record count isn't known without a full pass.

### Typed covers

`filing.cover` is the cover as a typed object: one class per form (`Form1`, `Form3`, `Form3P`,
`Form3X`, `Form24`, `Form99`, … — 15 forms, every cover libfec reads, legacy and paper included),
from `libfec.covers`, or [`Unknown`](#unknown) for a form with no typed class. The fields are named
for what they mean on the paper form, grouped the way the form groups them, and typed (`float`
amounts, `datetime.date`s, `bool` checkboxes, `str | None` text):

```python
import libfec as fec
from libfec.covers import Form3, Form3X

with fec.open("1926068.fec") as filing:
    match filing.cover:
        case Form3X() as f3x:
            f3x.committee_name                               # 'Ohio Democratic Party - Federal'
            f3x.coverage_from_date                           # datetime.date(2025, 10, 1)
            f3x.report_code_label()                          # 'November Monthly'
            f3x.treasurer.last_name                          # 'Frost-Brooks'
            f3x.summary.line6c_total_receipts.column_a       # 445072.35 (this period)
            f3x.summary.line6c_total_receipts.column_b       # 3484593.19 (calendar year to date)
        case Form3() as f3:
            ...
        case fec.Unknown():
            ...
```

The classes are `fec-parser`'s own Rust cover structs, exposed as-is, so the names and values are
exactly `libfec info -f json`'s `cover_data` (`to_dict()` gives the same shape, with `date`s).
Every field's docstring cites the form line and FEC source it comes from. The objects are
read-only; `==` compares field values. `CoverData` is the union of the form classes, for
annotations, and type checkers narrow it on `match`/`isinstance`.

### Typed itemizations

`filing.itemizations` yields one typed object per record, in file order, from
`libfec.itemizations`: `ScheduleA` through `ScheduleL`, the Form 5/6/7/9/13 itemizations, and
`TextRecord` for `TEXT` records. Filter by class rather than by row type:

```python
from libfec.itemizations import ScheduleA, ScheduleB

with libfec.open("1721696.fec") as filing:
    for item in filing.itemizations:
        match item:
            case ScheduleA(memo=False):
                item.contributor.display_name(), item.contribution_amount
            case ScheduleB():
                item.payee.display_name(), item.expenditure_amount
```

`item.form_type` is the row type as filed (`'SA11AI'`), and `item.line_number()` the line it
was reported on (`'11AI'`) for the schedules that have one.

Typing every record costs more than reading it raw (see [Performance](#performance)). When you
only need a few columns, or the exact raw text, use `itemization_records`; `record.itemization`
types a single `Record` on demand.

### `Unknown`

What the typed views yield for a record with no typed class:

```python
u.form_type     # 'F3PS' — the row type as filed, like the typed classes
u.record        # the underlying Record
u.to_dict()     # its typed values by column name, same as dict(u.record)
```

### `Header`

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

### `Record`

A `Record` is one raw line of the filing: a mapping from column name to a typed value, plus
positional access to the raw fields. `itemization_records` yields them, and `cover_record` is the
cover line as one — every column the form defines:

```python
filing.cover_record["col_a_total_receipts"]   # 83741.93 (float)
len(filing.cover_record)                       # 123 mapped columns, for an F3XN 8.4 cover
```

`len(record)` is the number of *mapped columns* — `len(record.fields())` is the raw field count,
which can differ for a short or an over-long line.

| Access | Result |
| --- | --- |
| `record["name"]`, amount/date column, parses | typed: `float` or `datetime.date` |
| `record["name"]`, amount/date column, empty | `None` |
| `record["name"]`, amount/date column, garbage (doesn't parse) | the raw `str` |
| `record["name"]`, text column | the raw `str` (`""` if empty — text never becomes `None`) |
| `record["name"]`, any column, past the end of a short line | `None` |
| `record[i]` / `record[a:b]` (by position) | always the raw `str` / `list[str]`, whatever the column |

```python
record = next(filing.itemization_records)
record["contribution_amount"]      # 104.17 (float)
record[20]                          # '104.17' (str) — same field, by position
dict(record)["contributor_last_name"]   # 'Aaronson'
```

Because a garbage value comes back as `str` in place of the expected type, guard with
`isinstance` rather than assuming every amount parsed:

```python
value = record["contribution_amount"]
if isinstance(value, str):
    ...  # the parser couldn't convert it; handle the raw text explicitly
```

`record.extra_fields` holds any fields past the last mapped column (`[]` unless one is non-empty),
`record.line` is its 1-based physical line in the file, and `record.itemization` is the record
typed on demand (or `None`). Two `Record`s compare and hash equal when their
`(row_type, fec_version, raw fields)` match, and a `Record` pickles and unpickles cleanly.

### Errors

| Exception | Raised when |
| --- | --- |
| `FileNotFoundError` | the source is a path that doesn't exist (`errno`/`filename` set, like `open()`) |
| `FecParseError` (a `FecError`, a `ValueError`) | the input isn't a parseable `.fec` filing |
| `MissingMappingError` (a `FecError`) | a record's `(row_type, fec_version)` has no column mapping — has `.row_type`, `.version`, `.line` |

A missing mapping is raised from the specific `next()` that reached that record, by either view,
and the pass stays usable — but only a `while`/`next()` loop can catch it and keep going; a `for`
loop can't, because the exception comes out of the `for` statement's own call to `__next__`
before your loop body ever runs:

```python
from libfec import MissingMappingError, open

items = open("1721696.fec").itemizations
kept = []
while True:
    try:
        kept.append(next(items))
    except StopIteration:
        break
    except MissingMappingError:
        continue   # skip this record; the pass is still usable
```

### `fec_header(source, /)`

Returns just the `fec_version` of a filing, from anything `open()` accepts. It reads only the
`HDR` record — the first line — so it works even when the rest of the file (say, an unmapped
cover form) would make `open()` raise:

```python
libfec.fec_header("1721696.fec")   # '8.4'
```

### pandas

`Record` registers as a `collections.abc.Mapping`, so `itemization_records` is exactly what
`pd.DataFrame` wants — and because values are already typed, no per-column conversion is needed:

```python
import pandas as pd
from libfec import open

df = pd.DataFrame(open("1721696.fec").itemization_records)
df["contribution_amount"].dtype   # dtype('float64')
```

Dates land as `datetime.date` objects in an `object`-dtype column — pandas does not auto-convert
`date` to `datetime64`. That's fine for most uses; if you need `datetime64` semantics (`.dt`
accessors, resampling), convert explicitly:

```python
pd.to_datetime(df["contribution_date"]).dt.year.min()   # 2023
```

## Known issues

Three known gaps come from the underlying Rust parser (`fec-parser`), not the bindings. They
are tracked as parser bugs and deliberately **not** worked around here, so `fecfile` output
differs from the `fecfile` package on exactly these points:

- **`[BEGINTEXT]…[ENDTEXT]` bodies are dropped.** F99 filings parse, but their free-form text
  never reaches `filing["text"]`.
- **`_TODO_DUP` cover column names.** A few Form 3X and Form 3P cover-page columns come back
  with placeholder names such as `col_a_total_receipts_TODO_DUP` — this shows up in
  `cover_record.keys()` (and `dict(cover_record)`) too, not just the `fecfile` API's `filing["filing"]`.
- **Non-UTF-8 bytes become U+FFFD.** Filings written in cp1252 (curly quotes, en dashes) decode
  lossily: the offending bytes are replaced with `�` rather than transcoded.

Background and the intended fixes are in [`plans/python/`](../../plans/python/) — see
[`00-decisions.md`](../../plans/python/00-decisions.md), "Deferred to `fec-parser`".

## Performance

Measured on an Apple M4 Pro, CPython 3.13, release build, parsing the 91 MB, 408,160-record
`1805248.fec` filing (gitignored; not one of the committed fixtures). The `libfec` rows were
measured 2026-10-23, the PyPI `fecfile` rows 2026-09-18.

| Operation | Time | Peak RSS |
| --- | --- | --- |
| `itemization_records`, streamed | 0.20 s | 38.0 MB |
| `itemizations` (typed), streamed | 0.80 s | 38.0 MB |
| `pd.DataFrame(open(p).itemization_records)` | 6.31 s | 2,276 MB |
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

`python/libfec/covers.pyi` is generated from `crates/fec-parser/src/covers/` — run
`make cover-stubs` after changing a cover struct (a test fails while it is stale).

## License

Dual-licensed under MIT or Apache-2.0, at your option.
