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

test_that("raw = TRUE keeps an F99's message text on the cover", {
  p <- fixture("F99_1909934.fec")
  t <- fec_read(p)
  r <- fec_read(p, raw = TRUE)
  expect_type(r$cover$text, "character")
  expect_equal(nchar(r$cover$text), nchar(t$cover$text))
  expect_equal(r$cover$text, t$cover$text)
  expect_equal(fec_cover(r)$text, t$cover$text)
})

test_that("typed and raw dates agree on a legacy filing", {
  p <- fixture("6.1_342096.fec")
  t <- fec_read(p)
  r <- fec_read(p, raw = TRUE)
  n <- 0L
  for (name in names(t$tables)) {
    tt <- t$tables[[name]]
    rr <- r$tables[[name]]
    for (col in intersect(names(tt), names(rr))) {
      if (inherits(tt[[col]], "Date")) {
        expect_equal(rr[[col]], tt[[col]], label = paste0("raw ", name, "$", col))
        n <- n + 1L
      }
    }
  }
  expect_gt(n, 0L)
})

test_that("raw dates accept an unpadded month and day, like typed dates", {
  # SA_1920342.fec (v8.5) with each SA row's contribution_date (field 20, 20250714) rewritten.
  lines <- readLines(fixture("SA_1920342.fec"))
  sa <- which(startsWith(lines, "SA"))
  dates <- c("2/9/2024", "7/14/2023", "01/2/2003", "12/31/1969")
  expect_length(sa, length(dates))
  for (i in seq_along(sa)) {
    fields <- strsplit(lines[[sa[[i]]]], "\x1c", fixed = TRUE)[[1L]]
    fields[[20L]] <- dates[[i]]
    lines[[sa[[i]]]] <- paste(fields, collapse = "\x1c")
  }
  path <- withr::local_tempfile(fileext = ".fec")
  writeLines(lines, path)
  expected <- as.Date(c("2024-02-09", "2023-07-14", "2003-01-02", "1969-12-31"))
  expect_equal(fec_read(path)$schedule_a$contribution_date, expected)
  expect_equal(fec_read(path, raw = TRUE)$schedule_a$contribution_date, expected)
})
