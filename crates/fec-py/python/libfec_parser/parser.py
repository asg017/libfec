"""Parsing primitives for FEC electronic filings."""

from collections.abc import Mapping

# `_native` is a single extension module; `_native.parser` is an attribute of it,
# not an importable submodule, so it is bound by attribute access rather than
# `from ._native.parser import ...`.
from ._native import parser as _parser

Cover = _parser.Cover
Filing = _parser.Filing
Header = _parser.Header
Row = _parser.Row
fec_header = _parser.fec_header

FecError = _parser.FecError
FecParseError = _parser.FecParseError


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

__all__ = [
    "Cover",
    "Filing",
    "Header",
    "Row",
    "fec_header",
    "FecError",
    "FecParseError",
    "MissingMappingError",
]
