//! Form 3P: Report of Receipts and Disbursements by an Authorized Committee of
//! a Candidate for the Office of President or Vice President.
//!
//! See [`Form3P`].

use crate::covers::fields::{
    amount, amount_opt, date, flag, person_name_or_legacy, text, text_or_empty, Data,
};
use crate::covers::{Address, DetailedSummaryRow, PersonName};
use jiff::civil::Date;

/// FEC Form 3P, "Report of Receipts and Disbursements by an Authorized
/// Committee of a Candidate for the Office of President or Vice President".
///
/// **Who files.** Every political committee authorized in writing by a
/// candidate for President or Vice President, whether or not publicly funded
/// ([fecfrm3pi.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=3)).
///
/// **Layout.** Pages 1–2 are the Summary Page, pages 3–4 the Detailed Summary
/// Page, pages 5–7 the "Allocation of Primary Expenditures by State" and page 8
/// a worksheet for expenditures subject to limitation
/// ([fecfrm3pi.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=1)).
/// The `F3P` record carries all of it in one row (FEC format workbook v8.4,
/// sheet `F3P`, fields 1–206): page 1 is the top-level fields here, page 2
/// [`Form3P::summary`], pages 3–4 [`Form3P::detailed_summary`], pages 5–7
/// [`Form3P::state_allocations`].
///
/// **Columns.** On the Detailed Summary Page, Column A
/// ([`DetailedSummaryRow::column_a`]) is "Total This Period" and Column B
/// ([`DetailedSummaryRow::column_b`]) is "Election Cycle-to-Date"
/// ([fecfrm3p.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=3),
/// [p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=4)).
/// Column B is the previous report's cycle-to-date figure plus this report's
/// Column A ([fecfrm3pi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=5)).
/// Summary Lines 6–15 and Line 31 are single amounts.
///
/// **Validation.** Over all 197 v8.x filings in the local corpus, every
/// arithmetic identity on the form (e.g. Line 7 = Line 22 Column A, Line 14 =
/// 17(e) Column B − 28(d) Column B, the state allocation totals) holds on 100%
/// of records with this column mapping; see `wiki/covers/F3P.md`.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct Form3P {
    /// Form type as filed, e.g. `F3PN`: the base form plus the
    /// amendment-indicator suffix (see [`crate::covers::base_form_type`]).
    /// Column `form_type` (FEC format workbook v8.4, sheet `F3P`, field 1).
    /// The suffix is `N` (new), `A` (amended; Line 4 "Is this report an
    /// amendment?") or `T` (the "Termination Report (TER)" box beside Line 3)
    /// ([fecfrm3p.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=1)).
    pub form_type: String,
    /// Line 2, the committee's FEC identification number
    /// (`filer_committee_id_number`;
    /// [fecfrm3p.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=1)).
    pub filer_committee_id: String,
    /// Line 1, the committee's full name (`committee_name`;
    /// [fecfrm3p.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=1)).
    pub committee_name: String,
    /// Line 1, the committee's mailing address (`street_1`, `street_2`, `city`,
    /// `state`, `zip_code`;
    /// [fecfrm3pi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=4)).
    pub address: Address,
    /// The "Check if different than previously reported (ACC)" box beside the
    /// address (`change_of_address`, `X` = yes;
    /// [fecfrm3p.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=1)).
    pub change_of_address: bool,
    /// "Activity Primary" checkbox (`activity_primary`, `X` = yes; FEC format
    /// workbook v8.4, sheet `F3P`, field 10). Electronic-only: no such box is
    /// printed on the paper form, and the FEC sources do not define it.
    pub activity_primary: bool,
    /// "Activity General" checkbox (`activity_general`, `X` = yes; FEC format
    /// workbook v8.4, sheet `F3P`, field 11). Electronic-only and undefined,
    /// like [`Form3P::activity_primary`].
    pub activity_general: bool,
    /// Line 3, type of report: quarterly (`Q1`–`Q3`, `YE`), monthly (`M2`–`M12`,
    /// `MYE` for a monthly filer's year-end report), pre-/post-election or
    /// `TER` (`report_code`;
    /// [fecfrm3p.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=1);
    /// FEC format workbook v8.4, sheet `F3P`, field 12). See
    /// [`Form3P::report_code_label`].
    pub report_code: Option<String>,
    /// Election type and year for a pre-/post-election report, e.g. `G2024`
    /// (`election_code`; FEC format workbook v8.4, sheet `F3P`, field 13).
    /// Electronic-only: page 1 prints no election-type box. See
    /// [`Form3P::election_code_label`].
    pub election_code: Option<String>,
    /// Line 3, "12-Day Pre-Election Report for the Election on" / "30-Day
    /// Post-Election Report for the General Election on": the election date
    /// (`date_of_election`;
    /// [fecfrm3p.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=1)).
    pub election_date: Option<Date>,
    /// Line 3, "in the State of" for a 12-day pre-election report
    /// (`state_of_election`;
    /// [fecfrm3p.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=1)).
    pub state_of_election: Option<String>,
    /// Line 5, first day of the covering period (`coverage_from_date`;
    /// [fecfrm3p.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=1)).
    pub coverage_from_date: Option<Date>,
    /// Line 5, last day of the covering period (`coverage_through_date`;
    /// [fecfrm3p.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=1)).
    pub coverage_through_date: Option<Date>,
    /// "Type or Print Name of Treasurer" (`treasurer_*`; the single
    /// caret-delimited `treasurer_name` of v1–v5.x formats is split into parts;
    /// [fecfrm3p.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=1)).
    pub treasurer: PersonName,
    /// Date the treasurer signed the certification (`date_signed`;
    /// [fecfrm3p.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=1)).
    pub date_signed: Option<Date>,
    /// Page 2, the Summary Page (Lines 6–15).
    pub summary: Form3PSummary,
    /// Pages 3–4, the Detailed Summary Page (Lines 16–31).
    pub detailed_summary: Form3PDetailedSummary,
    /// Pages 5–7, allocation of primary expenditures by state.
    pub state_allocations: Form3PStateAllocations,
}

