test_that("a file that isn't a .fec filing is a libfec_error_header", {
  path <- withr::local_tempfile(fileext = ".txt")
  writeLines(c("127.0.0.1 localhost", "::1 localhost"), path)
  cnd <- expect_error(fec_read(path), class = "libfec_error_header")
  expect_s3_class(cnd, "libfec_error")
  expect_s3_class(cnd, "rlang_error")
  expect_equal(cnd$path, path)
  expect_error(fec_cover(path), class = "libfec_error_header")
})

test_that("an empty file is a libfec_error_header", {
  path <- withr::local_tempfile(fileext = ".fec")
  file.create(path)
  expect_error(fec_read(path), class = "libfec_error_header")
})

test_that("a missing file is a libfec_error_io", {
  path <- file.path(withr::local_tempdir(), "missing.fec")
  cnd <- expect_error(fec_read(path), class = "libfec_error_io")
  expect_s3_class(cnd, "libfec_error")
  expect_equal(cnd$path, path)
  expect_error(fec_cover(path), class = "libfec_error_io")
  expect_snapshot(fec_read("does-not-exist.fec"), error = TRUE, cran = TRUE)
})

test_that("a directory is a libfec_error_io", {
  dir <- withr::local_tempdir()
  cnd <- expect_error(fec_read(dir), class = "libfec_error_io")
  expect_s3_class(cnd, "libfec_error")
})

test_that("the error message names the file and the reason", {
  withr::local_dir(withr::local_tempdir())
  writeLines("not a filing", "notes.txt")
  expect_snapshot(fec_read("notes.txt"), error = TRUE, cran = TRUE)
})

test_that("print() summarises the filing", {
  local_reproducible_output(unicode = TRUE)
  expect_snapshot(cran = TRUE, {
    fec_read(pfizer_path())
    fec_read(pfizer_path(), raw = TRUE)
    fec_read(pfizer_path(), n_max = 0)
    fec_read(fixture("SA3L_1921461.fec"))
  })
})

test_that("print() falls back to ASCII separators without UTF-8 output", {
  local_reproducible_output(unicode = FALSE)
  expect_snapshot(fec_read(pfizer_path()), cran = TRUE)
})

test_that("print() returns its input invisibly", {
  f <- fec_read(pfizer_path(), n_max = 0)
  expect_output(out <- withVisible(print(f)), "no itemizations")
  expect_false(out$visible)
  expect_identical(out$value, f)
  expect_type(format(f), "character")
})

test_that("a NUL byte in file text that reaches an error message is a classed error", {
  # fec-parser quotes file text in its messages (`Incorrect header record type: HD\0R`). An
  # R error message can't hold a NUL; before the fix this aborted the R process.
  path <- withr::local_tempfile(fileext = ".fec")
  writeBin(c(charToRaw("HD"), as.raw(0L), charToRaw("R\x1c8.4\n")), path)
  cnd <- expect_error(fec_read(path), class = "libfec_error_header")
  expect_match(conditionMessage(cnd), "HD\\0R", fixed = TRUE)
  expect_error(fec_cover(path), class = "libfec_error_header")

  # A NUL in the version and in the cover's form type (both quoted in fec-parser errors).
  lines <- readLines(pfizer_path(), n = 2L)
  writeBin(c(charToRaw(sub("8.4", "8.", lines[[1L]], fixed = TRUE)), as.raw(0L),
             charToRaw(paste0("4\n", lines[[2L]], "\n"))), path)
  expect_error(fec_read(path), class = "libfec_error_header")
  writeBin(c(charToRaw(paste0(lines[[1L]], "\nF3")), as.raw(0L),
             charToRaw(paste0("ZZ", substring(lines[[2L]], 5L), "\n"))), path)
  expect_error(fec_read(path), class = "libfec_error")
})
