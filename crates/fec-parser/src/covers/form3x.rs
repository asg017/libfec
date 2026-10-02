//! FEC Form 3X — Report of Receipts and Disbursements for Other Than an
//! Authorized Committee.
//!
//! The `F3X` cover record carries all five pages of the paper form in one row:
//! page 1 (identification, report type, coverage period, treasurer), page 2
//! (the Summary Page, Lines 6–10) and pages 3–5 (the Detailed Summary Page,
//! Lines 11–38) (FEC format workbook v8.4, sheet `F3X`, fields 1–123).
//!
//! Sources used throughout, abbreviated in field docs:
//!
//! - **form** — the blank form, [fecfrm3x.pdf](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf) (Rev. 05/2016).
//! - **instructions** — [fecfrm3xi.pdf](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf).
//! - **workbook** — FEC e-filing format workbook v8.4, sheet `F3X`; "field N"
//!   is the workbook's field number, which is also the column's 1-based
//!   position in a v6.1–v8.5 record.
//!
//! # Format versions
//!
//! Column names come from `mappings2.json`. v6.1–v8.5 share one 123-column
//! layout. Older layouts differ in ways this module absorbs:
//!
//! - v1–v5 have a single caret-delimited `treasurer_name` instead of the five
//!   `treasurer_*` columns; see [`Form3X::treasurer`].
//! - v1–v5 and the paper (`P*`) layouts reuse one column name for the
//!   Summary Page line and the Detailed Summary line it is copied from
//!   (6(c)/19, 7/31, 11(d)/33); the mapping renames the second copy
//!   `*_TODO_DUP`, as in v6+. The form defines those pairs as equal, so we
//!   read the `_TODO_DUP` name and fall back to the shared name.
//! - v1–v3 have no Levin-fund (18(b), 18(c)) or federal election activity
//!   (Line 30) columns; those read as `0.0`. Their single Line 18 (transfers
//!   from non-federal accounts) is read into 18(a).
//! - Paper (`P*`) records give Line 6(a)'s year before its amount; the
//!   mapping names those two columns in that order (see
//!   `crates/fec-parser-macros/MAPPINGS_CHANGES.md`). P1.0–P2.4 amounts are
//!   often keyed without a decimal point and are read as written (see
//!   `wiki/legacy/COVERS.md` in the research notes).

use crate::covers::fields::{amount, date, flag, person_name_or_legacy, text, text_or_empty, Data};
use crate::covers::{Address, DetailedSummaryRow, PersonName};
use jiff::civil::Date;
use serde::Serialize;

