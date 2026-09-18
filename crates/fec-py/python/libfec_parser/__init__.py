"""Python bindings for libfec's .fec parser."""

from importlib.metadata import version as _version

from . import fecfile, parser
from .parser import (
    Cover,
    FecError,
    FecParseError,
    FilingReader,
    Header,
    MissingMappingError,
    Row,
    fec_header,
    open,
)

__version__ = _version("libfec-parser")

__all__ = [
    "fecfile",
    "parser",
    "Cover",
    "FecError",
    "FecParseError",
    "FilingReader",
    "Header",
    "MissingMappingError",
    "Row",
    "fec_header",
    "open",
]
