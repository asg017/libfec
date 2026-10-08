//// The reader NIFs' control flow: batch ends, G11 unmapped rows (fail by
//// default, skip and count when asked), errors from `open`, and `close`.
//// Every `FecError` constructor is produced here at least once.
////
//// A mid-batch `ParseError` can't be produced from a file: `next_row` only
//// fails on an I/O error from the underlying reader (fec-parser's CSV
//// reader is lenient), so delivery-before-error is tested through the
//// `MissingMapping` path, which ends a batch the same way.

import gleam/list
import gleam/string
import libfec/error.{
  Closed, IoError, MissingMapping, ParseError, UnsupportedVersion,
}
import libfec/internal/nif.{Batch, Eof, Failed, More}
import libfec/row.{type Row}
import libfec/value.{Number}
import support

const unknown = "test/fixtures/unknown_rows.fec"

fn amounts(rows: List(Row)) -> List(value.Value) {
  list.map(rows, fn(r) {
    let assert Ok(v) = list.key_find(r.values, "contribution_amount")
    v
  })
}

pub fn missing_mapping_after_delivered_rows_test() {
  let opened = support.open(unknown, False)
  let assert Batch(
    rows,
    Failed(MissingMapping(row_type: "ZZZ9", version: "8.4", line: 5)),
  ) = nif.next_batch(opened.handle, 100)
  // Both rows before the unmapped one are delivered with the error.
  assert amounts(rows) == [Number(1.0), Number(2.0)]
  assert list.map(rows, fn(r) { r.line }) == [3, 4]
  assert nif.skipped(opened.handle) == 0
  // The unmapped row was consumed: the next batch continues after it.
  let assert Batch(rest, Eof) = nif.next_batch(opened.handle, 100)
  assert amounts(rest) == [Number(3.0)]
}

pub fn missing_mapping_at_batch_start_test() {
  let opened = support.open(unknown, False)
  let assert Batch(first, More) = nif.next_batch(opened.handle, 2)
  assert list.length(first) == 2
  let assert Batch([], Failed(MissingMapping(_, _, 5))) =
    nif.next_batch(opened.handle, 2)
}

pub fn skip_unknown_counts_test() {
  let opened = support.open(unknown, True)
  let #(rows, end) = support.read_all(opened, 100)
  assert end == Eof
  assert amounts(rows) == [Number(1.0), Number(2.0), Number(3.0)]
  assert nif.skipped(opened.handle) == 1
}

pub fn skipped_rows_count_towards_n_test() {
  // n bounds the rows read: rows 1-2 then the skipped row fill a batch of 3.
  let opened = support.open(unknown, True)
  let assert Batch(rows, More) = nif.next_batch(opened.handle, 3)
  assert list.length(rows) == 2
  // One row left: it arrives with `Eof` in the same batch.
  let assert Batch([_], Eof) = nif.next_batch(opened.handle, 3)
  assert nif.next_batch(opened.handle, 3) == Batch([], Eof)
}

pub fn batch_of_one_test() {
  let opened = support.open(unknown, True)
  let assert Batch([_], More) = nif.next_batch(opened.handle, 1)
  assert nif.next_batch(opened.handle, 0) == Batch([], More)
}

pub fn close_test() {
  let opened = support.open(unknown, False)
  let assert Batch([_], More) = nif.next_batch(opened.handle, 1)
  assert nif.close(opened.handle) == Nil
  assert nif.next_batch(opened.handle, 10) == Batch([], Failed(Closed))
  // Twice is fine; `skipped` still answers; header and cover were read at
  // open and are plain values.
  assert nif.close(opened.handle) == Nil
  assert nif.next_batch(opened.handle, 10) == Batch([], Failed(Closed))
  assert nif.skipped(opened.handle) == 0
  assert opened.header.fec_version == "8.4"
  assert opened.cover_summary.filer_id == "C00424242"
}

pub fn open_missing_file_test() {
  let assert Error(IoError(message)) =
    nif.open("test/fixtures/does_not_exist.fec", False)
  assert string.contains(message, "No such file")
}

pub fn open_unsupported_version_test() {
  assert nif.open("test/fixtures/unsupported_version.fec", False)
    == Error(UnsupportedVersion("9.9"))
}

pub fn open_not_a_filing_test() {
  let assert Error(ParseError(line: 0, message:)) =
    nif.open("test/fixtures/not_fec.fec", False)
  assert string.contains(message, "header")
}

pub fn open_directory_test() {
  // Not a crash, whatever the OS reports.
  let assert Error(_) = nif.open("test/fixtures", False)
}
