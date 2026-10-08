# typed schedule_a has the flattened struct columns and R types

    Code
      print(as.data.frame(head(sa[cols])))
    Output
        form_type contributor_name_last_name contributor_address_state
      1    SA11AI                   Aaronson                        NY
      2    SA11AI                   Aaronson                        NY
      3    SA11AI                      Aarts                        NY
      4    SA11AI                      Aarts                        NY
      5    SA11AI                      Adams                        NY
      6    SA11AI                      Adams                        NY
        contribution_date contribution_amount contribution_aggregate  memo
      1        2023-07-14              104.17                1458.38 FALSE
      2        2023-07-31              104.17                1458.38 FALSE
      3        2023-07-14               20.84                 291.76 FALSE
      4        2023-07-31               20.84                 291.76 FALSE
      5        2023-07-14               20.00                 280.00 FALSE
      6        2023-07-31               20.00                 280.00 FALSE

# bad arguments are plain errors, not libfec errors

    Code
      fec_read(path, n_max = -1)
    Condition
      Error in `fec_read()`:
      ! `n_max` must be a single non-negative whole number or `Inf`.
    Code
      fec_read(path, raw = NA)
    Condition
      Error in `fec_read()`:
      ! `raw` must be `TRUE` or `FALSE`.
    Code
      fec_read(c(path, path))
    Condition
      Error in `fec_read()`:
      ! `x` must be the path to a '.fec' file (a single string), not a character vector.
    Code
      fec_read(1721696)
    Condition
      Error in `fec_read()`:
      ! Reading a filing by its ID isn't supported yet.
      i Download it from <https://docquery.fec.gov/dcdev/posted/1721696.fec>, then pass the path.
    Code
      fec_read("https://docquery.fec.gov/dcdev/posted/1721696.fec")
    Condition
      Error in `fec_read()`:
      ! Reading a filing from a URL isn't supported yet.
      i Download it first, then pass the path to the file.
    Code
      fec_read(as.raw(1:3))
    Condition
      Error in `fec_read()`:
      ! Reading a filing from a raw vector isn't supported yet.
      i Write it to a file first, then pass the path.

