test_that("typed schedule_a has the flattened struct columns and R types", {
  f <- fec_read(pfizer_path())
  sa <- f$schedule_a

  expect_s3_class(f, "fec_filing")
  expect_s3_class(sa, "tbl_df")
  expect_equal(nrow(sa), 1354L)
  expect_equal(ncol(sa), 48L)
  expect_equal(nrow(f$schedule_b), 33L)
  expect_equal(names(f$tables), c("schedule_a", "schedule_b"))
  expect_equal(attr(f, "table_kinds"), c(schedule_a = "typed", schedule_b = "typed"))

  expect_equal(names(sa)[1:2], c("filing_id", "form_type"))
  expect_true(all(sa$filing_id == "1721696"))
  expect_s3_class(sa$contribution_date, "Date")
  expect_type(sa$contribution_amount, "double")
  expect_type(sa$contribution_aggregate, "double")
  expect_type(sa$memo, "logical")
  expect_false(anyNA(sa$memo))
  expect_type(sa$contributor_name_last_name, "character")
  expect_true(all(startsWith(sa$form_type, "SA")))

  cols <- c(
    "form_type", "contributor_name_last_name", "contributor_address_state",
    "contribution_date", "contribution_amount", "contribution_aggregate", "memo"
  )
  expect_snapshot(print(as.data.frame(head(sa[cols]))), cran = TRUE)
})

test_that("`$` and `[[` reach the parts and the tables; unknown names are NULL", {
  f <- fec_read(pfizer_path())
  expect_identical(f$schedule_a, f$tables$schedule_a)
  expect_identical(f[["schedule_b"]], f$tables$schedule_b)
  expect_type(f$header, "list")
  expect_type(f$cover, "list")
  expect_null(f$nonexistent)
  expect_null(f$schedule_e)
  expect_equal(f$header$filing_id, "1721696")
  expect_equal(f$header$fec_version, "8.4")
  expect_false(f$header$is_paper)
})

test_that("`raw = TRUE` gives the version's raw mapping columns", {
  f <- fec_read(pfizer_path(), raw = TRUE)
  sa <- f$schedule_a

  expect_true(attr(f, "raw"))
  expect_equal(attr(f, "table_kinds"), c(schedule_a = "raw", schedule_b = "raw"))
  expect_equal(dim(sa), c(1354L, 46L))
  expect_equal(
    names(sa)[1:10],
    c(
      "filing_id", "form_type", "filer_committee_id_number", "transaction_id",
      "back_reference_tran_id_number", "back_reference_sched_name", "entity_type",
      "contributor_organization_name", "contributor_last_name", "contributor_first_name"
    )
  )
  expect_true(all(c("memo_code", "memo_text_description", "conduit_street1") %in% names(sa)))
  expect_false("contributor_name_last_name" %in% names(sa))
  # DATE_COLUMNS -> Date, FLOAT_COLUMNS -> double, the rest text (memo_code too).
  expect_s3_class(sa$contribution_date, "Date")
  expect_type(sa$contribution_amount, "double")
  expect_type(sa$memo_code, "character")

  typed <- fec_read(pfizer_path())$schedule_a
  expect_equal(sa$contribution_amount, typed$contribution_amount)
  expect_equal(sa$contribution_date, typed$contribution_date)
  expect_equal(sa$contributor_last_name, typed$contributor_name_last_name)
})

test_that("`n_max` limits the itemization rows", {
  f <- fec_read(pfizer_path(), n_max = 10)
  expect_equal(names(f$tables), "schedule_a")
  expect_equal(nrow(f$schedule_a), 10L)
  expect_equal(f$schedule_a, head(fec_read(pfizer_path())$schedule_a, 10))

  # Counted across tables: 1,354 SA rows, then the SB rows.
  f <- fec_read(pfizer_path(), n_max = 1360)
  expect_equal(vapply(f$tables, nrow, integer(1)), c(schedule_a = 1354L, schedule_b = 6L))

  expect_equal(
    vapply(fec_read(pfizer_path(), n_max = Inf)$tables, nrow, integer(1)),
    c(schedule_a = 1354L, schedule_b = 33L)
  )

  # `n_max = 0` reads the header and cover only.
  f <- fec_read(pfizer_path(), n_max = 0)
  expect_length(f$tables, 0L)
  expect_equal(f$cover$committee_name, "PFIZER INC. PAC")
})

test_that("bad arguments are plain errors, not libfec errors", {
  path <- pfizer_path()
  for (n in list(-1, NA, 1.5, "10", c(1, 2), NaN)) {
    expect_error(fec_read(path, n_max = n), class = "rlang_error")
    cnd <- tryCatch(fec_read(path, n_max = n), error = identity)
    expect_false(inherits(cnd, "libfec_error"))
  }
  for (r in list(NA, "yes", 1, c(TRUE, FALSE))) {
    cnd <- tryCatch(fec_read(path, raw = r), error = identity)
    expect_s3_class(cnd, "rlang_error")
    expect_false(inherits(cnd, "libfec_error"))
  }
  expect_snapshot(cran = TRUE, error = TRUE, {
    fec_read(path, n_max = -1)
    fec_read(path, raw = NA)
    fec_read(c(path, path))
    fec_read(1721696)
    fec_read("https://docquery.fec.gov/dcdev/posted/1721696.fec")
    fec_read(as.raw(1:3))
  })
})
