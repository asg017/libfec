//! `NativeSource` and `NativeReader`: a filing's rows as batches the JS layer
//! turns into `Record`s and itemizations.
//!
//! Rust stays small: it parses, optionally types, and ships each batch as one
//! string plus typed arrays, because every JS value created from Rust costs a
//! Node-API call (plans/nodejs/01-bindings.md).
//!
//! A `NativeSource` holds the filing (a path, or the JS buffer, read in place)
//! and hands out any number of `NativeReader`s, each one pass from the top.
//! The mode is fixed when the reader is made:
//!
//! - **raw** (`itemizationRecords()`): fields only; nothing is typed.
//! - **typed** (`itemizations()`): every row goes through
//!   `Itemization::from_record`, and the batch carries one token value per
//!   row (`tokens.rs`): the itemization, or `null` for a row with no typed
//!   struct (a summary record, a schedule not typed yet).

use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use fec_parser::covers::Cover;
use fec_parser::itemizations::Itemization;
use fec_parser::mappings::{column_kind, column_names_for_field, ColumnKind};
use fec_parser::Filing;
use napi::bindgen_prelude::Uint8Array;
use napi_derive::napi;

use crate::arrays::Copied;
use crate::errors::{io_error, parse_error, Result};
use fec_js_core::fields::FieldsWriter;

use crate::tokens::{StructTable, TokenBatch, TokenWriter};
use crate::NativeHeader;

/// A borrowed JS `Uint8Array`, read in place. The reference keeps the buffer
/// alive while the source or any of its readers holds it; the JS docs say not
/// to mutate it meanwhile.
struct JsBytes(Rc<Uint8Array>);

impl AsRef<[u8]> for JsBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

type Source = Box<dyn Read>;

/// The six normalized cover fields (`FilingCover`), dates as ISO strings.
#[derive(Clone)]
#[napi(object, use_nullable = true)]
pub struct NativeCoverSummary {
    pub form_type: String,
    pub filer_id: String,
    pub filer_name: String,
    pub report_code: Option<String>,
    pub coverage_from_date: Option<String>,
    pub coverage_through_date: Option<String>,
}

/// Up to `n` rows: every field of every row concatenated into `text`, with
/// UTF-16 end offsets (JS `slice` counts UTF-16 units). No separators, so a
/// field holding `\n` or `\x1c` survives.
#[napi(object, object_from_js = false, use_nullable = true)]
pub struct Batch {
    pub text: String,
    /// `ends[k]`: UTF-16 end offset of field `k` within `text`.
    #[napi(ts_type = "Uint32Array")]
    pub ends: Copied<u32>,
    /// `row_ends[i]`: index into `ends` one past row `i`'s last field.
    #[napi(ts_type = "Uint32Array")]
    pub row_ends: Copied<u32>,
    /// `lines[i]`: row `i`'s 1-based physical line.
    #[napi(ts_type = "Uint32Array")]
    pub lines: Copied<u32>,
    /// Typed mode: one itemization per row of this batch. `None` in raw mode.
    pub typed: Option<TokenBatch>,
}

#[napi(object)]
pub struct Column {
    pub name: String,
    /// `"text"`, `"amount"` or `"date"`.
    pub kind: String,
}

#[napi]
pub struct NativeReader {
    filing: Option<Filing<Source>>,
    id: Option<String>,
    fec_version: String,
    name_delimiter: Option<String>,
    header: NativeHeader,
    summary: NativeCoverSummary,
    cover_fields: Vec<String>,
    cover_line: u32,
    cover: Option<Cover>,
    typed: bool,
    structs: StructTable,
    tokens: TokenWriter,
    /// A row error met mid-batch: the rows before it go out first, then this.
    pending_error: Option<napi::Error<String>>,
}

impl NativeReader {
    fn open(source: Source, id: Option<String>, len: usize, typed: bool) -> Result<Self> {
        let filing = Filing::from_reader(source, id.clone().unwrap_or_default(), len)
            .map_err(|e| parse_error(&e))?;
        let cover = &filing.cover;
        let summary = NativeCoverSummary {
            form_type: cover.form_type.clone(),
            filer_id: cover.filer_id.clone(),
            filer_name: cover.filer_name.clone(),
            report_code: cover.report_code.clone(),
            coverage_from_date: cover.coverage_from_date.map(|d| d.to_string()),
            coverage_through_date: cover.coverage_through_date.map(|d| d.to_string()),
        };
        Ok(NativeReader {
            id,
            fec_version: filing.header.fec_version.clone(),
            name_delimiter: filing.header.name_delimiter.clone(),
            header: (&filing.header).into(),
            summary,
            cover_fields: cover.record.iter().map(str::to_owned).collect(),
            // The cover is the HDR's next record; a 1.x/2.x header block is longer.
            cover_line: cover.record.position().map_or(2, |p| p.line() as u32),
            cover: cover.cover_data.clone(),
            filing: Some(filing),
            typed,
            structs: StructTable::default(),
            tokens: TokenWriter::default(),
            pending_error: None,
        })
    }
}

