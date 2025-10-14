use std::io::BufReader;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use fec_parser::{Filing, FilingHeader};
use tsify::Tsify;

/// FEC Filing header
#[derive(Tsify, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct FilingHeaderJs {
    /// doc string?
    pub fec_version: String,
}

impl FilingHeaderJs {
    fn from(filing_header: &FilingHeader) -> Self {
        Self {
            fec_version: filing_header.fec_version.clone(),
        }
    }
}

/// greet bro
#[wasm_bindgen]
pub fn greet(name: &str) -> FilingHeaderJs {
  let x = std::io::Cursor::new(include_bytes!("../../1890336.fec"));
  let f = Filing::from_reader(x, "123".to_string(), None).unwrap_or("TODO");
  FilingHeaderJs::from(&f.header)
}