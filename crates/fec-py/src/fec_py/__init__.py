# Import the native extension and expose submodules
from . import fec_py as _fec_py
import sys

# Make submodules accessible as fec_py.parser and fec_py.foo
parser = _fec_py.parser
foo = _fec_py.foo

# Also register them in sys.modules for "from fec_py.parser import ..."
sys.modules['fec_py.parser'] = parser
sys.modules['fec_py.foo'] = foo

__all__ = ["parser", "foo"]