impl Form3P {
    pub fn from_data(data: &Data) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            committee_name: text_or_empty(data, "committee_name"),
            address: Address::from_prefixed(data, ""),
            change_of_address: flag(data, "change_of_address"),
            activity_primary: flag(data, "activity_primary"),
            activity_general: flag(data, "activity_general"),
            report_code: text(data, "report_code"),
            election_code: text(data, "election_code"),
            election_date: date(data, "date_of_election"),
            state_of_election: text(data, "state_of_election"),
            coverage_from_date: date(data, "coverage_from_date"),
            coverage_through_date: date(data, "coverage_through_date"),
            treasurer: person_name_or_legacy(data, "treasurer_", "treasurer_name"),
            date_signed: date(data, "date_signed"),
            summary: Form3PSummary::from_data(data),
            detailed_summary: Form3PDetailedSummary::from_data(data),
            state_allocations: Form3PStateAllocations::from_data(data),
        })
    }

    /// True for an amended report (`F3PA`); see [`Form3P::form_type`].
    pub fn is_amendment(&self) -> bool {
        crate::covers::is_amendment_form_type(&self.form_type)
    }

    /// Description of [`Form3P::report_code`] (see [`crate::report_code_label`]).
    pub fn report_code_label(&self) -> Option<&'static str> {
        self.report_code
            .as_deref()
            .map(crate::report_code_label)
            .filter(|label| *label != "[Unknown report code]")
    }

    /// Election type named by [`Form3P::election_code`] (see
    /// [`crate::covers::election_code_label`]).
    pub fn election_code_label(&self) -> Option<&'static str> {
        self.election_code
            .as_deref()
            .and_then(crate::covers::election_code_label)
    }
}

