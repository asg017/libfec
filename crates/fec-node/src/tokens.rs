//! Typed-record tokens for JS. The serializer itself is napi-free and lives
//! in `fec-js-core` (shared with a future wasm build); this is the napi
//! object a batch crosses the boundary as. `js/tokens.ts` decodes it.

use napi_derive::napi;

use crate::arrays::Copied;

pub use fec_js_core::tokens::{StructTable, TokenWriter};

/// One batch of typed values, one top-level value per row
/// (`fec_js_core::tokens::Tokens`).
#[napi(object, object_from_js = false)]
pub struct TokenBatch {
    /// Top-level values in this batch.
    pub rows: u32,
    #[napi(ts_type = "Uint8Array")]
    pub tags: Copied<u8>,
    #[napi(ts_type = "Float64Array")]
    pub nums: Copied<f64>,
    /// Every string value, concatenated.
    pub text: String,
    /// UTF-16 end offset of each string value in `text`.
    #[napi(ts_type = "Uint32Array")]
    pub ends: Copied<u32>,
    /// Struct types first seen in this batch: `"id\x1fName\x1fkey1\x1fkey2…"`.
    pub new_structs: Vec<String>,
}

impl From<fec_js_core::tokens::Tokens> for TokenBatch {
    fn from(t: fec_js_core::tokens::Tokens) -> Self {
        TokenBatch {
            rows: t.rows,
            tags: t.tags.into(),
            nums: t.nums.into(),
            text: t.text,
            ends: t.ends.into(),
            new_structs: t.new_structs,
        }
    }
}
