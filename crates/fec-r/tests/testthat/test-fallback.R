# R6b: in typed mode nothing is dropped and nothing errors. Families with no struct, and row
# types with no layout, become raw-column fallback tables; unrecognised rows go to `other`.

test_that("the synthetic filing reads without error into typed, fallback and other tables", {
  path <- local_synthetic_filing()
  expect_no_error(f <- fec_read(path))
  expect_equal(f$header$filing_id, "9999001")
  expect_equal(
    attr(f, "table_kinds"),
    c(schedule_a = "typed", schedule_a_raw = "raw", schedule_i = "raw", other = "other")
  )
  expect_equal(
    vapply(f$tables, nrow, integer(1)),
    c(schedule_a = 2L, schedule_a_raw = 1L, schedule_i = 1L, other = 1L)
  )
  local_reproducible_output(unicode = TRUE)
  expect_snapshot(print(f), cran = TRUE)
})

test_that("a family with no struct (SI) becomes a raw fallback table", {
  f <- fec_read(local_synthetic_filing())
  si <- f$schedule_i
  expect_equal(attr(f, "table_kinds")[["schedule_i"]], "raw")
  # v8.4 has no SI layout, so every field (row type included) is an extra_* column.
  expect_equal(names(si), c("filing_id", paste0("extra_", 1:5)))
  expect_equal(unlist(si[1, -1], use.names = FALSE), c("SI", "C00016683", "SYN-SI", "x", "y"))
})

test_that("a row type with no layout in a typed family goes to <family>_raw", {
  f <- fec_read(local_synthetic_filing())
  expect_equal(f$schedule_a$transaction_id, c("2023071716378-1066", "SYN-NUL"))
  sa_raw <- f$schedule_a_raw
  expect_equal(names(sa_raw), c("filing_id", paste0("extra_", 1:44)))
  expect_equal(sa_raw$extra_1, "SA3X")
  expect_equal(sa_raw$extra_3, "SYN-3X")
  expect_equal(sa_raw$extra_8, "Aaronson")
  expect_true(is.na(sa_raw$extra_4)) # blank -> NA
})

test_that("unrecognised row types go to `other`", {
  f <- fec_read(local_synthetic_filing())
  expect_equal(attr(f, "table_kinds")[["other"]], "other")
  expect_equal(names(f$tables)[[length(f$tables)]], "other") # always last
  expect_equal(
    as.data.frame(f$other),
    data.frame(filing_id = "9999001", row_type = "ZZZ", field_1 = "hello", field_2 = "world")
  )
})

test_that("embedded NUL bytes are stripped from strings", {
  f <- fec_read(local_synthetic_filing())
  expect_equal(f$schedule_a$contributor_name_last_name, c("Aaronson", "Aaronson"))
  r <- fec_read(local_synthetic_filing(), raw = TRUE)
  expect_equal(r$schedule_a$contributor_last_name[[2L]], "Aaronson")
  expect_true(all(validUTF8(unlist(lapply(r$tables, function(t) Filter(is.character, t))))))
})

test_that("fields past the layout are kept on raw rows as extra_* columns", {
  # Typed rows keep only the struct's fields...
  f <- fec_read(local_synthetic_filing())
  expect_false(any(startsWith(names(f$schedule_a), "extra_")))
  expect_equal(ncol(f$schedule_a), 48L)

  # ...raw rows keep everything. v8.4 SA has 45 fields; row 2 has 3 more. Raw mode unions
  # every SA layout in the family (SA3X has none, so its 44 fields are extra_1..extra_44).
  r <- fec_read(local_synthetic_filing(), raw = TRUE)
  expect_equal(attr(r, "table_kinds"), c(schedule_a = "raw", schedule_i = "raw", other = "other"))
  sa <- r$schedule_a
  expect_equal(nrow(sa), 3L)
  expect_equal(sa$reference_code, c(NA_character_, NA_character_, NA_character_))
  expect_equal(sa$extra_1, c(NA, "extra one", "SA3X"))
  expect_equal(sa$extra_2, c(NA, "extra two", "C00016683"))
  expect_equal(sa$extra_3, c(NA, "extra three", "SYN-3X"))
  expect_equal(sa$extra_4, c(NA_character_, NA_character_, NA_character_))
})

