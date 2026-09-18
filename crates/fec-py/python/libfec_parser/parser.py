"""Parsing primitives for FEC electronic filings."""

# `_native` is a single extension module; `_native.parser` is an attribute of it,
# not an importable submodule, so it is bound by attribute access rather than
# `from ._native.parser import ...`.
from ._native import parser as _parser

Cover = _parser.Cover
Filing = _parser.Filing
Header = _parser.Header
Itemization = _parser.Itemization
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


__all__ = [
    "Cover",
    "Filing",
    "Header",
    "Itemization",
    "fec_header",
    "FecError",
    "FecParseError",
    "MissingMappingError",
]
