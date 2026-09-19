"""A drop-in `fecfile` API, in pure Python over :func:`libfec_parser.open`.

The spec is `fecfile` 0.9.1 (Apache-2.0), and "compatible" here means exact: the
same keys in the same order, with the same values *and* the same types, down to
tz-aware ``datetime``s in US/Eastern.  ``tests/test_fecfile_differential.py``
holds this module to that claim against the real package on every fixture and
documents the few differences that survive.

**Scope: FEC format versions 8.0-8.5**, which is what `fec-parser` reads.  The
whole-filing functions raise `FecParseError` on anything else -- an older
comma-delimited or 6.x/7.x filing, a paper ``P3.x`` one, a version string spelled
outside those six, a versions 1-2 ``/* ... */`` header -- where real `fecfile`
parses it.  :func:`parse_line` and :func:`parse_header` have no such limit; their
mappings match real's across every version real maps.  The README's "Where it
differs" is the full list, including how the two packages treat malformed input.

Values come from each row's **raw** fields plus `fecfile`'s own type table
(vendored as ``_fecfile_types.json``; see ``NOTICE``), never from libfec's typed
accessors: libfec types a handful of columns `fecfile` does not, and reads a
short row's missing columns as ``None`` where `fecfile` gives ``''``.  Going
through the vendored table is exact by construction and makes ``as_strings``
nothing more than "skip the converters".
"""

import contextlib
import csv
import json
import re
import warnings
import zoneinfo
from collections.abc import Callable, Iterable, Iterator, Mapping
from datetime import datetime
from importlib.resources import files
from typing import TYPE_CHECKING, Any, NamedTuple

if TYPE_CHECKING:
    import httpx2

from ._native import parser as _native_parser
from .parser import FecError as _FecError
from .parser import MissingMappingError as _MissingMappingError
from .parser import open as _open

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

#: docquery.fec.gov tries the "dcdev" (electronic) URL first, then falls back to
#: "paper" for filings that were only ever submitted on paper and scanned in.
_DCDEV_URL = "https://docquery.fec.gov/dcdev/posted/{n}.fec"
_PAPER_URL = "https://docquery.fec.gov/paper/posted/{n}.fec"

#: The ASCII 28 field separator every version since 6.x uses.
_COLUMN_SEPARATOR = "\x1c"

#: Format versions whose lines are comma-separated instead (`fecparser.py:36`).
_COMMA_VERSIONS = ("1", "2", "3", "5")

#: Spellings of "no value" a float column accepts (`fecparser.py:185`).
_NONES = ("none", "n/a")

_VALID_OPTIONS = ("filter_itemizations", "as_strings")

# HDR columns whose name differs from the `libfec_parser.Header` attribute holding
# them; every other column is same-named.  `Header` carries no `name_delim` (an HDR
# column in versions 3.x–5.x), which therefore reads as ''.
_HEADER_ATTRS = {"soft_name": "software_name", "soft_ver": "software_version"}

#: Placeholder column names `fec-parser` invents, and the name real `fecfile`'s
#: ``mappings.json`` has in that position.  This module's spec is `fecfile`'s
#: names, so every native name goes through `_spec_name` before it becomes a
#: dict key -- which is also what gives a duplicated column real's semantics:
#: with both copies under one name, `_record`'s plain assignment keeps the *last*
#: one's value at the *first* one's key position, exactly as real's loop does.
#: One `_TODO_DUP` suffix (a column the FEC layout names twice, such as an F3X
#: cover's ``col_a_total_receipts`` or an F2's ``candidate_state``) and one
#: `TODO_UNKNOWN_BLANK` (the F3L mapping's nameless column, ``''`` in real).
#: Deferred parser items: this table should shrink to nothing when `fec-parser`
#: stops inventing the names.
_DUP_SUFFIX = "_TODO_DUP"
_NATIVE_NAMES = {"TODO_UNKNOWN_BLANK": ""}

