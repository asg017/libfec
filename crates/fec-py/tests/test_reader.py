"""
Tests for ``libfec_parser.open()`` and the ``FilingReader`` it returns.

The fixtures (``sample_fec_file``, ``pac_fec_file``, …) live in conftest.py.
The primary one is tests/fixtures/1921705.fec: v8.5, F3N, filer C00900860
"Jason Byors for Congress", 20 itemizations (1 SA11AI, 13 SA11C, 1 SA11D,
5 SB17) in that file order, on lines 3-22.
"""
import shutil
import threading
import time
from datetime import date

import pytest
from libfec_parser.parser import (
    Cover,
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
            for _ in range(50):
                assert sum(1 for _ in open(pac_fec_file)) == 1387
            elapsed = time.monotonic() - started
        finally:
            stop.set()
            thread.join(timeout=5)

        assert elapsed >= 0.05, "parse was too fast to say anything about the GIL"
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
