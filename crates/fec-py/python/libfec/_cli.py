"""Console-script entry point for the `libfec` command (`uvx libfec`, `pip install libfec`)."""

import signal
import sys

from . import _native


def main() -> None:
    # Python's SIGINT handler only sets a flag the Rust CLI never checks; restore the
    # default so Ctrl-C kills a long export the way it does for the native binary.
    signal.signal(signal.SIGINT, signal.SIG_DFL)
    sys.exit(_native.run_cli(["libfec", *sys.argv[1:]]))