/// FEC Form 3X, the periodic report of receipts and disbursements filed by
/// "any political committee which is not an authorized committee" — party
/// committees and PACs. Committees authorized by House/Senate candidates file
/// Form 3 and presidential committees Form 3P instead
/// ([fecfrm3xi.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=3)).
///
/// # Columns A and B
///
/// Both summary pages have two columns: **Column A, "This Period"** and
/// **Column B, "Calendar Year-to-Date"**
/// ([fecfrm3x.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=2),
/// [p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=3)).
/// Unlike Forms 3 and 3P, Column B is *not* election-cycle-to-date: it resets
/// each January, is the previous report's year-to-date figure plus this
/// report's Column A, and on the first report of a calendar year equals
/// Column A ([fecfrm3xi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=6)).
/// Every [`DetailedSummaryRow`] in this struct has `column_a` = This Period and
/// `column_b` = Calendar Year-to-Date.
///
/// Some Summary Page lines exist in only one column: cash on hand January 1
/// (Line 6(a)) only in Column B, cash at the beginning of the period (6(b))
/// and debts (Lines 9, 10) only in Column A (workbook fields 23, 28–29, 74–75).
/// Those are plain `f64`s, not rows.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, Serialize)]
pub struct Form3X {
    /// Form type as filed, e.g. `F3XN`: the base form plus the
    /// amendment-indicator suffix (see [`crate::covers::base_form_type`]).
    /// Column `form_type` (FEC format workbook v8.4, sheet `F3X`, field 1;
    /// allowed values "F3X+[N|A|T]"). The form prints Line 3 "IS THIS REPORT
    /// NEW (N) OR AMENDED (A)"
    /// ([fecfrm3x.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=1));
    /// neither the form nor the workbook defines the `T` suffix.
    pub form_type: String,
    /// Line 2, "FEC Identification Number" of the filing committee
    /// (`filer_committee_id_number`, workbook field 2;
    /// [fecfrm3x.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=1)).
    pub filer_committee_id: String,
    /// Line 1, "Name of Committee (in full)" (`committee_name`, workbook
    /// field 3; [fecfrm3x.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=1)).
    pub committee_name: String,
    /// Committee mailing address from Line 1 (`street_1`, `street_2`, `city`,
    /// `state`, `zip_code`; workbook fields 5–9;
    /// [fecfrm3x.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=1)).
    pub address: Address,
    /// The page 1 checkbox "Check if different than previously reported.
    /// (ACC)" beside the address (`change_of_address`, workbook field 4,
    /// "X = Yes"; [fecfrm3x.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=1)).
    pub change_of_address: bool,
    /// Line 4, "Type of Report": the code printed beside the checked box —
    /// `Q1`–`Q3`, `YE`, `MY`, `TER`, `M2`–`M12`, `12P`/`12G`/`12R`/`12C`/`12S`,
    /// `30G`/`30R`/`30S` ([fecfrm3x.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=1)).
    /// A monthly filer's year-end report is coded `MYE` (`report_code`,
    /// workbook field 10). See [`Form3X::report_code_label`].
    pub report_code: Option<String>,
    /// Election code for pre- and post-election reports, e.g. `P2012`
    /// (`election_code`, workbook field 11, allowed values
    /// "C,G,P,R,S,E\[YYYY\]"). The paper form has no box for it; its letter
    /// matches the Primary/General/Runoff/Convention/Special checkbox of
    /// Line 4(c)/(d) ([fecfrm3x.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=1)).
    /// Absent from the paper (`P*`) layouts. See [`Form3X::election_code_label`].
    pub election_code: Option<String>,
    /// Line 4(c)/(d) "Election on MM/DD/YYYY" — the election a 12-day
    /// pre-election or 30-day post-election report is for
    /// (`date_of_election`, workbook field 12;
    /// [fecfrm3x.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=1)).
    pub election_date: Option<Date>,
    /// Line 4(c)/(d) "in the State of" for that election
    /// (`state_of_election`, workbook field 13;
    /// [fecfrm3x.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=1)).
    pub state_of_election: Option<String>,
    /// Line 5, "Covering Period" start date (`coverage_from_date`, workbook
    /// field 14; [fecfrm3x.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=1)).
    pub coverage_from_date: Option<Date>,
    /// Line 5, "Covering Period ... through" end date
    /// (`coverage_through_date`, workbook field 15;
    /// [fecfrm3x.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=1)).
    pub coverage_through_date: Option<Date>,
    /// Summary Page checkbox "This committee has qualified as a multicandidate
    /// committee. (see FEC FORM 1M)" (`qualified_committee`, workbook field 16,
    /// "X = Yes"; [fecfrm3x.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=2)).
    /// The instructions say to check it if the committee "has qualified as a
    /// 'multicandidate committee' and has filed FORM 1M"
    /// ([fecfrm3xi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=5)).
    pub qualified_committee: bool,
    /// "Type or Print Name of Treasurer" on page 1 (`treasurer_last_name`,
    /// `treasurer_first_name`, `treasurer_middle_name`, `treasurer_prefix`,
    /// `treasurer_suffix`; workbook fields 17–21;
    /// [fecfrm3x.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=1)).
    ///
    /// v1–v5 records have one `treasurer_name` column instead, sub-delimited
    /// as "Lastname^Firstname^Prefix^Suffix" (FEC format v5.30 documentation,
    /// `FEC_v530.rtf`, "Individual's Names"); it is split on `^` into those
    /// parts, and the middle name is left empty.
    pub treasurer: PersonName,
    /// Date the treasurer signed, beside "Signature of Treasurer" on page 1
    /// (`date_signed`, workbook field 22;
    /// [fecfrm3x.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=1)).
    pub date_signed: Option<Date>,
    /// Page 2, the Summary Page (Lines 6–10).
    pub summary: Form3XSummary,
    /// Pages 3–5, the Detailed Summary Page (Lines 11–38).
    pub detailed_summary: Form3XDetailedSummary,
}

impl Form3X {
    /// Build from a cover record's `column -> value` map. Returns `None` only
    /// when the record has no committee ID column at all (not an F3X layout).
    pub fn from_data(data: &Data) -> Option<Self> {
        if !data.contains_key("filer_committee_id_number") {
            return None;
        }
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            committee_name: text_or_empty(data, "committee_name"),
            address: Address::from_prefixed(data, ""),
            change_of_address: flag(data, "change_of_address"),
            report_code: text(data, "report_code"),
            election_code: text(data, "election_code"),
            election_date: date(data, "date_of_election"),
            state_of_election: text(data, "state_of_election"),
            coverage_from_date: date(data, "coverage_from_date"),
            coverage_through_date: date(data, "coverage_through_date"),
            qualified_committee: flag(data, "qualified_committee"),
            treasurer: treasurer(data),
            date_signed: date(data, "date_signed"),
            summary: Form3XSummary::from_data(data),
            detailed_summary: Form3XDetailedSummary::from_data(data),
        })
    }

    /// True for an amended report (`F3XA`); see [`Form3X::form_type`].
    pub fn is_amendment(&self) -> bool {
        crate::covers::is_amendment_form_type(&self.form_type)
    }

    /// Description of [`Form3X::report_code`], e.g. `Q1` → "April Quarterly".
    ///
    /// Uses [`crate::report_code_label`] (which includes `MYE`, the workbook's
    /// code for a monthly filer's year-end report, workbook field 10). `None`
    /// for an unknown or missing code.
    pub fn report_code_label(&self) -> Option<&'static str> {
        self.report_code
            .as_deref()
            .map(crate::report_code_label)
            .filter(|label| *label != "[Unknown report code]")
    }

    /// The election type named by the first letter of
    /// [`Form3X::election_code`]; see [`crate::covers::election_code_label`].
    /// The Line 4(c)/(d) checkboxes print the same letters in their report
    /// codes (12P, 12G, 12R, 12C, 12S, 30G, 30R, 30S;
    /// [fecfrm3x.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=1)).
    pub fn election_code_label(&self) -> Option<&'static str> {
        self.election_code
            .as_deref()
            .and_then(crate::covers::election_code_label)
    }
}

