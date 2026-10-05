//! Rustler NIF behind the `libfec` Gleam package (`gleam/`).
//!
//! Loaded by `gleam/src/libfec_nif.erl`; every NIF here needs a stub export
//! there and an `@external` in `gleam/src/libfec/internal/nif.gleam`.
//!
//! # The contract
//!
//! Nothing checks that a term built here matches the Gleam type its
//! `@external` promises: field order and constructor atoms are the contract
//! (a Gleam record `R(a, b)` is the tuple `{r, A, B}`; `Some(x)` is
//! `{some, X}`, `None` is `none`). The typed covers and itemizations are
//! encoded by their derived `GleamValue` impls (fec-parser, feature
//! `gleam`), generated from the same definitions as their Gleam types; so
//! are every `Option` and date here (never rustler's own `Option`, which is
//! Elixir's `nil` / bare value). The hand-written types are pinned field by
//! field by `gleam/test/`.
//!
//! | NIF | scheduler | returns (Gleam, `libfec/internal/nif`) |
//! |---|---|---|
//! | `open(path, skip_unknown)` | dirty IO | `Result(Opened, FecError)` |
//! | `next_batch(handle, n)` | dirty CPU | `Batch(rows, end)` |
//! | `skipped(handle)` | normal, no lock | `Int` |
//! | `close(handle)` | dirty IO | `Nil` |
//!
//! Header and cover are read and encoded once, by `open`; no NIF reads them
//! afterwards, so they never wait on the lock a dirty `next_batch` holds and
//! cannot fail after `close`.
//!
//! No NIF raises on anything a filing contains: every failure is a value
//! (`{error, _}` or `{failed, _}`), there is no `unwrap` on user data,
//! non-finite floats never reach `enif_make_double` (see [`value`] and
//! `fec_parser::gleam`'s `f64` encoder), and a poisoned mutex is recovered
//! (see [`lock`]).

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use fec_parser::gleam::GleamValue;
use fec_parser::itemizations::Itemization;
use fec_parser::mappings::{column_names_for_field, DATE_COLUMNS, FLOAT_COLUMNS};
use fec_parser::{Filing, FilingHeaderError, FilingRowReadError};
use jiff::civil::Date;
use rustler::types::tuple::make_tuple;
use rustler::{Binary, Encoder, Env, OwnedBinary, ResourceArc, Term};

mod atoms {
    rustler::atoms! {
        ok, error, opened, header, cover_summary,
        batch, more, eof, failed,
        row, text, number, day, empty,
        io_error, unsupported_version, parse_error, missing_mapping, closed,
    }
}

/// The crate version, so the Gleam side can check which NIF it loaded.
#[rustler::nif]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ---------------------------------------------------------------------------
// Reader resource
// ---------------------------------------------------------------------------

/// How a column's raw string becomes a `Value` (the Python binding's rule).
#[derive(Copy, Clone, PartialEq, Eq)]
enum Kind {
    Text,
    Float,
    Date,
}

/// The columns of one row type, with their kinds looked up once.
struct Schema {
    columns: &'static [String],
    kinds: Vec<Kind>,
}

impl Schema {
    fn new(columns: &'static [String]) -> Self {
        let kinds = columns
            .iter()
            .map(|c| {
                // Date first, then float: the precedence of the Python
                // binding and the CLI's Excel export.
                if DATE_COLUMNS.contains(c) {
                    Kind::Date
                } else if FLOAT_COLUMNS.contains(c) {
                    Kind::Float
                } else {
                    Kind::Text
                }
            })
            .collect();
        Schema { columns, kinds }
    }
}

/// What the mutex guards: the open filing and the per-row-type schema cache.
/// The version is fixed per filing, so the row type (as filed) is the whole
/// key; a miss is cached as `None`.
struct State {
    filing: Filing<File>,
    schemas: HashMap<String, Option<Arc<Schema>>>,
}

