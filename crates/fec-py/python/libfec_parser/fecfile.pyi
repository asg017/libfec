"""Type stubs for :mod:`libfec_parser.fecfile`.

Hand-maintained and checked against the built extension by
`python -m mypy.stubtest` — see `crates/fec-py/Makefile`'s `stubs` target.  The
implementation is `src/fecfile.rs`; the shapes mirror the `fecfile` package's
API.  Every value in a parsed filing is a raw `str` today (no type coercion),
which is why the dicts below are `dict[str, str]`.
"""

from typing import TypedDict

__all__ = [
    "from_file",
    "from_http",
    "loads",
    "parse_header",
    "parse_line",
    "print_example",
]

class Options(TypedDict, total=False):
    """Parse options, all optional.

    `filter_itemizations` keeps only the rows whose type starts with one of the
    given prefixes (`[]` keeps none); `as_strings` is accepted for `fecfile`
    compatibility and currently has no effect (values are always strings).
    """

    filter_itemizations: list[str]
    as_strings: bool

class Parsed(TypedDict):
    """The parsed filing: header, cover, itemizations grouped by schedule, TEXT rows."""

    header: dict[str, str]
    filing: dict[str, str]
    itemizations: dict[str, list[dict[str, str]]]
    text: list[dict[str, str]]

def loads(
    input: str | bytes | bytearray | memoryview | list[str],
    options: Options | None = None,
) -> Parsed:
    """Parse filing contents held in memory."""

def from_file(file_path: str, options: Options | None = None) -> Parsed:
    """Parse the filing at `file_path` (a `str`, not `os.PathLike`)."""

def from_http(
    file_number: int | str, options: Options | None = None
) -> Parsed | None:
    """Download and parse a filing from docquery.fec.gov; `None` if not found."""

def parse_header(hdr: str | list[str]) -> tuple[dict[str, str], str, int]:
    """Parse an `HDR` line into `(header, fec_version, lines_consumed)`."""

def parse_line(
    line: str, version: str, _line_num: int | None = None
) -> dict[str, str]:
    """Parse one row against the column mapping for `version`."""

def print_example(parsed: Parsed) -> None:
    """Print `parsed` as JSON, keeping only the first row of each schedule."""
