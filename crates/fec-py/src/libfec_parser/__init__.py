# Import the native extension and expose submodules
from . import libfec_parser as _libfec_parser
import sys

# Make submodules accessible as libfec_parser.parser, libfec_parser.foo, and libfec_parser.fecfile
parser = _libfec_parser.parser
foo = _libfec_parser.foo
fecfile = _libfec_parser.fecfile

# Also register them in sys.modules for "from libfec_parser.parser import ..."
sys.modules['libfec_parser.parser'] = parser
sys.modules['libfec_parser.foo'] = foo
sys.modules['libfec_parser.fecfile'] = fecfile

__all__ = ["parser", "foo", "fecfile"]
