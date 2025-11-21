from .fec_py import parser, foo
import sys

# Make submodules importable
sys.modules['fec_py.parser'] = parser
sys.modules['fec_py.foo'] = foo

__all__ = ["parser", "foo"]