/// The treasurer's name: the five v6+ `treasurer_*` columns, or the v1–v5
/// caret-delimited `treasurer_name` ("Lastname^Firstname^Prefix^Suffix").
fn treasurer(data: &Data) -> PersonName {
    person_name_or_legacy(data, "treasurer_", "treasurer_name")
}

/// Amount from the first of `keys` that the record's layout has (blank reads
/// as `0.0`). Used where older layouts name a column differently.
fn amount_first(data: &Data, keys: &[&str]) -> f64 {
    keys.iter()
        .find(|k| data.contains_key(**k))
        .map(|k| amount(data, k))
        .unwrap_or(0.0)
}

/// A two-column row: `col_a_{name}` / `col_b_{name}`.
fn row(data: &Data, name: &str) -> DetailedSummaryRow {
    DetailedSummaryRow::from_data(data, &format!("col_a_{name}"), &format!("col_b_{name}"))
}

/// A two-column row for a Detailed Summary line whose v6+ column is
/// `col_{a,b}_{name}_TODO_DUP` but whose older layouts reuse
/// `col_{a,b}_{name}` (the Summary Page line it is carried to).
fn row_dup(data: &Data, name: &str) -> DetailedSummaryRow {
    let a = format!("col_a_{name}");
    let b = format!("col_b_{name}");
    DetailedSummaryRow {
        column_a: amount_first(data, &[&format!("{a}_TODO_DUP"), &a]),
        column_b: amount_first(data, &[&format!("{b}_TODO_DUP"), &b]),
    }
}

/// Page 2, the Summary Page (Lines 6–10)
/// ([fecfrm3x.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=2)).
/// Column A is This Period, Column B Calendar Year-to-Date; see [`Form3X`].
/// Filers complete the Detailed Summary first and carry its totals here
/// ([fecfrm3xi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=4)).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, Serialize)]
pub struct Form3XSummary {
    /// Line 6(a), "Cash on Hand January 1, YYYY": cash on hand at the
    /// beginning of the calendar year. **Column B only**
    /// (`col_b_cash_on_hand_jan_1`, workbook field 74;
    /// [fecfrm3xi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=4)).
    pub line6a_cash_on_hand_jan_1: f64,
    /// The year printed after "January 1," on Line 6(a) (`col_b_year`,
    /// workbook field 75, "Year for Above";
    /// [fecfrm3x.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=2)).
    pub line6a_year: Option<i16>,
    /// Line 6(b), "Cash on Hand at Beginning of Reporting Period". **Column A
    /// only** (`col_a_cash_on_hand_beginning_period`, workbook field 23;
    /// [fecfrm3xi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=4)).
    pub line6b_cash_on_hand_beginning_period: f64,
    /// Line 6(c), "Total Receipts (from Line 19)" (`col_a_total_receipts`,
    /// `col_b_total_receipts`; workbook fields 24, 76, rule "= 19";
    /// [fecfrm3xi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=4)).
    pub line6c_total_receipts: DetailedSummaryRow,
    /// Line 6(d), "Subtotal": 6(b) + 6(c) in Column A, 6(a) + 6(c) in Column B
    /// (`col_a_subtotal`, `col_b_subtotal`; workbook fields 25, 77;
    /// [fecfrm3xi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=4)).
    pub line6d_subtotal: DetailedSummaryRow,
    /// Line 7, "Total Disbursements (from Line 31)"
    /// (`col_a_total_disbursements`, `col_b_total_disbursements`; workbook
    /// fields 26, 78; [fecfrm3xi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=4)).
    pub line7_total_disbursements: DetailedSummaryRow,
    /// Line 8, "Cash on Hand at Close of Reporting Period" = 6(d) − 7, which
    /// "should be the same for both columns"
    /// (`col_a_cash_on_hand_close_of_period`,
    /// `col_b_cash_on_hand_close_of_period`; workbook fields 27, 79;
    /// [fecfrm3xi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=4)).
    pub line8_cash_on_hand_close_of_period: DetailedSummaryRow,
    /// Line 9, "Debts and Obligations Owed TO the Committee", the total from
    /// Schedule C and/or D. **Column A only** (`col_a_debts_to`, workbook
    /// field 28; [fecfrm3xi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=5)).
    pub line9_debts_owed_to_committee: f64,
    /// Line 10, "Debts and Obligations Owed BY the Committee", the total from
    /// Schedule C and/or D. **Column A only** (`col_a_debts_by`, workbook
    /// field 29; [fecfrm3xi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=5)).
    pub line10_debts_owed_by_committee: f64,
}

