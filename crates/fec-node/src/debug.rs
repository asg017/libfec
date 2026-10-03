//! Test-only exports (debug builds only, never in a release addon): the
//! typed values of a whole file as tokens *and* as serde_json, so the JS
//! decoder can be checked against serde's output.

use napi_derive::napi;

use crate::tokens::{StructTable, TokenBatch, TokenWriter};

#[napi(object, object_from_js = false)]
pub struct DebugTyped {
    /// The typed cover (one value, `null` if the form has no struct).
    pub cover: TokenBatch,
    pub cover_json: String,
    /// One value per row: its `Itemization`, or `null`.
    pub rows: TokenBatch,
    pub rows_json: Vec<String>,
}

#[napi]
pub fn debug_typed_path(path: String) -> napi::Result<DebugTyped> {
    let err = |e: &dyn std::fmt::Display| napi::Error::from_reason(e.to_string());
    let mut filing =
        fec_parser::Filing::<std::fs::File>::from_path(path.as_ref()).map_err(|e| err(&e))?;
    let version = filing.header.fec_version.clone();
    let delimiter = filing.header.name_delimiter.clone();
    let mut table = StructTable::default();

    let mut w = TokenWriter::default();
    let cover_data = filing.cover.cover_data.as_ref();
    w.push(&mut table, cover_data).map_err(|e| err(&e))?;
    let cover = w.take().into();
    let cover_json = serde_json::to_string(&cover_data).map_err(|e| err(&e))?;

    let mut rows_json = Vec::new();
    while let Some(row) = filing.next_row() {
        let row = row.map_err(|e| err(&e))?;
        let it = fec_parser::itemizations::Itemization::from_record(
            &row.record,
            &version,
            delimiter.as_deref(),
        );
        w.push(&mut table, it.as_ref()).map_err(|e| err(&e))?;
        rows_json.push(serde_json::to_string(&it).map_err(|e| err(&e))?);
    }
    Ok(DebugTyped {
        cover,
        cover_json,
        rows: w.take().into(),
        rows_json,
    })
}
