use fec_parser::{Filing, FilingHeader};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

/// FEC Filing header
#[derive(Tsify, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct FilingHeaderJs {
    /// `HDR`, or `/*` for an electronic 1.x/2.x `/* Header` block.
    pub record_type: String,
    pub ef_type: String,
    pub fec_version: String,
    pub software_name: String,
    pub software_version: String,
    pub report_id: Option<String>,
    pub report_number: Option<String>,
    pub comment: Option<String>,
    /// How the header is written: `"hdr"`, `"legacy_block"` or `"paper"`.
    pub style: String,
    /// Body field delimiter: `"fs"` (0x1C) or `"comma"`.
    pub delimiter: String,
    /// Sub-delimiter of combined name fields (3.x–5.x and 1.x/2.x only).
    pub name_delimiter: Option<String>,
    /// Paper filings: FEC data-entry batch number.
    pub batch_number: Option<String>,
    /// Paper filings P2.6+: date the FEC received the filing.
    pub received_date: Option<String>,
    /// Whether this is FEC data entry of a paper filing.
    pub is_paper: bool,
    /// `/* Header` block: `[key, value]` pairs before `Schedule_Counts:`, keys
    /// as written, in file order. Empty otherwise.
    pub legacy_fields: Vec<(String, String)>,
    /// `/* Header` block: `[row type, count]` pairs. Empty otherwise.
    pub schedule_counts: Vec<(String, String)>,
}

fn pairs<'a>(m: impl IntoIterator<Item = (&'a String, &'a String)>) -> Vec<(String, String)> {
    m.into_iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

impl FilingHeaderJs {
    fn from(filing_header: &FilingHeader) -> Self {
        Self {
            record_type: filing_header.record_type.clone(),
            ef_type: filing_header.ef_type.clone(),
            fec_version: filing_header.fec_version.clone(),
            software_name: filing_header.software_name.clone(),
            software_version: filing_header.software_version.clone(),
            report_id: filing_header.report_id.clone(),
            report_number: filing_header.report_number.clone(),
            comment: filing_header.comment.clone(),
            style: filing_header.style.as_str().to_owned(),
            delimiter: filing_header.delimiter.as_str().to_owned(),
            name_delimiter: filing_header.name_delimiter.clone(),
            batch_number: filing_header.batch_number.clone(),
            received_date: filing_header.received_date.clone(),
            is_paper: filing_header.is_paper(),
            legacy_fields: pairs(&filing_header.legacy_fields),
            schedule_counts: pairs(&filing_header.schedule_counts),
        }
    }
}

/// Parse the header of a filing (its cover must also be readable); throws a
/// JS `Error` on unreadable input instead of trapping.
#[wasm_bindgen]
pub fn header(body: &[u8]) -> Result<FilingHeaderJs, JsError> {
    let f = Filing::from_reader(body, "123".to_string(), body.len())
        .map_err(|e| JsError::new(&format!("{e:#}")))?;
    Ok(FilingHeaderJs::from(&f.header))
}