impl Form3XSummary {
    pub fn from_data(data: &Data) -> Self {
        Self {
            line6a_cash_on_hand_jan_1: amount(data, "col_b_cash_on_hand_jan_1"),
            line6a_year: text(data, "col_b_year").and_then(|y| y.parse().ok()),
            line6b_cash_on_hand_beginning_period: amount(
                data,
                "col_a_cash_on_hand_beginning_period",
            ),
            line6c_total_receipts: row(data, "total_receipts"),
            line6d_subtotal: row(data, "subtotal"),
            line7_total_disbursements: row(data, "total_disbursements"),
            line8_cash_on_hand_close_of_period: row(data, "cash_on_hand_close_of_period"),
            line9_debts_owed_to_committee: amount(data, "col_a_debts_to"),
            line10_debts_owed_by_committee: amount(data, "col_a_debts_by"),
        }
    }
}

/// Pages 3–5, the Detailed Summary Page
/// ([fecfrm3x.pdf p3–5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=3)),
/// in the form's three sections. Column A is This Period, Column B Calendar
/// Year-to-Date; see [`Form3X`].
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, Serialize)]
pub struct Form3XDetailedSummary {
    /// Section I, Receipts (Lines 11–20).
    pub receipts: Form3XReceipts,
    /// Section II, Disbursements (Lines 21–32).
    pub disbursements: Form3XDisbursements,
    /// Section III, Net Contributions/Operating Expenditures (Lines 33–38).
    pub net: Form3XNetContributionsAndOperatingExpenditures,
}

impl Form3XDetailedSummary {
    pub fn from_data(data: &Data) -> Self {
        Self {
            receipts: Form3XReceipts::from_data(data),
            disbursements: Form3XDisbursements::from_data(data),
            net: Form3XNetContributionsAndOperatingExpenditures::from_data(data),
        }
    }
}

