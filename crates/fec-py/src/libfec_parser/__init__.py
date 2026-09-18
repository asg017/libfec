# Import the native extension and expose submodules
from . import libfec_parser as _libfec_parser
import sys

# Make submodules accessible as libfec_parser.parser and libfec_parser.fecfile
parser = _libfec_parser.parser
fecfile = _libfec_parser.fecfile

# Also register them in sys.modules for "from libfec_parser.parser import ..."
sys.modules['libfec_parser.parser'] = parser
sys.modules['libfec_parser.fecfile'] = fecfile

__all__ = ["parser", "fecfile"]