#: A column's converter: the raw field and the line number in, a typed value out.
_Converter = Callable[[str, "int | None"], Any]

_MAPPINGS: dict[tuple[str, str], tuple[tuple[str, ...], tuple[_Converter | None, ...]]] = {}
_TYPES: dict[str, Any] | None = None
_EASTERN: zoneinfo.ZoneInfo | None = None


class FecParserTypeWarning(UserWarning):
    """When a value in a filing does not parse as the type the table claims."""


class FecParserMissingMappingError(_FecError):
    """When a line's ``(form, version)`` pair has no column mapping.

    Constructed from an options dict, like the real package's; a `FecError` (so a
    `ValueError`) rather than a bare `Exception`, to match the rest of libfec.
    """

    def __init__(self, opts: Mapping[str, str], msg: str | None = None) -> None:
        if msg is None:
            msg = "cannot parse version {v} of form {f} - no mapping found".format(
                v=opts["version"], f=opts["form"]
            )
        super().__init__(msg)


class FilingUnavailableError(_FecError):
    """When neither the electronic nor the paper URL for a filing returns 200.

    Real `fecfile` doesn't distinguish a 404 (no such filing) from a 500 (the
    server is unhappy); this doesn't either, per its message
    (`fecfile/__init__.py:8-19`).
    """

    def __init__(self, opts: Mapping[str, Any], msg: str | None = None) -> None:
        if msg is None:
            msg = (
                "The requested FEC file number ({}) is unavailable. "
                "Status code {}.".format(opts["file_number"], opts["status_code"])
            )
        super().__init__(msg)


class FecItem:
    """One piece of a filing: ``data_type`` and ``data``.

    ``data_type`` is ``"header"``, ``"summary"``, ``"itemization"``, ``"text"`` or
    ``"F99_text"``; ``data`` is a dict for everything but ``F99_text``.
    """

    __slots__ = ("data_type", "data")

    def __init__(self, data_type: str, data: Any) -> None:
        self.data_type = data_type
        self.data = data

    def __repr__(self) -> str:
        return f"FecItem(data_type={self.data_type!r}, ...)"


class _Options(NamedTuple):
    """Validated ``options``.  ``prefixes is None`` means no ``filter_itemizations``."""

    prefixes: tuple[str, ...] | None
    as_strings: bool


class _IterReader:
    """A minimal binary file object over an iterator of lines or byte chunks.

    `open()` streams from anything whose ``read(n)`` hands back bytes, which is
    how an iterable of lines — real `fecfile`'s input for ``iter_lines`` and for
    an HTTP response — reaches the parser.  With ``lines=True`` each piece is
    normalized into exactly one ``\\n``-terminated line, so lines with or without
    their own terminators both work; with ``lines=False`` byte chunks pass
    through untouched.
    """

    def __init__(self, pieces: Iterable[str | bytes], *, lines: bool = True) -> None:
        self._pieces = iter(pieces)
        self._lines = lines
        self._pending = b""

    def _normalize(self, piece: str | bytes) -> bytes:
        if isinstance(piece, str):
            piece = piece.encode("utf-8")
        elif not isinstance(piece, (bytes, bytearray, memoryview)):
            raise TypeError(f"lines must be str or bytes, not {type(piece).__name__}")
        else:
            piece = bytes(piece)
        if not self._lines:
            return piece
        return piece.removesuffix(b"\n").removesuffix(b"\r") + b"\n"

    def read(self, n: int = -1, /) -> bytes:
        # Pieces accumulate in a list, not by `+=` on a bytes: a 64 KiB read is
        # hundreds of lines, and concatenating each one would copy the buffer again.
        parts = [self._pending]
        size = len(self._pending)
        self._pending = b""
        while n < 0 or size < n:
            piece = next(self._pieces, None)
            if piece is None:
                break
            normalized = self._normalize(piece)
            parts.append(normalized)
            size += len(normalized)
        buffer = b"".join(parts)
        if n < 0 or size <= n:
            return buffer
        self._pending = buffer[n:]
        return buffer[:n]