/// Detailed Summary Page, Section I — Receipts, Lines 11–20
/// ([fecfrm3x.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=3);
/// line instructions [fecfrm3xi.pdf p6–7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=6)).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, Serialize)]
pub struct Form3XReceipts {
    /// Line 11(a)(i), contributions from individuals/persons other than
    /// political committees, itemized on Schedule A (persons aggregating over
    /// $200 in the calendar year) (`col_a_individuals_itemized`,
    /// `col_b_individuals_itemized`; workbook fields 30, 80;
    /// [fecfrm3xi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=6)).
    pub line11a_i_individuals_itemized: DetailedSummaryRow,
    /// Line 11(a)(ii), contributions from individuals/persons not required to
    /// be itemized (`col_a_individuals_unitemized`,
    /// `col_b_individuals_unitemized`; workbook fields 31, 81;
    /// [fecfrm3xi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=6)).
    pub line11a_ii_individuals_unitemized: DetailedSummaryRow,
    /// Line 11(a)(iii), total from individuals/persons = 11(a)(i) + (ii)
    /// (`col_a_individual_contribution_total`,
    /// `col_b_individual_contribution_total`; workbook fields 32, 82;
    /// [fecfrm3xi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=6)).
    pub line11a_iii_individuals_total: DetailedSummaryRow,
    /// Line 11(b), contributions from political party committees — for
    /// committees other than party committees, which use Line 12
    /// (`col_a_political_party_committees`,
    /// `col_b_political_party_committees`; workbook fields 33, 83;
    /// [fecfrm3xi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=6)).
    pub line11b_political_party_committees: DetailedSummaryRow,
    /// Line 11(c), contributions from other political committees such as PACs
    /// (`col_a_other_political_committees_pacs`,
    /// `col_b_other_political_committees_pacs`; workbook fields 34, 84;
    /// [fecfrm3xi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=6)).
    pub line11c_other_political_committees: DetailedSummaryRow,
    /// Line 11(d), total contributions = 11(a)(iii) + 11(b) + 11(c)
    /// (`col_a_total_contributions`, `col_b_total_contributions`; workbook
    /// fields 35, 85; [fecfrm3xi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=6)).
    pub line11d_total_contributions: DetailedSummaryRow,
    /// Line 12, transfers from affiliated/other party committees, including
    /// loans and loan repayments from them
    /// (`col_a_transfers_from_aff_other_party_cmttees`,
    /// `col_b_transfers_from_aff_other_party_cmttees`; workbook fields 36, 86;
    /// [fecfrm3xi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=6)).
    pub line12_transfers_from_affiliated: DetailedSummaryRow,
    /// Line 13, all loans received, other than from affiliated/party
    /// committees (`col_a_total_loans`, `col_b_total_loans`; workbook fields
    /// 37, 87; [fecfrm3xi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=6)).
    pub line13_loans_received: DetailedSummaryRow,
    /// Line 14, loan repayments received
    /// (`col_a_total_loan_repayments_received`,
    /// `col_b_total_loan_repayments_received`; workbook fields 38, 88;
    /// [fecfrm3xi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=6)).
    pub line14_loan_repayments_received: DetailedSummaryRow,
    /// Line 15, offsets to operating expenditures (refunds, rebates, returns
    /// of deposits) (`col_a_offsets_to_expenditures`,
    /// `col_b_offsets_to_expenditures`; workbook fields 39, 89;
    /// [fecfrm3xi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=6)).
    pub line15_offsets_to_operating_expenditures: DetailedSummaryRow,
    /// Line 16, refunds of contributions the committee made to federal
    /// candidates and other political committees (`col_a_federal_refunds`,
    /// `col_b_federal_refunds`; workbook fields 40, 90;
    /// [fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
    pub line16_refunds_of_federal_contributions: DetailedSummaryRow,
    /// Line 17, other federal receipts (dividends, interest, etc.)
    /// (`col_a_other_federal_receipts`, `col_b_other_federal_receipts`;
    /// workbook fields 41, 91;
    /// [fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
    pub line17_other_federal_receipts: DetailedSummaryRow,
    /// Line 18(a), transfers from the nonfederal account for allocated
    /// federal/nonfederal activity, from Schedule H3
    /// (`col_a_transfers_from_nonfederal_h3`,
    /// `col_b_transfers_from_nonfederal_h3`; workbook fields 42, 92;
    /// [fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
    pub line18a_transfers_from_nonfederal_account: DetailedSummaryRow,
    /// Line 18(b), transfers from Levin funds for allocated federal election
    /// activity, from Schedule H5 (`col_a_levin_funds`, `col_b_levin_funds`;
    /// workbook fields 43, 93;
    /// [fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
    /// Not in v1–v3 layouts.
    pub line18b_transfers_from_levin_funds: DetailedSummaryRow,
    /// Line 18(c), total nonfederal transfers = 18(a) + 18(b)
    /// (`col_a_total_nonfederal_transfers`, `col_b_total_nonfederal_transfers`;
    /// workbook fields 44, 94;
    /// [fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
    /// Not in v1–v3 layouts.
    pub line18c_total_nonfederal_transfers: DetailedSummaryRow,
    /// Line 19, total receipts = 11(d) + 12 + 13 + 14 + 15 + 16 + 17 + 18(c);
    /// carried to Line 6(c) (`col_a_total_receipts_TODO_DUP`,
    /// `col_b_total_receipts_TODO_DUP`, falling back to `col_*_total_receipts`
    /// in older layouts; workbook fields 45, 95;
    /// [fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
    /// Includes nonfederal transfers in (18(c)).
    pub line19_total_receipts: DetailedSummaryRow,
    /// Line 20, total federal receipts = 19 − 18(c)
    /// (`col_a_total_federal_receipts`, `col_b_total_federal_receipts`;
    /// workbook fields 46, 96;
    /// [fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
    pub line20_total_federal_receipts: DetailedSummaryRow,
}

impl Form3XReceipts {
    pub fn from_data(data: &Data) -> Self {
        Self {
            line11a_i_individuals_itemized: row(data, "individuals_itemized"),
            line11a_ii_individuals_unitemized: row(data, "individuals_unitemized"),
            line11a_iii_individuals_total: row(data, "individual_contribution_total"),
            line11b_political_party_committees: row(data, "political_party_committees"),
            line11c_other_political_committees: row(data, "other_political_committees_pacs"),
            line11d_total_contributions: row(data, "total_contributions"),
            line12_transfers_from_affiliated: row(data, "transfers_from_aff_other_party_cmttees"),
            line13_loans_received: row(data, "total_loans"),
            line14_loan_repayments_received: row(data, "total_loan_repayments_received"),
            line15_offsets_to_operating_expenditures: row(data, "offsets_to_expenditures"),
            line16_refunds_of_federal_contributions: row(data, "federal_refunds"),
            line17_other_federal_receipts: row(data, "other_federal_receipts"),
            line18a_transfers_from_nonfederal_account: row(data, "transfers_from_nonfederal_h3"),
            line18b_transfers_from_levin_funds: row(data, "levin_funds"),
            line18c_total_nonfederal_transfers: row(data, "total_nonfederal_transfers"),
            line19_total_receipts: row_dup(data, "total_receipts"),
            line20_total_federal_receipts: row(data, "total_federal_receipts"),
        }
    }
}

