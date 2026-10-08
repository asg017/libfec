#' ---
#' title: "Campaign finance with libfec"
#' output: html_document
#' params:
#'   path: ""
#' ---
#'
#' A short tour of one FEC filing with the `libfec` R package: the cover
#' page's totals, who gave (Schedule A), where the money went (Schedule B).
#' Base R + libfec + tibble only.
#'
#' **How to run** (from the repo root; the package must be installed first,
#' since this script calls `library(libfec)`):
#'
#' ```sh
#' cd crates/fec-r
#' make install                          # R CMD INSTALL . (Rust + R; ~1.5 min cold)
#' Rscript examples/campaign-finance.R   # the bundled filing (Pfizer PAC, F3XN)
#' Rscript examples/campaign-finance.R path/to/filing.fec
#'
#' # A big one: Biden for President's June 2024 F3PN (91 MB, 404k rows, ~2 s)
#' curl -o /tmp/1805248.fec https://docquery.fec.gov/dcdev/posted/1805248.fec
#' Rscript examples/campaign-finance.R /tmp/1805248.fec
#'
#' # An HTML report (needs pandoc: `brew install pandoc`); writes
#' # examples/campaign-finance.html
#' Rscript -e 'rmarkdown::render("examples/campaign-finance.R")'
#' Rscript -e 'rmarkdown::render("examples/campaign-finance.R",
#'                               params = list(path = "/tmp/1805248.fec"))'
#' ```
#'
#' The file is a knitr "spin" script: `#'` lines are the report's text, the
#' rest is plain R.

#+ setup, message = FALSE
library(libfec)

# The filing to read: a command-line argument, the `path` render parameter,
# or the example filing that ships with the package.
args <- commandArgs(trailingOnly = TRUE)
path <- if (length(args) > 0) {
  args[[1]]
} else if (exists("params") && nzchar(params$path)) {
  params$path
} else {
  system.file("extdata", "1721696.fec", package = "libfec")
}

knitting <- isTRUE(getOption("knitr.in.progress"))

# Section headings on the console (the report has its own).
section <- function(title) {
  if (!knitting) cat("\n==", title, strrep("=", max(3, 72 - nchar(title))), "\n\n")
}

dollars <- function(x) paste0("$", formatC(x, format = "f", digits = 2, big.mark = ","))
count <- function(x) format(x, big.mark = ",")

# Sum `amount` by `group` (a vector, or a list of vectors for several
# columns), largest first, as a tibble: the base-R `count(..., wt = )`.
total_by <- function(amount, group, n = 10) {
  if (!is.list(group)) group <- list(group = group)
  key <- do.call(paste, c(group, sep = "\r"))
  totals <- tapply(amount, key, sum)
  rows <- tapply(amount, key, length)
  top <- head(order(totals, decreasing = TRUE), n)
  parts <- do.call(rbind, strsplit(names(totals)[top], "\r", fixed = TRUE))
  colnames(parts) <- names(group)
  tibble::as_tibble(as.data.frame(parts)) |>
    tibble::add_column(rows = as.integer(rows[top]), total = dollars(totals[top]))
}
# With dplyr this would be:
#   df |> count(contributor_address_state, wt = contribution_amount, sort = TRUE)

#' ## 1. Read the filing
#'
#' `fec_read()` returns a `<fec_filing>`: the header, the cover record, and one
#' tibble per record family (`schedule_a` for receipts, `schedule_b` for
#' disbursements, ...). Printing it gives a one-line summary and the table
#' sizes.

#+ read
section("1. Read the filing")
elapsed <- system.time(f <- fec_read(path))[["elapsed"]]
f
names(f$tables)
cat("read in", round(elapsed, 2), "s\n")

#' ## 2. The cover page
#'
#' The cover record is the report's summary page. Its fields depend on the form
#' (a PAC's F3X has different lines from a presidential campaign's F3P), so the
#' form-independent fields (filer, coverage period) are in
#' `attr(f, "cover_info")`, and the headline totals are looked up by name for
#' each form. `fec_cover()` gives the whole cover as a one-row tibble.

#+ cover
section("2. The cover page")
info <- attr(f, "cover_info")
cover <- fec_cover(f)

# The first of `fields` the cover has (the line numbers differ per form).
cover_value <- function(...) {
  fields <- intersect(c(...), names(cover))
  if (length(fields) == 0) NA_real_ else cover[[fields[[1]]]]
}
headline <- c(
  receipts = cover_value(
    "summary_line6c_total_receipts_column_a", # F3X
    "summary_line7_total_receipts"            # F3P
  ),
  disbursements = cover_value(
    "summary_line7_total_disbursements_column_a", # F3X
    "summary_line9_total_disbursements"           # F3P
  ),
  cash_on_hand_at_close = cover_value(
    "summary_line8_cash_on_hand_close_of_period_column_a", # F3X
    "summary_line10_cash_on_hand_end_period"               # F3P
  )
)

cat(
  "Filer:    ", info$filer_name, " (", info$filer_id, ")\n",
  "Form:     ", info$form_type, ", report ", info$report_code, "\n",
  "Coverage: ", format(info$coverage_from_date), " to ",
  format(info$coverage_through_date), "\n",
  sep = ""
)
data.frame(this_period = ifelse(is.na(headline), "(not on this form)", dollars(headline)),
           row.names = names(headline))
cat("The cover has", ncol(cover), "columns; the first few:\n")
cover[, 1:6]

