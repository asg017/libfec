//! Exposes the resolved fec-parser version as `FEC_PARSER_VERSION`.
//!
//! fec-parser has no version constant, and its path changes once the package
//! is vendored (ticket 03), so read it from this crate's committed Cargo.lock,
//! which travels with the package.

use std::{env, fs, path::Path};

fn main() {
    let lock = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.lock");
    println!("cargo:rerun-if-changed={}", lock.display());

    let text = fs::read_to_string(&lock).unwrap_or_default();
    let version = text
        .split("[[package]]")
        .find(|pkg| pkg.lines().any(|l| l.trim() == r#"name = "fec-parser""#))
        .and_then(|pkg| {
            pkg.lines()
                .find_map(|l| l.trim().strip_prefix("version = "))
                .map(|v| v.trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=FEC_PARSER_VERSION={version}");
}
