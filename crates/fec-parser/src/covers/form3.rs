//! Form 3: Report of Receipts and Disbursements for an Authorized Committee.
//!
//! See [`Form3`].

use crate::covers::fields::{amount, date, flag, person_name_or_legacy, text, text_or_empty, Data};
use crate::covers::{Address, DetailedSummaryRow, PersonName};
use jiff::civil::Date;

/// FEC Form 3, "Report of Receipts and Disbursements for an Authorized
/// Committee": the periodic report of a House or Senate candidate's principal
/// campaign committee.
///
/// **Who files.** The principal campaign committee designated by a candidate
/// for the House of Representatives or Senate; presidential committees use
/// Form 3P and committees not authorized by a candidate use Form 3X
/// ([fecfrm3i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=2)).
///
/// **Layout.** Page 1 identifies the committee, the type of report and the
/// coverage period; page 2 is the Summary Page; pages 3–4 the Detailed Summary
/// Page ([fecfrm3i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=1)).
/// The `F3` record carries all four pages in one row (FEC format workbook v8.4,
/// sheet `F3`, fields 1–93). Page 1 is the top-level fields of this struct;
/// page 2 is [`Form3::summary`]; pages 3–4 are [`Form3::detailed_summary`].
///
/// **Columns.** On the Summary and Detailed Summary pages, Column A
/// ([`DetailedSummaryRow::column_a`]) is "Total This Period" and Column B
/// ([`DetailedSummaryRow::column_b`]) is "Election Cycle-to-Date", not calendar
/// year-to-date ([fecfrm3.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=2),
/// [p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=3),
/// [p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=4)).
/// The election cycle begins the day after the previous general election for
/// the seat and ends on the day of the next one
/// ([fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)).
/// Lines 8–10 and 23–27 have a single amount, read from the record's Column A
/// block (FEC format workbook v8.4, sheet `F3`, fields 30–32 and 58–62).
///
/// **Derived lines.** The Summary Page is carried from the Detailed Summary
/// Page: 6(a) is 11(e), 6(b) is 20(d), 7(a) is 17, 7(b) is 14, 8 is 27
/// ([fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)).
/// Over 15,500 v8.x filings these identities hold on ≥ 99.7% of records; see
/// `wiki/covers/F3.md` for the full table.
///
/// **Post-general reports.** For the report covering the general election the
/// paper form substitutes a three-column Post-Election Detailed Summary Page
/// ([fecfrm3posti.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3posti.pdf#page=1));
/// the `F3` record has no Column C, and the FEC sources do not say which paper
/// column the record's Column B holds on such a report.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "gleam", derive(fec_parser_macros::GleamType))]
pub struct Form3 {
    /// Form type as filed, e.g. `F3N`: the base form plus the
    /// amendment-indicator suffix (see [`crate::covers::base_form_type`]).
    /// Column `form_type` (FEC format workbook v8.4, sheet `F3`, field 1).
    /// `N` (new) or `A` (amended) match Line 3 "Is this report New (N) or
    /// Amended (A)"
    /// ([fecfrm3.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=1));
    /// the workbook also lists `T` without defining it.
    pub form_type: String,
    /// Line 2, the committee's FEC identification number
    /// (`filer_committee_id_number`;
    /// [fecfrm3.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=1)).
    pub filer_committee_id: String,
    /// Line 1, the committee's full name (`committee_name`;
    /// [fecfrm3.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=1)).
    pub committee_name: String,
    /// Line 1, the committee's mailing address (`street_1`, `street_2`, `city`,
    /// `state`, `zip_code`;
    /// [fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)).
    pub address: Address,
    /// The "Check if different than previously reported (ACC)" box beside the
    /// address (`change_of_address`, `X` = yes;
    /// [fecfrm3.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=1);
    /// FEC format workbook v8.4, sheet `F3`, field 4).
    pub change_of_address: bool,
    /// The "STATE" box printed beside Line 2 on page 1 (`election_state`;
    /// [fecfrm3.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=1);
    /// FEC format workbook v8.4, sheet `F3`, field 10, "ELECTION STATE"). The
    /// instructions do not describe this box.
    pub election_state: Option<String>,
    /// The "DISTRICT" box printed beside Line 2 on page 1, `01`–`99`
    /// (`election_district`;
    /// [fecfrm3.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=1);
    /// FEC format workbook v8.4, sheet `F3`, field 11, "ELECTION DISTRICT").
    /// The instructions do not describe this box.
    pub election_district: Option<String>,
    /// Line 4, type of report: one of the codes printed beside the page-1
    /// checkboxes, e.g. `Q1`, `YE`, `12P`, `30G`, `TER` (`report_code`;
    /// [fecfrm3.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=1)).
    /// See [`Form3::report_code_label`].
    pub report_code: Option<String>,
    /// For a 12-day pre-election or 30-day post-election report, the type of
    /// election (primary, general, convention, special or runoff) chosen on
    /// Line 4 ([fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)),
    /// coded as a letter plus the election year, e.g. `P2026` (`election_code`;
    /// FEC format workbook v8.4, sheet `F3`, field 13). See
    /// [`Form3::election_code_label`].
    pub election_code: Option<String>,
    /// Line 4, "Election on": the date of the election a 12-day pre- or 30-day
    /// post-election report is for (`election_date`;
    /// [fecfrm3.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=1)).
    pub election_date: Option<Date>,
    /// Line 4, "in the State of": the state holding that election
    /// (`state_of_election`;
    /// [fecfrm3.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=1)).
    pub state_of_election: Option<String>,
    /// Line 5, first day of the covering period (`coverage_from_date`;
    /// [fecfrm3.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=1)).
    pub coverage_from_date: Option<Date>,
    /// Line 5, last day of the covering period (`coverage_through_date`;
    /// [fecfrm3.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=1)).
    /// All activity since the last report's ending date must be included
    /// ([fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)).
    pub coverage_through_date: Option<Date>,
    /// "Type or Print Name of Treasurer" (`treasurer_*`; the single
    /// caret-delimited `treasurer_name` of v1–v5.x formats is split into parts;
    /// [fecfrm3.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=1)).
    pub treasurer: PersonName,
    /// Date the treasurer signed the certification (`date_signed`;
    /// [fecfrm3.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=1)).
    pub date_signed: Option<Date>,
    /// Page 2, the Summary Page (Lines 6–10).
    pub summary: Form3Summary,
    /// Pages 3–4, the Detailed Summary Page (Lines 11–27).
    pub detailed_summary: Form3DetailedSummary,
}

