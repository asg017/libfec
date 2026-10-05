#' Print a filing
#'
#' Prints a one-line summary of the filing (filing ID, form type, filer,
#' coverage period, FEC format version) and the size of each table, rather
#' than the tables themselves. Tables made from raw columns in a typed read
#' are marked `(raw columns)`; see "Rows with no typed structure" in
#' [fec_read()].
#'
#' @param x A `<fec_filing>` from [fec_read()].
#' @param ... Ignored.
#' @return `format()` returns a character vector, one element per line.
#'   `print()` returns `x`, invisibly.
#' @name print.fec_filing
#' @examples
#' f <- fec_read(system.file("extdata", "1721696.fec", package = "libfec"))
#' print(f)
#' format(f)
NULL

#' @rdname print.fec_filing
#' @export
format.fec_filing <- function(x, ...) {
  header <- .subset2(x, "header")
  info <- attr(x, "cover_info")
  raw <- isTRUE(attr(x, "raw"))
  sep <- if (cli::is_utf8_output()) " \u00b7 " else " - "

  parts <- c(
    na_drop(info$form_type),
    na_drop(info$filer_name),
    format_coverage(info$coverage_from_date, info$coverage_through_date),
    if (!is.na(header$fec_version)) paste0("v", header$fec_version),
    if (isTRUE(header$is_paper)) "paper",
    if (raw) "raw = TRUE"
  )
  title <- cli::style_bold(paste0("<fec_filing ", header$filing_id, ">"))
  first <- paste(c(title, paste(parts, collapse = sep)), collapse = " ")

  tables <- .subset2(x, "tables")
  if (length(tables) == 0L) {
    return(c(first, cli::col_grey("  (no itemizations)")))
  }
  kinds <- attr(x, "table_kinds")
  nrows <- vapply(tables, nrow, integer(1))
  ncols <- vapply(tables, ncol, integer(1))
  times <- cli::symbol$times
  lines <- paste0(
    "  ",
    formatC(names(tables), width = -max(nchar(names(tables)))),
    "  ",
    format(nrows, big.mark = ","), # right-aligned to a common width
    " ", times, " ",
    format(ncols)
  )
  # Every table is raw with `raw = TRUE`, which the first line says once.
  fallback <- !raw & kinds[names(tables)] == "raw"
  lines[fallback] <- paste0(lines[fallback], "  ", cli::col_grey("(raw columns)"))
  c(first, lines)
}

#' @rdname print.fec_filing
#' @export
print.fec_filing <- function(x, ...) {
  cat(format(x, ...), sep = "\n")
  invisible(x)
}

na_drop <- function(x) {
  if (length(x) == 1L && !is.na(x) && nzchar(x)) x
}

format_coverage <- function(from, through) {
  from <- if (length(from) == 1L && !is.na(from)) format(from)
  through <- if (length(through) == 1L && !is.na(through)) format(through)
  if (!is.null(from) && !is.null(through)) {
    paste(from, "to", through)
  } else if (!is.null(through)) {
    paste("through", through)
  } else if (!is.null(from)) {
    paste("from", from)
  }
}
