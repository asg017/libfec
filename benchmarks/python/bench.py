"""Benchmarks for the ``libfec_parser`` Python bindings.

Moved and extended from the ``plans/python/probes/bench.py`` probe (Phase 2,
ticket 19): each scenario below parses the 91 MB ``1805248.fec`` benchmark
filing (gitignored; not present in a fresh checkout) and reports rows parsed,
wall time, and peak RSS.

Every scenario runs in its **own subprocess** — ``resource.getrusage(...).ru_maxrss``
is a process-wide high-water mark, so measuring several scenarios in one
interpreter would have each one see the high-water mark of everything before it.

Usage (from the repo root, with the ``libfec_parser`` dev environment active)::

    uv run python benchmarks/python/bench.py
    uv run python benchmarks/python/bench.py --filing /path/to/other.fec
    uv run python benchmarks/python/bench.py --only open,read

or, from ``crates/fec-py``: ``make bench``.
"""
from __future__ import annotations

import argparse
import subprocess
import sys
import textwrap
from pathlib import Path

DEFAULT_FILING = Path(__file__).resolve().parents[1] / "1805248.fec"

# Every scenario body runs after this preamble, with `sys.argv[1]` set to the
# filing path.  It defines `_peak_mb()` (None on Windows, where `resource` does
# not exist) and prints one line other code never needs to parse around:
# `RESULT <rows> <seconds> <rss_or_NA> [extra]` or `MISSING <reason>`.
_PREAMBLE = """
import sys, time

def _peak_mb():
    if sys.platform == "win32":
        return None
    import resource
    r = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    # ru_maxrss is bytes on macOS, kilobytes on Linux.
    return r / 1e6 if sys.platform == "darwin" else r / 1e3

path = sys.argv[1]
"""

# Each scenario is (name, description, subprocess body). The body must set
# `rows` and use `t0 = time.perf_counter()` right before the timed work, then
# print the RESULT/MISSING line itself (so it controls exactly what is timed).
_SCENARIOS: list[tuple[str, str, str]] = [
    (
        "open",
        "sum(1 for _ in libfec_parser.open(p))",
        """
import libfec_parser
t0 = time.perf_counter()
rows = sum(1 for _ in libfec_parser.open(path))
dt = time.perf_counter() - t0
print(f"RESULT {rows} {dt} {_peak_mb()}")
""",
    ),
    (
        "open_sb",
        'sum(1 for _ in libfec_parser.open(p).rows("SB"))',
        """
import libfec_parser
t0 = time.perf_counter()
rows = sum(1 for _ in libfec_parser.open(path).rows("SB"))
dt = time.perf_counter() - t0
print(f"RESULT {rows} {dt} {_peak_mb()}")
""",
    ),
    (
        "open_bytes",
        "same as open() over p.read_bytes()",
        """
import libfec_parser
data = open(path, "rb").read()
t0 = time.perf_counter()
rows = sum(1 for _ in libfec_parser.open(data))
dt = time.perf_counter() - t0
peak = _peak_mb()
extra = "NA" if peak is None else (peak - len(data) / 1e6)
print(f"RESULT {rows} {dt} {peak} {extra}")
""",
    ),
    (
        "read",
        "len(libfec_parser.read(p).rows)",
        """
import libfec_parser
t0 = time.perf_counter()
rows = len(libfec_parser.read(path).rows)
dt = time.perf_counter() - t0
print(f"RESULT {rows} {dt} {_peak_mb()}")
""",
    ),
    (
        "read_20x",
        "read() once, then 20x len(f.rows) -- the N2 regression guard",
        """
import libfec_parser
t0 = time.perf_counter()
f = libfec_parser.read(path)
for _ in range(20):
    rows = len(f.rows)
dt = time.perf_counter() - t0
print(f"RESULT {rows} {dt} {_peak_mb()}")
""",
    ),
    (
        "dataframe",
        "pd.DataFrame(read(p).rows).shape",
        """
try:
    import pandas as pd
except ImportError:
    print("MISSING pandas not installed")
else:
    import libfec_parser
    t0 = time.perf_counter()
    df = pd.DataFrame(libfec_parser.read(path).rows)
    dt = time.perf_counter() - t0
    print(f"RESULT {df.shape[0]} {dt} {_peak_mb()}")
""",
    ),
    (
        "compat",
        "libfec_parser.fecfile.from_file(p) -- compare with `real`",
        """
import warnings
warnings.simplefilter("ignore")
from libfec_parser import fecfile
t0 = time.perf_counter()
d = fecfile.from_file(path)
rows = sum(len(v) for v in d["itemizations"].values()) + len(d["text"])
dt = time.perf_counter() - t0
print(f"RESULT {rows} {dt} {_peak_mb()}")
""",
    ),
    (
        "compat_iter",
        "libfec_parser.fecfile.iter_file(p) -- compare with `real_iter`",
        """
import warnings
warnings.simplefilter("ignore")
from libfec_parser import fecfile
t0 = time.perf_counter()
rows = sum(1 for _ in fecfile.iter_file(path))
dt = time.perf_counter() - t0
print(f"RESULT {rows} {dt} {_peak_mb()}")
""",
    ),
    (
        "real",
        "PyPI fecfile.from_file(p)",
        """
import warnings
warnings.simplefilter("ignore")
try:
    import fecfile
except ImportError:
    print("MISSING fecfile not installed")
else:
    t0 = time.perf_counter()
    d = fecfile.from_file(path)
    rows = sum(len(v) for v in d["itemizations"].values()) + len(d["text"])
    dt = time.perf_counter() - t0
    print(f"RESULT {rows} {dt} {_peak_mb()}")
""",
    ),
    (
        "real_iter",
        "PyPI fecfile.iter_file(p)",
        """
import warnings
warnings.simplefilter("ignore")
try:
    import fecfile
except ImportError:
    print("MISSING fecfile not installed")
else:
    t0 = time.perf_counter()
    rows = sum(1 for _ in fecfile.iter_file(path))
    dt = time.perf_counter() - t0
    print(f"RESULT {rows} {dt} {_peak_mb()}")
""",
    ),
]