impl Form3 {
    pub fn from_data(data: &Data) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            committee_name: text_or_empty(data, "committee_name"),
            address: Address::from_prefixed(data, ""),
            change_of_address: flag(data, "change_of_address"),
            election_state: text(data, "election_state"),
            election_district: text(data, "election_district"),
            report_code: text(data, "report_code"),
            election_code: text(data, "election_code"),
            election_date: date(data, "election_date"),
            state_of_election: text(data, "state_of_election"),
            coverage_from_date: date(data, "coverage_from_date"),
            coverage_through_date: date(data, "coverage_through_date"),
            treasurer: person_name_or_legacy(data, "treasurer_", "treasurer_name"),
            date_signed: date(data, "date_signed"),
            summary: Form3Summary::from_data(data),
            detailed_summary: Form3DetailedSummary::from_data(data),
        })
    }

    /// True for an amended report (`F3A`); see [`Form3::form_type`].
    pub fn is_amendment(&self) -> bool {
        crate::covers::is_amendment_form_type(&self.form_type)
    }

    /// Description of [`Form3::report_code`] (see [`crate::report_code_label`]).
    pub fn report_code_label(&self) -> Option<&'static str> {
        self.report_code
            .as_deref()
            .map(crate::report_code_label)
            .filter(|label| *label != "[Unknown report code]")
    }

    /// Election type named by [`Form3::election_code`] (see [`crate::covers::election_code_label`]).
    pub fn election_code_label(&self) -> Option<&'static str> {
        self.election_code
            .as_deref()
            .and_then(crate::covers::election_code_label)
    }
}

