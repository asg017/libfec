//// The public reader API: `open`, `fold`, `fold_until`, `skipped_rows`,
//// `close`, `row.get`, `copy`, and the `by_state` example from the
//// `libfec` module doc.
////
//// A mid-batch `ParseError` can't be produced from a file (`next_row` only
//// fails on an I/O error; see `reader_nif_test`), so delivery-before-error
//// is tested through `MissingMapping`, which ends a batch the same way.

import gleam/dict.{type Dict}
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{Some}
import gleam/result
import gleam/string
import libfec.{Fail, Options, Skip}
import libfec/error.{
  type FecError, Closed, IoError, MissingMapping, ParseError, UnsupportedVersion,
}
import libfec/itemization.{ScheduleA}
import libfec/row.{type Row}
import libfec/value.{Empty, Number, Text}
import support

const unknown = "test/fixtures/unknown_rows.fec"

/// 2,500 SA rows, transaction_id "T<i>" on line i + 2.
const many = "test/fixtures/many_rows.fec"

/// T1–T10, 2,000 unmapped rows (lines 13–2012), T11–T20.
const skip_heavy = "test/fixtures/skip_heavy.fec"

const real = "../../fec-py/tests/fixtures/1721696.fec"

fn skipping(path: String) -> libfec.Reader {
  let assert Ok(reader) = libfec.open_with(path, Options(unknown_rows: Skip))
  reader
}

fn amount(row: Row) -> value.Value {
  let assert Ok(v) = row.get(row, "contribution_amount")
  v
}

fn transaction_id(row: Row) -> String {
  let assert Ok(Text(id)) = row.get(row, "transaction_id")
  id
}

/// `[from, from + 1, …, to]`.
fn seq(from: Int, to: Int) -> List(Int) {
  seq_loop(to, from, [])
}

fn seq_loop(i: Int, from: Int, acc: List(Int)) -> List(Int) {
  case i < from {
    True -> acc
    False -> seq_loop(i - 1, from, [i, ..acc])
  }
}

fn ids(from: Int, to: Int) -> List(String) {
  list.map(seq(from, to), fn(i) { "T" <> int.to_string(i) })
}

fn count(reader: libfec.Reader) -> Result(Int, FecError) {
  libfec.fold(reader, 0, fn(n, _) { n + 1 })
}

// G11 ----------------------------------------------------------------------

pub fn default_options_test() {
  assert libfec.default_options() == Options(unknown_rows: Fail)
}

pub fn unmapped_row_fails_after_delivering_rows_test() {
  support.reset_seen()
  let reader = support.reader(unknown)
  let result =
    libfec.fold(reader, 0, fn(n, row) {
      support.see(row.line)
      n + 1
    })
  // The callback saw both rows before the unmapped one (lines 3 and 4),
  // then the fold failed on line 5.
  assert result
    == Error(MissingMapping(row_type: "ZZZ9", version: "8.4", line: 5))
  assert support.seen() == [3, 4]
  assert libfec.skipped_rows(reader) == 0
  // The unmapped row was consumed: another fold continues after it.
  let assert Ok(rest) = support.collect(reader)
  assert list.map(rest, amount) == [Number(3.0)]
}

pub fn unmapped_row_skipped_and_counted_test() {
  let reader = skipping(unknown)
  let assert Ok(rows) = support.collect(reader)
  assert list.map(rows, amount) == [Number(1.0), Number(2.0), Number(3.0)]
  assert list.map(rows, fn(r) { r.line }) == [3, 4, 6]
  assert libfec.skipped_rows(reader) == 1
}

pub fn skipped_rows_across_empty_batches_test() {
  // Skipped rows count towards a batch of 1,000: the second batch has no
  // rows at all, and the fold must keep going.
  let reader = skipping(skip_heavy)
  let assert Ok(rows) = support.collect(reader)
  assert list.map(rows, transaction_id) == ids(1, 20)
  let assert [_, _, _, _, _, _, _, _, _, _, eleventh, ..] = rows
  assert eleventh.line == 2013
  assert libfec.skipped_rows(reader) == 2000
}