def _type_table() -> dict[str, Any]:
    """`fecfile`'s ``types.json``, read once on first use."""
    global _TYPES
    if _TYPES is None:
        text = files(__package__).joinpath("_fecfile_types.json").read_text(encoding="utf-8")
        _TYPES = json.loads(text)
    return _TYPES


def _eastern() -> zoneinfo.ZoneInfo:
    """US/Eastern, the zone every date in a filing is localized to.

    Real `fecfile` uses `pytz`; `zoneinfo` keeps this package dependency-free and
    gives the same instant, so the two compare equal.  Built on first use, not at
    import, so a platform without a tz database only fails for filings that
    actually contain a date.
    """
    global _EASTERN
    if _EASTERN is None:
        try:
            _EASTERN = zoneinfo.ZoneInfo("America/New_York")
        except zoneinfo.ZoneInfoNotFoundError as e:
            raise ImportError(
                "libfec_parser.fecfile needs the 'tzdata' package on this platform: "
                "pip install tzdata"
            ) from e
    return _EASTERN


def _spec_name(name: str) -> str:
    """One native column name, as real `fecfile`'s ``mappings.json`` spells it."""
    return _NATIVE_NAMES.get(name, name.removesuffix(_DUP_SUFFIX))


def _type_prop(form: str, version: str, field: str) -> dict[str, str] | None:
    """The type table's entry for one column, or `None` if it has none.

    `getTypeMapping_from_regex` (`fecfile/cache.py:50-63`): first match in dict
    order at each of the three levels, case-insensitively, and a form that
    matches but yields no field match does not stop the search.
    """
    for form_re, versions in _type_table().items():
        if re.match(form_re, form, re.IGNORECASE):
            for version_re, properties in versions.items():
                if re.match(version_re, version, re.IGNORECASE):
                    for field_re, prop in properties.items():
                        if re.match(field_re, field, re.IGNORECASE):
                            return prop
    return None


def _converter(form: str, version: str, field: str, prop: dict[str, str]) -> _Converter:
    """One column's `getTyped` (`fecfile/fecparser.py:188-224`), resolved up front."""
    kind = prop["type"]
    fmt = prop.get("format")

    def convert(value: str, line_num: int | None) -> Any:
        try:
            if kind == "integer":
                return int(value)
            if kind == "float":
                stripped = value.strip()
                if stripped == "" or stripped.lower() in _NONES:
                    return None
                return float(stripped.replace("%", ""))
            if kind == "date":
                stripped = value.strip()
                if stripped == "":
                    return None
                return datetime.strptime(stripped, fmt).replace(tzinfo=_eastern())
        except ValueError:
            warnings.warn(
                "cannot parse value: {v}, as type: {t}, for field: {f}, "
                "in form: {o}, version: {r} (line {n})".format(
                    v=value,
                    t=kind,
                    f=field,
                    o=form,
                    r=version,
                    n="unknown" if line_num is None else line_num + 1,
                ),
                FecParserTypeWarning,
            )
            return None
        # A type the table names but `getTyped` has no branch for: the raw string.
        return value

    return convert


def _mapping(form: str, version: str) -> tuple[tuple[str, ...], tuple[_Converter | None, ...]]:
    """``(column names, per-column converters)`` for one ``(form, version)`` pair.

    Names are the spec's, not the native side's (`_spec_name`), and the
    converters are looked up under those same names, which is how a duplicated
    column ends up typed the way real types it.  Cached: the walk over the type
    table is three levels of regexes, far too slow to repeat per field per row,
    and one shared tuple per ``(form, version)`` means every row's dict shares
    its keys.  ``None`` in place of a converter is a plain string column.
    """
    key = (form, version)
    cached = _MAPPINGS.get(key)
    if cached is not None:
        return cached

    native_names = _native_parser._column_names(form, version)
    if native_names is None:
        raise FecParserMissingMappingError({"form": form, "version": version})
    names = [_spec_name(name) for name in native_names]
    converters: list[_Converter | None] = []
    for name in names:
        prop = _type_prop(form, version, name)
        converters.append(_converter(form, version, name, prop) if prop else None)

    entry = (tuple(names), tuple(converters))
    _MAPPINGS[key] = entry
    return entry