impl State {
    fn schema(&mut self, row_type: &str, version: &str) -> Option<Arc<Schema>> {
        if let Some(s) = self.schemas.get(row_type) {
            return s.clone();
        }
        let schema = column_names_for_field(row_type, version)
            .ok()
            .map(|cols: &'static Vec<String>| Arc::new(Schema::new(cols.as_slice())));
        self.schemas.insert(row_type.to_owned(), schema.clone());
        schema
    }
}

/// The `Handle` Gleam holds. `close` sets the state to `None` (dropping the
/// file); otherwise the BEAM frees it when the last reference is collected.
pub struct Reader {
    state: Mutex<Option<State>>,
    /// Rows skipped because their type has no column mapping
    /// (`skip_unknown`). Read by `skipped` without the lock.
    skipped: AtomicU64,
    /// `header.fec_version`.
    version: String,
    /// `header.name_delimiter`, for legacy combined names in itemizations.
    name_delimiter: Option<String>,
    /// Skip and count rows with no column mapping instead of failing.
    skip_unknown: bool,
}

#[rustler::resource_impl]
impl rustler::Resource for Reader {}

/// The mutex is only held by this crate's short critical sections, so a
/// poisoned lock means a panic mid-batch; take the state anyway rather than
/// turning every later call into a panic (as `fec-py/src/source.rs` does).
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// `error.FecError`.
enum FecError {
    /// `IoError(message)`
    Io(String),
    /// `UnsupportedVersion(version)`
    UnsupportedVersion(String),
    /// `ParseError(line, message)`; line 0 when unknown.
    Parse(u64, String),
    /// `MissingMapping(row_type, version, line)`
    MissingMapping(String, String, u64),
    /// `Closed`
    Closed,
}

impl Encoder for FecError {
    fn encode<'a>(&self, env: Env<'a>) -> Term<'a> {
        match self {
            FecError::Io(m) => (atoms::io_error(), m.as_str()).encode(env),
            FecError::UnsupportedVersion(v) => {
                (atoms::unsupported_version(), v.as_str()).encode(env)
            }
            FecError::Parse(line, m) => (atoms::parse_error(), *line, m.as_str()).encode(env),
            FecError::MissingMapping(rt, v, line) => {
                (atoms::missing_mapping(), rt.as_str(), v.as_str(), *line).encode(env)
            }
            FecError::Closed => atoms::closed().encode(env),
        }
    }
}

/// An error from `Filing::from_path`: I/O, an unsupported version, or
/// anything else (a malformed header or cover) as `ParseError(0, message)`.
fn open_error(e: &anyhow::Error) -> FecError {
    if let Some(io) = e.downcast_ref::<std::io::Error>() {
        return FecError::Io(io.to_string());
    }
    if let Some(FilingHeaderError::UnsupportedVersion(msg)) = e.downcast_ref::<FilingHeaderError>()
    {
        // "Unsupported version '9.9', supported versions are …" → "9.9";
        // a `/* Header` block without a version line → "".
        let version = msg
            .strip_prefix("Unsupported version '")
            .and_then(|rest| rest.split_once('\''))
            .map(|(v, _)| v.to_owned())
            .unwrap_or_default();
        return FecError::UnsupportedVersion(version);
    }
    FecError::Parse(0, format!("{e:#}"))
}

/// A row-read error as `ParseError(line, message)`.
fn row_error(e: &FilingRowReadError) -> FecError {
    let line = match e {
        FilingRowReadError::CsvError(c) | FilingRowReadError::TextRecordError(c) => {
            c.position().map(|p| p.line()).unwrap_or(0)
        }
        FilingRowReadError::EmptyRecord(line) => *line,
    };
    FecError::Parse(line, e.to_string())
}

// ---------------------------------------------------------------------------
// open
// ---------------------------------------------------------------------------

/// `Some(s)` unless `s` is blank.
fn non_blank(s: Option<&str>) -> Option<String> {
    s.filter(|s| !s.trim().is_empty()).map(str::to_owned)
}

/// Opens a filing and reads its header and cover.
///
/// `{ok, {opened, FilingId, Header, CoverSummary, OptionCover, Handle}}` or
/// `{error, FecError}`, where
///
/// - `FilingId` is the file stem without a `FEC-` prefix
/// - `Header` = `{header, FecVersion, SoftwareName, SoftwareVersion,
///   OptReportId, OptReportNumber, OptComment}` (`header.Header`)
/// - `CoverSummary` = `{cover_summary, FormType, FilerId, FilerName,
///   OptReportCode, OptCoverageFrom, OptCoverageThrough}`
///   (`cover_summary.CoverSummary`; a blank report code is `none`)
/// - `OptionCover` = `none` or `{some, Cover}` (the generated `cover.Cover`;
///   `none` for a form type with no typed cover)
#[rustler::nif(schedule = "DirtyIo")]
fn open<'a>(env: Env<'a>, path: String, skip_unknown: bool) -> Term<'a> {
    let filing = match Filing::<File>::from_path(Path::new(&path)) {
        Ok(f) => f,
        Err(e) => return (atoms::error(), open_error(&e)).encode(env),
    };
    let h = &filing.header;
    let header = make_tuple(
        env,
        &[
            atoms::header().encode(env),
            h.fec_version.encode_gleam(env),
            h.software_name.encode_gleam(env),
            h.software_version.encode_gleam(env),
            h.report_id.encode_gleam(env),
            h.report_number.encode_gleam(env),
            h.comment.encode_gleam(env),
        ],
    );
    let c = &filing.cover;
    let summary = make_tuple(
        env,
        &[
            atoms::cover_summary().encode(env),
            c.form_type.encode_gleam(env),
            c.filer_id.encode_gleam(env),
            c.filer_name.encode_gleam(env),
            non_blank(c.report_code.as_deref()).encode_gleam(env),
            c.coverage_from_date.encode_gleam(env),
            c.coverage_through_date.encode_gleam(env),
        ],
    );
    let cover = c.cover_data.encode_gleam(env);
    let filing_id = filing.filing_id.encode_gleam(env);
    let reader = Reader {
        version: filing.header.fec_version.clone(),
        name_delimiter: filing.header.name_delimiter.clone(),
        skip_unknown,
        skipped: AtomicU64::new(0),
        state: Mutex::new(Some(State {
            filing,
            schemas: HashMap::new(),
        })),
    };
    let opened = make_tuple(
        env,
        &[
            atoms::opened().encode(env),
            filing_id,
            header,
            summary,
            cover,
            ResourceArc::new(reader).encode(env),
        ],
    );
    (atoms::ok(), opened).encode(env)
}

