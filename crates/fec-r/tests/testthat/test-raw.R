# raw = TRUE parses the same dates and amounts as the typed read. These columns are where the
# two modes disagreed.

test_that("raw amounts named like dates are amounts (SC loan_payment_to_date, H4 event_year_to_date)", {
  p <- fixture("6.1_342096.fec")
  t <- fec_read(p)
  r <- fec_read(p, raw = TRUE)
  expect_type(r$schedule_c$loan_payment_to_date, "double")
  expect_equal(r$schedule_c$loan_payment_to_date, t$schedule_c$loan_payment_to_date)
  expect_equal(r$schedule_c$loan_payment_to_date, 98398.47)
  expect_equal(r$h4$event_year_to_date, t$h4$event_year_to_date)

  p <- fixture("H4_181668.fec")
  t <- fec_read(p)
  r <- fec_read(p, raw = TRUE)
  expect_type(r$h4$event_year_to_date, "double")
  expect_equal(r$h4$event_year_to_date, t$h4$event_year_to_date)
  expect_equal(r$h4$event_year_to_date, rep(30315.22, 3))
})

test_that("raw SC1 loan_due_date is text (it holds terms such as ON DEMAND)", {
  p <- fixture("SC1_1906567.fec")
  t <- fec_read(p)
  r <- fec_read(p, raw = TRUE)
  expect_type(r$schedule_c1$loan_due_date, "character")
  expect_equal(r$schedule_c1$loan_due_date, t$schedule_c1$loan_due_date_terms)

  path <- withr::local_tempfile(fileext = ".fec")
  writeLines(sub("20251231", "ON DEMAND", readLines(p), fixed = TRUE), path)
  expect_equal(fec_read(path, raw = TRUE)$schedule_c1$loan_due_date, "ON DEMAND")
  expect_equal(fec_read(path)$schedule_c1$loan_due_date_terms, "ON DEMAND")
})
