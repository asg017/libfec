//! Rustler NIF behind the `libfec` Gleam package (`gleam/`).
//!
//! Loaded by `gleam/src/libfec_nif.erl`; every NIF here needs a stub export
//! there and an `@external` in `gleam/src/libfec/internal/nif.gleam`.

/// The crate version, so the Gleam side can check which NIF it loaded.
#[rustler::nif]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

rustler::init!("libfec_nif");