def _record(
    fields: list[str],
    names: tuple[str, ...],
    converters: tuple[_Converter | None, ...] | None,
    line_num: int | None,
) -> dict[str, Any]:
    """One line as a dict: every mapped column, in mapping order.

    A row shorter than its mapping reads ``''`` for the missing columns and types
    that like any other value; fields past the mapping are dropped, because
    `fecparser.parse_line` loops over the mapping rather than over the fields.
    A column the mapping names twice is written twice, so the last occurrence's
    value wins at the first occurrence's key position -- real's semantics, and
    the reason `_mapping` hands back the spec's names rather than the native
    side's disambiguated ones.  ``converters=None`` is ``as_strings``: no typing
    at all.
    """
    count = len(fields)
    if converters is None:
        return {name: fields[i] if i < count else "" for i, name in enumerate(names)}
    out: dict[str, Any] = {}
    for i, (name, convert) in enumerate(zip(names, converters)):
        value = fields[i] if i < count else ""
        out[name] = convert(value, line_num) if convert else value
    return out


def _check_options(options: Mapping[str, Any] | None) -> _Options:
    """Validate ``options`` up front rather than ignoring what we don't understand."""
    if options is None:
        return _Options(None, False)
    if not isinstance(options, Mapping):
        raise TypeError(f"options must be a dict, not {type(options).__name__}")
    for key in options:
        if key not in _VALID_OPTIONS:
            raise ValueError(
                f"unknown option {key!r}; valid options are "
                f"{' and '.join(repr(k) for k in _VALID_OPTIONS)}"
            )

    prefixes = options.get("filter_itemizations")
    if prefixes is not None:
        if isinstance(prefixes, str) or not isinstance(prefixes, (list, tuple)):
            raise TypeError(
                "filter_itemizations must be a list of str, e.g. ['SA', 'SB'], "
                f"not {type(prefixes).__name__}"
            )
        if not all(isinstance(prefix, str) for prefix in prefixes):
            raise TypeError("filter_itemizations must be a list of str")
        # Upper-cased so ['sb'] works: libfec's own prefix filter is
        # case-insensitive, real fecfile's is not, and this is the kinder of the two.
        prefixes = tuple(prefix.upper() for prefix in prefixes)

    as_strings = options.get("as_strings", False)
    if not isinstance(as_strings, bool):
        raise TypeError(f"as_strings must be a bool, not {type(as_strings).__name__}")
    return _Options(prefixes, as_strings)


def _uses_ascii_28(version: str | None) -> bool:
    """Whether ``version``'s lines are ASCII 28 separated (`fecparser.py:166-168`)."""
    return version is not None and version[:1] not in _COMMA_VERSIONS


def _fields_from_line(line: str, use_ascii_28: bool = False) -> list[str]:
    """Split one line into raw fields (`fecfile/fecparser.py:119-131`).

    ASCII 28 if the line has any, or if the version says so; comma-separated
    otherwise.  One layer of surrounding double quotes comes off either way.
    """
    if _COLUMN_SEPARATOR in line or use_ascii_28:
        fields = line.split(_COLUMN_SEPARATOR)
    else:
        fields = next(csv.reader([line]), [])
    return [
        field[1:-1] if field.startswith('"') and field.endswith('"') else field
        for field in fields
    ]