pub fn unmapped_row_fails_by_default_test() {
  support.reset_seen()
  let result =
    libfec.fold(support.reader(skip_heavy), Nil, fn(_, row) {
      support.see(row.line)
    })
  assert result == Error(MissingMapping("ZZZ9", "8.4", 13))
  assert support.seen() == seq(3, 12)
}

// Batches ------------------------------------------------------------------

pub fn batch_boundaries_test() {
  let reader = support.reader(many)
  let assert Ok(rows) = support.collect(reader)
  assert list.length(rows) == 2500
  assert list.map(rows, transaction_id) == ids(1, 2500)
  // Lines are exact, so strictly increasing across both batch boundaries.
  assert list.index_map(rows, fn(r, i) { r.line - i }) == list.repeat(3, 2500)
}

pub fn fold_on_exhausted_reader_test() {
  let reader = support.reader(many)
  assert count(reader) == Ok(2500)
  assert count(reader) == Ok(0)
  assert libfec.fold(reader, "acc", fn(_, _) { "called" }) == Ok("acc")
}

// fold_until ---------------------------------------------------------------

pub fn fold_until_stops_early_test() {
  support.reset_seen()
  // Stopping on the first row: no error, though the batch it came from ends
  // in `MissingMapping` two rows later.
  let result =
    libfec.fold_until(support.reader(unknown), 0, fn(n, row) {
      support.see(row.line)
      list.Stop(n + 1)
    })
  assert result == Ok(1)
  assert support.seen() == [3]
}

pub fn fold_until_continues_to_the_end_test() {
  let continue = fn(n, _) { list.Continue(n + 1) }
  assert libfec.fold_until(support.reader(many), 0, continue) == Ok(2500)
  // The same G11 rule as `fold`.
  assert libfec.fold_until(support.reader(unknown), 0, continue)
    == Error(MissingMapping("ZZZ9", "8.4", 5))
}

pub fn fold_after_fold_until_continues_at_next_batch_test() {
  // Stop on T3: the rest of that batch (T4–T1000) is dropped, and a later
  // fold continues with the next batch.
  let reader = support.reader(many)
  let assert Ok("T3") =
    libfec.fold_until(reader, "", fn(_, row) {
      case transaction_id(row) {
        "T3" -> list.Stop("T3")
        id -> list.Continue(id)
      }
    })
  let assert Ok(rest) = support.collect(reader)
  assert list.map(rest, transaction_id) == ids(1001, 2500)
}

// close --------------------------------------------------------------------

pub fn close_test() {
  let reader = support.reader(unknown)
  assert libfec.close(reader) == Nil
  assert count(reader) == Error(Closed)
  assert libfec.fold_until(reader, 0, fn(n, _) { list.Continue(n) })
    == Error(Closed)
  // Twice is fine, and the header and cover stay readable.
  assert libfec.close(reader) == Nil
  assert count(reader) == Error(Closed)
  assert libfec.skipped_rows(reader) == 0
  assert reader.header.fec_version == "8.4"
  assert reader.cover_summary.filer_id == "C00424242"
  let assert Some(_) = reader.cover
}

pub fn close_mid_file_test() {
  let reader = support.reader(many)
  let assert Ok(_) = libfec.fold_until(reader, 0, fn(n, _) { list.Stop(n) })
  libfec.close(reader)
  assert count(reader) == Error(Closed)
}

// open ---------------------------------------------------------------------

pub fn open_reads_header_and_cover_test() {
  let reader = support.reader("test/fixtures/FEC-424242.fec")
  assert reader.filing_id == "424242"
  assert reader.header.software_name == "SoftName"
  assert reader.cover_summary.form_type == "F3XN"
}

pub fn open_missing_file_test() {
  let assert Error(IoError(_)) = libfec.open("test/fixtures/does_not_exist.fec")
}

