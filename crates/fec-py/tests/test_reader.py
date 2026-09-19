"""
Tests for ``libfec_parser.open()`` and the ``FilingReader`` it returns.

The fixtures (``sample_fec_file``, ``pac_fec_file``, …) live in conftest.py.
The primary one is tests/fixtures/1921705.fec: v8.5, F3N, filer C00900860
"Jason Byors for Congress", 20 itemizations (1 SA11AI, 13 SA11C, 1 SA11D,
5 SB17) in that file order, on lines 3-22.
"""
import builtins
import io
import mmap
import shutil
import subprocess
import sys
import textwrap
import threading
import time
from array import array
from datetime import date

import pytest
from libfec_parser.parser import (
    Cover,
    FecParseError,
    FilingReader,
    Header,
    MissingMappingError,
    Row,
    open,
)

ROW_TYPES = ["SA11AI"] + ["SA11C"] * 13 + ["SA11D"] + ["SB17"] * 5


class TestOpen:
    """Accepted sources, and what `open()` hands back"""

    def test_open_path_str(self, sample_fec_file):
        reader = open(str(sample_fec_file))

        assert isinstance(reader, FilingReader)
        assert len(list(reader)) == 20

    def test_open_pathlib_path(self, sample_fec_file):
        """os.PathLike goes through os.fspath"""
        assert len(list(open(sample_fec_file))) == 20

    def test_open_bytes(self, sample_fec_bytes):
        assert len(list(open(sample_fec_bytes))) == 20

    def test_open_rejects_other_types(self):
        with pytest.raises(TypeError):
            open(12345)  # type: ignore[arg-type]  # invalid type, on purpose

    def test_open_missing_path_is_file_not_found(self, tmp_path):
        with pytest.raises(FileNotFoundError):
            open(str(tmp_path / "nope.fec"))

    def test_open_garbage_is_parse_error(self):
        with pytest.raises(ValueError):
            open(b"invalid fec data")

    def test_repr(self, sample_fec_file):
        assert repr(open(sample_fec_file)) == (
            "FilingReader(id='1921705', form_type='F3N', filer_id='C00900860')"
        )

    def test_no_len(self, sample_fec_file):
        """The row count is unknown without a full pass, so there is no __len__"""
        with pytest.raises(TypeError):
            len(open(sample_fec_file))  # type: ignore[arg-type]  # no __len__, on purpose

    def test_source_length(self, sample_fec_file):
        assert open(sample_fec_file).source_length == sample_fec_file.stat().st_size

    def test_fec_version(self, sample_fec_file):
        reader = open(sample_fec_file)

        assert reader.fec_version == "8.5"
        assert reader.fec_version == reader.header.fec_version


class TestIteration:
    """Iteration order, laziness and the rows it yields"""

    def test_iter_returns_self(self, sample_fec_file):
        reader = open(sample_fec_file)

        assert iter(reader) is reader

    def test_iteration_order_and_types(self, sample_fec_file):
        rows = list(open(sample_fec_file))

        assert [r.row_type for r in rows] == ROW_TYPES
        assert all(isinstance(r, Row) for r in rows)

    def test_lines(self, sample_fec_file):
        assert [r.line for r in open(sample_fec_file)] == list(range(3, 23))

    def test_single_pass(self, sample_fec_file):
        """A reader is exhausted after one pass; the second yields nothing"""
        reader = open(sample_fec_file)

        assert len(list(reader)) == 20
        assert list(reader) == []

    def test_typed_values_through_reader(self, sample_fec_file):
        """The version is plumbed through, so columns come back typed"""
        row = next(iter(open(sample_fec_file)))

        assert row["contribution_amount"] == 500.0
        assert row["contribution_date"] == date(2025, 7, 7)

    def test_pac_row_count(self, pac_fec_file):
        assert sum(1 for _ in open(pac_fec_file)) == 1387

    def test_f99_zero_rows(self, f99_fec_file):
        """The F99 fixture is a [BEGINTEXT] filing with no itemizations"""
        reader = open(f99_fec_file)

        assert reader.cover.form_type == "F99"
        assert list(reader) == []


