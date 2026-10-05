//// Test helpers over the raw NIF bindings (the public fold API is built on
//// the same calls).

import gleam/list
import libfec/internal/nif.{type BatchEnd, type Opened, Batch, More}
import libfec/row.{type Row}

pub const fixtures = "test/fixtures/"

/// The committed fec-parser test fixtures (real filings, cut down).
pub const parser_fixtures = "../../fec-parser/tests/fixtures/"

/// Opens `path`, asserting success.
pub fn open(path: String, skip_unknown: Bool) -> Opened {
  let assert Ok(opened) = nif.open(path, skip_unknown)
  opened
}

/// Every row up to the first non-`More` batch end, and that end.
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

/// All rows of a filing, skipping unmapped row types.
pub fn rows(path: String) -> List(Row) {
  let #(rows, _) = read_all(open(path, True), 1000)
  rows
}
