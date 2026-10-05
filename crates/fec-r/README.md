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

## Building the source tarball

```bash
make build      # -> libfec_<version>.tar.gz here (gitignored); installs offline from anywhere
```

`R CMD build .` alone produces a tarball that can't install: it copies only this directory, so
`path = '../../../fec-parser'` is missing (decision R9). `make build` stages a copy of the package
in `$TMPDIR`, then:

1. copies `../fec-parser` and `../fec-parser-macros` into the stage's `src/rust/local/` (without
   `target/`, `tests/fixtures/` and `tests/snapshots/`; the macros read `date_columns.txt`,
   `float-columns.txt` and `src/mappings2.json` at compile time, so the crate dirs go in whole);
2. rewrites the staged `src/rust/Cargo.toml` to `path = 'local/fec-parser'` (fec-parser's own
   `../fec-parser-macros` path then resolves to the sibling copy);
3. runs `make vendor` on the stage: `cargo vendor --locked` into `src/rust/vendor.tar.xz`, with the
   source replacement (crates.io + the extendr git rev) saved to `src/rust/vendor-config.toml`;
4. runs `R CMD build` there and moves the tarball back. The stage is deleted however the recipe
   exits, so the working tree is never modified.

With `NOT_CRAN` unset and `src/rust/vendor.tar.xz` present, `tools/config.R` sets
`@CRAN_FLAGS@` to `-j 2 --offline`, and `src/Makevars.in` untars the vendor directory and installs
`vendor-config.toml` as the cargo config of its private `CARGO_HOME`. `make vendor` also works in
place (`make clean-vendor` removes its output), but `R CMD INSTALL .` in place still resolves
fec-parser through the original path, so only a tarball installed from outside the repo proves the
build.

Tarball size (2026-10-05, Phase 0 scaffold): **4.3 MB** (`libfec_0.0.34.9000.tar.gz`; CRAN's soft
limit is 10 MB). Nearly all of it is `src/rust/vendor.tar.xz` (4.1 MB: 49 crates, 38 MB unpacked;
the largest are `csv` 6.0 MB, `jiff` 3.6 MB, `regex-automata` 2.9 MB and three `syn`s at ~7 MB).
The fec-parser + macros copies add 1.3 MB unpacked (`mappings2.json` is 369 KB). A cold offline
install takes ~1 m 45 s.

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
- A cargo `[patch]` (or a `paths = [...]` override) can't redirect a path dependency: cargo loads
  the original path's manifest first and fails on the missing file, so `make build` rewrites the
  path in a staged `Cargo.toml` instead.
- `src/rust/Cargo.toml` has `[workspace] exclude = ['local']`. Without it, the staged
  `local/fec-parser` sits under the workspace root and becomes a member, so cargo locks its
  dev-dependencies and optional deps (insta, pyo3: 22 extra crates) and `--locked` fails.

## Dev notes: the Rust reader (`fec_read_impl`)

`fec_read_impl(path, raw, n_max)` (internal, in `src/rust/src/lib.rs`) reads a whole filing and
returns plain lists; `fec_read()` wraps them. The shape is the contract between Rust and R:

```
list(
  header      = <named list of scalars>,  # filing_id, record_type, ef_type, fec_version,
                                          # software_name, software_version, report_id,
                                          # report_number, comment, style, delimiter,
                                          # is_paper (logical), name_delimiter,
                                          # batch_number, received_date (NA when absent)
  cover       = <named list of length-1 vectors>,
  cover_info  = list(form_type, filer_id, filer_name, report_code,
                     coverage_from_date, coverage_through_date,  # Date, NA when absent
                     cover_kind),                                # "typed" | "raw"
  tables      = <named list of named column lists>,  # equal lengths, first column filing_id
  table_kinds = <named chr: "typed" | "raw" | "other">
)
```

- **Tables** are named by record family (`fec_parser::itemizations::record_family`), never by
  row type: `SA` → `schedule_a`, `SA3L` → `schedule_a3l`, `SC1` → `schedule_c1`, `SL` →
  `schedule_l`, `SI` → `schedule_i`, `H4` → `h4`, `F57` → `f57`, `TEXT` → `text`
  (`src/rust/src/names.rs`, unit-tested against every family). Order: first seen in the file,
  then `other`.
- **Typed** (default): the family's typed struct flattened by `fec_parser::columnar`; the same
  names for every version. Fields past the layout are not kept on typed rows.
- **Raw** (`raw = TRUE` for every family; in typed mode, the fallback for a family with no
  struct, such as `SI`, or a row type with no layout): the version's mapping columns,
  `DATE_COLUMNS` → `Date`, `FLOAT_COLUMNS` → double, the rest text (trimmed, blank → `NA`).
  Layouts within one family are unioned by name in first-seen order, `NA`-filled; fields past
  a row's layout are `extra_1`, `extra_2`, …. A row type with no layout at all gets only
  `extra_*` columns. If a family has both typed and fallback rows in one filing, the typed
  rows are `<name>` and the fallback rows `<name>_raw`.
- **`other`**: rows no family matches (`F3PS`, `F1S`, unknown row types): `filing_id`,
  `row_type`, `field_1` (the field after the row type), `field_2`, …, all text, `NA`-padded.
- **Cover**: the typed cover flattened (`Date`, double, logical, integer, character); the raw
  cover record as text (plus `extra_*`) if the form has no struct, and always with
  `raw = TRUE`. A file without a cover record is a `header:` error, so `cover` is never `NULL`.
- **`n_max`**: itemization rows (every row after the cover, `other` included), checked before
  pulling the next row; `0` returns right after the cover; `Inf` reads everything.
- **Errors**: R errors whose message starts with `io: ` (open/read failures, a directory),
  `header: ` (not a `.fec` file, unsupported version, no cover) or `parse: line N: ` (a CSV
  read error mid-file; `parse: after line N: ` when the CSV layer gives no line). A bad
  `n_max` (NaN, negative) has no prefix; validate it in R.
- **Strings** are built with `Rf_mkCharLenCE` + `SET_STRING_ELT` into a `Strings` vector
  (never `Rstr::from` in a loop), marked UTF-8, with NUL bytes stripped first (an embedded NUL
  would make R `longjmp` over the Rust frames).
- **Panics**: extendr 74afddc wraps every `#[extendr]` function in `catch_unwind` and turns a
  panic into an R error with the panic message (an `Err` is turned into a panic and goes the
  same way); the panic hook registered in `entrypoint.c` keeps it quiet unless
  `EXTENDR_BACKTRACE=1`. The code doesn't rely on that: builders are always made from the
  columns of the value pushed into them.
- `cargo test --manifest-path src/rust/Cargo.toml` works here: extendr-ffi links `libR`, so
  the test binary links wherever R is installed. The name mapping and the raw-date parser are
  pure Rust and tested there.