/// Form 3P page 2, the Summary Page
/// ([fecfrm3p.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=2)).
///
/// One amount per line. Lines 6–10 reconcile cash for this period; Lines 14
/// and 15, printed under "Net Election Cycle-to-Date Contributions and
/// Expenditures", are computed from Column B (cycle-to-date) lines even though
/// the record files them in its Column A block
/// ([fecfrm3pi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=4)).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct Form3PSummary {
    /// Line 6, cash on hand at beginning of reporting period
    /// (`col_a_cash_on_hand_beginning_period`;
    /// [fecfrm3pi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=4)).
    pub line6_cash_on_hand_beginning_period: f64,
    /// Line 7, total receipts this period, carried from Line 22 Column A
    /// (`col_a_total_receipts`;
    /// [fecfrm3pi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=4)).
    pub line7_total_receipts: f64,
    /// Line 8, subtotal = 6 + 7 (`col_a_subtotal`;
    /// [fecfrm3pi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=4)).
    pub line8_subtotal: f64,
    /// Line 9, total disbursements this period, carried from Line 30 Column A
    /// (`col_a_total_disbursements`;
    /// [fecfrm3pi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=4)).
    pub line9_total_disbursements: f64,
    /// Line 10, cash on hand at close of the reporting period = 8 − 9
    /// (`col_a_cash_on_hand_close_of_period`;
    /// [fecfrm3pi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=4)).
    pub line10_cash_on_hand_end_period: f64,
    /// Line 11, debts and obligations owed TO the committee, from Schedules C-P
    /// and D-P (`col_a_debts_to`;
    /// [fecfrm3pi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=4)).
    pub line11_debts_owed_to_committee: f64,
    /// Line 12, debts and obligations owed BY the committee, from Schedules C-P
    /// and D-P (`col_a_debts_by`;
    /// [fecfrm3pi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=4)).
    pub line12_debts_owed_by_committee: f64,
    /// Line 13, expenditures subject to limitation, carried from item I of the
    /// page 8 worksheet (`col_a_expenditures_subject_to_limits`;
    /// [fecfrm3pi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=4)).
    pub line13_expenditures_subject_to_limits: f64,
    /// Line 14, net contributions (other than loans), election cycle-to-date:
    /// Line 17(e) Column B − Line 28(d) Column B (`col_a_net_contributions`;
    /// [fecfrm3pi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=4)).
    /// Holds on 197/197 v8.x corpus filings; the Column A version of the
    /// formula holds on only 79.
    pub line14_net_contributions_other_than_loans: f64,
    /// Line 15, net operating expenditures, election cycle-to-date: Line 23
    /// Column B − Line 20(a) Column B (only operating offsets are subtracted)
    /// (`col_a_net_operating_expenditures`;
    /// [fecfrm3pi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=4)).
    pub line15_net_operating_expenditures: f64,
}

impl Form3PSummary {
    pub fn from_data(data: &Data) -> Self {
        Self {
            line6_cash_on_hand_beginning_period: amount(
                data,
                "col_a_cash_on_hand_beginning_period",
            ),
            line7_total_receipts: amount(data, "col_a_total_receipts"),
            line8_subtotal: amount(data, "col_a_subtotal"),
            line9_total_disbursements: amount(data, "col_a_total_disbursements"),
            line10_cash_on_hand_end_period: amount(data, "col_a_cash_on_hand_close_of_period"),
            line11_debts_owed_to_committee: amount(data, "col_a_debts_to"),
            line12_debts_owed_by_committee: amount(data, "col_a_debts_by"),
            line13_expenditures_subject_to_limits: amount(
                data,
                "col_a_expenditures_subject_to_limits",
            ),
            line14_net_contributions_other_than_loans: amount(data, "col_a_net_contributions"),
            line15_net_operating_expenditures: amount(data, "col_a_net_operating_expenditures"),
        }
    }
}

/// Read a Detailed Summary line whose Column A sits under a duplicated column
/// name.
///
/// `mappings2.json` names Summary Line 7 `col_a_total_receipts` and Line 22
/// Column A `col_a_total_receipts_TODO_DUP` in v7.0–8.5 (likewise Line 9 /
/// Line 30 with `col_a_total_disbursements`). Older mappings (v6.x, P3.x) give
/// both positions the same name, so the record map keeps only the later one —
/// Line 22 / Line 30. Either way the line is recoverable, and the form makes
/// Line 7 equal Line 22 Column A and Line 9 equal Line 30 Column A
/// ([fecfrm3pi.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=4)).
fn row_dup(data: &Data, col_a_dup: &str, col_a: &str, col_b: &str) -> DetailedSummaryRow {
    DetailedSummaryRow {
        column_a: amount_opt(data, col_a_dup).unwrap_or_else(|| amount(data, col_a)),
        column_b: amount(data, col_b),
    }
}

