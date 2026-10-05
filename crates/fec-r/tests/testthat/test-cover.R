test_that("f$cover is the typed cover as a named list", {
  f <- fec_read(pfizer_path())
  cover <- f$cover
  expect_type(cover, "list")
  expect_false(is.data.frame(cover))
  expect_true(all(lengths(cover) == 1L))
  expect_equal(cover$form_type, "F3XN")
  expect_equal(cover$filer_committee_id, "C00016683")
  expect_equal(cover$committee_name, "PFIZER INC. PAC")
  expect_s3_class(cover$coverage_from_date, "Date")
  expect_equal(cover$coverage_from_date, as.Date("2023-07-01"))
  expect_equal(cover$coverage_through_date, as.Date("2023-07-31"))
  expect_type(cover$change_of_address, "logical")
  expect_type(cover$summary_line6a_year, "integer")
  expect_true(any(vapply(cover, is.double, logical(1))))
})

test_that("with raw = TRUE the cover is the raw cover record", {
  cover <- fec_read(pfizer_path(), raw = TRUE, n_max = 0)$cover
  expect_type(cover, "list")
  # The version's mapping names, all text (dates included).
  expect_equal(
    names(cover)[1:5],
    c("form_type", "filer_committee_id_number", "committee_name", "change_of_address", "street_1")
  )
  expect_true(all(vapply(cover, is.character, logical(1))))
  expect_equal(cover$form_type, "F3XN")
  expect_equal(cover$committee_name, "PFIZER INC. PAC")
  # print() still reads the coverage dates (from the form-independent cover info).
  expect_s3_class(attr(fec_read(pfizer_path(), raw = TRUE, n_max = 0), "cover_info")$coverage_from_date, "Date")
})

test_that("fec_cover() returns one row with filing_id and form_type first", {
  for (x in list(pfizer_path(), fec_read(pfizer_path(), n_max = 0))) {
    cv <- fec_cover(x)
    expect_s3_class(cv, "tbl_df")
    expect_equal(nrow(cv), 1L)
    expect_equal(names(cv)[1:2], c("filing_id", "form_type"))
    expect_equal(cv$filing_id, "1721696")
    expect_equal(cv$form_type, "F3XN")
    expect_false(anyDuplicated(names(cv)) > 0L)
    expect_s3_class(cv$coverage_through_date, "Date")
  }
  expect_identical(fec_cover(pfizer_path()), fec_cover(fec_read(pfizer_path())))
  f <- fec_read(pfizer_path())
  expect_equal(as.list(fec_cover(f))[-1], f$cover)
})

test_that("covers of the same form stack with base rbind(), across versions", {
  paths <- c(pfizer_path(), fixture("3.00_13801.fec")) # F3XN v8.4 and v3.00
  covers <- do.call(rbind, lapply(paths, fec_cover))
  expect_s3_class(covers, "tbl_df")
  expect_equal(nrow(covers), 2L)
  expect_equal(covers$filing_id, c("1721696", "3.00_13801"))
  expect_equal(covers$form_type, c("F3XN", "F3XN"))
  expect_s3_class(covers$coverage_from_date, "Date")
  expect_equal(covers$coverage_from_date, as.Date(c("2023-07-01", "2001-03-01")))
})

test_that("covers of different forms stack with vctrs::vec_rbind()", {
  skip_if_not_installed("vctrs")
  paths <- c(pfizer_path(), fixture("SA_1920342.fec"), fixture("F99_1909934.fec"))
  covers <- vctrs::vec_rbind(!!!lapply(paths, fec_cover))
  expect_equal(nrow(covers), 3L)
  expect_equal(covers$form_type, c("F3XN", "F3N", "F99"))
  expect_equal(covers$filing_id, c("1721696", "SA_1920342", "F99_1909934"))
  expect_false(anyDuplicated(names(covers)) > 0L)
  expect_s3_class(covers$coverage_from_date, "Date")
  expect_true(is.na(covers$coverage_from_date[[3L]])) # F99 has no coverage dates
  expect_type(covers$text, "character")
})

test_that("a cover with no coverage dates prints without them", {
  f <- fec_read(fixture("F99_1909934.fec"))
  expect_length(f$tables, 0L)
  expect_null(f$cover$coverage_from_date)
  expect_s3_class(f$cover$date_signed, "Date")
  local_reproducible_output(unicode = TRUE)
  expect_snapshot(print(f), cran = TRUE)
})
