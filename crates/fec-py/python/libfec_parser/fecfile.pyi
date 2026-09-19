"""Type stubs for :mod:`libfec_parser.fecfile`.

Hand-maintained and checked against the built package by `python -m mypy.stubtest`
— see `crates/fec-py/Makefile`'s `stubs` target.  The implementation is
`python/libfec_parser/fecfile.py`, pure Python over `libfec_parser.open()`; the
shapes are `fecfile` 0.9.1's, down to the types inside a parsed row.
"""

from collections.abc import Generator, Iterable, Mapping
from datetime import datetime
from typing import Any, TypeAlias

from .parser import FecError

__all__ = [
    "FecItem",
    "FecParserMissingMappingError",
    "FecParserTypeWarning",
    "FilingUnavailableError",
    "from_file",
    "from_http",
    "iter_file",
    "iter_http",
    "iter_lines",
    "loads",
    "parse_header",
    "parse_line",
    "print_example",
]

Value: TypeAlias = str | float | int | datetime | None
"""A column's value: a `float`, an `int`, a tz-aware `datetime` in US/Eastern or
`None` where the vendored type table types the column, the raw `str` otherwise."""

Options: TypeAlias = Mapping[str, Any]
"""The `options` dict: `filter_itemizations`, a list of row-type prefixes to keep
(`[]` keeps none, leaving just the header and the filing), and `as_strings`, which
turns off type coercion.  Validated at the call, not by the type checker: a
`TypedDict` here would reject every `dict` literal a caller builds elsewhere."""

Parsed: TypeAlias = dict[str, Any]
"""A whole parsed filing: `itemizations`, `text`, `header`, `filing`, in that
order, plus `F99_text` for a form 99 with a `[BEGINTEXT]` block."""

class FecParserTypeWarning(UserWarning):
    """When a value in a filing does not parse as the type the table claims."""

class FecParserMissingMappingError(FecError):
    """When a line's `(form, version)` pair has no column mapping."""

    def __init__(self, opts: Mapping[str, str], msg: str | None = None) -> None: ...

class FilingUnavailableError(FecError):
    """When neither the electronic nor the paper URL for a filing returns 200."""

    def __init__(self, opts: Mapping[str, Any], msg: str | None = None) -> None: ...

class FecItem:
    """One piece of a filing: `data_type` and `data`."""

    data_type: str
    data: Any
    def __init__(self, data_type: str, data: Any) -> None: ...
    def __repr__(self) -> str: ...

def loads(
    input: str | bytes | bytearray | memoryview | Iterable[str | bytes],
    options: Options | None = None,
) -> Parsed:
    """Parse filing contents held in memory, or any iterable of lines."""

def from_file(file_path: Any, options: Options | None = None) -> Parsed:
    """Parse the filing at `file_path` (a path `str` or `os.PathLike`)."""

def iter_file(
    file_path: Any, options: Options | None = None
) -> Generator[FecItem, None, None]:
    """Stream the filing at `file_path` as `FecItem`s, a batch of rows at a time."""

def iter_lines(
    lines: Iterable[str | bytes], options: Options | None = None
) -> Generator[FecItem, None, None]:
    """Stream `FecItem`s from an iterable of `str` or `bytes` lines."""

def from_http(
    file_number: int | str, options: Options | None = None
) -> Parsed | None:
    """Download and parse a filing from docquery.fec.gov.

    `None` if both the electronic and the paper URL 404; raises
    `FilingUnavailableError` for any other non-200 status. Requires the
    `[http]` extra (`httpx2`); raises `ImportError` naming it otherwise.
    """

def iter_http(
    file_number: int | str, options: Options | None = None
) -> Generator[FecItem, None, None]:
    """Stream a filing from docquery.fec.gov as `FecItem`s, never buffering it.

    Raises `FilingUnavailableError` for any non-200 status. Requires the
    `[http]` extra (`httpx2`); raises `ImportError` naming it otherwise.
    """

def parse_header(hdr: str | list[str]) -> tuple[dict[str, Value] | None, str, int]:
    """Parse an `HDR` line into `(header, fec_version, lines_consumed)`."""

def parse_line(
    line: str, version: str, line_num: int | None = None
) -> dict[str, Value] | None:
    """Parse one row against the column mapping for `version`.

    `None` for a line with fewer than two fields.
    """

def print_example(parsed: Mapping[str, Any]) -> None:
    """Print `parsed` as JSON, keeping only the first row of each schedule."""

# Private, and stubbed only because `tests/test_fecfile_differential.py` compares
# the column names this hands back against real `fecfile`'s `getMapping` over the
# whole mapping space.  Not part of the API.
def _mapping(form: str, version: str) -> tuple[tuple[str, ...], tuple[Any, ...]]:
    """`(column names, per-column converters)` for one `(form, version)` pair."""