class TestHeaderAndCover:
    """`header`/`cover`/`cover_row`, parsed eagerly and identity-stable"""

    def test_available_before_iteration(self, sample_fec_file):
        reader = open(sample_fec_file)

        assert isinstance(reader.header, Header)
        assert isinstance(reader.cover, Cover)
        assert isinstance(reader.cover_row, Row)

    def test_header_cover_identity(self, sample_fec_file):
        reader = open(sample_fec_file)

        assert reader.header is reader.header
        assert reader.cover is reader.cover
        assert reader.cover_row is reader.cover_row

    def test_cover_dates_are_dates(self, sample_fec_file):
        cover = open(sample_fec_file).cover

        assert cover.coverage_from_date == date(2025, 7, 1)
        assert cover.coverage_through_date == date(2025, 9, 30)
        assert isinstance(cover.coverage_from_date, date)

    def test_cover_fields_keys_and_typed_dates(self, sample_fec_file):
        fields = open(sample_fec_file).cover.fields()

        assert set(fields) == {
            "form_type",
            "filer_id",
            "filer_name",
            "report_code",
            "coverage_from_date",
            "coverage_through_date",
        }
        assert fields["coverage_from_date"] == date(2025, 7, 1)

    def test_cover_row_is_the_cover_line(self, sample_fec_file):
        cover_row = open(sample_fec_file).cover_row

        assert cover_row.row_type == "F3N"
        assert cover_row.line == 2
        assert cover_row["filer_committee_id_number"] == "C00900860"

    def test_cover_row_values(self, pac_fec_file):
        """`cover_row` follows the same rules as any Row (Q14-Q16, Q6):
        typed by name, raw by position, `row_type`/`line` from the record."""
        cover_row = open(pac_fec_file).cover_row

        assert cover_row["col_a_total_receipts"] == 83741.93
        assert cover_row["coverage_from_date"] == date(2023, 7, 1)
        assert cover_row[0] == "F3XN"
        assert cover_row.row_type == "F3XN"
        assert cover_row.line == 2

    def test_cover_row_is_row_and_identity(self, pac_fec_file):
        reader = open(pac_fec_file)

        assert isinstance(reader.cover_row, Row)
        assert reader.cover_row is reader.cover_row

    def test_cover_fields_six_keys_typed(self, sample_fec_file):
        """`Cover.fields()` keeps its six keys (Q19), typed dates"""
        fields = open(sample_fec_file).cover.fields()

        assert set(fields) == {
            "form_type",
            "filer_id",
            "filer_name",
            "report_code",
            "coverage_from_date",
            "coverage_through_date",
        }
        assert fields["coverage_from_date"] == date(2025, 7, 1)

    def test_cover_row_keys_count(self, pac_fec_file):
        """pac_fec_file is F3XN/8.4; column_names_for_field("F3XN", "8.4")
        (checked directly against fec_parser::mappings on this tip) returns
        123 columns, including the deferred `_TODO_DUP` names."""
        cover_row = open(pac_fec_file).cover_row

        assert len(cover_row) > 100
        assert len(cover_row) == 123

    def test_header_and_cover_survive_close(self, sample_fec_file):
        """They are parsed up front, so closing the source does not lose them"""
        reader = open(sample_fec_file)
        reader.close()

        assert reader.header.fec_version == "8.5"
        assert reader.cover.form_type == "F3N"


class TestId:
    """`reader.id`"""

    def test_id_from_path(self, sample_fec_file):
        assert open(sample_fec_file).id == "1921705"

    def test_id_none_for_bytes(self, sample_fec_bytes):
        assert open(sample_fec_bytes).id is None

    def test_id_strips_fec_prefix(self, sample_fec_file, tmp_path):
        prefixed = tmp_path / "FEC-1921705.fec"
        shutil.copyfile(sample_fec_file, prefixed)

        assert open(prefixed).id == "1921705"