// ---------------------------------------------------------------------------
// next_batch
// ---------------------------------------------------------------------------

/// A `Value`, before encoding; text is a span of the batch buffer.
enum V {
    Text(usize, usize),
    Number(f64),
    Day(Date),
    Empty,
}

/// One decoded row, before encoding.
struct Pending {
    row_type: (usize, usize),
    line: u64,
    schema: Arc<Schema>,
    values: Vec<V>,
    itemization: Option<Itemization>,
    extra: Vec<(usize, usize)>,
}

/// Appends `s` to the batch buffer; returns its span.
fn push(buf: &mut Vec<u8>, s: &str) -> (usize, usize) {
    let span = (buf.len(), s.len());
    buf.extend_from_slice(s.as_bytes());
    span
}

/// The value rule (plans/gleam G4, the Python binding's `Row.value`):
///
/// - a field past the end of a short row → `Empty`
/// - text column: `""` → `Empty`, anything else → `Text(raw)`
/// - date/float column: blank after trimming → `Empty`; a (trimmed)
///   `%Y%m%d` date → `Day`, a (trimmed) finite `f64` → `Number`; anything
///   else → `Text(raw)`, untrimmed, as filed
///
/// Non-finite floats (`nan`, `inf`, `1e999`, which `f64::from_str` accepts
/// and Python keeps) are `Text(raw)` here: the BEAM has no NaN or infinity
/// (`enif_make_double` would raise `badarg`), and they are garbage in an
/// amount column anyway.
fn value(buf: &mut Vec<u8>, kind: Kind, raw: Option<&str>) -> V {
    let Some(raw) = raw else { return V::Empty };
    if kind == Kind::Text {
        if raw.is_empty() {
            return V::Empty;
        }
        let (o, l) = push(buf, raw);
        return V::Text(o, l);
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return V::Empty;
    }
    match kind {
        Kind::Float => {
            if let Some(x) = trimmed.parse::<f64>().ok().filter(|x| x.is_finite()) {
                return V::Number(x);
            }
        }
        Kind::Date => {
            if let Ok(d) = Date::strptime("%Y%m%d", trimmed) {
                return V::Day(d);
            }
        }
        Kind::Text => {}
    }
    let (o, l) = push(buf, raw);
    V::Text(o, l)
}

