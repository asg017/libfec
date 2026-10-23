//! `NativeSource` and `NativeReader`: the wasm twins of
//! `crates/fec-node/src/reader.rs`, same methods and the same batch
//! encodings, so the shared `js/filing.ts` drives both.
//!
//! A `NativeSource` copies the bytes into wasm memory once; each of its
//! readers is one pass over that copy (no copy per pass). In typed mode every
//! row has one token value, `null` for a row with no typed struct.

use std::io::Cursor;
use std::rc::Rc;

use fec_js_core::fields::FieldsWriter;
use fec_js_core::tokens::{StructTable, TokenWriter, Tokens};
use fec_parser::covers::Cover;
use fec_parser::itemizations::Itemization;
use fec_parser::mappings::{column_kind, column_names_for_field, ColumnKind};
use fec_parser::Filing;
use js_sys::{Float64Array, Uint32Array, Uint8Array};
use wasm_bindgen::prelude::*;

use crate::js::{coded_error, opt_str, parse_error, str_array, unsupported_source, Obj};

/// The bytes, copied into wasm memory once (by wasm-bindgen, into the `Vec`),
/// shared by a source and its readers. `Rc<Vec<u8>>` rather than `Rc<[u8]>`:
/// moving the `Vec` in doesn't copy the bytes a second time.
#[derive(Clone)]
struct RcBytes(Rc<Vec<u8>>);

impl AsRef<[u8]> for RcBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

type Source = Cursor<RcBytes>;

/// A `fec_js_core::tokens::Tokens` batch as a `TokenBatch` object.
fn token_batch(t: Tokens) -> JsValue {
    Obj::new()
        .set("rows", t.rows)
        .set("tags", Uint8Array::from(&t.tags[..]))
        .set("nums", Float64Array::from(&t.nums[..]))
        .set("text", t.text)
        .set("ends", Uint32Array::from(&t.ends[..]))
        .set(
            "newStructs",
            str_array(t.new_structs.iter().map(String::as_str)),
        )
        .build()
}

#[wasm_bindgen]
pub struct NativeReader {
    filing: Option<Filing<Source>>,
    fec_version: String,
    name_delimiter: Option<String>,
    header: JsValue,
    summary: JsValue,
    cover_fields: Vec<String>,
    cover_line: u32,
    cover: Option<Cover>,
    typed: bool,
    structs: StructTable,
    tokens: TokenWriter,
    fields: FieldsWriter,
    /// A row error met mid-batch: the rows before it go out first, then this.
    pending_error: Option<JsValue>,
}

/// A filing's bytes, copied into wasm memory once, for any number of passes
/// ([`NativeSource::reader`]).
#[wasm_bindgen]
pub struct NativeSource {
    bytes: Option<RcBytes>,
    len: usize,
}

#[wasm_bindgen]
impl NativeSource {
    /// Always throws `ERR_UNSUPPORTED_SOURCE`: there are no paths in the browser.
    #[wasm_bindgen(js_name = openPath)]
    pub fn open_path(path: String) -> Result<NativeSource, JsValue> {
        Err(unsupported_source(&path))
    }

    /// Takes ownership of a copy of the bytes (the JS buffer is not kept).
    #[wasm_bindgen(js_name = openBytes)]
    pub fn open_bytes(bytes: Vec<u8>) -> NativeSource {
        NativeSource {
            len: bytes.len(),
            bytes: Some(RcBytes(Rc::new(bytes))),
        }
    }

    /// A new pass from the top of the filing over the same bytes: header and
    /// cover parsed again, then rows raw or `typed`.
    pub fn reader(&self, typed: bool) -> Result<NativeReader, JsValue> {
        let Some(bytes) = &self.bytes else {
            return Err(closed_error());
        };
        NativeReader::open(bytes.clone(), typed)
    }

    /// Always `null`: bytes have no file name (`new Filing(bytes, { id })` sets it).
    #[wasm_bindgen(getter, unchecked_return_type = "string | null")]
    pub fn id(&self) -> JsValue {
        JsValue::NULL
    }