#' ## 3. Receipts (Schedule A)
#'
#' Every itemized receipt is a row of `schedule_a`. `form_type` is the line of
#' the report it goes on: on an F3X, `SA11AI` is contributions from
#' individuals and `SA11C` from other committees; on an F3P, `SA17A` is
#' individuals and `SA18` transfers from other authorized committees (a joint
#' fundraising committee, say).
#'
#' **Memo rows** (`memo == TRUE`) are information, not money received: when
#' a $35M transfer arrives from a joint fundraising committee, or a conduit
#' like ActBlue forwards earmarked contributions, the money is the one
#' non-memo row, and the memo rows say who the original donors were. Summing
#' every row counts that money twice, so totals below drop memo rows with
#' `!memo`.
#'
#' A blank amount reads as `0`, not `NA` (see `?fec_read`), so `sum()` needs
#' no `na.rm`.

#+ receipts
section("3. Receipts (Schedule A)")
sa <- f$schedule_a
if (is.null(sa)) {
  cat("This filing has no Schedule A.\n")
  sa <- tibble::tibble(memo = logical(), contribution_amount = double())
} else {
  money <- sa[!sa$memo, ]
  cat(
    "Rows: ", count(nrow(sa)), " (memo rows: ", count(sum(sa$memo)), ")\n",
    "Itemized receipts, excluding memo rows: ", dollars(sum(money$contribution_amount)), "\n",
    "Summing every row instead would give:   ", dollars(sum(sa$contribution_amount)), "\n",
    sep = ""
  )
  # Rows and amounts by report line, memo rows apart.
  total_by(sa$contribution_amount, list(line = sa$form_type, memo = sa$memo))
}

#' ## 4. Where the money comes from
#'
#' Contributions from individuals by the donor's state and city.

#+ geography
section("4. Where the money comes from")
individuals <- sa[!sa$memo & sa$contributor_entity_type %in% "IND", ]
if (nrow(individuals) > 0) {
  print(total_by(individuals$contribution_amount,
                 list(state = individuals$contributor_address_state)))
  total_by(individuals$contribution_amount,
           list(city = toupper(individuals$contributor_address_city),
                state = individuals$contributor_address_state))
}

#' ## 5. Top individual donors
#'
#' A donor is a name and a ZIP code here (FEC data has no donor ID). Each row
#' is one contribution, so a donor's total is the sum of their rows. Real
#' deduplication needs name cleaning, e.g. with the campfin package.

#+ donors
section("5. Top individual donors")
if (nrow(individuals) > 0) {
  total_by(individuals$contribution_amount,
           list(last = individuals$contributor_name_last_name,
                first = individuals$contributor_name_first_name,
                zip5 = substr(individuals$contributor_address_zip_code, 1, 5)))
}

#' ## 6. Contributions over time
#'
#' `contribution_date` is a `Date`, so `format()` buckets it by month (or use
#' `cut(x, "week")`).

#+ over-time
section("6. Contributions over time")
if (nrow(individuals) > 0) {
  ym <- format(individuals$contribution_date, "%Y-%m")
  by_month <- tapply(individuals$contribution_amount, ym, sum) # sorted by month
  tibble::tibble(month = names(by_month),
                 rows = as.vector(table(ym)),
                 total = dollars(by_month))
}

#' ## 7. Disbursements (Schedule B)
#'
#' `schedule_b` has one row per itemized disbursement. The payee is an
#' organization (`payee_organization_name`) or a person (`payee_name_*`).

#+ disbursements
section("7. Disbursements (Schedule B)")
sb <- f$schedule_b
if (is.null(sb) || nrow(sb) == 0) {
  cat("This filing has no Schedule B.\n")
} else {
  sb <- sb[!sb$memo, ]
  payee <- ifelse(is.na(sb$payee_organization_name),
                  trimws(paste(sb$payee_name_first_name, sb$payee_name_last_name)),
                  sb$payee_organization_name)
  cat("Itemized disbursements, excluding memo rows:", dollars(sum(sb$expenditure_amount)),
      "in", count(nrow(sb)), "rows\n\nTop payees:\n")
  print(total_by(sb$expenditure_amount, list(payee = payee)))
  cat("\nTop purposes:\n")
  total_by(sb$expenditure_amount, list(purpose = sb$expenditure_purpose_description))
}

#' ## 8. Typed columns vs `raw = TRUE`, and `n_max`
#'
#' By default the columns are typed and named the same for every FEC format
#' version, so tables from different filings line up. `raw = TRUE` gives the
#' format version's own field names, as filed (text, plus `Date` and `double`
#' for dates and amounts). `n_max` stops after that many itemization rows, for
#' a quick look at a big file. (Note the raw layout lists the last name before
#' the first; the typed columns follow the struct's field order.)

#+ raw
section("8. Typed columns vs raw = TRUE, and n_max")
peek <- fec_read(path, raw = TRUE, n_max = 5)
peek
if (!is.null(peek$schedule_a)) {
  tibble::tibble(typed = head(names(f$schedule_a), 12),
                 raw = head(names(peek$schedule_a), 12))
}

#' ## 9. Stacking covers from several filings
#'
#' `fec_cover()` reads only the header and the cover, so it's cheap on any
#' file. Covers of the same form stack with `rbind()`; covers of different
#' forms have different columns, and `vctrs::vec_rbind()` (installed with
#' tibble) stacks them, filling the gaps with `NA`. (`dplyr::bind_rows()`
#' and `purrr::list_rbind()` do the same.)

#+ stack
section("9. Stacking covers from several filings")
example <- system.file("extdata", "1721696.fec", package = "libfec")
paths <- unique(c(example, path))
covers <- do.call(vctrs::vec_rbind, lapply(paths, fec_cover))
covers[, c("filing_id", "form_type", "committee_name", "coverage_through_date")]
cat("Stacked", nrow(covers), "cover(s) into", ncol(covers), "columns.",
    if (length(paths) == 1) "Pass a filing of another form to see the columns union.", "\n")