/// Detailed Summary Page, Section II — Disbursements, Lines 21–32
/// ([fecfrm3x.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=4);
/// line instructions [fecfrm3xi.pdf p7–9](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, Serialize)]
pub struct Form3XDisbursements {
    /// Line 21(a)(i), shared federal/nonfederal operating expenditures,
    /// federal share, from Schedule H4
    /// (`col_a_shared_operating_expenditures_federal`,
    /// `col_b_shared_operating_expenditures_federal`; workbook fields 47, 97;
    /// [fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
    /// Completed only by committees with allocated activity.
    pub line21a_i_shared_operating_federal_share: DetailedSummaryRow,
    /// Line 21(a)(ii), shared operating expenditures, nonfederal share, from
    /// Schedule H4 (`col_a_shared_operating_expenditures_nonfederal`,
    /// `col_b_shared_operating_expenditures_nonfederal`; workbook fields 48, 98;
    /// [fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
    pub line21a_ii_shared_operating_nonfederal_share: DetailedSummaryRow,
    /// Line 21(b), other federal operating expenditures, itemized and
    /// unitemized (`col_a_other_federal_operating_expenditures`,
    /// `col_b_other_federal_operating_expenditures`; workbook fields 49, 99;
    /// [fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
    pub line21b_other_federal_operating_expenditures: DetailedSummaryRow,
    /// Line 21(c), total operating expenditures = 21(a)(i) + 21(a)(ii) +
    /// 21(b) (`col_a_total_operating_expenditures`,
    /// `col_b_total_operating_expenditures`; workbook fields 50, 100;
    /// [fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
    pub line21c_total_operating_expenditures: DetailedSummaryRow,
    /// Line 22, transfers to affiliated/other party committees, including
    /// loans and loan repayments made to them
    /// (`col_a_transfers_to_affiliated`, `col_b_transfers_to_affiliated`;
    /// workbook fields 51, 101;
    /// [fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
    pub line22_transfers_to_affiliated: DetailedSummaryRow,
    /// Line 23, contributions to federal candidates/committees and other
    /// political committees, including in-kind
    /// (`col_a_contributions_to_candidates`,
    /// `col_b_contributions_to_candidates`; workbook fields 52, 102;
    /// [fecfrm3xi.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=8)).
    pub line23_contributions_to_federal_candidates: DetailedSummaryRow,
    /// Line 24, independent expenditures, from Schedule E
    /// (`col_a_independent_expenditures`, `col_b_independent_expenditures`;
    /// workbook fields 53, 103;
    /// [fecfrm3xi.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=8)).
    pub line24_independent_expenditures: DetailedSummaryRow,
    /// Line 25, coordinated party expenditures under 52 U.S.C. § 30116(d),
    /// from Schedule F; party committees only
    /// (`col_a_coordinated_expenditures_by_party_committees`,
    /// `col_b_coordinated_expenditures_by_party_committees`; workbook fields
    /// 54, 104; [fecfrm3xi.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=8)).
    pub line25_coordinated_party_expenditures: DetailedSummaryRow,
    /// Line 26, loan repayments made (`col_a_total_loan_repayments_made`,
    /// `col_b_total_loan_repayments_made`; workbook fields 55, 105;
    /// [fecfrm3xi.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=8)).
    pub line26_loan_repayments_made: DetailedSummaryRow,
    /// Line 27, loans made, excluding transfers on Line 22
    /// (`col_a_loans_made`, `col_b_loans_made`; workbook fields 56, 106;
    /// [fecfrm3xi.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=8)).
    pub line27_loans_made: DetailedSummaryRow,
    /// Line 28(a), contribution refunds to individuals/persons other than
    /// political committees (`col_a_refunds_to_individuals`,
    /// `col_b_refunds_to_individuals`; workbook fields 57, 107;
    /// [fecfrm3xi.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=8)).
    pub line28a_refunds_to_individuals: DetailedSummaryRow,
    /// Line 28(b), contribution refunds to political party committees
    /// (`col_a_refunds_to_party_committees`,
    /// `col_b_refunds_to_party_committees`; workbook fields 58, 108;
    /// [fecfrm3xi.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=8)).
    pub line28b_refunds_to_party_committees: DetailedSummaryRow,
    /// Line 28(c), contribution refunds to other political committees
    /// (`col_a_refunds_to_other_committees`,
    /// `col_b_refunds_to_other_committees`; workbook fields 59, 109;
    /// [fecfrm3xi.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=8)).
    pub line28c_refunds_to_other_committees: DetailedSummaryRow,
    /// Line 28(d), total contribution refunds = 28(a) + 28(b) + 28(c)
    /// (`col_a_total_refunds`, `col_b_total_refunds`; workbook fields 60, 110;
    /// [fecfrm3xi.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=8)).
    pub line28d_total_contribution_refunds: DetailedSummaryRow,
    /// Line 29, other disbursements, including donations to nonfederal
    /// candidates (`col_a_other_disbursements`, `col_b_other_disbursements`;
    /// workbook fields 61, 111;
    /// [fecfrm3xi.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=8)).
    pub line29_other_disbursements: DetailedSummaryRow,
    /// Line 30(a)(i), shared federal election activity, federal share, from
    /// Schedule H6 (`col_a_federal_election_activity_federal_share`,
    /// `col_b_federal_election_activity_federal_share`; workbook fields 62,
    /// 112; [fecfrm3x.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=4)).
    /// Only State, district and local party committees use Line 30
    /// ([fecfrm3xi.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=8)).
    /// Not in v1–v3 layouts.
    pub line30a_i_fea_federal_share: DetailedSummaryRow,
    /// Line 30(a)(ii), shared federal election activity, "Levin" share, from
    /// Schedule H6 (`col_a_federal_election_activity_levin_share`,
    /// `col_b_federal_election_activity_levin_share`; workbook fields 63, 113;
    /// [fecfrm3x.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=4)).
    pub line30a_ii_fea_levin_share: DetailedSummaryRow,
    /// Line 30(b), federal election activity paid entirely with federal funds
    /// (`col_a_federal_election_activity_all_federal`,
    /// `col_b_federal_election_activity_all_federal`; workbook fields 64, 114;
    /// [fecfrm3xi.pdf p9](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=9)).
    pub line30b_fea_all_federal: DetailedSummaryRow,
    /// Line 30(c), total federal election activity = 30(a)(i) + 30(a)(ii) +
    /// 30(b) (`col_a_federal_election_activity_total`,
    /// `col_b_federal_election_activity_total`; workbook fields 65, 115;
    /// [fecfrm3xi.pdf p9](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=9)).
    pub line30c_fea_total: DetailedSummaryRow,
    /// Line 31, total disbursements = 21(c) + 22 + 23 + 24 + 25 + 26 + 27 +
    /// 28(d) + 29 + 30(c); carried to Line 7
    /// (`col_a_total_disbursements_TODO_DUP`,
    /// `col_b_total_disbursements_TODO_DUP`, falling back to
    /// `col_*_total_disbursements` in older layouts; workbook fields 66, 116;
    /// [fecfrm3xi.pdf p9](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=9)).
    /// Includes the nonfederal and Levin shares.
    pub line31_total_disbursements: DetailedSummaryRow,
    /// Line 32, total federal disbursements = 31 − 21(a)(ii) − 30(a)(ii)
    /// (`col_a_total_federal_disbursements`,
    /// `col_b_total_federal_disbursements`; workbook fields 67, 117;
    /// [fecfrm3xi.pdf p9](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=9)).
    pub line32_total_federal_disbursements: DetailedSummaryRow,
}

