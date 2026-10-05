# Fixtures: `inst/extdata/1721696.fec` (shipped for examples) and the small files in
# `tests/testthat/fixtures/`, copied unchanged from `crates/fec-parser/tests/fixtures/`.
#
# Snapshots use `cran = TRUE`: CI checks the built tarball with NOT_CRAN unset (for the
# offline build), and without it every snapshot would be skipped there. They cover our own
# print output and error messages; print snapshots force `unicode = TRUE` (the `·`/`×`
# separators) and testthat turns colours off.

# F3XN v8.4, PFIZER INC. PAC: 1,354 SA rows, 33 SB rows.
pfizer_path <- function() {
  system.file("extdata", "1721696.fec", package = "libfec", mustWork = TRUE)
}

fixture <- function(name) {
  test_path("fixtures", name)
}

# The Schedule A fixtures, one per format generation (v8.5, 6.4, 5.3, 3.00).
sa_fixtures <- function() {
  c(
    "8.5" = fixture("SA_1920342.fec"),
    "6.4" = fixture("SA_462580.fec"),
    "5.3" = fixture("SA_265857.fec"),
    "3.00" = fixture("SA_42174.fec")
  )
}

all_fixtures <- function() {
  c(pfizer_path(), list.files(test_path("fixtures"), pattern = "\\.fec$", full.names = TRUE))
}

# A v8.4 filing (the HDR and F3XN cover of 1721696.fec, FS-delimited, `\n` line ends) with
# rows that exercise every table kind:
#
# 1. `SA11AI`: a typed `schedule_a` row (the first SA row of 1721696.fec);
# 2. `SI`: a family with no typed struct, so a fallback `schedule_i` table (v8.4 has no SI
#    layout either, so every field is an `extra_*` column);
# 3. `ZZZ`: a row type no family matches, so `other`;
# 4. `SA11AI` padded to the full v8.4 SA layout (45 fields) plus 3 extra fields, with a NUL
#    byte inside the contributor's last name (`Aar\0onson`);
# 5. `SA3X`: in the SA family but with no layout, so a `schedule_a_raw` table.
#
# R strings can't hold NUL, so the file is written as bytes. The file is named `9999001.fec`
# (the filing ID comes from the file name) inside a temporary directory that is removed when
# `env` exits.
local_synthetic_filing <- function(env = parent.frame()) {
  dir <- withr::local_tempdir(.local_envir = env)
  path <- file.path(dir, "9999001.fec")

  fs <- "\x1c"
  head_lines <- readLines(pfizer_path(), n = 3L)
  sa <- strsplit(head_lines[[3L]], fs, fixed = TRUE)[[1L]]
  sa45 <- c(sa, rep("", 45L - length(sa)))
  row <- function(fields) charToRaw(paste0(paste(fields, collapse = fs), "\n"))

  nul_row <- sa45
  nul_row[[3L]] <- "SYN-NUL"
  nul_row <- c(nul_row, "extra one", "extra two", "extra three")
  before <- paste(nul_row[1:7], collapse = fs)
  after <- paste(nul_row[9:length(nul_row)], collapse = fs)

  bytes <- c(
    charToRaw(paste0(head_lines[[1L]], "\n", head_lines[[2L]], "\n")),
    row(sa),
    row(c("SI", "C00016683", "SYN-SI", "x", "y")),
    row(c("ZZZ", "hello", "world")),
    charToRaw(paste0(before, fs, "Aar")), as.raw(0L), charToRaw(paste0("onson", fs, after, "\n")),
    row(replace(sa, c(1L, 3L), c("SA3X", "SYN-3X")))
  )
  writeBin(bytes, path)
  path
}
