//// Read FEC campaign finance filings (`.fec` files).
////
//// The parser is `fec-parser`, a Rust crate, loaded as a NIF. This module
//// is a placeholder until the reader API lands.

import libfec/internal/nif

/// The version of the native library this package loaded.
pub fn native_version() -> String {
  nif.version()
}
