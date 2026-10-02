# mappings2.json changes (legacy formats)

Changes made to `src/mappings2.json` on top of the fecfile 0.9.1 file it was
derived from, so legacy (v1–v7, paper P1.0–P3.4) rows resolve to correct column
lists. Each change cites its source.

Sources (none of the workbooks are committed):

- **Spec workbooks**: `fec-docs/eFilingFormats/FEC_v{1,2,3,5,6,7,8}x/*.xls[x]`
  in the main checkout (unzipped `eFilingFormats.zip`). Cited as
  `<workbook> <sheet> r<row>` or `<workbook> <sheet> seq <n>` (the "COL SEQ"
  column). Text dumps were made with xlrd/openpyxl.
- **8.5 workbook**: not in `fec-docs`; `~/Downloads/FEC_Format_v8.5.xlsx`
  (md5 `09dd480627bccc07c1d25748fef8d2ca`, identical to the five other copies there).
- **Header workbook**: `fec-docs/eFilingFormats/e-filing headers all versions.xlsx`
  (one row of column labels per form × version). Its labels contain copy
  errors (e.g. F1 repeats "STREET 1"), so lengths were taken from the spec
  workbooks, which number every field.
- **Samples**: `wiki/legacy/samples/` (995 filings, git-excluded). Cited as
  `paper/<id>.fec` / `electronic/<id>.fec`.

8.x guarantee: no column list used by a version 8.0–8.5 was edited. Where a
version key that also covers 8.x had to be split or extended, the 8.x list
was kept byte-identical. The only 8.x-visible change is that F2S rows now get
the F2S layout instead of F2's (see below).

## 1. Typos and holes (version keys, form regexes, missing lists)

