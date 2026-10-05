# the synthetic filing reads without error into typed, fallback and other tables

    Code
      print(f)
    Output
      <fec_filing 9999001> F3XN · PFIZER INC. PAC · 2023-07-01 to 2023-07-31 · v8.4
        schedule_a      2 × 48
        schedule_a_raw  1 × 45  (raw columns)
        schedule_i      1 ×  6  (raw columns)
        other           1 ×  4

# the legacy fixture's SI rows are a fallback table, flagged in print()

    Code
      print(f)
    Output
      <fec_filing 3.00_13801> F3XN · Democratic Congressional Campaign Committee - Expenditures · 2001-03-01 to 2001-03-31 · v3.00
        h4           2 × 36
        schedule_b  15 × 49
        h1           1 × 15
        h2           2 × 12
        h3           2 × 12
        schedule_c   1 × 40
        schedule_d   2 × 38
        schedule_a   6 × 48
        schedule_i   2 × 32  (raw columns)

# a paper filing's unrecognised summary rows go to `other`

    Code
      print(f)
    Output
      <fec_filing P2.6_716051> F3A · GRASSLEY COMMITTEE INC. · 2010-10-14 to 2010-11-22 · vP2.6 · paper
        schedule_a  12 × 48
        schedule_b   6 × 49
        schedule_d   1 × 38
        other        1 × 33