/// Form 3P page 3, Detailed Summary Page Part I, Receipts (Lines 16–22)
/// ([fecfrm3p.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=3)).
///
/// Column A is "Total This Period", Column B "Election Cycle-to-Date".
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct Form3PDetailedSummaryReceipts {
    /// Line 16, federal funds received this period, all itemized on Schedule
    /// A-P (`col_a_federal_funds`, `col_b_federal_funds`;
    /// [fecfrm3pi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=5)).
    pub line16_federal_funds: DetailedSummaryRow,
    /// Line 17(a)(i), itemized contributions (other than loans) from
    /// individuals/persons other than political committees, i.e. from persons
    /// over $200 in the election cycle (`col_a_individuals_itemized`,
    /// `col_b_individuals_itemized`;
    /// [fecfrm3pi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=5)).
    /// Absent (zero) in v6.x records, which have only 17(a)(iii).
    pub line17a_i_contributions_from_individuals_itemized: DetailedSummaryRow,
    /// Line 17(a)(ii), unitemized contributions from individuals
    /// (`col_a_individuals_unitemized`, `col_b_individuals_unitemized`;
    /// [fecfrm3pi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=5)).
    pub line17a_ii_contributions_from_individuals_unitemized: DetailedSummaryRow,
    /// Line 17(a)(iii), total contributions from individuals = 17(a)(i) +
    /// 17(a)(ii) (`col_a_individual_contribution_total`,
    /// `col_b_individual_contribution_total`;
    /// [fecfrm3pi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=5)).
    pub line17a_iii_contributions_from_individuals_total: DetailedSummaryRow,
    /// Line 17(b), contributions from political party committees
    /// (`col_a_political_party_committees_receipts`,
    /// `col_b_political_party_committees_receipts`;
    /// [fecfrm3pi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=5)).
    pub line17b_political_party_committees: DetailedSummaryRow,
    /// Line 17(c), contributions from other political committees
    /// (`col_a_other_political_committees_pacs`,
    /// `col_b_other_political_committees_pacs`;
    /// [fecfrm3pi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=5)).
    pub line17c_other_political_committees: DetailedSummaryRow,
    /// Line 17(d), contributions (other than loans) from the candidate
    /// (`col_a_the_candidate`, `col_b_the_candidate`;
    /// [fecfrm3pi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=5)).
    pub line17d_the_candidate: DetailedSummaryRow,
    /// Line 17(e), total contributions (other than loans) =
    /// 17(a)(iii) + 17(b) + 17(c) + 17(d) (`col_a_total_contributions`,
    /// `col_b_total_contributions_other_than_loans`;
    /// [fecfrm3pi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=5)).
    pub line17e_total_contributions: DetailedSummaryRow,
    /// Line 18, transfers from other authorized committees of the same
    /// candidate, including loans and loan repayments from them
    /// (`col_a_transfers_from_aff_other_party_cmttees`,
    /// `col_b_transfers_from_aff_other_party_cmttees`;
    /// [fecfrm3pi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=5)).
    /// The column name says "aff/other party committees" (the workbook's
    /// label), but the printed line is "Transfers from Other Authorized
    /// Committees" ([fecfrm3p.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=3)).
    pub line18_transfers_from_other_authorized_committee: DetailedSummaryRow,
    /// Line 19(a), loans received from or guaranteed by the candidate
    /// (`col_a_received_from_or_guaranteed_by_cand`,
    /// `col_b_received_from_or_guaranteed_by_cand`;
    /// [fecfrm3pi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=5)).
    pub line19a_loans_received_from_or_guaranteed_by_candidate: DetailedSummaryRow,
    /// Line 19(b), other loans (`col_a_other_loans`, `col_b_other_loans`;
    /// [fecfrm3pi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=5)).
    pub line19b_other_loans: DetailedSummaryRow,
    /// Line 19(c), total loans = 19(a) + 19(b) (`col_a_total_loans`,
    /// `col_b_total_loans`;
    /// [fecfrm3pi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=5)).
    pub line19c_total_loans: DetailedSummaryRow,
    /// Line 20(a), offsets to operating expenditures (refunds, rebates, returns
    /// of deposits) (`col_a_operating`, `col_b_operating`;
    /// [fecfrm3pi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=5)).
    pub line20a_offsets_to_expenditures_operating: DetailedSummaryRow,
    /// Line 20(b), offsets to fundraising disbursements (`col_a_fundraising`,
    /// `col_b_fundraising`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line20b_offsets_to_expenditures_fundraising: DetailedSummaryRow,
    /// Line 20(c), offsets to legal and accounting disbursements
    /// (`col_a_legal_and_accounting`, `col_b_legal_and_accounting`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line20c_offsets_to_expenditures_legal_and_accounting: DetailedSummaryRow,
    /// Line 20(d), total offsets to expenditures = 20(a) + 20(b) + 20(c)
    /// (`col_a_total_offsets_to_expenditures`,
    /// `col_b_total_offsets_to_operating_expenditures`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line20d_offsets_to_expenditures_total: DetailedSummaryRow,
    /// Line 21, other receipts (dividends, interest, repayments of loans made
    /// by the committee) (`col_a_other_receipts`, `col_b_other_receipts`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line21_other_receipts: DetailedSummaryRow,
    /// Line 22, total receipts = 16 + 17(e) + 18 + 19(c) + 20(d) + 21
    /// ([fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    /// Column A is `col_a_total_receipts_TODO_DUP` (fields.json places it on
    /// Line 22; workbook field 51), falling back to `col_a_total_receipts` for
    /// older mappings; Column B is `col_b_total_receipts`. Column A equals
    /// [`Form3PSummary::line7_total_receipts`] on 197/197 v8.x corpus filings.
    pub line22_total_receipts: DetailedSummaryRow,
}