impl Form3XDisbursements {
    pub fn from_data(data: &Data) -> Self {
        Self {
            line21a_i_shared_operating_federal_share: row(
                data,
                "shared_operating_expenditures_federal",
            ),
            line21a_ii_shared_operating_nonfederal_share: row(
                data,
                "shared_operating_expenditures_nonfederal",
            ),
            line21b_other_federal_operating_expenditures: row(
                data,
                "other_federal_operating_expenditures",
            ),
            line21c_total_operating_expenditures: row(data, "total_operating_expenditures"),
            line22_transfers_to_affiliated: row(data, "transfers_to_affiliated"),
            line23_contributions_to_federal_candidates: row(data, "contributions_to_candidates"),
            line24_independent_expenditures: row(data, "independent_expenditures"),
            line25_coordinated_party_expenditures: row(
                data,
                "coordinated_expenditures_by_party_committees",
            ),
            line26_loan_repayments_made: row(data, "total_loan_repayments_made"),
            line27_loans_made: row(data, "loans_made"),
            line28a_refunds_to_individuals: row(data, "refunds_to_individuals"),
            line28b_refunds_to_party_committees: row(data, "refunds_to_party_committees"),
            line28c_refunds_to_other_committees: row(data, "refunds_to_other_committees"),
            line28d_total_contribution_refunds: row(data, "total_refunds"),
            line29_other_disbursements: row(data, "other_disbursements"),
            line30a_i_fea_federal_share: row(data, "federal_election_activity_federal_share"),
            line30a_ii_fea_levin_share: row(data, "federal_election_activity_levin_share"),
            line30b_fea_all_federal: row(data, "federal_election_activity_all_federal"),
            line30c_fea_total: row(data, "federal_election_activity_total"),
            line31_total_disbursements: row_dup(data, "total_disbursements"),
            line32_total_federal_disbursements: row(data, "total_federal_disbursements"),
        }
    }
}

