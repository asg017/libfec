//! wasm-bindgen bindings for the browser build of `@alex.garcia/libfec`.
//!
//! This is the same native surface as `crates/fec-node` (napi), so the whole
//! TypeScript layer in `crates/fec-node/js` is shared: only `js/native.ts` is
//! swapped for this crate's `js/native.ts`. The batch encodings (raw field
//! offsets, typed-record tokens) are `fec-js-core`'s, byte for byte the
//! napi build's. Differences, all in the adapter's favor of the platform:
//!
//! - sources are bytes only (`openPath`/`readHeaderPath` throw
//!   `ERR_UNSUPPORTED_SOURCE`), copied once into wasm memory;
//! - batches are plain objects built with `js_sys`, typed arrays copied out.

use wasm_bindgen::prelude::*;

mod js;
pub mod reader;
pub mod rss;

use js::{opt_str, Obj};

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
/** One `key = value` line of a 1.x/2.x `/* Header` block, in file order. */
export interface NativeHeaderField {
  key: string
  value: string
}

/** `fec_parser::FilingHeader`, field for field. */
export interface NativeHeader {
  recordType: string
  efType: string
  fecVersion: string
  softwareName: string
  softwareVersion: string
  reportId: string | null
  reportNumber: string | null
  comment: string | null
  style: string
  delimiter: string
  nameDelimiter: string | null
  batchNumber: string | null
  receivedDate: string | null
  isPaper: boolean
  legacyFields: Array<NativeHeaderField>
  scheduleCounts: Array<NativeHeaderField>
}

/** The six normalized cover fields (`FilingCover`), dates as ISO strings. */
export interface NativeCoverSummary {
  formType: string
  filerId: string
  filerName: string
  reportCode: string | null
  coverageFromDate: string | null
  coverageThroughDate: string | null
}

/** One batch of typed values, one top-level value per row. */
export interface TokenBatch {
  rows: number
  tags: Uint8Array
  nums: Float64Array
  text: string
  ends: Uint32Array
  newStructs: Array<string>
}

/** Up to `n` rows as one string plus UTF-16 offsets. */
export interface Batch {
  text: string
  ends: Uint32Array
  rowEnds: Uint32Array
  lines: Uint32Array
  typed: TokenBatch | null
}

export interface Column {
  name: string
  /** `"text"`, `"amount"` or `"date"`. */
  kind: string
}

export interface NativeRssItem {
  title: string
  link: string
  description: string
  pubDate: number | null
  guid: string
  committeeId: string | null
  filingId: string | null
  formType: string | null
  coverageFrom: string | null
  coverageThrough: string | null
  reportType: string | null
  committeeName: string
}

export interface NativeRssFeed {
  title: string
  link: string
  description: string
  items: Array<NativeRssItem>
}
"#;

/// The crate version (lockstep with `crates/fec-node`).
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// `fec_parser::FilingHeader` as a `NativeHeader`.
pub(crate) fn header_object(h: &fec_parser::FilingHeader) -> JsValue {
    let fields = |m: &indexmap::IndexMap<String, String>| -> js_sys::Array {
        m.iter()
            .map(|(k, v)| {
                Obj::new()
                    .set("key", k.as_str())
                    .set("value", v.as_str())
                    .build()
            })
            .collect()
    };
    Obj::new()
        .set("recordType", h.record_type.as_str())
        .set("efType", h.ef_type.as_str())
        .set("fecVersion", h.fec_version.as_str())
        .set("softwareName", h.software_name.as_str())
        .set("softwareVersion", h.software_version.as_str())
        .set("reportId", opt_str(h.report_id.as_deref()))
        .set("reportNumber", opt_str(h.report_number.as_deref()))
        .set("comment", opt_str(h.comment.as_deref()))
        .set("style", h.style.as_str())
        .set("delimiter", h.delimiter.as_str())
        .set("nameDelimiter", opt_str(h.name_delimiter.as_deref()))
        .set("batchNumber", opt_str(h.batch_number.as_deref()))
        .set("receivedDate", opt_str(h.received_date.as_deref()))
        .set("isPaper", h.is_paper())
        .set("legacyFields", fields(&h.legacy_fields))
        .set("scheduleCounts", fields(&h.schedule_counts))
        .build()
}

/// Read only the header (the `HDR` record, or a 1.x/2.x `/* Header` block).
#[wasm_bindgen(js_name = readHeaderBytes, unchecked_return_type = "NativeHeader")]
pub fn read_header_bytes(bytes: &[u8]) -> Result<JsValue, JsValue> {
    fec_parser::read_header(bytes)
        .map(|(header, _lines)| header_object(&header))
        .map_err(|e| js::parse_error(&e))
}

/// `fec_parser::cover_end`: how many bytes of `prefix` settle the header and
/// cover, or `undefined` while they could depend on bytes after it.
#[wasm_bindgen(js_name = coverEnd)]
pub fn cover_end(prefix: &[u8]) -> Option<u32> {
    fec_parser::cover_end(prefix).map(|n| n as u32)
}

/// Always throws `ERR_UNSUPPORTED_SOURCE`: there are no paths in the browser.
#[wasm_bindgen(js_name = readHeaderPath, unchecked_return_type = "NativeHeader")]
pub fn read_header_path(path: String) -> Result<JsValue, JsValue> {
    Err(js::unsupported_source(&path))
}
