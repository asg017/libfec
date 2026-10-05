# R3/R4: one typed struct reads every layout, so a family's columns don't depend on the
# filing's format version, legacy included.

col_classes <- function(df) vapply(df, function(col) class(col)[[1L]], character(1))

test_that("schedule_a has identical names for v8.5, 6.4, 5.3 and 3.00", {
  tables <- lapply(sa_fixtures(), function(p) fec_read(p)$schedule_a)
  versions <- vapply(sa_fixtures(), function(p) fec_read(p, n_max = 0)$header$fec_version, "")
  expect_equal(unname(versions), c("8.5", "6.4", "5.3", "3.00"))

  ref <- names(tables[["8.5"]])
  expect_length(ref, 48L) # filing_id + the 47 Schedule A columns
  for (v in names(tables)) {
    expect_identical(names(tables[[v]]), ref, label = paste0("names(schedule_a) for v", v))
    expect_gt(nrow(tables[[v]]), 0L)
  }
  # And the same as the v8.4 fixture shipped in inst/extdata.
  expect_identical(names(fec_read(pfizer_path(), n_max = 1)$schedule_a), ref)
})

test_that("schedule_a has identical column types across versions", {
  classes <- lapply(sa_fixtures(), function(p) col_classes(fec_read(p)$schedule_a))
  for (v in names(classes)) {
    expect_identical(classes[[v]], classes[["8.5"]], label = paste0("column types for v", v))
  }
  expect_snapshot(table(classes[["8.5"]]), cran = TRUE)
})

test_that("schedule_a from different versions stacks with rbind()", {
  sa <- do.call(rbind, lapply(sa_fixtures(), function(p) fec_read(p)$schedule_a))
  expect_s3_class(sa, "tbl_df")
  expect_equal(nrow(sa), 16L)
  expect_setequal(
    unique(sa$filing_id),
    c("SA_1920342", "SA_462580", "SA_265857", "SA_42174")
  )
  expect_s3_class(sa$contribution_date, "Date")
  expect_false(anyNA(sa$contribution_amount))
})

test_that("raw = TRUE names may differ across versions", {
  raw <- lapply(sa_fixtures(), function(p) names(fec_read(p, raw = TRUE)$schedule_a))
  expect_false(all(vapply(raw, identical, logical(1), raw[["8.5"]])))
})