impl Form3PDetailedSummaryReceipts {
    pub fn from_data(data: &Data) -> Self {
        let r = |a: &str, b: &str| DetailedSummaryRow::from_data(data, a, b);
        Self {
            line16_federal_funds: r("col_a_federal_funds", "col_b_federal_funds"),
            line17a_i_contributions_from_individuals_itemized: r(
                "col_a_individuals_itemized",
                "col_b_individuals_itemized",
            ),
            line17a_ii_contributions_from_individuals_unitemized: r(
                "col_a_individuals_unitemized",
                "col_b_individuals_unitemized",
            ),
            line17a_iii_contributions_from_individuals_total: r(
                "col_a_individual_contribution_total",
                "col_b_individual_contribution_total",
            ),
            line17b_political_party_committees: r(
                "col_a_political_party_committees_receipts",
                "col_b_political_party_committees_receipts",
            ),
            line17c_other_political_committees: r(
                "col_a_other_political_committees_pacs",
                "col_b_other_political_committees_pacs",
            ),
            line17d_the_candidate: r("col_a_the_candidate", "col_b_the_candidate"),
            line17e_total_contributions: r(
                "col_a_total_contributions",
                "col_b_total_contributions_other_than_loans",
            ),
            line18_transfers_from_other_authorized_committee: r(
                "col_a_transfers_from_aff_other_party_cmttees",
                "col_b_transfers_from_aff_other_party_cmttees",
            ),
            line19a_loans_received_from_or_guaranteed_by_candidate: r(
                "col_a_received_from_or_guaranteed_by_cand",
                "col_b_received_from_or_guaranteed_by_cand",
            ),
            line19b_other_loans: r("col_a_other_loans", "col_b_other_loans"),
            line19c_total_loans: r("col_a_total_loans", "col_b_total_loans"),
            line20a_offsets_to_expenditures_operating: r("col_a_operating", "col_b_operating"),
            line20b_offsets_to_expenditures_fundraising: r(
                "col_a_fundraising",
                "col_b_fundraising",
            ),
            line20c_offsets_to_expenditures_legal_and_accounting: r(
                "col_a_legal_and_accounting",
                "col_b_legal_and_accounting",
            ),
            line20d_offsets_to_expenditures_total: r(
                "col_a_total_offsets_to_expenditures",
                "col_b_total_offsets_to_operating_expenditures",
            ),
            line21_other_receipts: r("col_a_other_receipts", "col_b_other_receipts"),
            line22_total_receipts: row_dup(
                data,
                "col_a_total_receipts_TODO_DUP",
                "col_a_total_receipts",
                "col_b_total_receipts",
            ),
        }
    }
}

