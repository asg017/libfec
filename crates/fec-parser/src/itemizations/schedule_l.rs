//! Schedule L: the Levin funds aggregation page.

use jiff::civil::Date;

use crate::covers::fields::{amount, date, text, text_or_empty, Fields};
use crate::covers::DetailedSummaryRow;
use crate::itemizations::text_any;

/// "SCHEDULE L (FEC Form 3X) AGGREGATION PAGE: LEVIN FUNDS": a summary of
/// one account's Levin-fund receipts and disbursements, for the period and
/// the calendar year to date
/// ([fecfrm3x.pdf p19](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=19),
/// [fecfrm3xi.pdf p33](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=33)).
/// Filed with Form 3X by state, district and local party committees, one per
/// account that handles Levin funds, so a filing may hold several
/// ([fecfrm3xi.pdf p33](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=33)).
///
/// Unlike the other itemizations this is a summary record, not a
/// transaction: its lines are totals of the memo Schedules L-A (receipts,
/// Lines 1–2) and L-B (disbursements, Lines 4–5) (fecfrm3xi.pdf p33). "The L
/// Schedules are memo schedules and do not affect totals on the Summary and
/// Detailed Summary Pages"
/// ([partygui.pdf p159](https://www.fec.gov/resources/cms-content/documents/policy-guidance/partygui.pdf#page=159)).
/// Schedule A and B rows tie themselves to the account through their
/// `reference_code`, which "must contain a valid system code used in a
/// Schedule I or L" (FEC format workbook v8.4, sheet `Sch A` field 45, sheet
/// `Sch B` field 44); see [`ScheduleL::record_id`].
///
/// # Columns A and B
///
/// Every line is a [`DetailedSummaryRow`] with `column_a` = "Total This
/// Period" and `column_b` = "Calendar Year-to-Date"; blank reads as `0.0`, as
/// on covers. Column B's Line 7 is cash on hand as of January 1, not at the
/// start of the period
/// ([fecfrm3xi.pdf p33](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=33)).
///
/// # Versions
///
/// v6.1–8.5 have all 41 fields; Column A's Lines 10 and 11 (fields 23–24)
/// are named `col_b_disbursements_period` and
/// `col_b_cash_on_hand_close_of_period`, and Column B's (fields 40–41) carry
/// a `_TODO_DUP` suffix (FEC format workbook v8.4, sheet `Sch L`, fields
/// 23–24, 40–41); this struct reads them into the right column. v5.x puts
/// `account_name` before `record_id_number`, its `transaction_id_number` last,
/// and its layout in `mappings2.json` has no Column A Line 11, which then
/// reads `0.0` (no v5.x Schedule L was seen to check against). Paper layouts
/// have no transaction or record ID or coverage dates but carry an
/// `image_number`. Only v8.4 and v8.5 Schedule L rows were seen in the
/// corpus.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        module = "libfec_parser.itemizations",
        frozen,
        get_all,
        skip_from_py_object
    )
)]
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ScheduleL {
    /// The row type as filed, `SL`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `Sch L`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this record, unique for the life of the report;
    /// electronic-only. Column `transaction_id_number` (field 3).
    pub transaction_id: Option<String>,
    /// "RECORD ID NUMBER (for account name)": the system code of the account,
    /// unique for the life of the report, which Schedule A and B rows give
    /// as their `reference_code` (fields 4; sheet `Sch A` field 45, sheet
    /// `Sch B` field 44). Electronic-only. Column `record_id_number`.
    pub record_id: Option<String>,
    /// "NAME OF ACCOUNT": the Levin or other nonfederal account summarized.
    /// Column `account_name` (field 5).
    pub account_name: Option<String>,
    /// Start of the period covered. Electronic-only. Column
    /// `coverage_from_date` (field 6).
    pub coverage_from_date: Option<Date>,
    /// End of the period covered. Column `coverage_through_date` (field 7).
    pub coverage_through_date: Option<Date>,
    /// Line 1(a), receipts from persons itemized on Schedule L-A. Columns
    /// `col_a_itemized_receipts_persons`, `col_b_itemized_receipts_persons`
    /// (fields 8, 25).
    pub line1a_itemized_receipts_from_persons: DetailedSummaryRow,
    /// Line 1(b), unitemized receipts from persons. Columns
    /// `col_a_unitemized_receipts_persons`, `col_b_…` (fields 9, 26).
    pub line1b_unitemized_receipts_from_persons: DetailedSummaryRow,
    /// Line 1(c), total receipts from persons, 1(a) + 1(b). Columns
    /// `col_a_total_receipts_persons`, `col_b_…` (fields 10, 27).
    pub line1c_total_receipts_from_persons: DetailedSummaryRow,
    /// Line 2, "the total of any other receipts disclosed in a memo Schedule
    /// L-A" (fecfrm3xi.pdf p33). Columns `col_a_other_receipts`, `col_b_…`
    /// (fields 11, 28).
    pub line2_other_receipts: DetailedSummaryRow,
    /// Line 3, total receipts, 1(c) + 2. Columns `col_a_total_receipts`,
    /// `col_b_…` (fields 12, 29).
    pub line3_total_receipts: DetailedSummaryRow,
    /// Line 4(a), voter registration. Line 4 is disbursements of Levin funds
    /// for federal election activity, by category, itemized on Schedule L-B
    /// (fecfrm3xi.pdf p33). Columns `col_a_voter_registration_disbursements`,
    /// `col_b_…` (fields 13, 30).
    pub line4a_voter_registration: DetailedSummaryRow,
    /// Line 4(b), voter ID. Columns `col_a_voter_id_disbursements`, `col_b_…`
    /// (fields 14, 31).
    pub line4b_voter_id: DetailedSummaryRow,
    /// Line 4(c), get-out-the-vote. Columns `col_a_gotv_disbursements`,
    /// `col_b_…` (fields 15, 32).
    pub line4c_gotv: DetailedSummaryRow,
    /// Line 4(d), generic campaign activity. Columns
    /// `col_a_generic_campaign_disbursements`, `col_b_…` (fields 16, 33).
    pub line4d_generic_campaign: DetailedSummaryRow,
    /// Line 4(e), total of Line 4. Columns `col_a_disbursements_subtotal`,
    /// `col_b_…` (fields 17, 34).
    pub line4e_total_federal_election_activity: DetailedSummaryRow,
    /// Line 5, "all other disbursements disclosed on a memo Schedule L-B"
    /// (fecfrm3xi.pdf p33). Columns `col_a_other_disbursements`, `col_b_…`
    /// (fields 18, 35).
    pub line5_other_disbursements: DetailedSummaryRow,
    /// Line 6, total disbursements, 4(e) + 5. Columns
    /// `col_a_total_disbursements`, `col_b_…` (fields 19, 36).
    pub line6_total_disbursements: DetailedSummaryRow,
    /// Line 7, beginning cash on hand: at the start of the period in Column
    /// A, as of January 1 in Column B (fecfrm3xi.pdf p33). Columns
    /// `col_a_cash_on_hand_beginning_period`, `col_b_…` (fields 20, 37).
    pub line7_beginning_cash_on_hand: DetailedSummaryRow,
    /// Line 8, receipts, from Line 3. Columns `col_a_receipts_period`,
    /// `col_b_receipts_period` (fields 21, 38).
    pub line8_receipts: DetailedSummaryRow,
    /// Line 9, subtotal, 7 + 8. Columns `col_a_subtotal_period`,
    /// `col_b_subtotal_period` (fields 22, 39).
    pub line9_subtotal: DetailedSummaryRow,
    /// Line 10, disbursements, from Line 6. Columns
    /// `col_b_disbursements_period` (Column A, field 23) and
    /// `col_b_disbursements_period_TODO_DUP` (Column B, field 40).
    pub line10_disbursements: DetailedSummaryRow,
    /// Line 11, Levin funds on hand at the close of the period, 9 − 10
    /// (fecfrm3xi.pdf p33). Columns `col_b_cash_on_hand_close_of_period`
    /// (Column A, field 24) and `col_b_cash_on_hand_close_of_period_TODO_DUP`
    /// (Column B, field 41).
    pub line11_ending_cash_on_hand: DetailedSummaryRow,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

/// Column A and Column B of one line.
fn row<F: Fields + ?Sized>(data: &F, column_a: &str, column_b: &str) -> DetailedSummaryRow {
    DetailedSummaryRow {
        column_a: amount(data, column_a),
        column_b: amount(data, column_b),
    }
}

impl ScheduleL {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text_any(data, &["transaction_id_number", "transaction_id"]),
            record_id: text(data, "record_id_number"),
            account_name: text(data, "account_name"),
            coverage_from_date: date(data, "coverage_from_date"),
            coverage_through_date: date(data, "coverage_through_date"),
            line1a_itemized_receipts_from_persons: row(
                data,
                "col_a_itemized_receipts_persons",
                "col_b_itemized_receipts_persons",
            ),
            line1b_unitemized_receipts_from_persons: row(
                data,
                "col_a_unitemized_receipts_persons",
                "col_b_unitemized_receipts_persons",
            ),
            line1c_total_receipts_from_persons: row(
                data,
                "col_a_total_receipts_persons",
                "col_b_total_receipts_persons",
            ),
            line2_other_receipts: row(data, "col_a_other_receipts", "col_b_other_receipts"),
            line3_total_receipts: row(data, "col_a_total_receipts", "col_b_total_receipts"),
            line4a_voter_registration: row(
                data,
                "col_a_voter_registration_disbursements",
                "col_b_voter_registration_disbursements",
            ),
            line4b_voter_id: row(
                data,
                "col_a_voter_id_disbursements",
                "col_b_voter_id_disbursements",
            ),
            line4c_gotv: row(data, "col_a_gotv_disbursements", "col_b_gotv_disbursements"),
            line4d_generic_campaign: row(
                data,
                "col_a_generic_campaign_disbursements",
                "col_b_generic_campaign_disbursements",
            ),
            line4e_total_federal_election_activity: row(
                data,
                "col_a_disbursements_subtotal",
                "col_b_disbursements_subtotal",
            ),
            line5_other_disbursements: row(
                data,
                "col_a_other_disbursements",
                "col_b_other_disbursements",
            ),
            line6_total_disbursements: row(
                data,
                "col_a_total_disbursements",
                "col_b_total_disbursements",
            ),
            line7_beginning_cash_on_hand: row(
                data,
                "col_a_cash_on_hand_beginning_period",
                "col_b_cash_on_hand_beginning_period",
            ),
            line8_receipts: row(data, "col_a_receipts_period", "col_b_receipts_period"),
            line9_subtotal: row(data, "col_a_subtotal_period", "col_b_subtotal_period"),
            // The workbook's names for Column A's Lines 10 and 11 say
            // `col_b`; Column B's are the `_TODO_DUP` copies (see # Versions).
            line10_disbursements: row(
                data,
                "col_b_disbursements_period",
                "col_b_disbursements_period_TODO_DUP",
            ),
            line11_ending_cash_on_hand: row(
                data,
                "col_b_cash_on_hand_close_of_period",
                "col_b_cash_on_hand_close_of_period_TODO_DUP",
            ),
            image_number: text(data, "image_number"),
        })
    }
}
