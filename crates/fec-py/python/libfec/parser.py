"""Parsing primitives for FEC electronic filings."""

# `open` below shadows the builtin for the rest of this module; reach the real one
# through `builtins.open`.
import builtins  # noqa: F401  (kept for modules that need the real `open`)
import mmap
import os
from collections.abc import Mapping
from typing import Protocol, TypeAlias

# `_native` is a single extension module; `_native.parser` is an attribute of it,
# not an importable submodule, so it is bound by attribute access rather than
# `from ._native.parser import ...`.
from ._native import parser as _parser

FilingIterator = _parser.FilingIterator
FilingReader = _parser.FilingReader
Header = _parser.Header
Record = _parser.Record
Unknown = _parser.Unknown
fec_header = _parser.fec_header
open = _parser.open

FecError = _parser.FecError
FecParseError = _parser.FecParseError


class _Readable(Protocol):
    """A binary file object: anything whose ``read(n)`` hands back ``bytes``."""

    def read(self, n: int, /) -> bytes: ...


# The union `open()` accepts. Defined at runtime (not stub-only like `Value`) so
# callers can import it for their own annotations, and so `parser.pyi` can spell
# it identically. `collections.abc.Buffer` would say
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


# `Record.__reduce__` names this function, and pickle resolves a callable through its
# `__module__`.  `_native.parser` is an attribute, not an importable module, so point
# it at this module — the one place `_record_from_parts` can actually be imported from.
_record_from_parts = _parser._record_from_parts
_record_from_parts.__module__ = __name__

# pandas only treats list items as records if `isinstance(x, Mapping)`.
Mapping.register(Record)


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