/// Form 3P page 4, Detailed Summary Page Part II, Disbursements (Lines 23–30)
/// ([fecfrm3p.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=4)).
///
/// Column A is "Total This Period", Column B "Election Cycle-to-Date".
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct Form3PDetailedSummaryDisbursements {
    /// Line 23, operating expenditures, e.g. advertising, salaries, travel,
    /// rent, telephones (`col_a_operating_expenditures`,
    /// `col_b_operating_expenditures`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line23_operating_expenditures: DetailedSummaryRow,
    /// Line 24, transfers to other authorized committees of the same candidate
    /// (`col_a_transfers_to_other_authorized_committees`,
    /// `col_b_transfers_to_other_authorized_committees`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line24_transfers_to_other_authorized_committees: DetailedSummaryRow,
    /// Line 25, fundraising disbursements (`col_a_fundraising_disbursements`,
    /// `col_b_fundraising_disbursements`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line25_fundraising_disbursements: DetailedSummaryRow,
    /// Line 26, exempt legal and accounting disbursements
    /// (`col_a_exempt_legal_accounting_disbursement`,
    /// `col_b_exempt_legal_accounting_disbursement`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line26_exempt_legal_and_accounting_disbursements: DetailedSummaryRow,
    /// Line 27(a), repayments of loans made or guaranteed by the candidate
    /// (`col_a_made_or_guaranteed_by_candidate`,
    /// `col_b_made_or_guaranteed_by_the_candidate`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line27a_loan_repayments_candidate: DetailedSummaryRow,
    /// Line 27(b), repayments of all other loans (`col_a_other_repayments`,
    /// `col_b_other_repayments`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line27b_loan_repayments_other: DetailedSummaryRow,
    /// Line 27(c), total loan repayments made = 27(a) + 27(b)
    /// (`col_a_total_loan_repayments_made`, `col_b_total_loan_repayments_made`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line27c_loan_repayments_total: DetailedSummaryRow,
    /// Line 28(a), contribution refunds to individuals/persons other than
    /// political committees (`col_a_individuals`, `col_b_individuals`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line28a_refunds_to_individuals: DetailedSummaryRow,
    /// Line 28(b), contribution refunds to political party committees
    /// (`col_a_political_party_committees_refunds`,
    /// `col_b_political_party_committees_refunds`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line28b_refunds_to_political_party_committees: DetailedSummaryRow,
    /// Line 28(c), contribution refunds to other political committees
    /// (`col_a_other_political_committees`, `col_b_other_political_committees`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line28c_refunds_to_other_political_committees: DetailedSummaryRow,
    /// Line 28(d), total contribution refunds = 28(a) + 28(b) + 28(c)
    /// (`col_a_total_contributions_refunds`, `col_b_total_contributions_refunds`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line28d_refunds_total: DetailedSummaryRow,
    /// Line 29, other disbursements (`col_a_other_disbursements`,
    /// `col_b_other_disbursements`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    pub line29_other_disbursements: DetailedSummaryRow,
    /// Line 30, total disbursements = 23 + 24 + 25 + 26 + 27(c) + 28(d) + 29
    /// ([fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    /// Column A is `col_a_total_disbursements_TODO_DUP` (fields.json places it
    /// on Line 30; workbook field 64), falling back to
    /// `col_a_total_disbursements` for older mappings; Column B is
    /// `col_b_total_disbursements`. Column A equals
    /// [`Form3PSummary::line9_total_disbursements`] on 197/197 v8.x corpus
    /// filings.
    pub line30_total_disbursements: DetailedSummaryRow,
}

impl Form3PDetailedSummaryDisbursements {
    pub fn from_data(data: &Data) -> Self {
        let r = |a: &str, b: &str| DetailedSummaryRow::from_data(data, a, b);
        Self {
            line23_operating_expenditures: r(
                "col_a_operating_expenditures",
                "col_b_operating_expenditures",
            ),
            line24_transfers_to_other_authorized_committees: r(
                "col_a_transfers_to_other_authorized_committees",
                "col_b_transfers_to_other_authorized_committees",
            ),
            line25_fundraising_disbursements: r(
                "col_a_fundraising_disbursements",
                "col_b_fundraising_disbursements",
            ),
            line26_exempt_legal_and_accounting_disbursements: r(
                "col_a_exempt_legal_accounting_disbursement",
                "col_b_exempt_legal_accounting_disbursement",
            ),
            line27a_loan_repayments_candidate: r(
                "col_a_made_or_guaranteed_by_candidate",
                "col_b_made_or_guaranteed_by_the_candidate",
            ),
            line27b_loan_repayments_other: r("col_a_other_repayments", "col_b_other_repayments"),
            line27c_loan_repayments_total: r(
                "col_a_total_loan_repayments_made",
                "col_b_total_loan_repayments_made",
            ),
            line28a_refunds_to_individuals: r("col_a_individuals", "col_b_individuals"),
            line28b_refunds_to_political_party_committees: r(
                "col_a_political_party_committees_refunds",
                "col_b_political_party_committees_refunds",
            ),
            line28c_refunds_to_other_political_committees: r(
                "col_a_other_political_committees",
                "col_b_other_political_committees",
            ),
            line28d_refunds_total: r(
                "col_a_total_contributions_refunds",
                "col_b_total_contributions_refunds",
            ),
            line29_other_disbursements: r("col_a_other_disbursements", "col_b_other_disbursements"),
            line30_total_disbursements: row_dup(
                data,
                "col_a_total_disbursements_TODO_DUP",
                "col_a_total_disbursements",
                "col_b_total_disbursements",
            ),
        }
    }
}