/// `col_a_{suffix}` / `col_b_{suffix}` as one two-column line.
fn row(data: &Data, suffix: &str) -> DetailedSummaryRow {
    DetailedSummaryRow::from_data(data, &format!("col_a_{suffix}"), &format!("col_b_{suffix}"))
}

/// Form 3 page 2, the Summary Page
/// ([fecfrm3.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=2)).
///
/// Lines 6 and 7 have Column A ("This Period") and Column B ("Election
/// Cycle-to-Date"); Lines 8–10 have one amount. Every line is carried from the
/// Detailed Summary Page or the debt schedules
/// ([fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "gleam", derive(fec_parser_macros::GleamType))]
pub struct Form3Summary {
    /// Line 6(a), total contributions (other than loans), carried from Line
    /// 11(e) (`col_a_total_contributions_no_loans`,
    /// `col_b_total_contributions_no_loans`;
    /// [fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)).
    pub line6a_total_contributions: DetailedSummaryRow,
    /// Line 6(b), total contribution refunds, carried from Line 20(d)
    /// (`col_a_total_contributions_refunds`, `col_b_total_contributions_refunds`;
    /// [fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)).
    pub line6b_total_contribution_refunds: DetailedSummaryRow,
    /// Line 6(c), net contributions (other than loans) = 6(a) − 6(b)
    /// (`col_a_net_contributions`, `col_b_net_contributions`;
    /// [fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)).
    pub line6c_net_contributions: DetailedSummaryRow,
    /// Line 7(a), total operating expenditures, carried from Line 17
    /// (`col_a_total_operating_expenditures`,
    /// `col_b_total_operating_expenditures`;
    /// [fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)).
    pub line7a_total_operating_expenditures: DetailedSummaryRow,
    /// Line 7(b), total offsets to operating expenditures, carried from Line 14
    /// (`col_a_total_offset_to_operating_expenditures`,
    /// `col_b_total_offset_to_operating_expenditures`;
    /// [fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)).
    pub line7b_total_offsets_to_operating_expenditures: DetailedSummaryRow,
    /// Line 7(c), net operating expenditures = 7(a) − 7(b)
    /// (`col_a_net_operating_expenditures`, `col_b_net_operating_expenditures`;
    /// [fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)).
    pub line7c_net_operating_expenditures: DetailedSummaryRow,
    /// Line 8, cash on hand at close of reporting period, carried from Line 27
    /// (`col_a_cash_on_hand_close_of_period`;
    /// [fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)).
    pub line8_cash_on_hand_close_of_period: f64,
    /// Line 9, debts and obligations owed TO the committee, from Schedule C or
    /// D (`col_a_debts_to`;
    /// [fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)).
    pub line9_debts_owed_to_committee: f64,
    /// Line 10, debts and obligations owed BY the committee, from Schedule C or
    /// D (`col_a_debts_by`;
    /// [fecfrm3i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=3)).
    pub line10_debts_owed_by_committee: f64,
}

impl Form3Summary {
    pub fn from_data(data: &Data) -> Self {
        Self {
            line6a_total_contributions: row(data, "total_contributions_no_loans"),
            line6b_total_contribution_refunds: row(data, "total_contributions_refunds"),
            line6c_net_contributions: row(data, "net_contributions"),
            line7a_total_operating_expenditures: row(data, "total_operating_expenditures"),
            line7b_total_offsets_to_operating_expenditures: row(
                data,
                "total_offset_to_operating_expenditures",
            ),
            line7c_net_operating_expenditures: row(data, "net_operating_expenditures"),
            line8_cash_on_hand_close_of_period: amount(data, "col_a_cash_on_hand_close_of_period"),
            line9_debts_owed_to_committee: amount(data, "col_a_debts_to"),
            line10_debts_owed_by_committee: amount(data, "col_a_debts_by"),
        }
    }
}