class TestRowsFilter:
    """`rows(*prefixes)` — filtered in Rust, before any Row is built"""

    def test_returns_self(self, sample_fec_file):
        reader = open(sample_fec_file)

        assert reader.rows("SB") is reader

    def test_prefix_filter(self, sample_fec_file):
        assert sum(1 for _ in open(sample_fec_file).rows("SB")) == 5

    def test_prefix_filter_is_case_insensitive(self, sample_fec_file):
        assert sum(1 for _ in open(sample_fec_file).rows("sa11c")) == 13

    def test_several_prefixes(self, sample_fec_file):
        assert sum(1 for _ in open(sample_fec_file).rows("SA", "SB17")) == 20

    def test_no_match(self, sample_fec_file):
        assert list(open(sample_fec_file).rows("ZZ")) == []

    def test_no_args_clears_the_filter(self, sample_fec_file):
        reader = open(sample_fec_file).rows("SB")

        assert sum(1 for _ in reader.rows()) == 20

    def test_replaces_previous_filter(self, sample_fec_file):
        reader = open(sample_fec_file).rows("SB").rows("SA11C")

        assert sum(1 for _ in reader) == 13

    def test_rejects_non_str_prefix(self, sample_fec_file):
        with pytest.raises(TypeError):
            open(sample_fec_file).rows(17)  # type: ignore[arg-type]  # on purpose


class TestCloseAndContextManager:
    def test_context_manager_yields_self_and_closes(self, sample_fec_file):
        with open(sample_fec_file) as reader:
            assert isinstance(reader, FilingReader)
            assert not reader.closed

        assert reader.closed

    def test_iterate_after_close_raises(self, sample_fec_file):
        reader = open(sample_fec_file)
        reader.close()

        with pytest.raises(ValueError, match="closed filing"):
            next(iter(reader))

    def test_close_idempotent(self, sample_fec_file):
        reader = open(sample_fec_file)
        reader.close()
        reader.close()

        assert reader.closed

    def test_closed_is_false_while_open(self, sample_fec_file):
        assert open(sample_fec_file).closed is False


class TestMissingMapping:
    def test_missing_mapping_is_raised_and_iteration_continues(self, sample_fec_bytes):
        """An unmapped row type raises, but the reader stays usable"""
        raw = sample_fec_bytes.replace(b"\nSA11C\x1c", b"\nZZZZ\x1c", 1)
        reader = open(raw)
        rows, errors = [], []

        while True:
            try:
                rows.append(next(reader))
            except StopIteration:
                break
            except MissingMappingError as e:
                errors.append(e)

        assert len(errors) == 1
        assert (errors[0].row_type, errors[0].version, errors[0].line) == (
            "ZZZZ",
            "8.5",
            4,
        )
        # every row but the one that was renamed, still in file order
        expected = list(ROW_TYPES)
        expected.remove("SA11C")
        assert [r.row_type for r in rows] == expected


class TestThreading:
    """The GIL is released while rows are pulled"""

    def test_gil_released(self, pac_fec_file):
        """A background thread makes progress while the main thread parses.

        The workload is duration- not count-based: a release build parses the
        263 KB fixture an order of magnitude faster than a debug one, so a fixed
        iteration count would finish before the ticker could say anything.
        """
        parse_for = 0.25  # seconds of wall time spent parsing
        ticks: list[float] = []
        stop = threading.Event()

        def ticker() -> None:
            while not stop.is_set():
                ticks.append(time.monotonic())
                time.sleep(0.001)

        thread = threading.Thread(target=ticker, daemon=True)
        thread.start()
        try:
            started = time.monotonic()
            while time.monotonic() - started < parse_for:
                assert sum(1 for _ in open(pac_fec_file)) == 1387
            elapsed = time.monotonic() - started
        finally:
            stop.set()
            thread.join(timeout=5)

        # Loose on purpose: a smoke test, not a benchmark.  The old eager
        # binding, which held the GIL for the whole parse, got ~0 ticks here.
        during = [t for t in ticks if t >= started]
        assert len(during) >= 10, f"only {len(during)} ticks in {elapsed:.3f}s"

    def test_many_readers_in_threads(self, pac_fec_file):
        """Four threads, four independent readers, no shared state"""
        counts: list[int] = []
        lock = threading.Lock()

        def run() -> None:
            n = sum(1 for _ in open(pac_fec_file))
            with lock:
                counts.append(n)

        threads = [threading.Thread(target=run) for _ in range(4)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=60)

        assert counts == [1387] * 4