/// Form 3P pages 3–4, the Detailed Summary Page
/// ([fecfrm3p.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=3),
/// [p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=4)).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct Form3PDetailedSummary {
    /// Part I, Receipts (Lines 16–22).
    pub receipts: Form3PDetailedSummaryReceipts,
    /// Part II, Disbursements (Lines 23–30).
    pub disbursements: Form3PDetailedSummaryDisbursements,
    /// Part III, Line 31, items on hand to be liquidated: contributions
    /// received as stocks, bonds, art objects and similar items, valued as of
    /// the close of the period (`col_a_items_on_hand_to_be_liquidated`;
    /// [fecfrm3pi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=6)).
    /// The paper line has two columns but the record only one (FEC format
    /// workbook v8.4, sheet `F3P`, field 65); the items themselves are listed
    /// in `F3P31` records.
    pub line31_items_on_hand_to_be_liquidated: f64,
}

impl Form3PDetailedSummary {
    pub fn from_data(data: &Data) -> Self {
        Self {
            receipts: Form3PDetailedSummaryReceipts::from_data(data),
            disbursements: Form3PDetailedSummaryDisbursements::from_data(data),
            line31_items_on_hand_to_be_liquidated: amount(
                data,
                "col_a_items_on_hand_to_be_liquidated",
            ),
        }
    }
}

/// Form 3P pages 5–7, "Allocation of Primary Expenditures by State for a
/// Presidential Candidate", used only by primary committees receiving or
/// expecting to receive federal funds
/// ([fecfrm3p.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=5)).
///
/// These are not contributions by state: each expenditure is allocated to the
/// state it was intended to influence, "not necessarily the state in which the
/// expenditure was incurred or paid", and amounts include allocable
/// expenditures by the candidate's other authorized committees
/// ([fecfrm3pi.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3pi.pdf#page=8)).
/// For each state, Column A is "Allocation This Period" (`col_a_<state>`) and
/// Column B "Total Allocation To Date" (`col_b_<state>`)
/// ([fecfrm3p.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=5)).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct Form3PStateAllocations {
    /// The 54 jurisdictions in the order the form prints them (50 states,
    /// District of Columbia, Puerto Rico, Guam, Virgin Islands).
    pub states: Vec<Form3PStateAllocation>,
    /// "TOTALS" row on page 7 (`col_a_totals`, `col_b_totals`;
    /// [fecfrm3p.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=7)).
    /// Equals the sum of the state rows on 197/197 v8.x corpus filings.
    pub totals: DetailedSummaryRow,
}

/// One state's row on the Form 3P state allocation pages.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct Form3PStateAllocation {
    /// The state as printed on the form, e.g. `"District of Columbia"`
    /// ([fecfrm3p.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=5)).
    pub state: &'static str,
    /// Column A "Allocation This Period" and Column B "Total Allocation To
    /// Date".
    pub allocation: DetailedSummaryRow,
}