def _header_record(header: Any, version: str, options: _Options) -> dict[str, Any]:
    """The ``header`` item, built from the native `Header`.

    `open()` has already consumed the HDR line, so there are no raw fields to
    read; the columns the HDR mapping names are filled from `Header`'s attributes
    instead (see `_HEADER_ATTRS`).  A column `Header` does not carry, and one it
    carries as `None` because the filing left it empty, both read as ``''`` —
    which is what real `fecfile` gives for a field a short HDR line omits.
    """
    names, converters = _mapping(header.record_type, version)
    fields = [getattr(header, _HEADER_ATTRS.get(name, name), None) or "" for name in names]
    return _record(fields, names, None if options.as_strings else converters, 0)


def _fields_record(
    fields: list[str], version: str, options: _Options, line: int | None
) -> dict[str, Any]:
    """One line's raw fields as a `fecfile` dict."""
    names, converters = _mapping(fields[0].strip(), version)
    return _record(fields, names, None if options.as_strings else converters, line)


def _row_record(row: Any, version: str, options: _Options) -> dict[str, Any]:
    """One `Row` as a `fecfile` dict, from its raw fields."""
    return _fields_record(row.fields(), version, options, row.line)


def _iter_items(reader: Any, options: _Options) -> Iterator[FecItem]:
    """Every `FecItem` of an open reader: the header, the summary, then the rows.

    A row is an ``itemization`` if its dict has a ``form_type`` key and ``text``
    otherwise — real `fecfile`'s own rule (`fecparser.py:109-113`), which works
    because the ``TEXT`` mapping calls that column ``rec_type``.
    ``filter_itemizations`` applies from the summary on, to text lines as much as
    to itemizations, again like real.

    Two kinds of line are not records at all, and are skipped rather than turned
    into an item or an error, because that is what real does with them: a line
    with fewer than two fields (``parse_line`` returns ``None``, `:170-171`), and
    a whitespace-only line, which reaches the native reader as a
    `MissingMappingError` for a blank row type.  The reader stays usable after
    raising, so the loop below pulls rows with `next` rather than ``for``: a
    ``for`` would hand the exception out through its own ``__next__`` call and
    end the loop.
    """
    version = reader.fec_version
    yield FecItem("header", _header_record(reader.header, version, options))
    yield FecItem("summary", _row_record(reader.cover_row, version, options))

    if options.prefixes is not None and not options.prefixes:
        # `{'filter_itemizations': []}`: the header and the summary, nothing else.
        # `reader.rows()` with no prefixes would clear the filter, not reject
        # everything, so this case never reaches the reader.
        return
    rows = iter(reader if options.prefixes is None else reader.rows(*options.prefixes))
    while True:
        try:
            row = next(rows)
        except StopIteration:
            return
        except _MissingMappingError as e:
            if e.row_type.strip() == "":
                continue  # a whitespace-only line: not a record, skip it
            raise FecParserMissingMappingError(
                {"form": e.row_type, "version": e.version}
            ) from e
        fields = row.fields()
        if len(fields) < 2:
            continue  # fewer than two fields: not a record either
        record = _fields_record(fields, version, options, row.line)
        yield FecItem("itemization" if "form_type" in record else "text", record)


def _assemble(items: Iterator[FecItem]) -> dict[str, Any]:
    """Collect items into the dict `fecparser.loads` returns (`:46-66`).

    Key order is real's — ``itemizations``, ``text``, ``header``, ``filing``, then
    ``F99_text`` if the filing has one — and the itemization groups come out in
    file order, because a plain dict keeps insertion order.
    """
    out: dict[str, Any] = {"itemizations": {}, "text": [], "header": {}, "filing": {}}
    for item in items:
        if item.data_type == "header":
            out["header"] = item.data
        elif item.data_type == "summary":
            out["filing"] = item.data
        elif item.data_type == "F99_text":
            out["F99_text"] = item.data
        elif item.data_type == "text":
            out["text"].append(item.data)
        elif item.data_type == "itemization":
            form_type = item.data["form_type"]
            if form_type[0] == "S":
                form_type = "Schedule " + form_type[1]
            out["itemizations"].setdefault(form_type, []).append(item.data)
    return out


