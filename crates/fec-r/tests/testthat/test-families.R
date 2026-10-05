test_that("SA3L rows get their own schedule_a3l table, not schedule_a", {
  f <- fec_read(fixture("SA3L_1921461.fec"))
  expect_equal(names(f$tables), "schedule_a3l")
  expect_equal(attr(f, "table_kinds"), c(schedule_a3l = "typed"))
  expect_null(f$schedule_a)
  expect_equal(nrow(f$schedule_a3l), 1L)
  expect_true(all(startsWith(f$schedule_a3l$form_type, "SA3L")))
  # Its own struct, so its own columns.
  expect_false(identical(names(f$schedule_a3l), names(fec_read(pfizer_path(), n_max = 1)$schedule_a)))
})

test_that("a filing with many families gets one table per family", {
  f <- fec_read(fixture("3.00_13801.fec"))
  expect_equal(
    attr(f, "table_kinds"),
    c(
      h4 = "typed", schedule_b = "typed", h1 = "typed", h2 = "typed", h3 = "typed",
      schedule_c = "typed", schedule_d = "typed", schedule_a = "typed", schedule_i = "raw"
    )
  )
  expect_equal(names(f$tables), names(attr(f, "table_kinds")))
})

test_that("table names are clean and no table has duplicate column names", {
  paths <- all_fixtures()
  for (raw in c(FALSE, TRUE)) {
    for (p in c(paths, local_synthetic_filing())) {
      f <- fec_read(p, raw = raw)
      label <- paste0(basename(p), if (raw) " (raw = TRUE)")
      tbl_names <- names(f$tables)
      expect_identical(names(attr(f, "table_kinds")), tbl_names, label = label)
      expect_false(anyDuplicated(tbl_names) > 0L, label = label)
      expect_true(all(grepl("^[a-z][a-z0-9_]*$", tbl_names)), label = paste(label, "table names"))
      expect_false(any(grepl("/", tbl_names, fixed = TRUE)), label = label)
      for (nm in tbl_names) {
        tbl <- f$tables[[nm]]
        tlabel <- paste0(label, " $", nm)
        expect_s3_class(tbl, "tbl_df")
        expect_false(anyDuplicated(names(tbl)) > 0L, label = tlabel)
        expect_identical(names(tbl)[[1L]], "filing_id", label = tlabel)
        expect_true(all(tbl$filing_id == f$header$filing_id), label = tlabel)
      }
    }
  }
})