/// Detailed Summary Page, Section III — Net Contributions/Operating
/// Expenditures, Lines 33–38: earlier lines repeated to compute two net
/// figures ([fecfrm3x.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=5)).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, Serialize)]
pub struct Form3XNetContributionsAndOperatingExpenditures {
    /// Line 33, total contributions (other than loans), from Line 11(d)
    /// (`col_a_total_contributions_TODO_DUP`,
    /// `col_b_total_contributions_TODO_DUP`, falling back to
    /// `col_*_total_contributions` in older layouts; workbook fields 68, 118).
    pub line33_total_contributions: DetailedSummaryRow,
    /// Line 34, total contribution refunds, from Line 28(d)
    /// (`col_a_total_contributions_refunds`,
    /// `col_b_total_contributions_refunds`; workbook fields 69, 119).
    pub line34_total_contribution_refunds: DetailedSummaryRow,
    /// Line 35, net contributions (other than loans) = 33 − 34
    /// (`col_a_net_contributions`, `col_b_net_contributions`; workbook fields
    /// 70, 120).
    pub line35_net_contributions: DetailedSummaryRow,
    /// Line 36, total federal operating expenditures = 21(a)(i) + 21(b) —
    /// excludes the nonfederal share
    /// (`col_a_total_federal_operating_expenditures`,
    /// `col_b_total_federal_operating_expenditures`; workbook fields 71, 121).
    pub line36_total_federal_operating_expenditures: DetailedSummaryRow,
    /// Line 37, offsets to operating expenditures, from Line 15
    /// (`col_a_total_offsets_to_expenditures`,
    /// `col_b_total_offsets_to_expenditures`; workbook fields 72, 122).
    pub line37_offsets_to_operating_expenditures: DetailedSummaryRow,
    /// Line 38, net operating expenditures = 36 − 37
    /// (`col_a_net_operating_expenditures`, `col_b_net_operating_expenditures`;
    /// workbook fields 73, 123).
    pub line38_net_operating_expenditures: DetailedSummaryRow,
}

impl Form3XNetContributionsAndOperatingExpenditures {
    pub fn from_data(data: &Data) -> Self {
        Self {
            line33_total_contributions: row_dup(data, "total_contributions"),
            line34_total_contribution_refunds: row(data, "total_contributions_refunds"),
            line35_net_contributions: row(data, "net_contributions"),
            line36_total_federal_operating_expenditures: row(
                data,
                "total_federal_operating_expenditures",
            ),
            line37_offsets_to_operating_expenditures: row(data, "total_offsets_to_expenditures"),
            line38_net_operating_expenditures: row(data, "net_operating_expenditures"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn data(pairs: &[(&str, &str)]) -> Data {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn legacy_treasurer_name() {
        let d = data(&[
            ("filer_committee_id_number", "C00000000"),
            ("treasurer_name", "Smith^John W.^Dr.^Jr."),
        ]);
        let f = Form3X::from_data(&d).unwrap();
        assert_eq!(f.treasurer.last_name, "Smith");
        assert_eq!(f.treasurer.first_name, "John W.");
        assert_eq!(f.treasurer.prefix.as_deref(), Some("Dr."));
        assert_eq!(f.treasurer.suffix.as_deref(), Some("Jr."));
        assert_eq!(f.treasurer.to_string(), "Dr. John W. Smith Jr.");
    }

    #[test]
    fn dup_fallback() {
        // v8: separate _TODO_DUP column wins.
        let d = data(&[
            ("filer_committee_id_number", "C00000000"),
            ("col_a_total_receipts", "1.00"),
            ("col_a_total_receipts_TODO_DUP", "2.00"),
        ]);
        let f = Form3X::from_data(&d).unwrap();
        assert_eq!(f.summary.line6c_total_receipts.column_a, 1.0);
        assert_eq!(
            f.detailed_summary.receipts.line19_total_receipts.column_a,
            2.0
        );
        // legacy: only the shared name.
        let d = data(&[
            ("filer_committee_id_number", "C00000000"),
            ("col_a_total_receipts", "3.00"),
        ]);
        let f = Form3X::from_data(&d).unwrap();
        assert_eq!(
            f.detailed_summary.receipts.line19_total_receipts.column_a,
            3.0
        );
    }

    #[test]
    fn labels() {
        let mut d = data(&[
            ("filer_committee_id_number", "C00000000"),
            ("report_code", "MYE"),
            ("election_code", "G2024"),
        ]);
        let f = Form3X::from_data(&d).unwrap();
        assert_eq!(f.report_code_label(), Some("Monthly Year-End"));
        assert_eq!(f.election_code_label(), Some("General"));
        d.insert("report_code".into(), "Q1".into());
        assert_eq!(
            Form3X::from_data(&d).unwrap().report_code_label(),
            Some("April Quarterly")
        );
    }
}