    /// The source's size in bytes.
    #[wasm_bindgen(getter, js_name = sourceLength)]
    pub fn source_length(&self) -> u32 {
        self.len as u32
    }

    /// Drop this handle on the bytes; later `reader()` calls throw
    /// `ERR_FILING_CLOSED`. The memory is freed once every reader is closed
    /// too. Idempotent.
    pub fn close(&mut self) {
        self.bytes = None;
    }

    #[wasm_bindgen(getter)]
    pub fn closed(&self) -> bool {
        self.bytes.is_none()
    }
}

fn closed_error() -> JsValue {
    coded_error("ERR_FILING_CLOSED", "the filing is closed")
}

impl NativeReader {
    fn open(bytes: RcBytes, typed: bool) -> Result<NativeReader, JsValue> {
        let len = bytes.0.len();
        let filing = Filing::from_reader(Cursor::new(bytes), String::new(), len)
            .map_err(|e| parse_error(&e))?;
        let cover = &filing.cover;
        let summary = Obj::new()
            .set("formType", cover.form_type.as_str())
            .set("filerId", cover.filer_id.as_str())
            .set("filerName", cover.filer_name.as_str())
            .set("reportCode", opt_str(cover.report_code.as_deref()))
            .set(
                "coverageFromDate",
                opt_str(cover.coverage_from_date.map(|d| d.to_string()).as_deref()),
            )
            .set(
                "coverageThroughDate",
                opt_str(
                    cover
                        .coverage_through_date
                        .map(|d| d.to_string())
                        .as_deref(),
                ),
            )
            .build();
        Ok(NativeReader {
            fec_version: filing.header.fec_version.clone(),
            name_delimiter: filing.header.name_delimiter.clone(),
            header: crate::header_object(&filing.header),
            summary,
            cover_fields: cover.record.iter().map(str::to_owned).collect(),
            // The cover is the HDR's next record; a 1.x/2.x header block is longer.
            cover_line: cover.record.position().map_or(2, |p| p.line() as u32),
            cover: cover.cover_data.clone(),
            filing: Some(filing),
            typed,
            structs: StructTable::default(),
            tokens: TokenWriter::default(),
            fields: FieldsWriter::default(),
            pending_error: None,
        })
    }
}

#[wasm_bindgen]
impl NativeReader {
    /// Always throws `ERR_UNSUPPORTED_SOURCE`: there are no paths in the browser.
    #[wasm_bindgen(js_name = openPath)]
    pub fn open_path(path: String) -> Result<NativeReader, JsValue> {
        Err(unsupported_source(&path))
    }

    /// A raw reader over a copy of the bytes; see [`NativeSource`] for passes.
    #[wasm_bindgen(js_name = openBytes)]
    pub fn open_bytes(bytes: Vec<u8>) -> Result<NativeReader, JsValue> {
        NativeReader::open(RcBytes(Rc::new(bytes)), false)
    }

    /// Always `null`: bytes have no file name (`new Filing(bytes, { id })` sets it).
    #[wasm_bindgen(getter, unchecked_return_type = "string | null")]
    pub fn id(&self) -> JsValue {
        JsValue::NULL
    }

    #[wasm_bindgen(getter, js_name = fecVersion)]
    pub fn fec_version(&self) -> String {
        self.fec_version.clone()
    }

    #[wasm_bindgen(getter, unchecked_return_type = "NativeHeader")]
    pub fn header(&self) -> JsValue {
        self.header.clone()
    }

    #[wasm_bindgen(getter, js_name = coverSummary, unchecked_return_type = "NativeCoverSummary")]
    pub fn cover_summary(&self) -> JsValue {
        self.summary.clone()
    }

    /// The raw cover record.
    #[wasm_bindgen(getter, js_name = coverFields)]
    pub fn cover_fields(&self) -> Vec<String> {
        self.cover_fields.clone()
    }

    /// The cover record's 1-based physical line.
    #[wasm_bindgen(getter, js_name = coverLine)]
    pub fn cover_line(&self) -> u32 {
        self.cover_line
    }