pub fn open_not_a_filing_test() {
  let assert Error(ParseError(..)) = libfec.open("test/fixtures/not_fec.fec")
  let assert Error(_) = libfec.open("test/fixtures")
}

pub fn open_unsupported_version_test() {
  assert libfec.open_with(
      "test/fixtures/unsupported_version.fec",
      Options(unknown_rows: Skip),
    )
    |> result.map(fn(_) { Nil })
    == Error(UnsupportedVersion("9.9"))
}

// row.get, copy ------------------------------------------------------------

pub fn row_get_test() {
  let assert [row, ..] = support.rows("test/fixtures/FEC-424242.fec")
  assert row.get(row, "contribution_amount") == Ok(Number(250.75))
  assert row.get(row, "contributor_city") == Ok(Text("SA.contributor_city"))
  assert row.get(row, "no_such_column") == Error(Nil)
  // A column past the end of a short row is there, and empty.
  let assert [_, _, _, _, _, short, ..] =
    support.rows("test/fixtures/FEC-424242.fec")
  assert short.row_type == "SB23"
  assert row.get(short, "memo_text_description") == Ok(Empty)
}

@external(erlang, "binary", "referenced_byte_size")
fn referenced_byte_size(s: String) -> Int

pub fn copy_test() {
  let assert [row, ..] = support.rows(many)
  let assert Ok(Text(long)) = row.get(row, "back_reference_tran_id_number")
  assert string.byte_size(long) == 100
  // A long string is a slice of its batch's binary; the copy owns its bytes.
  assert referenced_byte_size(long) > 100
  let copied = libfec.copy(long)
  assert copied == long
  assert referenced_byte_size(copied) == 100
  // Short strings (under 64 bytes) are copied out by the runtime already.
  let id = transaction_id(row)
  assert referenced_byte_size(id) == string.byte_size(id)
  assert libfec.copy(id) == "T1"
}

// A real filing ------------------------------------------------------------

pub fn real_filing_row_counts_test() {
  // 1721696.fec: 1,387 rows. The same counts as `libfec fastfec` (one CSV
  // per row type) and the Python bindings' tests.
  let reader = support.reader(real)
  let assert Ok(counts) =
    libfec.fold(reader, dict.new(), fn(acc, row) {
      dict.upsert(acc, row.row_type, fn(n) { option.unwrap(n, 0) + 1 })
    })
  assert counts
    == dict.from_list([#("SA11AI", 1354), #("SB23", 24), #("SB29", 9)])
  assert libfec.skipped_rows(reader) == 0
  libfec.close(reader)
}

/// The `libfec` module doc's example, verbatim.
pub fn by_state(path: String) -> Result(Dict(String, Float), FecError) {
  use reader <- result.try(libfec.open(path))
  use acc, row <- libfec.fold(reader, from: dict.new())
  case row.itemization {
    Some(ScheduleA(a)) -> {
      let state = option.unwrap(a.contributor.address.state, "")
      dict.upsert(acc, state, fn(total) {
        option.unwrap(total, 0.0) +. a.contribution_amount
      })
    }
    _ -> acc
  }
}

pub fn module_doc_example_test() {
  let assert Ok(totals) = by_state(real)
  // The typed totals agree with the column-keyed amounts.
  let assert Ok(expected) =
    libfec.fold(support.reader(real), 0.0, fn(sum, row) {
      case row.row_type, row.get(row, "contribution_amount") {
        "SA" <> _, Ok(Number(x)) -> sum +. x
        _, _ -> sum
      }
    })
  let total = dict.fold(totals, 0.0, fn(sum, _, x) { sum +. x })
  assert float.loosely_equals(total, expected, tolerating: 0.001)
  // Every contributor in this filing is in New York; the sum of column 21
  // of its SA11AI lines (awk) is 57,161.47.
  assert dict.keys(totals) == ["NY"]
  assert float.loosely_equals(total, 57_161.47, tolerating: 0.001)
}