class TestBufferSources:
    """Every bytes-like source goes through one zero-copy buffer path"""

    def test_open_bytearray(self, sample_fec_bytes):
        assert len(list(open(bytearray(sample_fec_bytes)))) == 20

    def test_open_memoryview(self, sample_fec_bytes):
        assert len(list(open(memoryview(sample_fec_bytes)))) == 20

    def test_open_memoryview_slice(self, sample_fec_bytes):
        """A strided memoryview is not contiguous: gathered into a Vec, then parsed

        Every other byte is not a filing, so the only sane outcomes are a parse
        error or a filing-shaped nothing — never a crash.
        """
        try:
            rows = list(open(memoryview(sample_fec_bytes)[::2]))
        except FecParseError:
            return
        assert isinstance(rows, list)

    def test_open_mmap(self, sample_fec_file):
        with builtins.open(sample_fec_file, "rb") as f:
            with mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ) as mapped:
                assert len(list(open(mapped))) == 20

    def test_buffer_source_length(self, sample_fec_bytes):
        assert open(bytearray(sample_fec_bytes)).source_length == len(sample_fec_bytes)

    def test_buffer_source_has_no_id(self, sample_fec_bytes):
        """A buffer does not name itself"""
        assert open(memoryview(sample_fec_bytes)).id is None

    def test_non_byte_buffer_raises_typeerror(self):
        """`array('i')`, a NumPy float array, `memoryview(...).cast('I')`: not bytes-like"""
        with pytest.raises(TypeError, match="bytes-like"):
            open(array("i", [1, 2, 3]))  # type: ignore[arg-type]  # on purpose


class TestFileObjectSources:
    """Binary file objects are pulled a chunk at a time, never `.read()` whole"""

    def test_open_binary_file_object(self, sample_fec_file):
        with builtins.open(sample_fec_file, "rb") as f:
            assert len(list(open(f))) == 20

    def test_open_bytesio(self, sample_fec_bytes):
        assert len(list(open(io.BytesIO(sample_fec_bytes)))) == 20

    def test_open_urlopen_like(self, sample_fec_bytes):
        """A response object: `read(n)` and nothing else useful"""

        class Response:
            def __init__(self, data):
                self._buf = io.BytesIO(data)

            def read(self, n):
                return self._buf.read(n)

        assert len(list(open(Response(sample_fec_bytes)))) == 20

    def test_read_is_chunked_not_whole(self, pac_fec_bytes_source):
        """The 263 KB fixture takes several reads, each bounded by the chunk size"""
        source, sizes = pac_fec_bytes_source

        assert sum(1 for _ in open(source)) == 1387
        assert len(sizes) > 1
        assert max(sizes) <= 64 * 1024

    def test_id_from_file_object_name(self, sample_fec_file):
        with builtins.open(sample_fec_file, "rb") as f:
            assert open(f).id == "1921705"

    def test_id_none_without_a_name(self, sample_fec_bytes):
        assert open(io.BytesIO(sample_fec_bytes)).id is None

    def test_source_length_from_fileno(self, sample_fec_file):
        with builtins.open(sample_fec_file, "rb") as f:
            assert open(f).source_length == sample_fec_file.stat().st_size

    def test_source_length_unknown_without_fileno(self, sample_fec_bytes):
        """`BytesIO.fileno()` raises; an unknown length is 0, not an error"""
        assert open(io.BytesIO(sample_fec_bytes)).source_length == 0


