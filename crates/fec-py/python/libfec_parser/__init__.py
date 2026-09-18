"""Python bindings for libfec's .fec parser."""

from importlib.metadata import version as _version

from . import fecfile, parser

__version__ = _version("libfec-parser")

__all__ = ["fecfile", "parser"]
