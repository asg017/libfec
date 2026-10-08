#' Read an FEC electronic filing
#'
#' `fec_read()` reads a `.fec` file into a `<fec_filing>`: the header, the
#' cover record, and **one tibble per record family** (`schedule_a`,
#' `schedule_b`, `text`, ...). Get a table with `f$schedule_a` and the cover
#' with `f$cover`.
#'
#' @section Values in typed columns:
#' By default every table is typed: its columns come from the record's typed
#' structure in the Rust `fec-parser` crate, so amounts are `double`, dates are
#' `Date`, checkboxes are `logical` and everything else is `character`. When a
#' field can't be read as its type:
#'
#' * **A blank or garbage *main* amount (such as `contribution_amount` or
#'   `expenditure_amount`) is `0`, not `NA`.** This matters for `mean()` and
#'   for counting rows that have no amount.
#' * A blank or garbage secondary amount (such as `contribution_aggregate`) is
#'   `NA`.
#' * A blank or garbage date, including an impossible one such as `20230231`,
#'   is `NA`. Legacy `MM/DD/YYYY` dates are read too.
#' * Blank text is `NA`, except for the fields the record requires (such as
#'   `contributor_name_last_name` and `contributor_name_first_name`), which
#'   are `""` when blank: `contributor_name_first_name` is `""` on every
#'   organization's row.
#'
#' The original text of a garbage value isn't kept. `raw = TRUE` shows the
#' original text of text fields only: it parses dates and amounts too, so a
#' garbage date or amount is `NA` with `raw = TRUE` as well (see "Column
#' names").
#'
#' @section Column names:
#' A typed column is named by the path to its field, joined with `_` and never
#' shortened: a contributor's last name is `contributor_name_last_name`, and
#' the address state is `contributor_address_state`. The names are the same
#' for every FEC format version, legacy ones included, so tables from
#' different filings stack with `dplyr::bind_rows()` or
#' `purrr::list_rbind()`. Every table starts with `filing_id`.
#'
#' With `raw = TRUE`, the columns are the format version's own field names
#' (which can differ between versions). They're all `character` (the text as
#' filed, trimmed, blank as `NA`), except dates (`Date`) and amounts
#' (`double`), which are parsed as in typed columns: one that can't be read
#' is `NA`, main amounts included.
#'
#' @section Rows with no typed structure:
#' Nothing is dropped and nothing errors:
#' * A record family with no typed structure (such as Schedule I), or a row
#'   in a format version with no layout for it, becomes a table with raw
#'   columns. If the same family also has typed rows in the filing, the raw
#'   rows go in `<name>_raw` (such as `schedule_a_raw`). `print()` marks these
#'   tables `(raw columns)`.
#' * Rows that belong to no record family go in an `other` table, with
#'   `filing_id`, `row_type`, `field_1`, `field_2`, ... as text.
#'
#' A field past the end of a row's layout is kept as `extra_1`, `extra_2`,
#' ... on raw rows, and dropped on typed rows.
#'
#' @section Errors:
#' Errors are classed so you can catch them with [tryCatch()]; all inherit
#' from `libfec_error`:
#' * `libfec_error_io`: the file doesn't exist, is a directory or can't be
#'   read.
#' * `libfec_error_header`: the file isn't a `.fec` filing (no `HDR` record,
#'   an unsupported format version, or no cover record).
#' * `libfec_error_parse`: the file broke off mid-way in a way the CSV reader
#'   can't recover from.
#'
#' The condition's `path` field holds the path. Invalid arguments (`raw`,
#' `n_max`, or `x` that isn't a path) are plain errors, not `libfec_error`s.
#'
#' @param x Path to a `.fec` file. (Raw vectors, filing IDs and URLs are not
#'   supported yet.)
#' @param raw `FALSE` (the default) for typed columns. `TRUE` for every
#'   table, and the cover, as the format version's raw fields.
#' @param n_max Maximum number of itemization rows (every row after the
#'   cover) to read. `Inf` (the default) reads them all; `0` reads only the
#'   header and the cover.
#'
#' @return A `<fec_filing>`: a list with
#' * `header`: a named list of the header record's fields (`filing_id`,
#'   `fec_version`, `software_name`, ...);
#' * `cover`: the cover record as a named list of length-1 vectors (typed
#'   like the tables; raw text with `raw = TRUE` or for a form with no typed
#'   structure). [fec_cover()] returns the same as a one-row tibble;
#' * `tables`: a named list of tibbles, one per record family, in the order
#'   they first appear in the file, then `other`.
#'
#' `f$schedule_a` and `f[["schedule_a"]]` reach into `tables`, and return
#' `NULL` (like a list) if the filing has no such table. List the tables with
#' `names(f$tables)`: `names(f)` is `c("header", "cover", "tables")`.
#'
#' Attributes:
#' * `table_kinds`: a named character vector with the same names as
#'   `tables`: `"typed"`, `"raw"` (raw columns) or `"other"`.
#' * `cover_info`: the cover fields every form has, with the same names for
#'   every form: `form_type`, `filer_id`, `filer_name`, `report_code`,
#'   `coverage_from_date` and `coverage_through_date` (`Date`, `NA` for forms
#'   with no coverage period), and `cover_kind` (`"typed"` or `"raw"`).
#' * `raw`: the `raw` argument.
#'
#' @seealso [fec_cover()] for the cover as a one-row tibble.
#' @export
#' @examples
#' path <- system.file("extdata", "1721696.fec", package = "libfec")
#' f <- fec_read(path)
#' f
#'
#' # One tibble per record family
#' names(f$tables)
#' f$schedule_a
#'
#' # Total itemized receipts by state
#' sa <- f$schedule_a
#' sort(tapply(sa$contribution_amount, sa$contributor_address_state, sum), decreasing = TRUE)
#'
#' # The cover, as a named list
#' f$cover$committee_name
#' f$cover$coverage_through_date
#'
#' # The raw fields, as filed
#' fec_read(path, raw = TRUE)
#'
#' # Only the header and cover
#' fec_read(path, n_max = 0)
#'
#' # Errors are classed
#' tryCatch(
#'   fec_read(tempfile()),
#'   libfec_error_io = function(e) "no such file"
#' )
fec_read <- function(x, raw = FALSE, n_max = Inf) {
  check_source(x)
  if (!rlang::is_bool(raw)) {
    cli::cli_abort("{.arg raw} must be {.code TRUE} or {.code FALSE}.")
  }
  check_n_max(n_max)
  res <- read_impl(x, raw, as.double(n_max), call = rlang::current_env())
  new_fec_filing(res, raw = raw)
}

