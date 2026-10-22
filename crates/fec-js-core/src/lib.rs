//! The encodings `fec-node` uses to hand batches to JS, kept free of napi so
//! a future wasm32 (browser) binding can share them unchanged
//! (plans/nodejs/06-wasm-spike.md, W2):
//!
//! - [`tokens`]: typed records (any `serde::Serialize`) as a token stream.

pub mod tokens;
