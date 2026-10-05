# a missing file is a libfec_error_io

    Code
      fec_read("does-not-exist.fec")
    Condition
      Error in `fec_read()`:
      ! Can't read 'does-not-exist.fec'.
      x The file doesn't exist.

# the error message names the file and the reason

    Code
      fec_read("notes.txt")
    Condition
      Error in `fec_read()`:
      ! 'notes.txt' isn't a readable FEC filing.
      x Incorrect header record type: not a filing

# print() summarises the filing

    Code
      fec_read(pfizer_path())
    Output
      <fec_filing 1721696> F3XN · PFIZER INC. PAC · 2023-07-01 to 2023-07-31 · v8.4
        schedule_a  1,354 × 48
        schedule_b     33 × 49
    Code
      fec_read(pfizer_path(), raw = TRUE)
    Output
      <fec_filing 1721696> F3XN · PFIZER INC. PAC · 2023-07-01 to 2023-07-31 · v8.4 · raw = TRUE
        schedule_a  1,354 × 46
        schedule_b     33 × 45
    Code
      fec_read(pfizer_path(), n_max = 0)
    Output
      <fec_filing 1721696> F3XN · PFIZER INC. PAC · 2023-07-01 to 2023-07-31 · v8.4
        (no itemizations)
    Code
      fec_read(fixture("SA3L_1921461.fec"))
    Output
      <fec_filing SA3L_1921461> F3LN · Allred for Texas · 2025-07-01 to 2025-09-30 · v8.5
        schedule_a3l  1 × 32

# print() falls back to ASCII separators without UTF-8 output

    Code
      fec_read(pfizer_path())
    Output
      <fec_filing 1721696> F3XN - PFIZER INC. PAC - 2023-07-01 to 2023-07-31 - v8.4
        schedule_a  1,354 x 48
        schedule_b     33 x 49

