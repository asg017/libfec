"""Type stubs for :mod:`libfec_parser.parser`.

Hand-maintained (pyo3's `experimental-inspect` output is the first draft only) and
checked against the built extension by `python -m mypy.stubtest` — see
`crates/fec-py/Makefile`'s `stubs` target.  The implementation is `src/parser.rs`.
"""

import os
from collections.abc import Iterator
from datetime import date
from types import TracebackType
from typing import Any, Self, TypeAlias, final, overload

__all__ = [
    "Cover",
    "FilingReader",
    "Header",
    "Row",
    "fec_header",
    "open",
    "FecError",
    "FecParseError",
    "MissingMappingError",
]

Value: TypeAlias = str | float | date | None
"""A column's value: typed if it parses, the raw `str` if it is garbage, `None` if empty."""

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
    def __repr__(self) -> str: ...

@final
class Cover:
    """The filing's cover record (F3, F3X, …)."""

    @property
    def form_type(self) -> str: ...
    @property
    def filer_id(self) -> str: ...
    @property
    def filer_name(self) -> str: ...
    @property
    def report_code(self) -> str | None: ...
    @property
    def coverage_from_date(self) -> date | None: ...
    @property
    def coverage_through_date(self) -> date | None: ...
    def fields(self) -> dict[str, str | date | None]:
        """The six cover attributes above as a dict."""

@final
class Row:
    """One itemization row: a mapping from column name to typed value.

    By name the value follows the value rule (`Value` above); by position it is
    the raw `str` exactly as it appears in the file.  `len(row)` is the number of
    mapped columns — `len(row.fields())` is the raw field count.
    """

    @property
    def row_type(self) -> str: ...
    @property
    def line(self) -> int:
        """The row's 1-based physical line in the file."""

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
        """Every raw field of the row, in file order."""

    def __eq__(self, other: object, /) -> bool: ...
    def __hash__(self) -> int: ...
    def __reduce__(self) -> tuple[Any, ...]: ...
    def __repr__(self) -> str: ...

def _row_from_parts(
    row_type: str, version: str, fields: list[str], line: int, /
) -> Row:
    """Rebuild a `Row` from its pickled parts; named by `Row.__reduce__`."""

@final
class FilingReader:
    """A streaming, single-pass reader over one filing.

    Built by `open()`.  The `HDR` and cover records are parsed eagerly, so
    `header`, `cover` and `cover_row` are available before iteration; the
    itemization rows are pulled lazily, a batch at a time, with the GIL released.
    There is no `len()` — the row count is unknown without a full pass.
    """

    @property
    def header(self) -> Header:
        """The filing's `HDR` record; the same object every time."""

    @property
    def cover(self) -> Cover:
        """The six normalized cover attributes; the same object every time."""

    @property
    def cover_row(self) -> Row:
        """The full cover line as a `Row`; the same object every time."""

    @property
    def id(self) -> str | None:
        """The file stem for a path source (`FEC-` stripped), `None` for bytes."""

    @property
    def fec_version(self) -> str:
        """Shortcut for `header.fec_version`."""

    @property
    def source_length(self) -> int:
        """The source's size in bytes, `0` if unknown."""

    @property
    def closed(self) -> bool: ...
    def rows(self, *prefixes: str) -> Self:
        """Only yield rows whose type starts with one of `prefixes` (case-insensitive).

        Returns the reader itself.  A second call replaces the filter; `rows()`
        with no arguments clears it.
        """

    def close(self) -> None:
        """Drop the source.  Idempotent; iterating afterwards raises `ValueError`."""

    def __enter__(self) -> Self: ...
    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
        /,
    ) -> None: ...
    def __iter__(self) -> Self: ...
    def __next__(self) -> Row: ...
    def __repr__(self) -> str: ...

def open(source: str | os.PathLike[str] | bytes, /) -> FilingReader:
    """Open a filing for streaming.

    `source` is a filesystem path (`str` or `os.PathLike`) or the filing's bytes.
    A missing path raises `FileNotFoundError`; unparseable input raises
    `FecParseError`.
    """

def fec_header(contents: bytes) -> str:
    """The `fec_version` of a filing held entirely in memory."""

class FecError(ValueError):
    """Base class for libfec_parser errors."""

class FecParseError(FecError):
    """The input is not a parseable .fec filing."""

class MissingMappingError(FecError):
    """Raised by iteration for a row whose ``(row_type, fec_version)`` has no column mapping."""

    row_type: str
    version: str
    line: int
    def __init__(self, row_type: str, version: str, line: int) -> None: ...
