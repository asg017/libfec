# libfec (R)

R bindings for libfec: read FEC electronic filings with the `fec-parser` Rust crate, via
[extendr](https://extendr.github.io/). Work in progress.

```r
library(libfec)
fec_version_info()
#> [1] "0.1.0"
```

## Layout

This directory is an R package root. The Rust crate (`fecr`, static lib `libfec`) lives in
`src/rust/`, depends on `../../../fec-parser` by path, and has its own `[workspace]`. The root
`Cargo.toml` excludes `crates/fec-r`, so `cargo` at the repo root never sees it.
`src/rust/Cargo.lock` is committed (offline builds need it).

## Development

Needs R >= 4.2, rustc >= 1.85, and `devtools`.

```bash
cd crates/fec-r
make document   # devtools::document(): compiles Rust, regenerates R/extendr-wrappers.R, man/, NAMESPACE
make install    # R CMD INSTALL .
make test       # devtools::test()
```

If `R/extendr-wrappers.R` still shows stale functions after `make document`, the wrapper step
failed; read the build output. A cold build takes ~1.5 min: the Makevars points `CARGO_HOME` at a
temporary `src/.cargo` (CRAN forbids writing to `~`) and deletes it after every build.

## Dev notes: scaffold gotchas

The package was scaffolded with:

```r
options(usethis.allow_nested_project = TRUE)
usethis::create_package("crates/fec-r", open = FALSE, rstudio = FALSE, check_name = FALSE,
                        fields = list(Package = "libfec"))
setwd("crates/fec-r"); rextendr::use_extendr(crate_name = "fecr", lib_name = "libfec")
```

- `usethis::create_package()` refuses to nest inside the libfec repo when run non-interactively
  unless `options(usethis.allow_nested_project = TRUE)` is set.
- `src/rust/Cargo.toml` sits under the libfec workspace, so cargo errors that it "believes it's in
  a workspace when it's not". It needs its own empty `[workspace]`, and the root `Cargo.toml`
  `exclude`s `crates/fec-r`.
- **extendr is pinned to git rev `74afddc`, not crates.io.** rextendr 0.5.0's
  `src/rust/document.c` calls `write__make_<lib>_wrappers`, but extendr-api 0.9.0 on crates.io
  only exports `wrap__make_<lib>_wrappers`, so `devtools::document()` fails to link with
  `Undefined symbols … _write__make_libfec_wrappers`. The git version generates the `write__`
  symbol (extendr PR #1092, unreleased as of 0.9.0). Move to a crates.io release once extendr
  0.9.1/0.10 ships. `extendr-ffi` is pinned to the same rev.
- The scaffold writes `extendr-api = '*'` (from `options("rextendr.extendr_deps")`), which
  resolves to the broken crates.io version above. Always pin.
- In extendr 0.9, an `#[extendr] impl` block also needs `#[extendr]` on the struct (otherwise
  you get `TryFrom<&Robj>` trait-bound errors), and `extendr_api::prelude` has no one-argument
  `Result<T>`: define `type Result<T> = std::result::Result<T, Error>`.
- `Package: libfec` and `lib_name = "libfec"` must agree: `useDynLib(libfec, …)` in
  `R/extendr-wrappers.R`, `R_init_libfec` in `src/entrypoint.c`, and `mod libfec;` in
  `extendr_module!` all derive from it. The cargo static library is `liblibfec.a`
  (`-llibfec` in `src/Makevars.in`).
- `fec_version_info()` reads the fec-parser version from `src/rust/Cargo.lock` at build time
  (`src/rust/build.rs`), since fec-parser exposes no version constant and its path changes once
  the package is vendored.
- `R CMD INSTALL .` in place hides broken path dependencies: `R CMD build` copies only this
  directory, so `../../../fec-parser` doesn't exist in the tarball. Only a tarball installed from
  outside the repo proves the package builds.
