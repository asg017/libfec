//// The raw NIF bindings. Every `@external` to `libfec_nif` lives here.
//// Internal: modules under `libfec/internal` are not part of the public API.
////
//// An `@external` return type is not checked: the Rust side
//// (`crates/fec-gleam/src/lib.rs`) must build exactly the term the Gleam
//// type describes, field order included. The round-trip tests pin it.

import gleam/option.{type Option}
import libfec/cover.{type Cover}
import libfec/cover_summary.{type CoverSummary}
import libfec/error.{type FecError}
import libfec/header.{type Header}
import libfec/row.{type Row}

/// The NIF's reader resource. It closes its file when garbage collected.
pub type Handle

/// What `open` reads up front: header and cover are never read again.
pub type Opened {
  Opened(
    /// The file stem, without a `FEC-` prefix.
    filing_id: String,
    header: Header,
    cover_summary: CoverSummary,
    /// `None` for a form type with no typed cover.
    cover: Option(Cover),
    handle: Handle,
  )
}

/// One `next_batch` result: the rows read, then how the batch ended.
pub type Batch {
  Batch(rows: List(Row), end: BatchEnd)
}

pub type BatchEnd {
  /// Possibly more rows: call `next_batch` again.
  More
  /// The file is exhausted; every later call returns `Batch([], Eof)`.
  Eof
  /// `rows` holds every row before the error. The offending row is
  /// consumed (a later call continues after it). `Failed(Closed)` after
  /// `close`.
  Failed(FecError)
}

/// The version of the loaded NIF crate (`fec-gleam`'s Cargo version).
@external(erlang, "libfec_nif", "version")
pub fn version() -> String

/// Opens a filing and reads its header and cover (dirty IO scheduler).
/// With `skip_unknown`, rows whose type has no column mapping are skipped
/// and counted (`skipped`) instead of ending a batch with `MissingMapping`.
@external(erlang, "libfec_nif", "open")
pub fn open(path: String, skip_unknown: Bool) -> Result(Opened, FecError)

/// Reads up to `n` rows (`n` >= 0; skipped rows count towards it, so a
/// `More` batch can be short). Dirty CPU scheduler; concurrent calls on one
/// handle are serialised and get interleaved rows.
@external(erlang, "libfec_nif", "next_batch")
pub fn next_batch(handle: Handle, n: Int) -> Batch

/// Rows skipped so far under `skip_unknown`.
@external(erlang, "libfec_nif", "skipped")
pub fn skipped(handle: Handle) -> Int

/// Closes the file; later batches are `Failed(Closed)`. Idempotent.
@external(erlang, "libfec_nif", "close")
pub fn close(handle: Handle) -> Nil