    /// The typed cover as a self-contained token batch: one
    /// `Cover { form, data }`, or `null` when the form has no struct.
    #[wasm_bindgen(js_name = coverData, unchecked_return_type = "TokenBatch")]
    pub fn cover_data(&self) -> Result<JsValue, JsValue> {
        let mut table = StructTable::default();
        let mut w = TokenWriter::default();
        w.push(&mut table, self.cover.as_ref())
            .map_err(|e| parse_error(&e))?;
        Ok(token_batch(w.take()))
    }

    /// Whether batches carry typed tokens (fixed when the reader is made).
    #[wasm_bindgen(getter)]
    pub fn typed(&self) -> bool {
        self.typed
    }

    /// Up to `n` rows, or `null` at the end of the file. In typed mode every
    /// row has one token value, `null` when it has no typed struct.
    #[wasm_bindgen(js_name = nextBatch, unchecked_return_type = "Batch | null")]
    pub fn next_batch(&mut self, n: u32) -> Result<JsValue, JsValue> {
        if let Some(e) = self.pending_error.take() {
            return Err(e);
        }
        let Some(filing) = self.filing.as_mut() else {
            return Err(closed_error());
        };
        let w = &mut self.fields;
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
                None => Ok(JsValue::NULL),
            };
        }
        let fields = w.take();
        let typed = if self.typed {
            token_batch(self.tokens.take())
        } else {
            JsValue::NULL
        };
        Ok(Obj::new()
            .set("text", fields.text)
            .set("ends", Uint32Array::from(&fields.ends[..]))
            .set("rowEnds", Uint32Array::from(&fields.row_ends[..]))
            .set("lines", Uint32Array::from(&fields.lines[..]))
            .set("typed", typed)
            .build())
    }

    /// Drop the parser and this reader's handle on the bytes. Idempotent.
    /// (The wasm heap keeps its size; the memory is reused by the next filing.)
    pub fn close(&mut self) {
        self.filing = None;
        self.pending_error = None;
    }

    #[wasm_bindgen(getter)]
    pub fn closed(&self) -> bool {
        self.filing.is_none()
    }
}

/// Type one record on demand (`Record.itemization`): `fields` as filed (field
/// 0 the row type), with the filing's version and header name delimiter. A
/// self-contained one-row token batch (its own struct ids: decode it with a
/// fresh decoder), `null` when the record has no typed struct.
#[wasm_bindgen(js_name = typeRecord, unchecked_return_type = "TokenBatch")]
pub fn type_record(
    fields: Vec<String>,
    #[wasm_bindgen(js_name = fecVersion)] fec_version: String,
    #[wasm_bindgen(js_name = nameDelimiter)] name_delimiter: Option<String>,
) -> Result<JsValue, JsValue> {
    let record = csv::StringRecord::from(fields);
    let it = Itemization::from_record(&record, &fec_version, name_delimiter.as_deref());
    let mut table = StructTable::default();
    let mut w = TokenWriter::default();
    w.push(&mut table, it.as_ref())
        .map_err(|e| parse_error(&e))?;
    Ok(token_batch(w.take()))
}

/// Column names and kinds for `(row_type, fec_version)`.
#[wasm_bindgen(unchecked_return_type = "Array<Column>")]
pub fn schema(
    #[wasm_bindgen(js_name = rowType)] row_type: String,
    #[wasm_bindgen(js_name = fecVersion)] fec_version: String,
) -> Result<JsValue, JsValue> {
    let names = column_names_for_field(&row_type, &fec_version)
        .map_err(|e| coded_error("FEC_MISSING_MAPPING", &format!("{e:#}")))?;
    Ok(names
        .iter()
        .map(|name| {
            let kind = match column_kind(name) {
                ColumnKind::Text => "text",
                ColumnKind::Amount => "amount",
                ColumnKind::Date => "date",
            };
            Obj::new()
                .set("name", name.as_str())
                .set("kind", kind)
                .build()
        })
        .collect::<js_sys::Array>()
        .into())
}