#' The cover record of a filing, as a one-row tibble
#'
#' `fec_cover()` returns the cover record (the summary page: the filer, the
#' coverage period, the totals) as a one-row tibble. It's the same as
#' `f$cover` from [fec_read()], with `filing_id` and `form_type` first, made
#' for stacking the covers of many filings:
#' `purrr::map(paths, fec_cover) |> purrr::list_rbind()`.
#'
#' Given a path, it reads only the header and the cover, not the
#' itemizations.
#'
#' The columns depend on the form (an F3X cover has different fields from an
#' F3P or an F99 cover), and follow the typed rules in [fec_read()]: see
#' "Values in typed columns" and "Column names" there.
#'
#' @param x Path to a `.fec` file, or a `<fec_filing>` from [fec_read()].
#'
#' @return A one-row tibble: `filing_id`, `form_type`, then the cover's
#'   fields.
#' @seealso [fec_read()]
#' @export
#' @examples
#' path <- system.file("extdata", "1721696.fec", package = "libfec")
#' cover <- fec_cover(path)
#' cover[, 1:6]
#' cover$summary_line8_cash_on_hand_close_of_period_column_a
#'
#' # The same from a filing already read
#' f <- fec_read(path)
#' identical(fec_cover(f), cover)
fec_cover <- function(x) {
  if (!inherits(x, "fec_filing")) {
    check_source(x)
    res <- read_impl(x, FALSE, 0, call = rlang::current_env())
    x <- new_fec_filing(res, raw = FALSE)
  }
  cover <- .subset2(x, "cover")
  cover <- cover[setdiff(names(cover), c("filing_id", "form_type"))]
  cols <- c(
    list(
      filing_id = .subset2(x, "header")$filing_id,
      form_type = attr(x, "cover_info")$form_type
    ),
    cover
  )
  tibble::new_tibble(cols, nrow = 1L)
}

# Phase 0 reads paths only.
check_source <- function(x, call = rlang::caller_env()) {
  if (is.character(x) && length(x) == 1L && !is.na(x)) {
    if (grepl("^[a-z][a-z0-9+.-]*://", x, ignore.case = TRUE)) {
      cli::cli_abort(
        c(
          "Reading a filing from a URL isn't supported yet.",
          i = "Download it first, then pass the path to the file."
        ),
        call = call
      )
    }
    return(invisible(x))
  }
  if (is.raw(x)) {
    cli::cli_abort(
      c(
        "Reading a filing from a raw vector isn't supported yet.",
        i = "Write it to a file first, then pass the path."
      ),
      call = call
    )
  }
  if (is.numeric(x)) {
    cli::cli_abort(
      c(
        "Reading a filing by its ID isn't supported yet.",
        i = "Download it from {.url https://docquery.fec.gov/dcdev/posted/{x[[1]]}.fec}, then pass the path."
      ),
      call = call
    )
  }
  cli::cli_abort(
    "{.arg x} must be the path to a {.file .fec} file (a single string), not {.obj_type_friendly {x}}.",
    call = call
  )
}

check_n_max <- function(n_max, call = rlang::caller_env()) {
  ok <- is.numeric(n_max) && length(n_max) == 1L && !is.na(n_max) &&
    n_max >= 0 && n_max == floor(n_max)
  if (!ok) {
    cli::cli_abort(
      "{.arg n_max} must be a single non-negative whole number or {.code Inf}.",
      call = call
    )
  }
  invisible(n_max)
}

new_fec_filing <- function(res, raw) {
  tables <- lapply(res$tables, function(cols) {
    n <- if (length(cols) == 0L) 0L else length(cols[[1L]])
    tibble::new_tibble(cols, nrow = n)
  })
  structure(
    list(header = res$header, cover = res$cover, tables = tables),
    table_kinds = res$table_kinds,
    cover_info = res$cover_info,
    raw = raw,
    class = "fec_filing"
  )
}

#' @export
`$.fec_filing` <- function(x, name) {
  fec_filing_get(x, name)
}

#' @export
`[[.fec_filing` <- function(x, i, ...) {
  if (is.character(i) && length(i) == 1L) {
    return(fec_filing_get(x, i))
  }
  .subset2(x, i, ...)
}

fec_filing_get <- function(x, name) {
  if (name %in% c("header", "cover", "tables")) {
    return(.subset2(x, name))
  }
  .subset2(.subset2(x, "tables"), name)
}

# Tab completion for `f$`: the parts, then the tables.
#' @importFrom utils .DollarNames
#' @export
.DollarNames.fec_filing <- function(x, pattern = "") {
  nms <- c("header", "cover", "tables", names(.subset2(x, "tables")))
  grep(pattern, nms, value = TRUE)
}
