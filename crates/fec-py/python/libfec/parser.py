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

__all__ = ["Cover", "Filing", "Header", "Itemization", "fec_header"]
