//! Node-API bindings for `fec-parser`, loaded by the generated
//! `native/native.js`. Everything here is private to the package: the public
//! API is the TypeScript layer in `js/`.

use std::fs::File;
use std::io::Read;

use napi_derive::napi;

pub mod arrays;
#[cfg(debug_assertions)]
pub mod debug;
pub mod errors;
pub mod reader;
pub mod tokens;

/// The crate version (lockstep with the workspace).
#[napi]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// One `key = value` line of a 1.x/2.x `/* Header` block, in file order.
#[derive(Clone)]
#[napi(object)]
pub struct NativeHeaderField {
    pub key: String,
    pub value: String,
}

/// `fec_parser::FilingHeader`, field for field (camelCased by napi-rs).
#[derive(Clone)]
#[napi(object, use_nullable = true)]
pub struct NativeHeader {
    pub record_type: String,
    pub ef_type: String,
    pub fec_version: String,
    pub software_name: String,
    pub software_version: String,
    pub report_id: Option<String>,
    pub report_number: Option<String>,
    pub comment: Option<String>,
    pub style: String,
    pub delimiter: String,
    pub name_delimiter: Option<String>,
    pub batch_number: Option<String>,
    pub received_date: Option<String>,
    pub is_paper: bool,
    pub legacy_fields: Vec<NativeHeaderField>,
    pub schedule_counts: Vec<NativeHeaderField>,
}

impl From<&fec_parser::FilingHeader> for NativeHeader {
    fn from(h: &fec_parser::FilingHeader) -> Self {
        let fields = |m: &indexmap::IndexMap<String, String>| {
            m.iter()
                .map(|(key, value)| NativeHeaderField {
                    key: key.clone(),
                    value: value.clone(),
                })
                .collect()
        };
        NativeHeader {
            is_paper: h.is_paper(),
            style: h.style.as_str().to_owned(),
            delimiter: h.delimiter.as_str().to_owned(),
            record_type: h.record_type.clone(),
            ef_type: h.ef_type.clone(),
            fec_version: h.fec_version.clone(),
            software_name: h.software_name.clone(),
            software_version: h.software_version.clone(),
            report_id: h.report_id.clone(),
            report_number: h.report_number.clone(),
            comment: h.comment.clone(),
            name_delimiter: h.name_delimiter.clone(),
            batch_number: h.batch_number.clone(),
            received_date: h.received_date.clone(),
            legacy_fields: fields(&h.legacy_fields),
            schedule_counts: fields(&h.schedule_counts),
        }
    }
}

/// Read only the header (the `HDR` record, or a 1.x/2.x `/* Header` block).
fn read_header(rdr: impl Read) -> errors::Result<NativeHeader> {
    fec_parser::read_header(rdr)
        .map(|(header, _lines)| (&header).into())
        .map_err(|e| errors::parse_error(&e))
}

#[napi]
pub fn read_header_path(path: String) -> errors::Result<NativeHeader> {
    read_header(File::open(&path).map_err(|e| errors::io_error(e, &path))?)
}

#[napi]
pub fn read_header_bytes(bytes: &[u8]) -> errors::Result<NativeHeader> {
    read_header(bytes)
}
