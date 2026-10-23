"""Type stubs for :mod:`libfec.parser`.

Hand-maintained (pyo3's `experimental-inspect` output is the first draft only) and
checked against the built extension by `python -m mypy.stubtest` — see
`crates/fec-py/Makefile`'s `stubs` target.  The implementation is `src/parser.rs`.
"""

import mmap
import os
from collections.abc import Iterator
from datetime import date
from types import TracebackType
from typing import Any, Literal, Protocol, Self, TypeAlias, final, overload

from .covers import CoverData
from .itemizations import Itemization

__all__ = [
    "FilingIterator",
    "FilingReader",
    "Header",
    "Record",
    "Unknown",
    "fec_header",
    "open",
    "FecError",
    "FecParseError",
    "MissingMappingError",
]

Value: TypeAlias = str | float | date | None
"""A column's value: typed if it parses, the raw `str` if it is garbage, `None` if empty."""

class _Readable(Protocol):
    """A binary file object: anything whose `read(n)` hands back `bytes`."""

    def read(self, n: int, /) -> bytes: ...

Source: TypeAlias = (
    str | os.PathLike[str] | bytes | bytearray | memoryview | mmap.mmap | _Readable
)
"""What `open()`/`fec_header()` accept as a filing source.

A path, any bytes-like object (read without copying), or a binary file object
(read in chunks).  A text-mode file raises `TypeError`.
"""

@final
class Header:
    """The filing's `HDR` record."""

    @property
    def record_type(self) -> str: ...
    @property
    def ef_type(self) -> str: ...
    @property
    def fec_version(self) -> str: ...
    @property
    def software_name(self) -> str: ...
    @property
    def software_version(self) -> str: ...
    @property
    def report_id(self) -> str | None: ...
    @property
    def report_number(self) -> str | None: ...
    @property
    def comment(self) -> str | None: ...
    @property
    def style(self) -> Literal["hdr", "legacy_block", "paper"]:
        """How the header is written: an `HDR` record, a 1.x/2.x `/* Header` block, or paper."""
    @property
    def delimiter(self) -> Literal["fs", "comma"]:
        """Body field delimiter: `"fs"` (0x1C) or `"comma"`."""
    @property
    def name_delimiter(self) -> str | None:
        """Sub-delimiter of combined name fields (3.x-5.x and 1.x/2.x only)."""
    @property
    def batch_number(self) -> str | None:
        """Paper filings: FEC data-entry batch number."""
    @property
    def received_date(self) -> str | None:
        """Paper filings P2.6+: date the FEC received the filing."""
    @property
    def is_paper(self) -> bool:
        """Whether this is FEC data entry of a paper filing."""
    @property
    def legacy_fields(self) -> dict[str, str]:
        """`/* Header` block (1.x/2.x): every `key = value` line, in file order; empty otherwise."""
    @property
    def schedule_counts(self) -> dict[str, str]:
        """`/* Header` block: the `Schedule_Counts:` lines; empty otherwise."""
    def __repr__(self) -> str: ...

@final
class Record:
    """One raw record of a filing: a mapping from column name to typed value.

    By name the value follows the value rule (`Value` above); by position it is
    the raw `str` exactly as it appears in the file.  `len(record)` is the number
    of mapped columns — `len(record.fields())` is the raw field count.
    """

    @property
    def row_type(self) -> str: ...
    @property
    def line(self) -> int:
        """The record's 1-based physical line in the file."""

    @property
    def itemization(self) -> Itemization | None:
        """The record as a typed itemization (`ScheduleA`, … from
        `libfec.itemizations`), or `None` for a record that is not one or
        whose schedule has no class yet.  Typed anew on every access."""

    @property
    def extra_fields(self) -> list[str]:
        """Fields past the last mapped column, `[]` unless one of them is non-empty."""

    @overload
    def __getitem__(self, key: str, /) -> Value: ...
    @overload
    def __getitem__(self, key: int, /) -> str: ...
    @overload
    def __getitem__(self, key: slice, /) -> list[str]: ...
    def __len__(self) -> int: ...
    def __iter__(self) -> Iterator[str]: ...
    def __contains__(self, key: object, /) -> bool: ...
    def keys(self) -> list[str]: ...
    def values(self) -> list[Value]: ...
    def items(self) -> list[tuple[str, Value]]: ...
    def get(self, key: str, default: Value = None, /) -> Value: ...
    def fields(self) -> list[str]:
        """Every raw field of the record, in file order."""

    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...
    def __reduce__(self) -> tuple[Any, ...]: ...
    def __repr__(self) -> str: ...