def _format_peak(value: str) -> str:
    if value == "NA":
        return "n/a"
    return f"{float(value):.1f}"


def _run_scenario(name: str, body: str, filing: Path) -> str:
    """Run one scenario's code in a fresh subprocess; format its Markdown row."""
    code = _PREAMBLE + body
    result = subprocess.run(
        [sys.executable, "-c", textwrap.dedent(code), str(filing)],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return f"| {name} | ERROR | ERROR | {result.stderr.strip().splitlines()[-1:] or 'see stderr'} |"

    line = ""
    for candidate in result.stdout.splitlines():
        if candidate.startswith(("RESULT", "MISSING")):
            line = candidate
    if not line:
        return f"| {name} | ERROR | ERROR | no RESULT/MISSING line in output |"

    if line.startswith("MISSING"):
        reason = line[len("MISSING "):].strip()
        return f"| {name} (n/a: {reason}) | n/a | n/a | n/a |"

    parts = line.split()
    rows, seconds, peak = parts[1], parts[2], parts[3]
    peak_str = _format_peak(peak)
    if name == "open_bytes" and len(parts) > 4:
        delta = parts[4]
        peak_str = f"{peak_str} ({'+' if float(delta) >= 0 else ''}{float(delta):.1f} over bytes)"
    return f"| {name} | {rows} | {float(seconds):.2f} s | {peak_str} MB |"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--filing", type=Path, default=DEFAULT_FILING)
    parser.add_argument(
        "--only",
        type=str,
        default=None,
        help="comma-separated scenario names to run (default: all)",
    )
    args = parser.parse_args()

    if not args.filing.exists():
        print(f"error: filing not found: {args.filing}", file=sys.stderr)
        raise SystemExit(1)

    only = set(args.only.split(",")) if args.only else None
    scenarios = [s for s in _SCENARIOS if only is None or s[0] in only]
    if only is not None:
        missing = only - {s[0] for s in _SCENARIOS}
        if missing:
            print(f"error: unknown scenario(s): {', '.join(sorted(missing))}", file=sys.stderr)
            raise SystemExit(1)

    print(f"Filing: {args.filing} ({args.filing.stat().st_size / 1e6:.1f} MB)\n")
    print("| name | rows | seconds | peak MB |")
    print("|---|---|---|---|")
    for name, _description, body in scenarios:
        print(_run_scenario(name, body, args.filing))


if __name__ == "__main__":
    main()
