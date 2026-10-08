"""`fecfile`-compatible API backed by libfec."""

# `_native` is a single extension module; `_native.fecfile` is an attribute of it,
# not an importable submodule, so it is bound by attribute access rather than
# `from ._native.fecfile import ...`.
from ._native import fecfile as _fecfile

from_file = _fecfile.from_file
from_http = _fecfile.from_http
loads = _fecfile.loads
parse_header = _fecfile.parse_header
parse_line = _fecfile.parse_line
print_example = _fecfile.print_example

__all__ = [
    "from_file",
    "from_http",
    "loads",
    "parse_header",
    "parse_line",
    "print_example",
]
