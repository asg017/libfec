//! Node-API bindings for `fec-parser`, loaded by the generated
//! `native/native.js`. Everything here is private to the package: the public
//! API is the TypeScript layer in `js/`.

use napi_derive::napi;

/// The crate version (lockstep with the workspace).
#[napi]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
