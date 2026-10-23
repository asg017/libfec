"""Python bindings for libfec's .fec parser."""

from importlib.metadata import version as _version

from . import fecfile, parser, rss
from .parser import (
    FecError,
    FecParseError,
    Filing,
    Header,
    MissingMappingError,
    Record,
    Unknown,
    fec_header,
)

__version__ = _version("libfec")

__all__ = [
    "fecfile",
    "parser",
    "rss",
    "FecError",
    "FecParseError",
    "Filing",
    "Header",
    "MissingMappingError",
    "Record",
    "Unknown",
    "fec_header",
]