/// Up to `n` rows: `{batch, Rows, End}`, `End` = `more | eof | {failed, FecError}`.
///
/// - `n` bounds the rows *read*, skipped ones included, so `Rows` can be
///   shorter than `n` with `more`. `eof` only once the file is exhausted;
///   every later call returns `{batch, [], eof}` too.
/// - On an error, `Rows` holds every row decoded before it (G11). A row with
///   no column mapping ends the batch with `{failed, {missing_mapping,
///   RowType, Version, Line}}` (with `skip_unknown`, it is counted and
///   skipped instead); a read error with `{failed, {parse_error, Line,
///   Msg}}`. The offending row is consumed: another call continues after it.
/// - After `close`: `{batch, [], {failed, closed}}`.
///
/// `Row` = `{row, RowType, Line, Values, OptItemization, ExtraFields}`;
/// `Values` = `[{Column, Value}]`, one per mapped column in column order;
/// `Value` = `{text, S} | {number, F} | {day, Date} | empty`;
/// `OptItemization` = `none | {some, Itemization}` (generated encoder);
/// `ExtraFields` = the raw fields past the mapped columns.
///
/// Encoding: row types, text values and extra fields are sub-binaries of
/// one binary per batch (one allocation instead of one per field; a kept
/// sub-binary keeps its whole batch binary alive). Column names are their
/// own small binaries, encoded once per batch and shared by every row of it,
/// so keeping a column name never retains a batch. The typed itemizations'
/// strings are one binary each.
#[rustler::nif(schedule = "DirtyCpu")]
fn next_batch<'a>(env: Env<'a>, reader: ResourceArc<Reader>, n: usize) -> Term<'a> {
    let mut guard = lock(&reader.state);
    let Some(state) = guard.as_mut() else {
        let end = (atoms::failed(), FecError::Closed).encode(env);
        return (atoms::batch(), Vec::<Term>::new(), end).encode(env);
    };

    let mut buf: Vec<u8> = Vec::with_capacity(n.min(10_000) * 256);
    let mut rows: Vec<Pending> = Vec::with_capacity(n.min(10_000));
    // `None` = more.
    let mut end: Option<Result<(), FecError>> = None;
    for _ in 0..n {
        let row = match state.filing.next_row() {
            None => {
                end = Some(Ok(()));
                break;
            }
            Some(Err(e)) => {
                end = Some(Err(row_error(&e)));
                break;
            }
            Some(Ok(row)) => row,
        };
        let Some(schema) = state.schema(&row.row_type, &reader.version) else {
            if reader.skip_unknown {
                reader.skipped.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            end = Some(Err(FecError::MissingMapping(
                row.row_type,
                reader.version.clone(),
                row.line,
            )));
            break;
        };
        let row_type = push(&mut buf, &row.row_type);
        let values = schema
            .kinds
            .iter()
            .enumerate()
            .map(|(i, kind)| value(&mut buf, *kind, row.record.get(i)))
            .collect();
        let extra = row
            .record
            .iter()
            .skip(schema.columns.len())
            .map(|f| push(&mut buf, f))
            .collect();
        let itemization = Itemization::from_record(
            &row.record,
            &reader.version,
            reader.name_delimiter.as_deref(),
        );
        rows.push(Pending {
            row_type,
            line: row.line,
            schema,
            values,
            itemization,
            extra,
        });
    }
    drop(guard);

    let bin: Option<Binary<'a>> = OwnedBinary::new(buf.len()).map(|mut owned| {
        owned.as_mut_slice().copy_from_slice(&buf);
        Binary::from_owned(owned, env)
    });
    // Spans are in range by construction; if the allocation failed, fall
    // back to one binary per string rather than raise.
    let sub = |(o, l): (usize, usize)| -> Term<'a> {
        bin.and_then(|b| b.make_subbinary(o, l).ok())
            .map(|b| b.to_term(env))
            .unwrap_or_else(|| String::from_utf8_lossy(&buf[o..o + l]).encode(env))
    };

    let mut keys: HashMap<*const String, Term<'a>> = HashMap::new();
    let (row_atom, text, number, day) =
        (atoms::row(), atoms::text(), atoms::number(), atoms::day());
    let empty = atoms::empty().encode(env);
    let out: Vec<Term<'a>> = rows
        .into_iter()
        .map(|p| {
            let values: Vec<Term<'a>> = p
                .schema
                .columns
                .iter()
                .zip(p.values)
                .map(|(col, v)| {
                    let key = *keys
                        .entry(col as *const String)
                        .or_insert_with(|| col.as_str().encode(env));
                    let v = match v {
                        V::Empty => empty,
                        V::Text(o, l) => (text, sub((o, l))).encode(env),
                        V::Number(x) => (number, x).encode(env),
                        V::Day(d) => (day, d.encode_gleam(env)).encode(env),
                    };
                    (key, v).encode(env)
                })
                .collect();
            let extra: Vec<Term<'a>> = p.extra.into_iter().map(sub).collect();
            make_tuple(
                env,
                &[
                    row_atom.encode(env),
                    sub(p.row_type),
                    p.line.encode(env),
                    values.encode(env),
                    p.itemization.encode_gleam(env),
                    extra.encode(env),
                ],
            )
        })
        .collect();
    let end = match end {
        None => atoms::more().encode(env),
        Some(Ok(())) => atoms::eof().encode(env),
        Some(Err(e)) => (atoms::failed(), e).encode(env),
    };
    (atoms::batch(), out, end).encode(env)
}

/// Rows skipped so far because their type has no column mapping. No lock.
#[rustler::nif]
fn skipped(reader: ResourceArc<Reader>) -> u64 {
    reader.skipped.load(Ordering::Relaxed)
}

/// Closes the file: later batches are `{failed, closed}`. Waits for a batch
/// in flight to finish; closing twice is fine.
#[rustler::nif(schedule = "DirtyIo")]
fn close(reader: ResourceArc<Reader>) -> rustler::Atom {
    lock(&reader.state).take();
    rustler::types::atom::nil()
}

rustler::init!("libfec_nif");
