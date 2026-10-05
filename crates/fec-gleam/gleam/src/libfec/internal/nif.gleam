//// The raw NIF bindings. Every `@external` to `libfec_nif` lives here.
////
//// An `@external` return type is not checked: the Rust side must build
//// exactly the term the Gleam type describes.

/// The version of the loaded NIF crate (`fec-gleam`'s Cargo version).
@external(erlang, "libfec_nif", "version")
pub fn version() -> String