def loads(
    input: str | bytes | bytearray | memoryview | Iterable[str | bytes],
    options: Mapping[str, Any] | None = None,
) -> dict[str, Any]:
    """Deserialize filing contents held in memory.

    ``input`` is the whole document as a ``str`` or a bytes-like object, or any
    iterable of lines (with or without their newlines).  ``options`` takes
    ``filter_itemizations``, a list of row-type prefixes to keep (``[]`` keeps
    none, so only the header and the filing come back), and ``as_strings``, which
    turns off type coercion.
    """
    opts = _check_options(options)
    if isinstance(input, str):
        source: Any = input.encode("utf-8")
    elif isinstance(input, (bytes, bytearray, memoryview)):
        source = input
    elif isinstance(input, Iterable):
        source = _IterReader(input)
    else:
        raise TypeError(
            "input must be a str, a bytes-like object, or an iterable of lines, "
            f"not {type(input).__name__}"
        )
    with _open(source) as reader:
        return _assemble(_iter_items(reader, opts))


def from_file(file_path: Any, options: Mapping[str, Any] | None = None) -> dict[str, Any]:
    """Parse the filing at ``file_path``; see :func:`loads` for ``options``."""
    opts = _check_options(options)
    with _open(file_path) as reader:
        return _assemble(_iter_items(reader, opts))


def iter_file(file_path: Any, options: Mapping[str, Any] | None = None) -> Iterator[FecItem]:
    """Stream the filing at ``file_path`` as :class:`FecItem`s.

    Never holds more than a batch of rows, so this is the way to read a filing
    too large for :func:`from_file`.  Closing the generator closes the file.
    """
    opts = _check_options(options)
    with _open(file_path) as reader:
        yield from _iter_items(reader, opts)


def iter_lines(
    lines: Iterable[str | bytes], options: Mapping[str, Any] | None = None
) -> Iterator[FecItem]:
    """Stream :class:`FecItem`s from an iterable of ``str`` or ``bytes`` lines.

    Each line's trailing newline is optional, and ``str`` and ``bytes`` lines may
    be mixed; anything else raises `TypeError`.
    """
    opts = _check_options(options)
    with _open(_IterReader(lines)) as reader:
        yield from _iter_items(reader, opts)


def _client() -> "httpx2.Client":
    """A configured `httpx2` client, imported lazily.

    `httpx2` backs only `from_http`/`iter_http` and is an optional dependency
    (the `[http]` extra) — importing it here, rather than at module load, keeps
    the rest of this module usable without it installed.
    """
    try:
        import httpx2
    except ImportError as e:
        raise ImportError(
            "libfec_parser.fecfile.from_http needs httpx2: "
            "pip install 'libfec-parser[http]'"
        ) from e
    from . import __version__

    return httpx2.Client(
        timeout=30.0,
        follow_redirects=True,
        headers={"User-Agent": f"libfec_parser/{__version__}"},
    )


@contextlib.contextmanager
def _fetch(file_number: int | str) -> "Iterator[httpx2.Response]":
    """A streamed response for ``file_number``, closed with its client on exit.

    Tries the electronic ("dcdev") URL first and the paper URL if that is a 404.
    Whatever the second URL answers is handed over as-is: the two callers
    disagree about what a 404 means.
    """
    with _client() as client:
        for url in (_DCDEV_URL, _PAPER_URL):
            with client.stream("GET", url.format(n=file_number)) as response:
                if response.status_code == 404 and url is _DCDEV_URL:
                    continue
                yield response
                return


def _iter_response(
    response: "httpx2.Response", file_number: int | str, opts: _Options
) -> Iterator[FecItem]:
    """The items of a 200 response, streamed; `FilingUnavailableError` otherwise."""
    if response.status_code != 200:
        raise FilingUnavailableError(
            {"file_number": file_number, "status_code": response.status_code}
        )
    with _open(_IterReader(response.iter_bytes(), lines=False)) as reader:
        yield from _iter_items(reader, opts)


