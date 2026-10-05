//// Read FEC campaign finance filings (`.fec` files) with typed records.
////
//// The parser is `fec-parser`, a Rust crate, loaded as a NIF. `open` reads
//// the header and cover; `fold` (or `fold_until`) then pulls the rest of the
//// file, 1,000 rows at a time, on a dirty scheduler. Each `Row` carries its
//// column-keyed `values` and, for row types with a typed family, the typed
//// `itemization` record.
////
//// Sum Schedule A contributions by state, using the typed record:
////
//// ```gleam
//// import gleam/dict.{type Dict}
//// import gleam/option.{Some}
//// import gleam/result
//// import libfec
//// import libfec/error.{type FecError}
//// import libfec/itemization.{ScheduleA}
////
//// pub fn by_state(path: String) -> Result(Dict(String, Float), FecError) {
////   use reader <- result.try(libfec.open(path))
////   use acc, row <- libfec.fold(reader, from: dict.new())
////   case row.itemization {
////     Some(ScheduleA(a)) -> {
////       let state = option.unwrap(a.contributor.address.state, "")
////       dict.upsert(acc, state, fn(total) {
////         option.unwrap(total, 0.0) +. a.contribution_amount
////       })
////     }
////     _ -> acc
////   }
//// }
//// ```
////
//// Column-keyed access works for any row type, typed family or not:
//// `row.get(row, "contribution_amount")` (`libfec/row`).
////
//// ## Errors
////
//// `fold` stops at the first error, after handing every row before it to
//// the callback. A row whose type has no column mapping is an error
//// (`MissingMapping`) unless the reader was opened with
//// `Options(unknown_rows: Skip)`.
////
//// ## Readers are cursors
////
//// A `Reader` is an immutable record, but its `handle` is a position in the
//// file: each `fold` continues where the previous one stopped. One reader
//// folded from two processes at once is safe (the native side serialises
//// the calls), but each process gets an arbitrary, interleaved share of the
//// rows. Use one reader per process; to read many filings in parallel, open
//// each in its own process.

import gleam/list
import gleam/option.{type Option}
import libfec/cover.{type Cover}
import libfec/cover_summary.{type CoverSummary}
import libfec/error.{type FecError}
import libfec/header.{type Header}
import libfec/internal/nif
import libfec/row.{type Row}

/// Rows pulled from the NIF per call.
const batch_size = 1000

/// An open filing: the header and cover, read at `open`, and a handle on
/// the rest of the file.
///
/// The header and cover are plain values: they stay readable after `close`.
pub type Reader {
  Reader(
    /// The file name without its directory, extension and any `FEC-`
    /// prefix: `"1721696"` for `FEC-1721696.fec`.
    filing_id: String,
    /// The filing's header record.
    header: Header,
    /// The fields every cover has, whatever the form type.
    cover_summary: CoverSummary,
    /// The typed cover; `None` for a form type with no typed cover.
    cover: Option(Cover),
    /// The cursor `fold` pulls rows through.
    handle: Handle,
  )
}

/// The native reader behind a `Reader`: an open file and a position in it.
///
/// Opaque: only `fold`, `fold_until`, `skipped_rows` and `close` use it. A
/// handle that is never closed closes its file when it is garbage collected.
pub opaque type Handle {
  Handle(nif.Handle)
}

/// What to do with a row whose type has no column mapping for the filing's
/// version (an unknown or misspelled row type).
pub type UnknownRows {
  /// End the fold with `Error(MissingMapping(row_type, version, line))`,
  /// after delivering every row before it. The default.
  Fail
  /// Skip the row and count it (`skipped_rows`).
  Skip
}

/// Options for `open_with`.
pub type Options {
  Options(
    /// Rows whose type has no column mapping. Default `Fail`.
    unknown_rows: UnknownRows,
  )
}

/// The options `open` uses: `Options(unknown_rows: Fail)`.
pub fn default_options() -> Options {
  Options(unknown_rows: Fail)
}

/// Opens a filing with the default options and reads its header and cover.
/// Rows are read later, by `fold` or `fold_until`.
///
/// Errors: `IoError` if the file can't be read, `UnsupportedVersion` for a
/// format version the parser doesn't know, `ParseError` for a file that
/// isn't a filing or has a malformed header or cover.
pub fn open(path: String) -> Result(Reader, FecError) {
  open_with(path, default_options())
}

/// `open` with options:
///
/// ```gleam
/// libfec.open_with(path, libfec.Options(unknown_rows: libfec.Skip))
/// ```
pub fn open_with(path: String, options: Options) -> Result(Reader, FecError) {
  let skip_unknown = case options.unknown_rows {
    Fail -> False
    Skip -> True
  }
  case nif.open(path, skip_unknown) {
    Ok(nif.Opened(filing_id:, header:, cover_summary:, cover:, handle:)) ->
      Ok(Reader(
        filing_id:,
        header:,
        cover_summary:,
        cover:,
        handle: Handle(handle),
      ))
    Error(e) -> Error(e)
  }
}