/// Form 3 page 3, Detailed Summary Page Part I, Receipts (Lines 11–16)
/// ([fecfrm3.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=3)).
///
/// Column A is "Total This Period", Column B "Election Cycle-to-Date". Each
/// schedule's total is added to the unitemized amount for its category before
/// being entered here, so summing Schedule A rows will not generally reproduce
/// these lines ([fecfrm3i.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=6)).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "gleam", derive(fec_parser_macros::GleamType))]
pub struct Form3DetailedSummaryReceipts {
    /// Line 11(a)(i), contributions from individuals/persons other than
    /// political committees that must be itemized on Schedule A (over $200 in
    /// the election cycle) (`col_a_individual_contributions_itemized`,
    /// `col_b_individual_contributions_itemized`;
    /// [fecfrm3i.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=4)).
    pub line11a_i_contributions_from_individuals_itemized: DetailedSummaryRow,
    /// Line 11(a)(ii), contributions from individuals not required to be
    /// itemized (`col_a_individual_contributions_unitemized`,
    /// `col_b_individual_contributions_unitemized`;
    /// [fecfrm3i.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=4)).
    pub line11a_ii_contributions_from_individuals_unitemized: DetailedSummaryRow,
    /// Line 11(a)(iii), total contributions from individuals = 11(a)(i) +
    /// 11(a)(ii) (`col_a_total_individual_contributions`,
    /// `col_b_total_individual_contributions`;
    /// [fecfrm3i.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=4)).
    pub line11a_iii_contributions_from_individuals_total: DetailedSummaryRow,
    /// Line 11(b), contributions (other than loans) from political party
    /// committees, all itemized on Schedule A
    /// (`col_a_political_party_contributions`,
    /// `col_b_political_party_contributions`;
    /// [fecfrm3i.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=4)).
    pub line11b_political_party_committees: DetailedSummaryRow,
    /// Line 11(c), contributions (other than loans) from other political
    /// committees such as PACs, all itemized on Schedule A
    /// (`col_a_pac_contributions`, `col_b_pac_contributions`;
    /// [fecfrm3i.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=4)).
    pub line11c_other_political_committees_pacs: DetailedSummaryRow,
    /// Line 11(d), contributions (other than loans) from the candidate
    /// (`col_a_candidate_contributions`, `col_b_candidate_contributions`;
    /// [fecfrm3i.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=4)).
    pub line11d_the_candidate: DetailedSummaryRow,
    /// Line 11(e), total contributions (other than loans) =
    /// 11(a)(iii) + 11(b) + 11(c) + 11(d) (`col_a_total_contributions`,
    /// `col_b_total_contributions`;
    /// [fecfrm3i.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=4)).
    pub line11e_total_contributions: DetailedSummaryRow,
    /// Line 12, transfers from other authorized committees of the same
    /// candidate, including loans and loan repayments from them
    /// (`col_a_transfers_from_authorized`, `col_b_transfers_from_authorized`;
    /// [fecfrm3i.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=4)).
    pub line12_transfers_from_authorized: DetailedSummaryRow,
    /// Line 13(a), loans made or guaranteed by the candidate
    /// (`col_a_candidate_loans`, `col_b_candidate_loans`;
    /// [fecfrm3i.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=4)).
    pub line13a_loans_from_candidate: DetailedSummaryRow,
    /// Line 13(b), all other loans (`col_a_other_loans`, `col_b_other_loans`;
    /// [fecfrm3i.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=4)).
    pub line13b_other_loans: DetailedSummaryRow,
    /// Line 13(c), total loans = 13(a) + 13(b) (`col_a_total_loans`,
    /// `col_b_total_loans`;
    /// [fecfrm3.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=3)).
    pub line13c_total_loans: DetailedSummaryRow,
    /// Line 14, offsets to operating expenditures (refunds, rebates, returns of
    /// deposits) (`col_a_offset_to_operating_expenditures`,
    /// `col_b_offset_to_operating_expenditures`;
    /// [fecfrm3i.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=4)).
    pub line14_offset_to_operating_expenditures: DetailedSummaryRow,
    /// Line 15, other receipts (dividends, interest, etc.)
    /// (`col_a_other_receipts`, `col_b_other_receipts`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line15_other_receipts: DetailedSummaryRow,
    /// Line 16, total receipts = 11(e) + 12 + 13(c) + 14 + 15
    /// (`col_a_total_receipts`, `col_b_total_receipts`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line16_total_receipts: DetailedSummaryRow,
}