def from_http(
    file_number: int | str, options: Mapping[str, Any] | None = None
) -> dict[str, Any] | None:
    """Download and parse a filing from docquery.fec.gov.

    Tries the electronic URL, then the paper URL on a 404. ``None`` if both are
    404 (real `fecfile`'s behaviour — the idiom in the wild is
    ``if fecfile.from_http(n) is None``); any other non-200 raises
    :class:`FilingUnavailableError` instead of trying to parse an error page as
    a filing. Network errors (DNS, TLS, timeout, …) propagate as `httpx2`
    exceptions. Requires the ``[http]`` extra (`httpx2`); see :func:`iter_http`
    for a streaming version that never holds the whole filing in memory.
    """
    opts = _check_options(options)
    with _fetch(file_number) as response:
        if response.status_code == 404:
            return None
        return _assemble(_iter_response(response, file_number, opts))


def iter_http(
    file_number: int | str, options: Mapping[str, Any] | None = None
) -> Iterator[FecItem]:
    """Stream a filing from docquery.fec.gov as :class:`FecItem`s.

    Tries the electronic URL, then the paper URL on a 404; any status other
    than 200 raises :class:`FilingUnavailableError`. Never buffers the
    response body — the first item can arrive before the download finishes.
    The response and the client are closed whether the generator runs to
    completion or is closed early (``gen.close()``). Requires the ``[http]``
    extra (`httpx2`).
    """
    opts = _check_options(options)
    with _fetch(file_number) as response:
        yield from _iter_response(response, file_number, opts)


def parse_header(hdr: str | list[str]) -> tuple[dict[str, Any] | None, str, int]:
    """Parse a filing's header into ``(header, version, lines_consumed)``.

    ``hdr`` is the ``HDR`` line, or the file's lines as a list.
    ``lines_consumed`` only ever tells you anything for versions 1 and 2, whose
    header was a multi-line ``/* ... /*`` block — a form `fec_parser` does not
    read, so this raises :class:`FecParserMissingMappingError` for it.
    """
    if isinstance(hdr, str):
        lines = [hdr]
    elif isinstance(hdr, list):
        lines = hdr
    else:
        raise TypeError(f"hdr must be a str or a list of str, not {type(hdr).__name__}")

    if lines[0].startswith("/*"):
        raise FecParserMissingMappingError(
            {"form": "/* ... /* header", "version": "1 or 2"},
            "the multi-line header of FEC file format versions 1 and 2 is not supported",
        )

    fields = _fields_from_line(lines[0])
    if len(fields) < 2:
        # Real fecfile reaches `fields[1]` and raises IndexError here; say what is
        # wrong instead.
        raise ValueError(f"not an FEC header line: {lines[0]!r}")
    # 'HDR<sep>FEC<sep>3.00...' in the older versions, 'HDR<sep>8.5...' since.
    version = fields[2] if fields[1] == "FEC" else fields[1]
    return parse_line(lines[0], version, 0), version, 1


def parse_line(line: str, version: str, line_num: int | None = None) -> dict[str, Any] | None:
    """Parse one line against the column mapping for ``version``.

    ``None`` for a line with fewer than two fields, which is how real `fecfile`
    says "not a record".  ``line_num`` only shows up in the warning a value that
    does not parse produces.
    """
    fields = _fields_from_line(line, use_ascii_28=_uses_ascii_28(version))
    if len(fields) < 2:
        return None
    form = fields[0].strip()
    names, converters = _mapping(form, version)
    return _record(fields, names, converters, line_num)



def print_example(parsed: Mapping[str, Any]) -> None:
    """Print ``parsed`` as JSON, keeping only the first row of each schedule."""
    out: dict[str, Any] = {"filing": parsed["filing"], "itemizations": {}}
    for k in parsed["itemizations"].keys():
        out["itemizations"][k] = parsed["itemizations"][k][0]
    print(json.dumps(out, sort_keys=True, indent=2, default=str))
