"""Slow tests for the Phase 2 done-when numbers (roadmap: `plans/python/05-roadmap.md:64-66`).

All ``@pytest.mark.slow`` (opt-in, see ``tests/conftest.py``), and all use
``benchmark_fec_file`` (``conftest.py``), which ``pytest.skip``s if the 91 MB
filing is absent -- the one sanctioned skip, since that file is gitignored.

RSS is measured in a fresh subprocess: ``resource.getrusage(...).ru_maxrss`` is
a process-wide high-water mark, so measuring it in the already-running pytest
process would include whatever pytest/mypy/etc. had already allocated.
"""
import subprocess
import sys
import threading
import time

import pytest

from libfec_parser import read


def _run(code: str) -> str:
    """Run `code` in a fresh interpreter; return its stdout."""
    result = subprocess.run([sys.executable, "-c", code], capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    return result.stdout


RSS_SNIPPET = """
import resource, sys
{body}
r = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
print(r / 1e6 if sys.platform == 'darwin' else r / 1e3)
"""


def _peak_mb(body: str) -> float:
    """Run `body` in a fresh interpreter, returning its peak RSS in MB.

    ``ru_maxrss`` is bytes on macOS and KB on Linux; normalised above.
    """
    return float(_run(RSS_SNIPPET.format(body=body)).strip())


@pytest.mark.slow
@pytest.mark.skipif(sys.platform == "win32", reason="no getrusage on Windows")
def test_open_streams_under_100mb(benchmark_fec_file):
    """Phase 2 done-when: iterating the 91 MB filing via open() peaks under 100 MB.

    Measured 2026-09-18 (this ticket, M-series Mac, release build, CPython 3.13):
    ~32-39 MB peak RSS across a few runs, against a ~15-20 MB base interpreter --
    well under the roadmap's 100 MB done-when. The assert below is tightened to
    64 MB per ticket 19 ("if the real number lands near 60 MB, tighten to 64"):
    a regression guard with headroom, not the done-when itself.
    """
    body = (
        "import libfec_parser\n"
        f"n = sum(1 for _ in libfec_parser.open({str(benchmark_fec_file)!r}))\n"
        "assert n == 408160"
    )
    peak = _peak_mb(body)
    assert peak < 64, f"peak RSS {peak:.1f} MB"


@pytest.mark.slow
def test_read_rows_access_is_free(benchmark_fec_file):
    """N2 regression guard: 20 accesses to `.rows` cost ~nothing (it's a cached list)."""
    f = read(benchmark_fec_file)
    t = time.perf_counter()
    for _ in range(20):
        len(f.rows)
    assert time.perf_counter() - t < 0.01


@pytest.mark.slow
def test_background_thread_progresses_during_parse(benchmark_fec_file):
    """Phase 2 done-when: a second thread makes progress during a parse.

    The honest version of ``test_reader.py::test_gil_released`` (which uses the
    263 KB fixture so it stays fast on a debug build): here we parse the full
    91 MB filing once and check that a ticking background thread got at least
    50% of the ticks its measured duration implies -- duration-based, not an
    absolute tick count, because ``make test-slow``/``make bench``/CI install
    the --release extension, where this parse is much faster than under
    ``make test``'s debug build. Same tick-counting pattern as the old
    ``plans/python/probes/edge.py`` probe.
    """
    import libfec_parser

    tick_interval = 0.01
    ticks: list[float] = []
    stop = threading.Event()

    def ticker() -> None:
        while not stop.is_set():
            ticks.append(time.monotonic())
            time.sleep(tick_interval)

    thread = threading.Thread(target=ticker, daemon=True)
    thread.start()
    try:
        time.sleep(0.05)  # let the ticker warm up before starting the clock
        n0 = len(ticks)
        started = time.monotonic()
        n = sum(1 for _ in libfec_parser.open(benchmark_fec_file))
        elapsed = time.monotonic() - started
        n1 = len(ticks)
    finally:
        stop.set()
        thread.join(timeout=5)

    assert n == 408160
    during = n1 - n0
    expected = elapsed / tick_interval
    assert during >= 0.5 * expected, (
        f"only {during} ticks in {elapsed:.3f}s (expected ~{expected:.0f})"
    )


# The subprocess the zero-copy test measures: read the filing into a `bytes`,
# iterate it through `open()`, report peak RSS alongside the size of the bytes.
# Moved here from test_reader.py (ticket 15) to share this module's RSS harness.
_ZERO_COPY_SCRIPT = """
import resource, sys
import libfec_parser

with open(sys.argv[1], "rb") as f:
    data = f.read()
rows = sum(1 for _ in libfec_parser.open(data))
peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
# ru_maxrss is bytes on macOS, kilobytes on Linux.
if sys.platform != "darwin":
    peak *= 1024
print(peak, len(data), rows)
"""


@pytest.mark.slow
def test_zero_copy_bytes(benchmark_fec_file):
    """Iterating a 91 MB `bytes` costs well under 100 MB on top of the bytes

    The buffer is read in place, so the only per-row cost is the row being
    handed to Python, which iteration drops again immediately.
    """
    result = subprocess.run(
        [sys.executable, "-c", _ZERO_COPY_SCRIPT, str(benchmark_fec_file)],
        capture_output=True,
        text=True,
        check=True,
    )
    peak, size, rows = (int(v) for v in result.stdout.split())

    assert rows > 0
    overhead = peak - size
    assert overhead < 100 * 1024 * 1024, (
        f"peak RSS {peak / 1e6:.0f} MB over a {size / 1e6:.0f} MB bytes object "
        f"= {overhead / 1e6:.0f} MB of overhead"
    )

