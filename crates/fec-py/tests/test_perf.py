"""Slow tests for the Phase 2 and Phase 3 done-when numbers.

Roadmap: `plans/python/05-roadmap.md:64-66` (Phase 2) and `:83-85` (Phase 3 --
"``iter_file`` on the benchmark filing stays under 100 MB", and a differential
test with a printable allowlist).

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
import warnings
from collections import Counter
from collections.abc import Iterator
from typing import Any

import pytest

from libfec_parser import fecfile as ours_fecfile
from libfec_parser import read

#: Rows in ``benchmarks/1805248.fec`` as ``libfec_parser.open()`` counts them
#: (the cover line is ``reader.cover_row``, not one of these), and items as the
#: `fecfile` APIs count them: the same rows plus a header and a summary.
BENCHMARK_ROWS = 408160
BENCHMARK_ITEMS = BENCHMARK_ROWS + 2


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
        f"assert n == {BENCHMARK_ROWS}, n"
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

    assert n == BENCHMARK_ROWS
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


# --- The `fecfile` compat layer at scale (Phase 3) ------------------------


@pytest.mark.slow
@pytest.mark.skipif(sys.platform == "win32", reason="no getrusage on Windows")
def test_compat_iter_file_under_100mb(benchmark_fec_file):
    """Phase 3 done-when: ``fecfile.iter_file`` on the 91 MB filing under 100 MB.

    The compat layer builds a dict per row on top of the native reader, so this
    is the streaming number that matters to anyone migrating off the real
    package -- whose own ``iter_file`` measures 35-47 MB on the same filing.

    Measured 2026-09-18 (this ticket, M4 Pro, release build, CPython 3.13):
    32.6-38.8 MB peak RSS across runs, against the ~31-39 MB that bare
    ``open()`` costs -- the dicts are built and dropped one at a time, so they
    never accumulate, and the layer is free in memory terms.

    The assert is 64 MB, ~1.5x the measured value rounded up, and deliberately
    the same threshold ``test_open_streams_under_100mb`` uses: the whole claim
    of this layer is that its dicts cost nothing that lasts, so a compat number
    drifting away from the ``open()`` number is the regression worth hearing
    about -- not the 100 MB done-when, which has far more slack.
    """
    body = (
        "import warnings\n"
        "warnings.simplefilter('ignore')\n"
        "from libfec_parser import fecfile\n"
        f"n = sum(1 for _ in fecfile.iter_file({str(benchmark_fec_file)!r}))\n"
        f"assert n == {BENCHMARK_ITEMS}, n"
    )
    peak = _peak_mb(body)
    assert peak < 64, f"peak RSS {peak:.1f} MB"


def _form_of(record: dict[str, Any], data_type: str) -> str:
    """The three-letter form a mismatch belongs to, for the `Counter` key.

    Itemizations carry ``form_type``; ``TEXT`` rows call the same column
    ``rec_type``; the header and the summary have neither worth grouping by, so
    they group under their ``data_type``.
    """
    form = record.get("form_type") or record.get("rec_type")
    return form[:3] if isinstance(form, str) else data_type


def _routed(
    items: Iterator[Any], sink: list[str], active: list[list[str]]
) -> Iterator[Any]:
    """Yield from ``items``, attributing warnings raised during each pull to ``sink``.

    ``catch_warnings(record=True)`` pools every warning into one list, which is
    no use when two libraries are being walked in lockstep, and entering it once
    per ``next()`` would cost more than the comparison it serves over 400k
    items. So ``showwarning`` is replaced once, for the whole walk, and this
    wrapper names whose pull is running before each one.
    """
    while True:
        active[0] = sink
        try:
            yield next(items)
        except StopIteration:
            return


@pytest.mark.slow
def test_compat_differential_benchmark_filing(benchmark_fec_file):
    """Ours against the real package over all 408,162 items of the 91 MB filing.

    The fixture-based differential (``test_fecfile_differential.py``) covers
    1,413 rows of FEC 8.4/8.5 and no ``TEXT`` row; this covers 290x that, and is
    what lets the README call the layer a drop-in rather than a lookalike.

    Both sides are streamed and zipped -- materialising either costs well over a
    gigabyte -- and mismatches are counted rather than raised, so one bad column
    reports as "this column, 400k times" instead of stopping at the first row.

    The allowlist is imported from the fixture differential, not restated here.
    Only (b) real's trailing newline applies: this filing has no ``[BEGINTEXT]``
    block, so (a) ``F99_text`` never comes up -- and if one ever did, the
    ``data_type`` comparison below would say so.  Nothing is dropped from ours'
    side at all, so the cover page's duplicated columns are compared here like
    any other, values included.
    """
    real = pytest.importorskip("fecfile")
    # Imported here rather than at module scope: that module skips itself when
    # the real package is missing, and it should not take this file's Phase 2
    # tests down with it.
    from .test_fecfile_differential import without_line_terminator

    path = str(benchmark_fec_file)
    mismatches: Counter[tuple[str, str, str]] = Counter()
    examples: dict[tuple[str, str, str], str] = {}
    ours_warnings: list[str] = []
    real_warnings: list[str] = []
    active = [ours_warnings]
    count = 0

    def show(message, category, filename, lineno, file=None, line=None) -> None:
        active[0].append(str(message))

    def note(key: tuple[str, str, str], detail: str) -> None:
        mismatches[key] += 1
        examples.setdefault(key, detail)

    mine_items = ours_fecfile.iter_file(path)
    real_items = real.iter_file(path)
    leftover = (-1, -1)
    try:
        with warnings.catch_warnings():
            warnings.simplefilter("always")
            warnings.showwarning = show
            pairs = zip(
                _routed(mine_items, ours_warnings, active),
                _routed(real_items, real_warnings, active),
            )
            for count, (mine, theirs) in enumerate(pairs, start=1):
                if mine.data_type != theirs.data_type:
                    note(
                        ("?", "<item>", "data_type"),
                        f"item {count}: {mine.data_type} vs {theirs.data_type}",
                    )
                    continue
                if not isinstance(mine.data, dict):
                    # An ``F99_text`` item's data is a plain str, not a record.
                    if mine.data != theirs.data:
                        note((mine.data_type, "<text>", "value"), f"item {count}")
                    continue
                got = mine.data
                want = without_line_terminator(theirs.data)
                form = _form_of(got, mine.data_type)
                if list(got) != list(want):
                    note(
                        (form, "<columns>", "names/order"),
                        f"item {count}: {list(got)[:6]} vs {list(want)[:6]}",
                    )
                    continue
                for column, expected in want.items():
                    have = got[column]
                    if type(have) is not type(expected):
                        kind = f"type {type(have).__name__} vs {type(expected).__name__}"
                    elif have != expected:
                        kind = "value"
                    else:
                        continue
                    note((form, column, kind), f"item {count}: {have!r} vs {expected!r}")
        # `zip` stops at the shorter side, so neither may have anything left.
        leftover = (sum(1 for _ in mine_items), sum(1 for _ in real_items))
    finally:
        mine_items.close()
        real_items.close()

    assert count == BENCHMARK_ITEMS, f"compared {count} items"
    assert leftover == (0, 0), f"items left over (ours, real): {leftover}"
    assert not mismatches, (
        f"{sum(mismatches.values())} mismatches in {len(mismatches)} classes "
        f"over {count} items; top 20:\n"
        + "\n".join(
            f"  {key} x{n}: {examples[key]}" for key, n in mismatches.most_common(20)
        )
    )
    # Every warning either side raises names the value, type, column, form,
    # version and line, so comparing the messages is strictly stronger than the
    # (line, field) pairs the ticket asks for -- and it needs no guess about
    # whose line numbering is whose. This filing happens to raise none at all
    # (all 408,160 rows type cleanly); `test_type_warnings_match_real` in the
    # fixture differential is what actually exercises the message format.
    assert Counter(ours_warnings) == Counter(real_warnings), (
        f"warnings differ: ours {len(ours_warnings)}, real {len(real_warnings)}"
    )