test_that("the legacy fixture's SI rows are a fallback table, flagged in print()", {
  f <- fec_read(fixture("3.00_13801.fec"))
  expect_equal(attr(f, "table_kinds")[["schedule_i"]], "raw")
  si <- f$schedule_i
  expect_equal(nrow(si), 2L)
  expect_true(all(c("bank_account_id", "coverage_from_date", "col_a_total_receipts") %in% names(si)))
  expect_s3_class(si$coverage_from_date, "Date")
  expect_type(si$col_a_total_receipts, "double")
  local_reproducible_output(unicode = TRUE)
  expect_snapshot(print(f), cran = TRUE)
})

test_that("a paper filing's unrecognised summary rows go to `other`", {
  f <- fec_read(fixture("P2.6_716051.fec"))
  expect_true(f$header$is_paper)
  expect_equal(attr(f, "table_kinds")[["other"]], "other")
  expect_equal(f$other$row_type, "F3S")
  expect_equal(names(f$other)[1:3], c("filing_id", "row_type", "field_1"))
  expect_true(all(vapply(f$other, is.character, logical(1))))
  local_reproducible_output(unicode = TRUE)
  expect_snapshot(print(f), cran = TRUE)
})

test_that("whitespace-only lines and a DOS end-of-file byte are not rows", {
  path <- withr::local_tempfile(fileext = ".fec")
  lines <- readLines(pfizer_path(), n = 4L)
  writeBin(c(
    charToRaw(paste0(paste(lines[1:2], collapse = "\n"), "\n   \n", lines[[3L]], "\n \t \n")),
    charToRaw(paste0(lines[[4L]], "\n\x1a"))
  ), path)
  f <- fec_read(path)
  expect_equal(names(f$tables), "schedule_a")
  expect_equal(nrow(f$schedule_a), 2L)
  expect_equal(names(fec_read(path, raw = TRUE)$tables), "schedule_a")
  # Skipped lines don't count toward n_max.
  expect_equal(nrow(fec_read(path, n_max = 1)$schedule_a), 1L)
  expect_equal(nrow(fec_read(path, n_max = 2)$schedule_a), 2L)
})

test_that("print() flags a cover form with no typed structure as a raw cover", {
  # F3Z (v8.4) has no typed cover struct, so the cover is its raw fields even in typed mode.
  dir <- withr::local_tempdir()
  path <- file.path(dir, "9999002.fec")
  writeBin(charToRaw(paste0(
    "HDR\x1cFEC\x1c8.4\x1cFECFile\x1c8.4\x1c\x1c\x1c0\x1c\n",
    "F3Z1\x1cC00016683\x1cPFIZER INC. PAC\x1c20230701\x1c20230731\n"
  )), path)
  f <- fec_read(path)
  expect_equal(attr(f, "cover_info")$cover_kind, "raw")
  local_reproducible_output(unicode = TRUE)
  expect_match(format(f)[[1L]], "v8.4 · raw cover$")
  # With raw = TRUE everything is raw, and the header line says so once.
  expect_no_match(format(fec_read(path, raw = TRUE))[[1L]], "raw cover")
  expect_no_match(format(fec_read(pfizer_path(), n_max = 0))[[1L]], "raw cover")
  expect_snapshot(print(f), cran = TRUE)
})

test_that("print() shows a paper filing's version without a `v`", {
  local_reproducible_output(unicode = TRUE)
  expect_match(format(fec_read(fixture("P2.6_716051.fec")))[[1L]], " · P2.6 · paper$")
  expect_match(format(fec_read(pfizer_path(), n_max = 0))[[1L]], " · v8.4$")
})