/// Folds `f` over every remaining row, in file order.
///
/// Rows are pulled 1,000 at a time, and the fold runs in constant stack
/// space however large the file. It returns:
///
/// - `Ok(acc)` at the end of the file. The reader is a cursor, so a second
///   `fold` on the same reader returns `Ok(acc)` at once.
/// - `Error(e)` at the first error, after `f` has seen every row before it.
///   The offending row is consumed: another `fold` would continue after it.
/// - `Error(Closed)` after `close`.
///
/// Folding one reader from two processes at once is safe, but each fold
/// gets an interleaved share of the rows (see "Readers are cursors" above).
///
/// Strings in a row are slices of one binary per batch (~250 KB per 1,000
/// rows of a typical filing): keeping one alive keeps its whole batch alive.
/// To keep a few strings from many batches in the accumulator, `copy` them.
pub fn fold(
  over reader: Reader,
  from acc: a,
  with f: fn(a, Row) -> a,
) -> Result(a, FecError) {
  let Handle(handle) = reader.handle
  fold_loop(handle, acc, f)
}

fn fold_loop(
  handle: nif.Handle,
  acc: a,
  f: fn(a, Row) -> a,
) -> Result(a, FecError) {
  let nif.Batch(rows:, end:) = nif.next_batch(handle, batch_size)
  let acc = list.fold(rows, acc, f)
  case end {
    nif.More -> fold_loop(handle, acc, f)
    nif.Eof -> Ok(acc)
    nif.Failed(e) -> Error(e)
  }
}

/// Like `fold`, but `f` can stop early by returning `list.Stop(acc)`:
/// `Ok(acc)` is returned at once, and nothing more is read.
///
/// The reader is left mid-file, and a later `fold` continues from there,
/// though not from the row after the one that stopped: rows are read in
/// batches of 1,000, and the rest of the batch holding that row is dropped.
/// Open the file again to read it from the start.
pub fn fold_until(
  over reader: Reader,
  from acc: a,
  with f: fn(a, Row) -> list.ContinueOrStop(a),
) -> Result(a, FecError) {
  let Handle(handle) = reader.handle
  fold_until_loop(handle, acc, f)
}

fn fold_until_loop(
  handle: nif.Handle,
  acc: a,
  f: fn(a, Row) -> list.ContinueOrStop(a),
) -> Result(a, FecError) {
  let nif.Batch(rows:, end:) = nif.next_batch(handle, batch_size)
  case fold_rows_until(rows, acc, f), end {
    list.Stop(acc), _ -> Ok(acc)
    list.Continue(acc), nif.More -> fold_until_loop(handle, acc, f)
    list.Continue(acc), nif.Eof -> Ok(acc)
    list.Continue(_), nif.Failed(e) -> Error(e)
  }
}

fn fold_rows_until(
  rows: List(Row),
  acc: a,
  f: fn(a, Row) -> list.ContinueOrStop(a),
) -> list.ContinueOrStop(a) {
  case rows {
    [] -> list.Continue(acc)
    [row, ..rest] ->
      case f(acc, row) {
        list.Continue(acc) -> fold_rows_until(rest, acc, f)
        list.Stop(acc) -> list.Stop(acc)
      }
  }
}

/// The number of rows skipped so far because their type has no column
/// mapping: only with `Options(unknown_rows: Skip)`, always 0 otherwise.
/// Still answers after `close`.
pub fn skipped_rows(reader: Reader) -> Int {
  let Handle(handle) = reader.handle
  nif.skipped(handle)
}

/// Closes the file. Later folds return `Error(Closed)`; `header`, `cover`
/// and `cover_summary` stay readable. Closing twice is fine.
///
/// Optional: an unclosed reader closes its file when it is garbage
/// collected. Call `close` to release the file sooner.
pub fn close(reader: Reader) -> Nil {
  let Handle(handle) = reader.handle
  nif.close(handle)
}

/// A copy of `s` that shares no memory with the batch it came from
/// (Erlang's `binary:copy/1`).
///
/// Strings in a `Row` are slices of one ~250 KB binary shared by the 1,000
/// rows of their batch, so keeping any one of them keeps the whole batch in
/// memory. Copy the strings you keep past a fold over a large file (dict
/// keys of names, say); strings you only compare, match or parse need no
/// copy. (Strings under 64 bytes are already copied by the runtime on
/// recent OTP releases; copying them again is harmless.)
@external(erlang, "binary", "copy")
pub fn copy(s: String) -> String

/// The version of the native library this package loaded.
pub fn native_version() -> String {
  nif.version()
}
