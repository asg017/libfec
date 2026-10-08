//// Test helpers: the public API (`reader`, `rows`, `collect`) and the raw
//// NIF bindings (`open`, `read_all`) for the tests that pin the NIF itself.

import gleam/dynamic.{type Dynamic}
import gleam/list
import gleam/result
import libfec.{type Reader}
import libfec/error.{type FecError}
import libfec/internal/nif.{type BatchEnd, type Opened, Batch, More}
import libfec/row.{type Row}

pub const fixtures = "test/fixtures/"

/// The committed fec-parser test fixtures (real filings, cut down).
pub const parser_fixtures = "../../fec-parser/tests/fixtures/"

/// Opens `path` with `libfec.open`, asserting success.
pub fn reader(path: String) -> Reader {
  let assert Ok(reader) = libfec.open(path)
  reader
}

/// Every remaining row of `reader`, in order, through `libfec.fold`.
pub fn collect(reader: Reader) -> Result(List(Row), FecError) {
  libfec.fold(reader, [], fn(acc, row) { [row, ..acc] })
  |> result.map(list.reverse)
}

/// All rows of a filing, skipping unmapped row types, asserting no error.
pub fn rows(path: String) -> List(Row) {
  let assert Ok(reader) =
    libfec.open_with(path, libfec.Options(unknown_rows: libfec.Skip))
  let assert Ok(rows) = collect(reader)
  rows
}

/// Opens `path` through the raw NIF, asserting success.
pub fn open(path: String, skip_unknown: Bool) -> Opened {
  let assert Ok(opened) = nif.open(path, skip_unknown)
  opened
}

/// Every row up to the first non-`More` batch end, and that end (raw NIF).
pub fn read_all(opened: Opened, batch_size: Int) -> #(List(Row), BatchEnd) {
  read_loop(opened.handle, batch_size, [])
}

fn read_loop(
  handle: nif.Handle,
  n: Int,
  acc: List(List(Row)),
) -> #(List(Row), BatchEnd) {
  let Batch(rows, end) = nif.next_batch(handle, n)
  case end {
    More -> read_loop(handle, n, [rows, ..acc])
    _ -> #(list.flatten(list.reverse([rows, ..acc])), end)
  }
}

// A side channel for what a fold callback saw when the fold itself returns
// `Error` (and so drops its accumulator): the process dictionary.

const seen_key = "libfec_test_seen"

@external(erlang, "erlang", "put")
fn pdict_put(key: String, value: List(Int)) -> Dynamic

@external(erlang, "erlang", "get")
fn pdict_get(key: String) -> List(Int)

/// Forgets every recorded line.
pub fn reset_seen() -> Nil {
  pdict_put(seen_key, [])
  Nil
}

/// Records that a callback saw `line`.
pub fn see(line: Int) -> Nil {
  pdict_put(seen_key, [line, ..pdict_get(seen_key)])
  Nil
}

/// The lines recorded since `reset_seen`, in order.
pub fn seen() -> List(Int) {
  list.reverse(pdict_get(seen_key))
}