impl Form3DetailedSummaryReceipts {
    pub fn from_data(data: &Data) -> Self {
        Self {
            line11a_i_contributions_from_individuals_itemized: row(
                data,
                "individual_contributions_itemized",
            ),
            line11a_ii_contributions_from_individuals_unitemized: row(
                data,
                "individual_contributions_unitemized",
            ),
            line11a_iii_contributions_from_individuals_total: row(
                data,
                "total_individual_contributions",
            ),
            line11b_political_party_committees: row(data, "political_party_contributions"),
            line11c_other_political_committees_pacs: row(data, "pac_contributions"),
            line11d_the_candidate: row(data, "candidate_contributions"),
            line11e_total_contributions: row(data, "total_contributions"),
            line12_transfers_from_authorized: row(data, "transfers_from_authorized"),
            line13a_loans_from_candidate: row(data, "candidate_loans"),
            line13b_other_loans: row(data, "other_loans"),
            line13c_total_loans: row(data, "total_loans"),
            line14_offset_to_operating_expenditures: row(data, "offset_to_operating_expenditures"),
            line15_other_receipts: row(data, "other_receipts"),
            line16_total_receipts: row(data, "total_receipts"),
        }
    }
}

/// Form 3 page 4, Detailed Summary Page Part II, Disbursements (Lines 17–22)
/// ([fecfrm3.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=4)).
///
/// Column A is "Total This Period", Column B "Election Cycle-to-Date".
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "gleam", derive(fec_parser_macros::GleamType))]
pub struct Form3DetailedSummaryDisbursements {
    /// Line 17, operating expenditures, e.g. advertising, salaries, travel,
    /// rent, telephones (`col_a_operating_expenditures`,
    /// `col_b_operating_expenditures`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line17_operating_expenditures: DetailedSummaryRow,
    /// Line 18, transfers to other authorized committees of the same candidate
    /// (`col_a_transfers_to_authorized`, `col_b_transfers_to_authorized`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line18_transfers_to_authorized: DetailedSummaryRow,
    /// Line 19(a), repayments of loans made or guaranteed by the candidate
    /// (`col_a_candidate_loan_repayments`, `col_b_candidate_loan_repayments`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line19a_candidate_loan_repayments: DetailedSummaryRow,
    /// Line 19(b), repayments of all other loans
    /// (`col_a_other_loan_repayments`, `col_b_other_loan_repayments`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line19b_other_loan_repayments: DetailedSummaryRow,
    /// Line 19(c), total loan repayments = 19(a) + 19(b)
    /// (`col_a_total_loan_repayments`, `col_b_total_loan_repayments`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line19c_total_loan_repayments: DetailedSummaryRow,
    /// Line 20(a), contribution refunds to individuals/persons other than
    /// political committees (`col_a_refunds_to_individuals`,
    /// `col_b_refunds_to_individuals`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line20a_refunds_to_individuals: DetailedSummaryRow,
    /// Line 20(b), contribution refunds to political party committees
    /// (`col_a_refunds_to_party_committees`, `col_b_refunds_to_party_committees`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line20b_refunds_to_party_committees: DetailedSummaryRow,
    /// Line 20(c), contribution refunds to other political committees such as
    /// PACs (`col_a_refunds_to_other_committees`,
    /// `col_b_refunds_to_other_committees`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line20c_refunds_to_other_committees: DetailedSummaryRow,
    /// Line 20(d), total contribution refunds = 20(a) + 20(b) + 20(c)
    /// (`col_a_total_refunds`, `col_b_total_refunds`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line20d_total_refunds: DetailedSummaryRow,
    /// Line 21, other disbursements (`col_a_other_disbursements`,
    /// `col_b_other_disbursements`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line21_other_disbursements: DetailedSummaryRow,
    /// Line 22, total disbursements = 17 + 18 + 19(c) + 20(d) + 21
    /// (`col_a_total_disbursements`, `col_b_total_disbursements`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line22_total_disbursements: DetailedSummaryRow,
}