def _record_from_parts(
    row_type: str,
    version: str,
    fields: list[str],
    line: int,
    name_delimiter: str | None = None,
    /,
) -> Record:
    """Rebuild a `Record` from its pickled parts; named by `Record.__reduce__`."""

@final
class Unknown:
    """A record with no typed class: a cover `libfec.covers` has no struct for,
    a summary record (`F3PS`, …), or an itemization schedule not typed yet.

    Stands in for the typed object in `FilingReader.cover` and
    `FilingReader.itemizations`, so neither is ever `None`.
    """

    @property
    def form_type(self) -> str:
        """The row type as filed, e.g. `F3PS`."""

    @property
    def record(self) -> Record:
        """The underlying raw `Record`."""

    def to_dict(self) -> dict[str, Value]:
        """The record's typed values by column name."""

    def __eq__(self, other: object, /) -> bool: ...
    def __repr__(self) -> str: ...

@final
class FilingIterator:
    """One pass over a filing's records after the cover; see
    `FilingReader.itemizations` and `FilingReader.itemization_records`."""

    def __iter__(self) -> Self: ...
    def __next__(self) -> Any: ...
    def __repr__(self) -> str: ...

@final
class FilingReader:
    """A streaming reader over one filing.

    Built by `open()`.  The `HDR` and cover records are parsed eagerly, so
    `header`, `cover` and the filer attributes are available up front.  The
    records after the cover are streamed by `itemizations` (typed) or
    `itemization_records` (raw), a batch at a time with the GIL released.
    There is no `len()` — the record count is unknown without a full pass.
    """

    @property
    def header(self) -> Header:
        """The filing's `HDR` record; the same object every time."""

    @property
    def cover(self) -> CoverData | Unknown:
        """The typed cover (`Form3X`, `Form1`, … from `libfec.covers`), or
        `Unknown` for a form with no typed class; the same object every time."""

    @property
    def cover_record(self) -> Record:
        """The cover line as a raw `Record`; the same object every time."""

    @property
    def itemizations(self) -> Iterator[Itemization | Unknown]:
        """Every record after the cover, typed, in file order: `ScheduleA`, …
        from `libfec.itemizations`, or `Unknown` for a record with no class.

        Each access starts a new pass from the top of the filing.  A filing
        opened from a file object can be read only once; a second pass raises
        `ValueError`.
        """

    @property
    def itemization_records(self) -> Iterator[Record]:
        """Every record after the cover as a raw `Record`, in file order — the
        same records as `itemizations`, one-to-one.

        Each access starts a new pass, as with `itemizations`.
        """

    @property
    def form_type(self) -> str:
        """The cover's form type, e.g. `F3XN`."""

    @property
    def filer_id(self) -> str:
        """The filer's FEC id, whatever the form calls it."""

    @property
    def filer_name(self) -> str:
        """The filer's name, whatever the form calls it."""

    @property
    def report_code(self) -> str | None:
        """`Q1`, `M8`, …; `None` for a form that is not a periodic report."""

    @property
    def coverage_from_date(self) -> date | None: ...
    @property
    def coverage_through_date(self) -> date | None: ...
    @property
    def id(self) -> str | None:
        """The file stem of a path source, or of a file object's `name`
        (`FEC-` stripped); `None` when the source does not name itself."""

    @property
    def fec_version(self) -> str:
        """Shortcut for `header.fec_version`."""

    @property
    def source_length(self) -> int:
        """The source's size in bytes, `0` if unknown."""

    @property
    def closed(self) -> bool: ...
    def close(self) -> None:
        """Drop the source and stop every pass.  Idempotent; iterating
        afterwards raises `ValueError`."""

    def __enter__(self) -> Self: ...
    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
        /,
    ) -> None: ...
    def __repr__(self) -> str: ...

def open(source: Source, /) -> FilingReader:
    """Open a filing for streaming.

    `source` is a filesystem path (`str` or `os.PathLike`), a bytes-like object
    (`bytes`, `bytearray`, `memoryview`, `mmap` — read without copying), or a
    binary file object, which is read a chunk at a time.  A missing path raises
    `FileNotFoundError`, a text-mode file raises `TypeError`, and unparseable
    input raises `FecParseError`.  An exception raised by the source's own
    `read()` propagates unchanged.
    """

def fec_header(source: Source, /) -> str:
    """The `fec_version` of a filing, from anything `open()` accepts."""

class FecError(ValueError):
    """Base class for libfec errors."""

class FecParseError(FecError):
    """The input is not a parseable .fec filing."""

class MissingMappingError(FecError):
    """Raised by iteration for a row whose ``(row_type, fec_version)`` has no column mapping."""

    row_type: str
    version: str
    line: int
    def __init__(self, row_type: str, version: str, line: int) -> None: ...