/// Open `path`, with its length and its ID (the file stem minus any `FEC-`).
fn open_file(path: &str) -> Result<(File, usize, Option<String>)> {
    let (file, len) = open_at(Path::new(path), path)?;
    let stem = Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned());
    let id = stem.map(|s| s.strip_prefix("FEC-").map(str::to_owned).unwrap_or(s));
    Ok((file, len, id))
}

/// Open `path` and get its length; errors name `given`, the path as the
/// caller wrote it.
fn open_at(path: &Path, given: &str) -> Result<(File, usize)> {
    let file = File::open(path).map_err(|e| io_error(e, given))?;
    let len = file.metadata().map(|m| m.len() as usize).unwrap_or(0);
    Ok((file, len))
}

enum SourceData {
    /// `absolute` is what each pass opens; `given` is for error messages.
    Path {
        absolute: PathBuf,
        given: String,
    },
    Bytes(Rc<Uint8Array>),
}

/// A filing's source, held once, for any number of passes
/// ([`NativeSource::reader`]). A path is reopened for each pass, as the
/// absolute path it named when the source was opened, so a later
/// `process.chdir()` doesn't change which file a pass reads (Python's
/// `Filing` likewise). Bytes are the JS buffer, read in place by every reader.
#[napi]
pub struct NativeSource {
    data: Option<SourceData>,
    id: Option<String>,
    len: usize,
}

#[napi]
impl NativeSource {
    /// Checks the path opens (so `ENOENT` surfaces now), then keeps only the
    /// path, made absolute against the current directory.
    #[napi(factory)]
    pub fn open_path(path: String) -> Result<Self> {
        let (_file, len, id) = open_file(&path)?;
        let absolute = std::path::absolute(&path).map_err(|e| io_error(e, &path))?;
        Ok(NativeSource {
            data: Some(SourceData::Path {
                absolute,
                given: path,
            }),
            id,
            len,
        })
    }

    /// Keeps a reference to the JS buffer (no copy).
    #[napi(factory)]
    pub fn open_bytes(bytes: Uint8Array) -> Self {
        let len = bytes.len();
        NativeSource {
            data: Some(SourceData::Bytes(Rc::new(bytes))),
            id: None,
            len,
        }
    }

    /// A new pass from the top of the filing: header and cover parsed again,
    /// then rows raw or `typed`.
    #[napi]
    pub fn reader(&self, typed: bool) -> Result<NativeReader> {
        match &self.data {
            None => Err(closed_error()),
            Some(SourceData::Path { absolute, given }) => {
                let (file, len) = open_at(absolute, given)?;
                NativeReader::open(Box::new(file), self.id.clone(), len, typed)
            }
            Some(SourceData::Bytes(bytes)) => NativeReader::open(
                Box::new(Cursor::new(JsBytes(bytes.clone()))),
                None,
                self.len,
                typed,
            ),
        }
    }

    /// The file stem minus any `FEC-` prefix; `null` for bytes.
    #[napi(getter)]
    pub fn id(&self) -> Option<String> {
        self.id.clone()
    }

    /// The source's size in bytes (`0` if unknown).
    #[napi(getter)]
    pub fn source_length(&self) -> u32 {
        self.len.min(u32::MAX as usize) as u32
    }

    /// Drop the path or the buffer reference; later `reader()` calls throw
    /// `ERR_FILING_CLOSED`. Readers already made keep their own. Idempotent.
    #[napi]
    pub fn close(&mut self) {
        self.data = None;
    }

    #[napi(getter)]
    pub fn closed(&self) -> bool {
        self.data.is_none()
    }
}

fn closed_error() -> napi::Error<String> {
    napi::Error::new("ERR_FILING_CLOSED".to_owned(), "the filing is closed")
}

#[napi]
impl NativeReader {
    /// A raw (untyped) reader over a path; see [`NativeSource`] for passes.
    #[napi(factory)]
    pub fn open_path(path: String) -> Result<Self> {
        let (file, len, id) = open_file(&path)?;
        Self::open(Box::new(file), id, len, false)
    }

    /// A raw reader over the JS buffer, read in place (no copy).
    #[napi(factory)]
    pub fn open_bytes(bytes: Uint8Array) -> Result<Self> {
        let len = bytes.len();
        Self::open(
            Box::new(Cursor::new(JsBytes(Rc::new(bytes)))),
            None,
            len,
            false,
        )
    }

    /// The file stem minus any `FEC-` prefix; `null` for bytes.
    #[napi(getter)]
    pub fn id(&self) -> Option<String> {
        self.id.clone()
    }

    #[napi(getter)]
    pub fn fec_version(&self) -> String {
        self.fec_version.clone()
    }

    #[napi(getter)]
    pub fn header(&self) -> NativeHeader {
        self.header.clone()
    }