/// `(column suffix, name printed on the form)` for each allocation row.
const STATE_ALLOCATION_COLUMNS: [(&str, &str); 54] = [
    ("alabama", "Alabama"),
    ("alaska", "Alaska"),
    ("arizona", "Arizona"),
    ("arkansas", "Arkansas"),
    ("california", "California"),
    ("colorado", "Colorado"),
    ("connecticut", "Connecticut"),
    ("delaware", "Delaware"),
    ("dist_of_columbia", "District of Columbia"),
    ("florida", "Florida"),
    ("georgia", "Georgia"),
    ("hawaii", "Hawaii"),
    ("idaho", "Idaho"),
    ("illinois", "Illinois"),
    ("indiana", "Indiana"),
    ("iowa", "Iowa"),
    ("kansas", "Kansas"),
    ("kentucky", "Kentucky"),
    ("louisiana", "Louisiana"),
    ("maine", "Maine"),
    ("maryland", "Maryland"),
    ("massachusetts", "Massachusetts"),
    ("michigan", "Michigan"),
    ("minnesota", "Minnesota"),
    ("mississippi", "Mississippi"),
    ("missouri", "Missouri"),
    ("montana", "Montana"),
    ("nebraska", "Nebraska"),
    ("nevada", "Nevada"),
    ("new_hampshire", "New Hampshire"),
    ("new_jersey", "New Jersey"),
    ("new_mexico", "New Mexico"),
    ("new_york", "New York"),
    ("north_carolina", "North Carolina"),
    ("north_dakota", "North Dakota"),
    ("ohio", "Ohio"),
    ("oklahoma", "Oklahoma"),
    ("oregon", "Oregon"),
    ("pennsylvania", "Pennsylvania"),
    ("rhode_island", "Rhode Island"),
    ("south_carolina", "South Carolina"),
    ("south_dakota", "South Dakota"),
    ("tennessee", "Tennessee"),
    ("texas", "Texas"),
    ("utah", "Utah"),
    ("vermont", "Vermont"),
    ("virginia", "Virginia"),
    ("washington", "Washington"),
    ("west_virginia", "West Virginia"),
    ("wisconsin", "Wisconsin"),
    ("wyoming", "Wyoming"),
    ("puerto_rico", "Puerto Rico"),
    ("guam", "Guam"),
    ("virgin_islands", "Virgin Islands"),
];

impl Form3PStateAllocations {
    pub fn from_data(data: &Data) -> Self {
        Self {
            states: STATE_ALLOCATION_COLUMNS
                .iter()
                .map(|(suffix, state)| Form3PStateAllocation {
                    state,
                    allocation: DetailedSummaryRow::from_data(
                        data,
                        &format!("col_a_{suffix}"),
                        &format!("col_b_{suffix}"),
                    ),
                })
                .collect(),
            totals: DetailedSummaryRow::from_data(data, "col_a_totals", "col_b_totals"),
        }
    }

    /// True when every state row and the totals row are zero.
    pub fn is_empty(&self) -> bool {
        let zero = |r: &DetailedSummaryRow| r.column_a == 0.0 && r.column_b == 0.0;
        zero(&self.totals) && self.states.iter().all(|s| zero(&s.allocation))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(pairs: &[(&str, &str)]) -> Data {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn line22_prefers_todo_dup_column() {
        // v7.0–8.5 mapping: Line 7 and Line 22 Column A have distinct names.
        let d = data(&[
            ("col_a_total_receipts", "10"),
            ("col_a_total_receipts_TODO_DUP", "11"),
            ("col_b_total_receipts", "20"),
        ]);
        let r = Form3PDetailedSummaryReceipts::from_data(&d).line22_total_receipts;
        assert_eq!((r.column_a, r.column_b), (11.0, 20.0));
    }

    #[test]
    fn line22_falls_back_for_older_mappings() {
        // v6.x / P3.x mapping: one `col_a_total_receipts` key, holding Line 22.
        let d = data(&[
            ("col_a_total_receipts", "10"),
            ("col_b_total_receipts", "20"),
        ]);
        let r = Form3PDetailedSummaryReceipts::from_data(&d).line22_total_receipts;
        assert_eq!((r.column_a, r.column_b), (10.0, 20.0));
    }

    #[test]
    fn state_allocations_cover_every_mapped_state_column() {
        let s = Form3PStateAllocations::from_data(&data(&[("col_b_iowa", "5")]));
        assert_eq!(s.states.len(), 54);
        assert!(!s.is_empty());
        let iowa = s.states.iter().find(|r| r.state == "Iowa");
        assert_eq!(iowa.map(|r| r.allocation.column_b), Some(5.0));
    }
}