| form | change | source |
|---|---|---|
| `^f93` | `^(P3.4\|3.3\|3.2)` → `^(P3.4\|P3.3\|P3.2)`, `^(P3.1\|3.0\|P2\|P1)` → `^(P3.1\|P3.0\|P2\|P1)`. Before, P3.2/P3.3/P3.0 F93 matched nothing and electronic 3.00 F93 hit a paper list. | Every other paper key uses `P`; there is no F93 sheet in Fec_v300.xls (F9x was added in v5). |
| `^f93` | split `^8.5…6.1` into `^8.5…8.0` (list unchanged) and new `^7.0\|6.4\|6.3\|6.2\|6.1` (26 cols: `expenditure_purpose_code` inserted at index 21). | FEC_Format_v6.1.xls … FEC_Format_v7.0.xlsx F93 r31 "EXPENDITURE PURPOSE CODE" (seq 22), dropped in FEC_Format_v8.0.xlsx (25 fields). Rows in electronic/359563.fec and 444807.fec have 26 fields, `''` at 21 and the purpose text at 22. |
| `^f1s` | `^5.3\|5.2\|5.1\|5.0` and `^3.0` were empty lists; now copies of F1's `^5.3…5.0` / `^3.0` lists. | FEC_v530.xls F1S r1 "Secondary Record (Note: Same layout as F1)"; Fec_v300.xls "F1 S" r1 (same note). 19 sample files (3.00, 5.x) have F1S rows, max 63 fields. |
| `^sa3l` | added paper keys `^P3.2\|^P3.3\|^P3.4` (24 cols) and `^(P2.6\|P3.0\|P3.1)` (23 cols), copies of the paper Schedule A lists. | No paper workbook exists. paper/490884.fec (P2.6), 884011 (P3.0), 998451 (P3.1), 1314260 (P3.4): SA3L rows have exactly 23/24 fields and line up with the SA paper columns (name, address, employer, aggregate, amount, image number). |
| `^f3z1`, `^f3z2` | `^(P3.4\|8.4\|8.3\|8.2)` split into `^P3.4` (8.x list + `image_number`) and `^(8.4\|8.3\|8.2)` (list unchanged). | paper/1596550.fec: F3Z1 37 / F3Z2 35 fields; the last is an 18-digit image number like every other paper row. |
| `(^f3pz1)` | added `^P3.4` = 8.4 list + `image_number` (45). | paper/1958458.fec F3PZ1 row 1: 45 fields, image number last. (Row 2 of that filing is mis-keyed by the data-entry vendor; nothing to map.) |
| `^f56` | `^8.5…6.2` → `^8.5…6.2\|6.1` (list unchanged). | FEC_Format_v6.1.xls F56 seq 1–20 equal FEC_Format_v6.2.xls F56 label for label. |
| `^f2s` (new, before F2) | `^8.5\|…\|6.1` 9 cols; `^P3.4\|^P3.3\|^P3.2` 10 cols (+`image_number`). Before, F2S hit `(^f2$)\|(^f2[^4])` and got F2's 38–44 columns. | FEC_Format_v6.1.xls F2S r1 "New layout in V6.0", seq 1–9; same 9 fields in v6.2–v8.5. Samples: 8.4/8.5 F2S rows 9 fields (electronic/1910645, 1923519); P3.2–P3.4 rows 10 fields ending in an image number (paper/1093482, 1109115, 1208761). |
| `^f8ii$` → `(^f8ii$)\|(^f82$)`, `^f8iii$` → `(^f8iii$)\|(^f83$)` | 6.x row types are F82/F83. | Fec_v1.xls … FEC_v530.xls F82 seq 1 "value = F8II", F83 "F8III"; FEC_Format_v6.1.xls / v6.4.xls F82 r5 "F82", F83 r5 "F83". FEC_Format_v7.0.xlsx F8/F82/F83 r3 "no longer electronically filed" → no 7.0+ entries. |
| `(^f8$)\|(^f8[an])` | `^5` → `^5\|^3\|^2\|^1`. | Fec_v1/v2/v300/v500 F8: 32 fields, same labels. |
| F82/F83 | `^5` → `^5\|^3`; new `^2` (v3 list + `orig_tran_id`, `supr_tran_id`) and `^1` (23 / 15 cols). | Fec_v300 F82 (29) / F83 (21) = Fec_v500; Fec_v2.xls F82 seq 1–31, F83 seq 1–23 (ORIG_TRAN_ID, SUPR_TRAN_ID last); Fec_v1.xls F82 seq 1–23, F83 seq 1–15 (SEQUENCE NUMBER at 3, no entity/candidate block). |
| `^h1` `^3.0\|^2\|^1` | column 0 `ballot_local_candidates` → `form_type` (the name also sits at index 22). | Fec_v300.xls "Sch H1" seq 1 FORM TYPE; electronic/13801.fec H1 rows start with "H1". |
| `^sc[^1-2]` | `^5.0\|^3\|^2\|^1` → `^5.0\|^3\|^2`; new `^1` with 46 cols (three embedded endorser/guarantor blocks `guarantor_{1,2,3}_*`). | Fec_v1.xls SC seq 1–46. electronic/119.fec (1.02) SC/10 rows have 46 fields. |
| `^f1[an]` | `^2` → `^2\|^1`. | Fec_v1.xls F1 = Fec_v2.xls F1, seq 1–60, same labels. |
| F1M, F4, F5, F7 | v1/v2 added to the v3 key (`…\|^2\|^1`). | Fec_v1.xls / Fec_v2.xls F1M (47), F4 (80), F5 (25), F7 (18): same field count and labels as Fec_v300.xls. |
| F2 | `^3.0` → `^3.0\|^2`; new `^1` (28 = v3 list without `candidate_signature_name`). | Fec_v2.xls F2 seq 1–29 = Fec_v300.xls; Fec_v1.xls F2 seq 1–28 (no "NAME (as signed)"). |
| F3P31 | new `^2` (34) and `^1` (30). | Fec_v2.xls F3PLine31 seq 1–34 (v3's 32 + ORIG/SUPR_TRAN_ID); Fec_v1.xls F3PL31 seq 1–30 (form type value F3P31AL; no entity type, has committee address block). |
| F56 / F65 | new `^1` (14 cols) for both; F56 new `^2` (29). F65's `^2` was already mapped (27, v3 layout). | Fec_v1.xls F56/F65 seq 1–14 (SEQUENCE NUMBER, AMENDED); Fec_v2.xls F56 seq 1–29. |
| F57 | `^3`: index 4 `payee_street_2` → `payee_street_1`, index 5 `''` → `payee_street_2`, appended `transaction_id_number` (31 → 32). New `^2` (34) and `^1` (19). | Fec_v300.xls F57 seq 5 STREET 1, 6 STREET 2, 32 TRAN ID; Fec_v2.xls F57 seq 1–34; Fec_v1.xls F57 seq 1–19. |
| F76 | new `^2` (18) and `^1` (16). | Fec_v2.xls F76 seq 1–18; Fec_v1.xls F76 seq 1–16 (SEQUENCE NUMBER at 3). |
| SC1, SF, H3, H4 | new `^1` lists (40, 26, 33, 21). | Fec_v1.xls SC1 seq 1–40, SF seq 1–26 (designating committee address, no subordinate block), SH3 seq 1–33 (four event rows per activity), SH4 seq 1–21. |
| SI | new `^2` (32) and `^1` (29). | Fec_v2.xls SI seq 1–32 (v3's first 30 + ORIG/SUPR_TRAN_ID); Fec_v1.xls SI seq 1–29 (v3's first 29). |
| `^f11` (new) | `^5.1\|5.0`, 28 cols. | Fec_v500.xls / Fec_v510.xls F11 seq 1–28 (form removed in v5.2). |

### Looked at, not changed

- **F3Z1/F3Z2/F3PZ1/F3PZ2 8.5**: the 8.5 workbook's SUMMARY OF CHANGES r12/r13/r17/r18
  says these are "Excluded from format specifications". None of the 48.7k
  cached filings has an 8.5 F3Z*/F3PZ* row. No 8.5 entries added (the sqlite
  export already falls back to 8.4 for them).
- **F3Z 8.2+**: FEC_Format_v8.2.xlsx replaced F3Z with F3Z1/F3Z2 (sheets "F3Z1(NEW)", "F3Z2(NEW)").
- **F3L 6.1–6.3, SA3L 6.1**: no F3L sheet before FEC_Format_v6.4.xls; F3L/SA3L start in 6.4.
- **F8 7.0+**: no longer electronically filed (FEC_Format_v7.0.xlsx F8 r3).
- **F12 (v5.0/5.1)**: Fec_v510.xls F12 numbers seq 1–19 then jumps to 27–28; seq 20–26
  are not in the workbook, so the row width is unknown. Left unmapped; no samples.
- **F24, TEXT, F99, F3S, F3PS, F9x, F13, H5, H6, SL in v1/v2**: no such sheet in Fec_v1.xls/Fec_v2.xls.
- **SA32** (paper/1018710.fec, P3.1): the filing is an F3LN with a single SA32
  row whose aggregate/amount equal the F3L totals — a data-entry typo for SA3L.
  Left unmapped rather than adding a regex for one mis-keyed row.
- **5.20 F3X width 107** (electronic/186657.fec, 178154.fec): one vendor
  (Public Affairs Support Services) writes the 107 v3-era fields and pads with
  empties. The v5 list's first 107 names equal the v3 list, so no change.
- **Paper SC P3.3/P3.4** rows have a 27th field (`Y`/`N`) after `image_number`
  (paper/1147684.fec, 1313756.fec). No source for its meaning; within the
  test's +2 slack.
- Other spec/list length mismatches that real rows never exceed (e.g. v2 SB 33
  vs 34, v2 SD 30 vs 28, v2 H1/H2 trailing ORIG/SUPR_TRAN_ID, v5 F92/F93 short
  lists) were left alone.

## 2. Duplicate and blank column names (legacy lists only)

`FilingCover` and the exporters key rows by column name, so a repeated name
loses one value and `""` collides with every other blank. Lists that also
serve 8.0–8.5 were not touched (they already use `_TODO_DUP`, except the
known `TODO_UNKNOWN_BLANK` in F3L 6.4+).

**Duplicates**: the second occurrence gets `<name>_TODO_DUP`, the convention
the 8.x lists already use and that `covers/form3x.rs` (`row_dup`),
`covers/form4.rs` (`dup_key`), `covers/form3p.rs` (`row_dup`) and
`covers/form2.rs` (`candidate_state_TODO_DUP` = office state) read first.
The second occurrence is the same form line as the 8.x `_TODO_DUP` column
in every case (e.g. F3X v3 `col_a_total_receipts` at 17 = Line 6(c), at 36 =
Line 19).

| form | lists | renamed |
|---|---|---|
| F2 | `^3.0\|^2`, `^5.3…5.0`, `^6.3\|6.2\|6.1`, all four paper keys | `candidate_state` (office state) |
| F3L | `^P(3.4\|3.3\|3.2)`, `^P(3.1\|3.0\|2.6)` | `election_state` |
| F3P | `^(5.1\|5.0\|3\|2\|1)`, `^5.3\|5.2`, `^6.4…6.1`, `^P3.2…` | `col_a_total_receipts`, `col_a_total_disbursements` |
| F3PS | `^5.3…^3`, `^6.4…6.1` | `a_individuals`, `b_political_party_committees` |
| F3X | `^3\|^2\|^1`, `^5.3…5.0` (receipts already done there), both paper keys | `col_{a,b}_total_{receipts,disbursements,contributions}` |
| F4 | `^5.3…^1`, both paper keys | `col_{a,b}_total_{receipts,disbursements}` |
| SC1 | `^2`, `^3`, `^5.2\|5.1\|5.0`, `^5.3`, `^(P1\|P2\|P3.0\|P3.1)` | `description` (E.2, future-income description) |

Exceptions where the workbook gives a different name:

- `^sl` (5.x and paper): named after the **8.x** columns by meaning, including
  8.x's own mislabels, so exports (which remap other versions into the 8.5
  layout by name) put each amount where 8.x rows put it. The 8.x list (`^8.5…6.1`)
  is left as is (renaming it would change every 8.x export's schema), but its
  names are wrong: FEC_Format_v8.0.xlsx "Sch L" seq 23 "10. DISBURSEMENTS" and
  seq 24 "11. ENDING CASH ON HAND" are **column A** (column B starts at seq 25
  "1a."), yet 8.x index 22/23 are named `col_b_disbursements_period` /
  `col_b_cash_on_hand_close_of_period`; the real column-B line 10/11 (seq 40/41,
  index 39/40) are `col_b_disbursements_period_TODO_DUP` /
  `col_b_cash_on_hand_close_of_period_TODO_DUP`. **Known upstream naming bug.**
  Legacy lists follow it:
  - `^5.3\|5.2\|5.1\|5.0` index 21 (FEC_v530.xls "Sch L" r32 seq 22
    "10.Disbursements", column A; column B starts at seq 23 "1a.") →
    `col_b_disbursements_period`; index 37 (column B line 10) →
    `col_b_disbursements_period_TODO_DUP`; index 38 (column B line 11) →
    `col_b_cash_on_hand_close_of_period_TODO_DUP`. 5.x has no column-A line
    11. (Ticket 03 had renamed index 21 `col_a_disbursements_period`, the
    correct meaning but a name 8.x doesn't use, so exports dropped it and
    filled 8.x's column-A slot from column B; electronic/123528.fec.)
  - `^(P3\|P2\|P1)` index 18/19 (column A line 10/11) →
    `col_b_disbursements_period` / `col_b_cash_on_hand_close_of_period`; index
    35/36 (column B line 10/11) → the two `_TODO_DUP` names.
- `^text` `^5.3`, `^5.2\|5.1\|5.0`, `^3`: index 1 `form_type` →
  `back_reference_sched_form_name`, the 8.x name of the same field: the form or
  schedule line the text belongs to (FEC_v530.xls "Text" r9 seq 2 "FORM TYPE",
  Fec_v300.xls "Text" r7 seq 2; FEC_Format_v8.0.xlsx "Text" r9 seq 5 "BACK
  REFERENCE SCHED / FORM NAME"). Values are `F3XT`, `SA15`, ...
  (electronic/19450.fec); under `form_type` exports dropped them (column 0
  of a TEXT row is `rec_type`).
- `^h1` `^5.3\|5.2`: index 2 `transaction_id` → `unused_3` and index 28 `""` →
  `transaction_id`: FEC_v530.xls "Sch H1" r12 seq 3 SPACE HOLDER, r39 seq 29
  TRAN ID; electronic/266203.fec H1 has the tran id ("H1J15") at index 28.
- `^h4` `^5.1\|5.0` index 14 → `admin_voter_drive_activity`: Fec_v510.xls "Sch H4" r25
  "YESNO (Activity Is Admin./Voter Drive)" is the old combined flag; index 38
  ("Activity is Administrative") keeps `administrative_voter_drive_activity`,
  as in the 8.x list.

**F92/F93 v5 lists replaced**: Fec_v500/v510/FEC_v520/FEC_v530 F92 equals
"Sch A" and F93 equals "Sch B" label for label (5.3 only swaps unused fields
for SPACE HOLDER). The old lists were short (f92 `^5.3|5.2` 36 of 44,
`^5.0` 36 of 38) and f93 `^5.0` was shifted by one from index 9 on. They are
now copies of `^sa[^3]` `^5.2`/`^5.1`/`^5.0` and `^sb` `^5.2|5.1`/`^5.0`.

**Blank names** (`""`) were named from the spec label at that position,
reusing the name the same form (or SA/SB for F92/F93, F56/F65 for F57) uses
for that label in another version; `AMENDED CD` → `amended_cd` (F57:
`amended_code`, as its v3 list already had). Positions that are SPACE
HOLDER / "Unused field" in every version a key covers are named
`unused_<seq>` (1-based field number):

| form | key | index → name (spec) |
|---|---|---|
| F132 / F133 | `^5.3\|5.2` | 17 / 16 → `internal_use_only` (FEC_v520.xls "INTERNAL USE ONLY") |
| F1S | `^6.1` | 10 → `affiliated_organization_type` (FEC_Format_v6.1.xls F1S "6. ORGANIZATION TYPE") |
| F24 | `^5.0…5.3`, `^3` | 8 → `treasurer_name` ("NAME/TREASURER (as signed)") |
| F3P31 | `^5.3` / `^5.2…^3` | 30 → `unused_31` (5.3 SPACE HOLDER) / `amended_cd` |
| F3S | `^5.3…^3` | 25 `b_loan_repayments_all_other_loans`, 28 `b_refund_political_party_committees`, 29 `c_refund_other_political_committees` (Fec_v300.xls F3S 19(b), 20(b), 20(c); names as in the 6.x+ list) |
| F5 | `^5.3` | 11 `individual_occupation` (INDOCC); 13–15, 22–24 → `unused_14…16`, `unused_23…25` |
| F56, F76, F94, H5, H6 | v5 keys | `amended_cd` |
| F57 | `^5.3…5.0`, `^3` | 18 `payee_cmtte_fec_id_number` (8.x name for the payee committee id), 19–23 `unused_20…24`, 30 `amended_code` |
| F91 | `^5.3` | 10 → `unused_11` |
| H1 | `^5.3\|5.2` / `^5.1\|5.0` | 3–26 → `unused_4…27`, 27 `internal_use_only` / 27 `amended_cd` |
| H2 | `^5.3\|5.2\|5.1` | 4 `exempt_activity` (FEC_v510 "Activity Is Exempt"), 9 `amended_cd` |
| H4 | `^5.3\|5.2` / `^5.1\|5.0` | 14 `unused_15`, 32 `internal_use_only` / 14 see above, 32 `amended_cd` |
| SE | `^5.3`, `^5.2…`, `^3` | 19–23 `unused_20…24`; 35 `unused_36` (5.3) / `amended_cd` |
| SE | `^2` | 19–23 `payee_candidate_{id_number,name,office,state,district}` (Fec_v2.xls SE seq 20–24) |
| SF | `^5.3` / `^5.2…` | 35 `unused_36` / `amended_cd` |
| SL | `^5.3…5.0` | 39 `amended_cd` (label "AMENDED CODE") |

## 2b. Cover layouts found by value checks (ticket legacy/04)

Found by comparing typed cover values to the raw records over the samples
(see `wiki/legacy/COVERS.md`). No 8.x list changed.

| form | change | source |
|---|---|---|
| `^f1[an]` | `^6.3\|6.2\|6.1` (99 cols, the 6.4–8.3 list) split into `^6.3` (unchanged), `^6.2` (90) and `^6.1` (89). Before, every 6.1/6.2 F1 after field 10 was shifted (custodian name in `effective_date`, committee type in `date_signed`, …). | FEC_Format_v6.1.xls F1 seq 1–89 (COMMITTEE EMAIL, WEB URL, FAX at 11–13, no change-of flags; custodian before treasurer; affiliated block at 57–65 with RELATIONSHIP text + ORGANIZATION TYPE). FEC_Format_v6.2.xls F1 seq 1–90 (5(e) ORGANIZATION TYPE, 5(f) LEADERSHIP PAC at 33–34; affiliated block 35–42 before custodian); the sheet's two stray rows numbered 64/65 between seq 41 and 42 are 6.1 leftovers and were skipped. FEC_Format_v6.3.xls F1 lists 100 rows but two are numbered 13 (a stale FAX row): the real layout is the 99-field 6.4 one. Samples: electronic/337691.fec (6.1, 89 fields), 357167.fec (6.2, 90), 6.3 files 99. New names reuse existing ones: FAX → `committee_fax_number`, 6.1 RELATIONSHIP → `affiliated_relationship_code`, 6.1 "6. ORGANIZATION TYPE" → `organization_type` (the v5 list's names for the same fields). |
| `(^f3x$)\|(^f3x[ant])` paper `^P1\|^P2\|^P3.0\|^P3.1` and `^P3.2\|^P3.3\|^P3.4` | swapped `col_b_cash_on_hand_jan_1` / `col_b_year` (Line 6(a)). | 106 of the 107 paper F3X samples with a value there have a 4-digit year first and the amount second (the other, paper/1147759.fec, has `990`, `990.00`: a keying error) (e.g. paper/265165.fec P2.2 `2006`, `10961`; 1215766.fec P3.4 `2017`, `32.00`); the paper form prints "Cash on Hand January 1, 20__" with the year before the amount. No paper workbook exists. |

## 3. Version-regex hardening (proc macro, JSON keys unchanged)

The JSON version keys keep their original text. `gen_form_type_version_set!`
compiles `harden_version_regex(key)` instead: every `.` is escaped and a key
with a top-level `|` becomes `^(?:a|b|…)` with each alternative's leading `^`
stripped (`^8.5|8.4` → `^(?:8\.5|8\.4)`; `^(P3.4|P3.3)` → `^(P3\.4|P3\.3)`).
Before, `^8.5|8.4` meant `(^8.5)|(8.4 anywhere)`. Checked: for all 60 form
keys × 39 version strings (every version in the samples plus the ticket's
list) the first matching key is the same before and after. The
`tests/mappings.rs` resolution snapshot pins the resulting table.

`build.rs` now says `rerun-if-changed=src/mappings2.json` (it pointed at a
non-existent `mappings2.json`, so JSON edits did not rebuild the macro crate).

## Observed-rows fixture

`crates/fec-parser/tests/fixtures/legacy/observed_row_types.tsv` lists every
(fec_version, row_type) in `wiki/legacy/samples` with the widest row seen
(trailing empty/whitespace fields trimmed) and an example file. It was
generated once with this script (not committed; ~60 lines of Python):

- skip the `/* Header … /*` block of v1/v2 files (up to the second line
  starting `/*`); HDR lines are row type `HDR` with the version from field 2
  (paper: field 1);
- skip blank lines and `[BEGINTEXT]`…`[ENDTEXT]` blocks (case-insensitive,
  optional space);
- split each line with Python's `csv.reader` (quote-aware, quoting confined to
  the line), delimiter 0x1C if the line contains one, else `,`;
- row type = field 0 stripped and upper-cased; width = index of the last
  non-blank (whitespace-stripped) field + 1; keep the max per pair.

Mapping misses over the samples (row types with no list or an empty list,
counted as distinct (version, row type) pairs / files): before 12 pairs in 32
files (F1S 3.00/5.x, SA3L P2.6–P3.4, F3PZ1 P3.4, SA32 P3.1); after 1 pair in
1 file (SA32, see above).
