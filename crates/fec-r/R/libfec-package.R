#' libfec: Parse FEC Electronic Filings
#'
#' Read Federal Election Commission (FEC) electronic filings (`.fec` files)
#' into tibbles, with the `fec-parser` Rust crate from
#' [libfec](https://github.com/asg017/libfec).
#'
#' * [fec_read()] reads a filing: its header, its cover record and one tibble
#'   per record family (`f$schedule_a`, `f$schedule_b`, ...), with typed
#'   columns whose names are the same for every FEC format version.
#' * [fec_cover()] returns just the cover record as a one-row tibble, for
#'   comparing many filings.
#'
#' Read "Values in typed columns" in [fec_read()] before summarising amounts:
#' **a blank or garbage main amount is `0`**, other blank or garbage values
#' are `NA`, and `raw = TRUE` shows the original text.
#'
#' @keywords internal
"_PACKAGE"
