use extendr_api::prelude::*;

/// Version of the bundled fec-parser crate
///
/// Smoke test for the Rust bindings.
///
/// @return A string such as `"0.1.0"`.
/// @export
/// @examples
/// fec_version_info()
#[extendr]
fn fec_version_info() -> &'static str {
    env!("FEC_PARSER_VERSION")
}

// Macro to generate exports.
// This ensures exported functions are registered with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod libfec;
    fn fec_version_info;
}