class TestSourceErrors:
    """Text-mode files, and exceptions raised by the source's own `read()`"""

    def test_text_mode_file_raises_typeerror(self, sample_fec_file):
        with builtins.open(sample_fec_file) as f:
            with pytest.raises(TypeError, match="'rb'"):
                open(f)  # type: ignore[arg-type]  # text mode, on purpose

    def test_stringio_raises_typeerror(self, sample_fec_content):
        with pytest.raises(TypeError, match="'rb'"):
            open(io.StringIO(sample_fec_content))  # type: ignore[arg-type]  # on purpose

    def test_read_returning_str_raises_typeerror(self, sample_fec_content):
        """Not an `io.TextIOBase`, but still hands back `str`: same message"""

        class TextLike:
            def read(self, n):
                return "HDR\x1cFEC\x1c8.5"

        with pytest.raises(TypeError, match="'rb'"):
            open(TextLike())  # type: ignore[arg-type]  # on purpose

    def test_read_returning_junk_raises_typeerror(self):
        class Junk:
            def read(self, n):
                return [1, 2, 3]

        with pytest.raises(TypeError, match="must return bytes"):
            open(Junk())  # type: ignore[arg-type]  # on purpose

    def test_read_error_propagates(self):
        """The source's exception comes back as itself, not as FecParseError"""

        class Boom:
            def read(self, n):
                raise ZeroDivisionError("boom")

        with pytest.raises(ZeroDivisionError, match="boom"):
            open(Boom())  # type: ignore[arg-type]  # on purpose

    def test_read_error_mid_iteration_propagates(self, sample_fec_bytes):
        """Same when the source fails after the header and cover are already parsed"""

        # The header and cover records end at byte 599 of the fixture, so 1 KB is
        # enough for `open()` to succeed and not enough to finish iterating.
        class BoomLater:
            def __init__(self, data):
                self._buf = io.BytesIO(data)

            def read(self, n):
                if self._buf.tell() >= 1024:
                    raise ZeroDivisionError("late boom")
                return self._buf.read(min(n, 512))

        reader = open(BoomLater(sample_fec_bytes))  # type: ignore[arg-type]  # on purpose

        with pytest.raises(ZeroDivisionError, match="late boom"):
            list(reader)

    def test_read_returning_too_much_raises(self, sample_fec_bytes):
        """A source that ignores its size argument is a bug, not a buffer overrun"""

        class TooMuch:
            def read(self, n):
                return sample_fec_bytes * 100

        with pytest.raises(ValueError, match="more than asked for"):
            open(TooMuch())  # type: ignore[arg-type]  # on purpose


class TestSharedReaderThreading:
    """One reader, many threads — the GIL and the reader's mutexes must not deadlock

    A binary file object is the interesting case: pulling a row runs Python code
    (`read()`) from inside the reader's own locks, so a thread holding the GIL
    that blocks on one of those locks closes a cycle.
    """

    def test_one_file_object_reader_shared_by_threads(self, pac_fec_file):
        class SlowFile:
            """A real file whose `read` yields the GIL, to force interleaving."""

            def __init__(self, path):
                self._f = builtins.open(path, "rb")

            def read(self, n):
                data = self._f.read(n)
                time.sleep(0.0005)
                return data

            def close(self):
                self._f.close()

        source = SlowFile(pac_fec_file)
        reader = open(source)  # type: ignore[arg-type]  # a binary file object
        lines: list[int] = []
        guard = threading.Lock()
        failures: list[BaseException] = []

        def drain() -> None:
            try:
                while True:
                    try:
                        row = next(reader)
                    except StopIteration:
                        return
                    with guard:
                        lines.append(row.line)
            except BaseException as e:  # noqa: BLE001  - reported, not swallowed
                with guard:
                    failures.append(e)

        threads = [threading.Thread(target=drain, daemon=True) for _ in range(4)]
        for t in threads:
            t.start()
        # Daemon threads plus an explicit timeout: a deadlock must fail the test,
        # not hang the suite.
        stuck = []
        for t in threads:
            t.join(timeout=30)
            if t.is_alive():
                stuck.append(t.name)
        source.close()

        assert not stuck, f"threads still alive after 30s (deadlock): {stuck}"
        assert not failures, f"worker raised: {failures!r}"
        assert len(lines) == 1387
        assert len(set(lines)) == 1387, "a row was delivered to more than one thread"

    def test_close_from_another_thread_while_reading(self, pac_fec_file):
        """`close()` holds the GIL and wants `inner`, which a pull may be holding"""

        class SlowFile:
            def __init__(self, path):
                self._f = builtins.open(path, "rb")

            def read(self, n):
                time.sleep(0.001)
                return self._f.read(n)

        reader = open(SlowFile(pac_fec_file))  # type: ignore[arg-type]  # file object
        started = threading.Event()

        def drain() -> None:
            try:
                for _ in reader:
                    started.set()
            except ValueError:
                pass  # the expected "closed filing" once close() lands

        thread = threading.Thread(target=drain, daemon=True)
        thread.start()
        started.wait(timeout=30)
        reader.close()  # must not deadlock against the in-flight pull
        thread.join(timeout=30)

        assert not thread.is_alive(), "close() deadlocked against an in-flight pull"


