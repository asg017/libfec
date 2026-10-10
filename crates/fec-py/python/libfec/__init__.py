"""Python bindings for libfec's .fec parser."""

from importlib.metadata import version as _version

from . import fecfile, parser, rss
from .parser import (
    Cover,
    FecError,
    FecParseError,
    Filing,
    FilingReader,
    Header,
    MissingMappingError,
    Row,
    fec_header,
    open,
    read,
)

__version__ = _version("libfec")

__all__ = [
    "fecfile",
    "parser",
    "rss",
    "Cover",
    "FecError",
    "FecParseError",
    "Filing",
    "FilingReader",
    "Header",
    "MissingMappingError",
    "Row",
    "fec_header",
    "open",
    "read",
]