impl Form3DetailedSummaryDisbursements {
    pub fn from_data(data: &Data) -> Self {
        Self {
            line17_operating_expenditures: row(data, "operating_expenditures"),
            line18_transfers_to_authorized: row(data, "transfers_to_authorized"),
            line19a_candidate_loan_repayments: row(data, "candidate_loan_repayments"),
            line19b_other_loan_repayments: row(data, "other_loan_repayments"),
            line19c_total_loan_repayments: row(data, "total_loan_repayments"),
            line20a_refunds_to_individuals: row(data, "refunds_to_individuals"),
            line20b_refunds_to_party_committees: row(data, "refunds_to_party_committees"),
            line20c_refunds_to_other_committees: row(data, "refunds_to_other_committees"),
            line20d_total_refunds: row(data, "total_refunds"),
            line21_other_disbursements: row(data, "other_disbursements"),
            line22_total_disbursements: row(data, "total_disbursements"),
        }
    }
}

/// Form 3 page 4, Detailed Summary Page Part III, Cash Summary (Lines 23–27),
/// single amounts for this period
/// ([fecfrm3.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=4)).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "gleam", derive(fec_parser_macros::GleamType))]
pub struct Form3CashSummary {
    /// Line 23, cash on hand at beginning of reporting period: currency, bank
    /// balances, traveler's checks, CDs, treasury bills and other investments
    /// valued at cost (`col_a_cash_beginning_reporting_period`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line23_cash_on_hand_beginning: f64,
    /// Line 24, total receipts this period, carried from Line 16 Column A
    /// (`col_a_total_receipts_period`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line24_total_receipts: f64,
    /// Line 25, subtotal = 23 + 24 (`col_a_subtotals`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line25_subtotal: f64,
    /// Line 26, total disbursements this period, carried from Line 22 Column A
    /// (`col_a_total_disbursements_period`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line26_total_disbursements: f64,
    /// Line 27, cash on hand at close of reporting period = 25 − 26
    /// (`col_a_cash_on_hand_close`;
    /// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5)).
    pub line27_cash_on_hand_close: f64,
}

impl Form3CashSummary {
    pub fn from_data(data: &Data) -> Self {
        Self {
            line23_cash_on_hand_beginning: amount(data, "col_a_cash_beginning_reporting_period"),
            line24_total_receipts: amount(data, "col_a_total_receipts_period"),
            line25_subtotal: amount(data, "col_a_subtotals"),
            line26_total_disbursements: amount(data, "col_a_total_disbursements_period"),
            line27_cash_on_hand_close: amount(data, "col_a_cash_on_hand_close"),
        }
    }
}

/// Form 3 pages 3–4, the Detailed Summary Page
/// ([fecfrm3.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=3),
/// [p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=4)).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "gleam", derive(fec_parser_macros::GleamType))]
pub struct Form3DetailedSummary {
    /// Part I, Receipts (Lines 11–16).
    pub receipts: Form3DetailedSummaryReceipts,
    /// Part II, Disbursements (Lines 17–22).
    pub disbursements: Form3DetailedSummaryDisbursements,
    /// Part III, Cash Summary (Lines 23–27).
    pub cash_summary: Form3CashSummary,
}

impl Form3DetailedSummary {
    pub fn from_data(data: &Data) -> Self {
        Self {
            receipts: Form3DetailedSummaryReceipts::from_data(data),
            disbursements: Form3DetailedSummaryDisbursements::from_data(data),
            cash_summary: Form3CashSummary::from_data(data),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn election_codes() {
        assert_eq!(crate::covers::election_code_label("P2026"), Some("Primary"));
        assert_eq!(crate::covers::election_code_label("g2024"), Some("General"));
        assert_eq!(crate::covers::election_code_label("E2020"), Some("Recount"));
        assert_eq!(crate::covers::election_code_label(""), None);
        assert_eq!(crate::covers::election_code_label("X2020"), None);
    }

    #[test]
    fn blank_record_is_typed() {
        let data: Data = [("form_type", "F3N")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let Some(form) = Form3::from_data(&data) else {
            panic!("blank F3 record should be typed");
        };
        assert_eq!(form.form_type, "F3N");
        assert_eq!(form.date_signed, None);
        assert_eq!(form.summary.line8_cash_on_hand_close_of_period, 0.0);
    }
}