# A source whose `read()` calls back into the reader that is reading it. `inner`
# is not a reentrant lock, so this has to be refused, not waited on. Run in a
# subprocess: if the guard ever regresses this must time out, not hang the suite.
_REENTRANT_SCRIPT = """
import sys
import libfec_parser


class Reentrant:
    def __init__(self, path, how):
        self._f = open(path, "rb")
        self._how = how
        self.reader = None

    def read(self, n):
        if self.reader is not None:
            try:
                next(self.reader) if self._how == "next" else self.reader.close()
            except RuntimeError as e:
                print("REFUSED", type(e).__name__, e)
            except Exception as e:  # noqa: BLE001
                print("WRONG", type(e).__name__, e)
        return self._f.read(n)


source = Reentrant(sys.argv[1], sys.argv[2])
source.reader = libfec_parser.open(source)
print("ROWS", sum(1 for _ in source.reader))
"""


class TestReentrantSource:
    @pytest.mark.parametrize("how", ["next", "close"])
    def test_source_reentering_its_own_reader_is_refused(self, sample_fec_file, how):
        try:
            result = subprocess.run(
                [sys.executable, "-c", _REENTRANT_SCRIPT, str(sample_fec_file), how],
                capture_output=True,
                text=True,
                timeout=60,
            )
        except subprocess.TimeoutExpired:
            pytest.fail(f"re-entrant {how}() deadlocked instead of raising")

        assert result.returncode == 0, result.stderr
        assert "REFUSED RuntimeError" in result.stdout, result.stdout
        assert "WRONG" not in result.stdout, result.stdout
        # The reader survives the refusal and still delivers the filing.
        assert "ROWS 20" in result.stdout, result.stdout

    def test_closed_is_answerable_during_a_pull(self, sample_fec_file):
        """`closed` must not block on `inner` when this thread is the puller"""

        class Peeking:
            def __init__(self, path):
                self._f = builtins.open(path, "rb")
                self.reader = None
                self.seen = []

            def read(self, n):
                if self.reader is not None:
                    self.seen.append(self.reader.closed)
                return self._f.read(n)

        source = Peeking(sample_fec_file)
        source.reader = open(source)  # type: ignore[arg-type]  # a binary file object

        assert sum(1 for _ in source.reader) == 20
        assert source.seen and not any(source.seen)


@pytest.fixture
def pac_fec_bytes_source(pac_fec_file):
    """A file object over the 263 KB fixture that records every read size."""
    sizes: list[int] = []
    buf = io.BytesIO(pac_fec_file.read_bytes())

    class Recording:
        def read(self, n):
            sizes.append(n)
            return buf.read(n)

    return Recording(), sizes


# The subprocess the zero-copy test measures: read the filing into a `bytes`,
# iterate it through `open()`, report peak RSS alongside the size of the bytes.
_RSS_SCRIPT = """
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


class TestZeroCopy:
    @pytest.mark.slow
    def test_zero_copy_bytes(self, benchmark_fec_file):
        """Iterating a 91 MB `bytes` costs well under 100 MB on top of the bytes

        The buffer is read in place, so the only per-row cost is the row being
        handed to Python, which iteration drops again immediately.
        """
        result = subprocess.run(
            [sys.executable, "-c", textwrap.dedent(_RSS_SCRIPT), str(benchmark_fec_file)],
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
