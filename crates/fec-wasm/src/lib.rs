use fec_parser::{Filing, FilingHeader};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

/// FEC Filing header
#[derive(Tsify, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct FilingHeaderJs {
    /// Record type XXX
    pub record_type: String,
    pub ef_type: String,
    pub fec_version: String,
    pub software_name: String,
    pub software_version: String,
    pub report_id: Option<String>,
    pub report_number: Option<String>,
    pub comment: Option<String>,
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
        }
    }
}

/// greet bro
#[wasm_bindgen]
pub fn header(body: &[u8]) -> FilingHeaderJs {
    let f = Filing::from_reader(body, "123".to_string(), body.len()).unwrap();
    FilingHeaderJs::from(&f.header)
}
