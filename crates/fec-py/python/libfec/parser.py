"""Parsing primitives for FEC electronic filings."""

# `open` below shadows the builtin for the rest of this module; reach the real one
# through `builtins.open`.
import builtins  # noqa: F401  (kept for modules that need the real `open`)
import mmap
import os
import warnings
from collections.abc import Iterator, Mapping
from typing import Protocol, TypeAlias

# `_native` is a single extension module; `_native.parser` is an attribute of it,
# not an importable submodule, so it is bound by attribute access rather than
# `from ._native.parser import ...`.
from ._native import parser as _parser

Cover = _parser.Cover
FilingReader = _parser.FilingReader
Header = _parser.Header
Row = _parser.Row
fec_header = _parser.fec_header
open = _parser.open

FecError = _parser.FecError
FecParseError = _parser.FecParseError


class _Readable(Protocol):
    """A binary file object: anything whose ``read(n)`` hands back ``bytes``."""

    def read(self, n: int, /) -> bytes: ...


# The union `open()`/`Filing()` accept. Defined at runtime (not stub-only like
# `Value`) so it doubles as the annotation on `Filing.__init__` below, and so
# `parser.pyi` can spell it identically. `collections.abc.Buffer` would say
# "bytes-like" in one word, but it is 3.12+ and the floor here is 3.11.
Source: TypeAlias = (
    str | os.PathLike[str] | bytes | bytearray | memoryview | mmap.mmap | _Readable
)


class MissingMappingError(FecError):
    """Raised by iteration for a row whose ``(row_type, fec_version)`` has no column mapping."""

    def __init__(self, row_type: str, version: str, line: int) -> None:
        super().__init__(row_type, version, line)
        self.row_type, self.version, self.line = row_type, version, line

    def __str__(self) -> str:
        return (
            f"no column mapping for row type {self.row_type!r} "
            f"in FEC version {self.version} (line {self.line})"
        )


# `Row.__reduce__` names this function, and pickle resolves a callable through its
# `__module__`.  `_native.parser` is an attribute, not an importable module, so point
# it at this module — the one place `_row_from_parts` can actually be imported from.
_row_from_parts = _parser._row_from_parts
_row_from_parts.__module__ = __name__

# pandas only treats list items as records if `isinstance(x, Mapping)`.
Mapping.register(Row)


class Filing:
    """A whole filing in memory: header, cover and every row, parsed once.

    ``read(source)`` is the same thing as a function.  For filings too large to hold,
    use :func:`open`, which streams.
    """

    __slots__ = ("id", "header", "cover", "cover_row", "rows")

    def __init__(self, source: Source) -> None:
        with open(source) as reader:  # this module's open(), not builtins.open
            self.id = reader.id
            self.header = reader.header
            self.cover = reader.cover
            self.cover_row = reader.cover_row
            self.rows: list[Row] = list(reader)  # MissingMappingError propagates (Q15: eager = strict)

    @property
    def fec_version(self) -> str:
        return self.header.fec_version

    @property
    def itemizations(self) -> list[Row]:
        warnings.warn("Filing.itemizations is deprecated; use Filing.rows", DeprecationWarning, stacklevel=2)
        return self.rows

    def __iter__(self) -> Iterator[Row]:
        return iter(self.rows)

    def __len__(self) -> int:
        return len(self.rows)

    def __repr__(self) -> str:
        return (
            f"Filing(id={self.id!r}, form_type={self.cover.form_type!r}, "
            f"filer_id={self.cover.filer_id!r}, {len(self.rows)} rows)"
        )


def read(source: Source) -> Filing:
    """Parse ``source`` eagerly; see :class:`Filing`."""
    return Filing(source)


__all__ = [
    "Cover",
    "Filing",
    "FilingReader",
    "Header",
    "Row",
    "fec_header",
    "open",
    "read",
    "FecError",
    "FecParseError",
    "MissingMappingError",
]