    #[napi(getter)]
    pub fn cover_summary(&self) -> NativeCoverSummary {
        self.summary.clone()
    }

    /// The raw cover record.
    #[napi(getter)]
    pub fn cover_fields(&self) -> Vec<String> {
        self.cover_fields.clone()
    }

    /// The cover record's 1-based physical line.
    #[napi(getter)]
    pub fn cover_line(&self) -> u32 {
        self.cover_line
    }

    /// The typed cover as a self-contained token batch (its own struct ids:
    /// decode it with a fresh decoder): one `Cover { form, data }`, or `null`
    /// when the form has no struct.
    #[napi]
    pub fn cover_data(&self) -> Result<TokenBatch> {
        let mut table = StructTable::default();
        let mut w = TokenWriter::default();
        w.push(&mut table, self.cover.as_ref())
            .map_err(|e| parse_error(&e))?;
        Ok(w.take().into())
    }

    /// Whether batches carry typed tokens (fixed when the reader is made).
    #[napi(getter)]
    pub fn typed(&self) -> bool {
        self.typed
    }

    /// Up to `n` rows, or `null` at the end of the file. In typed mode every
    /// row has one token value, `null` when it has no typed struct.
    #[napi]
    pub fn next_batch(&mut self, n: u32) -> Result<Option<Batch>> {
        if let Some(e) = self.pending_error.take() {
            return Err(e);
        }
        let Some(filing) = self.filing.as_mut() else {
            return Err(closed_error());
        };
        let mut w = FieldsWriter::default();
        while (w.rows() as u32) < n {
            let row = match filing.next_row() {
                None => break,
                Some(Ok(row)) => row,
                Some(Err(e)) => {
                    self.pending_error = Some(parse_error(&e));
                    break;
                }
            };
            if self.typed {
                let it = Itemization::from_record(
                    &row.record,
                    &self.fec_version,
                    self.name_delimiter.as_deref(),
                );
                self.tokens
                    .push(&mut self.structs, it.as_ref())
                    .map_err(|e| parse_error(&e))?;
            }
            w.push(row.record.iter(), row.line as u32);
        }
        if w.rows() == 0 {
            return match self.pending_error.take() {
                Some(e) => Err(e),
                None => Ok(None),
            };
        }
        let fields = w.take();
        Ok(Some(Batch {
            text: fields.text,
            ends: fields.ends.into(),
            row_ends: fields.row_ends.into(),
            lines: fields.lines.into(),
            typed: self.typed.then(|| self.tokens.take().into()),
        }))
    }

    /// Drop the parser (and the file or buffer). Idempotent.
    #[napi]
    pub fn close(&mut self) {
        self.filing = None;
        self.pending_error = None;
    }

    #[napi(getter)]
    pub fn closed(&self) -> bool {
        self.filing.is_none()
    }
}

/// Column names and kinds for `(row_type, fec_version)`.
#[napi]
pub fn schema(row_type: String, fec_version: String) -> Result<Vec<Column>> {
    let names = column_names_for_field(&row_type, &fec_version)
        .map_err(|e| napi::Error::new("FEC_MISSING_MAPPING".to_owned(), format!("{e:#}")))?;
    Ok(names
        .iter()
        .map(|name| Column {
            name: name.clone(),
            kind: match column_kind(name) {
                ColumnKind::Text => "text",
                ColumnKind::Amount => "amount",
                ColumnKind::Date => "date",
            }
            .to_owned(),
        })
        .collect())
}

/// Type one record on demand (`Record.itemization`): `fields` as filed (field
/// 0 the row type), with the filing's version and header name delimiter. A
/// self-contained one-row token batch (its own struct ids: decode it with a
/// fresh decoder), `null` when the record has no typed struct.
#[napi]
pub fn type_record(
    fields: Vec<String>,
    fec_version: String,
    name_delimiter: Option<String>,
) -> Result<TokenBatch> {
    let record = csv::StringRecord::from(fields);
    let it = Itemization::from_record(&record, &fec_version, name_delimiter.as_deref());
    let mut table = StructTable::default();
    let mut w = TokenWriter::default();
    w.push(&mut table, it.as_ref())
        .map_err(|e| parse_error(&e))?;
    Ok(w.take().into())
}

#[cfg(test)]
mod tests {
    /// `js/row.ts` keys `row.values` by column name and counts columns with
    /// `Object.keys`; both assume no layout repeats a name.
    #[test]
    fn mappings_have_no_duplicate_column_names() {
        let mut dups = vec![];
        for (f, versions) in fec_parser::mappings::COLUMN_NAMES.iter().enumerate() {
            for (v, cols) in versions.iter().enumerate() {
                let mut seen = std::collections::HashSet::new();
                for c in cols {
                    if !seen.insert(c) {
                        dups.push(format!("{} v{v}: {c}", fec_parser::mappings::FORM_TYPES[f]));
                    }
                }
            }
        }
        assert!(dups.is_empty(), "{dups:#?}");
    }
}
