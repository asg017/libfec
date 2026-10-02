//! Form 4: Report of Receipts and Disbursements for a Committee or
//! Organization Supporting a Nominating Convention.

use crate::covers::fields::{amount, date, text, text_or_empty, Data};
use crate::covers::{Address, DetailedSummaryRow, PersonName};
use jiff::civil::Date;
use serde::Serialize;

/// FEC Form 4, "Report of Receipts and Disbursements for a Committee or
/// Organization Supporting a Nominating Convention" — the `F4N` / `F4A` /
/// `F4T` cover record.
///
/// **Who files.** Convention committees established by a national party
/// committee; host committees and other organizations representing a State,
/// municipality or other political subdivision in dealing with national party
/// officials on a presidential nominating convention; and any other
/// organization that makes convention arrangements for a political party
/// ([fecfrm4i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=1)).
///
/// **When.** Quarterly reports, a post-convention report (report code `60D`)
/// and, for a host committee, a final report once its convention activity has
/// ceased
/// ([fecfrm4i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=1)).
///
/// **Pages.** Page 1 is the Summary Page: Section A, the cash summary (Lines
/// 6–10), and Section B, expenditures subject to limitation (Lines 11–12(c))
/// ([fecfrm4.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=1)).
/// Page 2 is the Detailed Summary Page: receipts (Lines 13–20) and
/// disbursements (Lines 21–25)
/// ([fecfrm4.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=2)).
/// They are [`Form4::summary`] and [`Form4::detailed_summary`].
///
/// **Columns.** In every [`DetailedSummaryRow`] on this form, **Column A is
/// "This Period"** and **Column B is "Calendar Year-to-Date"**
/// ([fecfrm4.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=1),
/// [p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=2));
/// Column B is the previous report's year-to-date figure plus this period's
/// Column A
/// ([fecfrm4i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
/// Column B exists only for subtotals and totals: the (a)/(b) sub-lines, Line
/// 6(b) and Lines 9–10 are Column A only, and Lines 6(a) and 12(c) are Column B
/// only (FEC format workbook v8.4, sheet `F4`, fields 20–83). Those lines are
/// plain `f64` fields here.
///
/// **Versions.** v6.1–v8.5 share one layout. The legacy v3/v5 layouts lack the
/// split treasurer name (one `treasurer_name` column, read into
/// [`PersonName::last_name`]) and name Lines 20/25 without the
/// `_TODO_DUP` suffix, so the Line 6(c)/7 and 20/25 columns collide and both
/// read the Line 20/25 value, which the instructions require to be equal
/// anyway.
#[derive(Debug, Clone, Serialize)]
pub struct Form4 {
    /// Line 1(a), name of the committee (`committee_name`)
    /// ([fecfrm4.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=1)).
    pub committee_name: String,
    /// Line 2, FEC identification number (`filer_committee_id_number`)
    /// ([fecfrm4.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=1)).
    pub committee_id: String,
    /// Line 1(b)–(c), mailing address (`street_1`, `street_2`, `city`,
    /// `state`, `zip_code`)
    /// ([fecfrm4.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=1)).
    pub address: Address,
    /// Line 3, type of committee/organization (`committee_type`): `A`, `H` or
    /// `O`. See [`Form4::committee_type_label`].
    pub committee_type: Option<String>,
    /// Line 3 "Other (specify)" text (`committee_type_description`)
    /// (FEC format workbook v8.4, sheet `F4`, field 10).
    pub committee_type_description: Option<String>,
    /// Line 4, type of report (`report_code`). See
    /// [`Form4::report_code_label`].
    pub report_code: Option<String>,
    /// Line 5, first day of the covering period (`coverage_from_date`)
    /// ([fecfrm4.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=1)).
    pub coverage_from: Option<Date>,
    /// Line 5, last day of the covering period (`coverage_through_date`).
    pub coverage_through: Option<Date>,
    /// The treasurer who signed the certification (`treasurer_last_name`,
    /// `treasurer_first_name`, `treasurer_middle_name`, `treasurer_prefix`,
    /// `treasurer_suffix`; legacy `treasurer_name` into `last_name`)
    /// ([fecfrm4.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=1)).
    pub treasurer: PersonName,
    /// Date signed (`date_signed`).
    pub date_signed: Option<Date>,
    /// Page 1, Summary Page (Lines 6–12(c)).
    pub summary: Form4Summary,
    /// Page 2, Detailed Summary Page (Lines 13–25).
    pub detailed_summary: Form4DetailedSummary,
}

/// Page 1, Summary Page
/// ([fecfrm4.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=1)).
/// Column A = This Period, Column B = Calendar Year-to-Date.
#[derive(Debug, Clone, Serialize)]
pub struct Form4Summary {
    /// Line 6(a), cash on hand January 1 of the year — Column B only
    /// (`col_b_cash_on_hand_beginning_year`)
    /// ([fecfrm4i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=1)).
    pub line6a_cash_on_hand_january_1: f64,
    /// Line 6(a), the year printed after "January 1, 20__"
    /// (`col_b_beginning_year`) (FEC format workbook v8.4, sheet `F4`,
    /// field 61).
    pub line6a_year: Option<String>,
    /// Line 6(b), cash on hand at the beginning of the reporting period —
    /// Column A only (`col_a_cash_on_hand_beginning_reporting_period`)
    /// ([fecfrm4i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=1)).
    pub line6b_cash_on_hand_beginning_period: f64,
    /// Line 6(c), total receipts, from Line 20 (`col_a_total_receipts`,
    /// `col_b_total_receipts`)
    /// ([fecfrm4i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=1)).
    pub line6c_total_receipts: DetailedSummaryRow,
    /// Line 6(d), subtotal: 6(b) + 6(c) for Column A, 6(a) + 6(c) for Column
    /// B (`col_a_subtotal`, `col_b_subtotal`)
    /// ([fecfrm4i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=1)).
    pub line6d_subtotal: DetailedSummaryRow,
    /// Line 7, total disbursements, from Line 25
    /// (`col_a_total_disbursements`, `col_b_total_disbursements`)
    /// ([fecfrm4i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
    pub line7_total_disbursements: DetailedSummaryRow,
    /// Line 8, cash on hand at close of the reporting period: 6(d) − 7, "which
    /// should be the same for both columns"
    /// (`col_a_cash_on_hand_close_of_period`, `col_b_cash_on_hand_close_of_period`)
    /// ([fecfrm4i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
    pub line8_cash_on_hand_close_of_period: DetailedSummaryRow,
    /// Line 9, debts and obligations owed TO the committee, from Schedule C
    /// or D — Column A only (`col_a_debts_to`)
    /// ([fecfrm4i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
    pub line9_debts_owed_to_committee: f64,
    /// Line 10, debts and obligations owed BY the committee, from Schedule C
    /// or D — Column A only (`col_a_debts_by`)
    /// ([fecfrm4i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
    pub line10_debts_owed_by_committee: f64,
    /// Line 11, convention expenditures, from Line 21(c)
    /// (`col_a_convention_expenditures`, `col_b_convention_expenditures`)
    /// ([fecfrm4i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
    pub line11_convention_expenditures: DetailedSummaryRow,
    /// Line 12, refunds, rebates and returns of deposits relating to
    /// convention expenditures, from Line 17(c) (`col_a_convention_refunds`,
    /// `col_b_convention_refunds`)
    /// ([fecfrm4i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
    pub line12_convention_refunds: DetailedSummaryRow,
    /// Line 12(a), expenditures subject to limitation: 11 − 12
    /// (`col_a_expenditures_subject_to_limits`,
    /// `col_b_expenditures_subject_to_limits`)
    /// ([fecfrm4i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
    pub line12a_expenditures_subject_to_limitation: DetailedSummaryRow,
    /// Line 12(b), expenditures from prior years subject to limitation
    /// (`col_a_prior_expenditures_subject_to_limits`,
    /// `col_b_prior_expendiutres_subject_to_limits` — sic)
    /// ([fecfrm4i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
    pub line12b_prior_years_expenditures_subject_to_limitation: DetailedSummaryRow,
    /// Line 12(c), total expenditures subject to limitation: 12(a) + 12(b) —
    /// Column B only (`col_b_total_expenditures_subject_to_limits`)
    /// ([fecfrm4i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2);
    /// FEC format workbook v8.4, sheet `F4`, field 70).
    pub line12c_total_expenditures_subject_to_limitation: f64,
}

/// A detailed-summary category split into (a) itemized, (b) unitemized and
/// (c) subtotal, as on Form 4 Lines 14, 17, 18, 19, 21 and 24
/// ([fecfrm4.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=2)).
/// The (a)/(b) boxes are Column A (This Period) only; the subtotal has both
/// columns.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Form4ItemizedLine {
    /// (a) itemized amount, This Period.
    pub itemized: f64,
    /// (b) unitemized amount, This Period.
    pub unitemized: f64,
    /// (c) subtotal: (a) + (b) in Column A; Calendar Year-to-Date in Column B.
    pub subtotal: DetailedSummaryRow,
}

/// Loans and loan repayments, Form 4 Lines 16 (received) and 23 (made):
/// (a) loans, (b) loan repayments, (c) subtotal
/// ([fecfrm4.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=2)).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Form4LoanLine {
    /// (a) loans received / made, This Period.
    pub loans: f64,
    /// (b) loan repayments received / made, This Period.
    pub loan_repayments: f64,
    /// (c) subtotal: (a) + (b) in Column A; Calendar Year-to-Date in Column B.
    pub subtotal: DetailedSummaryRow,
}

/// Page 2, Detailed Summary Page
/// ([fecfrm4.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=2)).
#[derive(Debug, Clone, Serialize)]
pub struct Form4DetailedSummary {
    /// "RECEIPTS", Lines 13–20.
    pub receipts: Form4Receipts,
    /// "DISBURSEMENTS", Lines 21–25.
    pub disbursements: Form4Disbursements,
}

/// Detailed Summary Page, receipts (Lines 13–20)
/// ([fecfrm4i.pdf p2–3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
#[derive(Debug, Clone, Serialize)]
pub struct Form4Receipts {
    /// Line 13, federal funds: "receipts from the Presidential Election
    /// Campaign Fund (US Treasury)" (`col_a_federal_funds`,
    /// `col_b_federal_funds`)
    /// ([fecfrm4i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
    pub line13_federal_funds: DetailedSummaryRow,
    /// Line 14, contributions to defray convention expenses
    /// (`col_a_contributions_itemized`, `col_a_contributions_unitemized`,
    /// `col_a_contributions_subtotal`, `col_b_contributions_subtotal`)
    /// ([fecfrm4i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
    pub line14_contributions: Form4ItemizedLine,
    /// Line 15, transfers from affiliated committees, including loans and
    /// loan repayments received from them (`col_a_transfers_from_affiliated`,
    /// `col_b_transfers_from_affiliated`)
    /// ([fecfrm4i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
    pub line15_transfers_from_affiliated_committees: DetailedSummaryRow,
    /// Line 16, loans (a) and loan repayments (b) received, other than from
    /// affiliated committees (`col_a_loans_received`,
    /// `col_a_loan_repayments_received`, `col_a_loan_receipts_subtotal`,
    /// `col_b_loan_receipts_subtotal`)
    /// ([fecfrm4i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
    pub line16_loans_received: Form4LoanLine,
    /// Line 17, refunds, rebates and returns of deposits relating to
    /// convention expenditures (`col_a_convention_refunds_itemized`,
    /// `col_a_convention_refunds_unitemized`,
    /// `col_a_convention_refunds_subtotal`,
    /// `col_b_convention_refunds_subtotal`)
    /// ([fecfrm4i.pdf p2–3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=2)).
    pub line17_convention_refunds: Form4ItemizedLine,
    /// Line 18, other refunds, rebates and returns of deposits — those
    /// relating to "Other Disbursements" on Line 24
    /// (`col_a_other_refunds_itemized`, `col_a_other_refunds_unitemized`,
    /// `col_a_other_refunds_subtotal`, `col_b_other_refunds_subtotal`)
    /// ([fecfrm4i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=3)).
    pub line18_other_refunds: Form4ItemizedLine,
    /// Line 19, other income. "For convention committees this would include
    /// interest and dividends. For host committees other income would include
    /// contributions received to promote the city and its commerce"
    /// (`col_a_other_income_itemized`, `col_a_other_income_unitemized`,
    /// `col_a_other_income_subtotal`, `col_b_other_income_subtotal`)
    /// ([fecfrm4i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=3)).
    pub line19_other_income: Form4ItemizedLine,
    /// Line 20, total receipts: 13 + 14(c) + 15 + 16(c) + 17(c) + 18(c) +
    /// 19(c) (`col_a_total_receipts_TODO_DUP`, `col_b_total_receipts_TODO_DUP`)
    /// ([fecfrm4i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=3)).
    pub line20_total_receipts: DetailedSummaryRow,
}

/// Detailed Summary Page, disbursements (Lines 21–25)
/// ([fecfrm4i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=3)).
#[derive(Debug, Clone, Serialize)]
pub struct Form4Disbursements {
    /// Line 21, convention expenditures: "disbursements made to defray
    /// convention expenses" (`col_a_convention_expenses_itemized`,
    /// `col_a_convention_expenses_unitemized`,
    /// `col_a_convention_expenses_subtotal`,
    /// `col_b_convention_expenses_subtotal`)
    /// ([fecfrm4i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=3)).
    pub line21_convention_expenditures: Form4ItemizedLine,
    /// Line 22, transfers to affiliated committees, including loans and loan
    /// repayments made to them (`col_a_transfers_to_affiliated`,
    /// `col_b_transfers_to_affiliated`)
    /// ([fecfrm4i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=3)).
    pub line22_transfers_to_affiliated_committees: DetailedSummaryRow,
    /// Line 23, loans made (a) and loan repayments made (b), excluding
    /// transfers on Line 22 (`col_a_loans_made`, `col_a_loan_repayments_made`,
    /// `col_a_loan_disbursements_subtotal`, `col_b_loan_disbursements_subtotal`)
    /// ([fecfrm4i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=3)).
    pub line23_loans_made: Form4LoanLine,
    /// Line 24, other disbursements (`col_a_other_disbursements_itemized`,
    /// `col_a_other_disbursements_unitemized`,
    /// `col_a_other_disbursements_subtotal`,
    /// `col_b_other_disbursements_subtotal`)
    /// ([fecfrm4i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=3)).
    pub line24_other_disbursements: Form4ItemizedLine,
    /// Line 25, total disbursements: 21(c) + 22 + 23(c) + 24(c)
    /// (`col_a_total_disbursements_TODO_DUP`,
    /// `col_b_total_disbursements_TODO_DUP`)
    /// ([fecfrm4i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4i.pdf#page=3)).
    pub line25_total_disbursements: DetailedSummaryRow,
}

/// `{prefix}{name}_TODO_DUP` when the record has it (v6.1+), else the legacy
/// unsuffixed `{prefix}{name}`.
fn dup_key(data: &Data, prefix: &str, name: &str) -> String {
    let suffixed = format!("{prefix}{name}_TODO_DUP");
    if data.contains_key(&suffixed) {
        suffixed
    } else {
        format!("{prefix}{name}")
    }
}

fn row(data: &Data, name: &str) -> DetailedSummaryRow {
    DetailedSummaryRow::from_data(data, &format!("col_a_{name}"), &format!("col_b_{name}"))
}

fn itemized_line(data: &Data, name: &str) -> Form4ItemizedLine {
    Form4ItemizedLine {
        itemized: amount(data, &format!("col_a_{name}_itemized")),
        unitemized: amount(data, &format!("col_a_{name}_unitemized")),
        subtotal: row(data, &format!("{name}_subtotal")),
    }
}

impl Form4 {
    /// Build from a cover record's column map. `None` only if the record has
    /// no `committee_name` column at all.
    pub fn from_data(data: &Data) -> Option<Self> {
        data.get("committee_name")?;

        let mut treasurer = PersonName::from_data(data);
        if treasurer.is_empty() {
            if let Some(name) = text(data, "treasurer_name") {
                treasurer.last_name = name;
            }
        }

        let summary = Form4Summary {
            line6a_cash_on_hand_january_1: amount(data, "col_b_cash_on_hand_beginning_year"),
            line6a_year: text(data, "col_b_beginning_year"),
            line6b_cash_on_hand_beginning_period: amount(
                data,
                "col_a_cash_on_hand_beginning_reporting_period",
            ),
            line6c_total_receipts: row(data, "total_receipts"),
            line6d_subtotal: row(data, "subtotal"),
            line7_total_disbursements: row(data, "total_disbursements"),
            line8_cash_on_hand_close_of_period: row(data, "cash_on_hand_close_of_period"),
            line9_debts_owed_to_committee: amount(data, "col_a_debts_to"),
            line10_debts_owed_by_committee: amount(data, "col_a_debts_by"),
            line11_convention_expenditures: row(data, "convention_expenditures"),
            line12_convention_refunds: row(data, "convention_refunds"),
            line12a_expenditures_subject_to_limitation: row(data, "expenditures_subject_to_limits"),
            line12b_prior_years_expenditures_subject_to_limitation: DetailedSummaryRow::from_data(
                data,
                "col_a_prior_expenditures_subject_to_limits",
                "col_b_prior_expendiutres_subject_to_limits",
            ),
            line12c_total_expenditures_subject_to_limitation: amount(
                data,
                "col_b_total_expenditures_subject_to_limits",
            ),
        };

        let receipts = Form4Receipts {
            line13_federal_funds: row(data, "federal_funds"),
            line14_contributions: itemized_line(data, "contributions"),
            line15_transfers_from_affiliated_committees: row(data, "transfers_from_affiliated"),
            line16_loans_received: Form4LoanLine {
                loans: amount(data, "col_a_loans_received"),
                loan_repayments: amount(data, "col_a_loan_repayments_received"),
                subtotal: row(data, "loan_receipts_subtotal"),
            },
            line17_convention_refunds: itemized_line(data, "convention_refunds"),
            line18_other_refunds: itemized_line(data, "other_refunds"),
            line19_other_income: itemized_line(data, "other_income"),
            line20_total_receipts: DetailedSummaryRow::from_data(
                data,
                &dup_key(data, "col_a_", "total_receipts"),
                &dup_key(data, "col_b_", "total_receipts"),
            ),
        };

        let disbursements = Form4Disbursements {
            line21_convention_expenditures: itemized_line(data, "convention_expenses"),
            line22_transfers_to_affiliated_committees: row(data, "transfers_to_affiliated"),
            line23_loans_made: Form4LoanLine {
                loans: amount(data, "col_a_loans_made"),
                loan_repayments: amount(data, "col_a_loan_repayments_made"),
                subtotal: row(data, "loan_disbursements_subtotal"),
            },
            line24_other_disbursements: itemized_line(data, "other_disbursements"),
            line25_total_disbursements: DetailedSummaryRow::from_data(
                data,
                &dup_key(data, "col_a_", "total_disbursements"),
                &dup_key(data, "col_b_", "total_disbursements"),
            ),
        };

        Some(Self {
            committee_name: text_or_empty(data, "committee_name"),
            committee_id: text_or_empty(data, "filer_committee_id_number"),
            address: Address::from_prefixed(data, ""),
            committee_type: text(data, "committee_type"),
            committee_type_description: text(data, "committee_type_description"),
            report_code: text(data, "report_code"),
            coverage_from: date(data, "coverage_from_date"),
            coverage_through: date(data, "coverage_through_date"),
            treasurer,
            date_signed: date(data, "date_signed"),
            summary,
            detailed_summary: Form4DetailedSummary {
                receipts,
                disbursements,
            },
        })
    }

    /// The FEC's description of [`Form4::committee_type`]. The record codes
    /// are "A=arrange; H=host; O=other" (FEC format workbook v8.4, sheet `F4`,
    /// field 9); the labels are the three Line 3 boxes printed on the form
    /// ([fecfrm4.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=1)).
    pub fn committee_type_label(&self) -> Option<&'static str> {
        Some(match self.committee_type.as_deref()? {
            "A" => "Convention Committee",
            "H" => "Host Committee",
            "O" => "Other",
            _ => return None,
        })
    }

    /// The FEC's description of [`Form4::report_code`], for the codes Form 4
    /// accepts: `Q1; Q2; Q3; YE; TER; 60D` (FEC e-filing specifications v8.4,
    /// p15, "Accepted Report Codes by Type of Filing"; labels from p14,
    /// "Report Codes").
    pub fn report_code_label(&self) -> Option<&'static str> {
        Some(match self.report_code.as_deref()? {
            "Q1" => "April Quarterly",
            "Q2" => "July Quarterly",
            "Q3" => "October Quarterly",
            "YE" => "Year-end",
            "TER" => "Termination Report",
            "60D" => "60 Day Post Convention of a Presidential Host Committee",
            _ => return None,
        })
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
    fn legacy_layout_fallbacks() {
        // v3/v5 records: one `treasurer_name` column and unsuffixed Line 20/25 names.
        let d = data(&[
            ("committee_name", "HOST CMTE"),
            ("treasurer_name", "DOE, JANE"),
            ("col_a_total_receipts", "100.00"),
            ("col_b_total_receipts", "250.00"),
            ("col_a_total_disbursements", "40.00"),
        ]);
        let form = Form4::from_data(&d).unwrap();
        assert_eq!(form.treasurer.last_name, "DOE, JANE");
        let r = form.detailed_summary.receipts.line20_total_receipts;
        assert_eq!((r.column_a, r.column_b), (100.0, 250.0));
        assert_eq!(
            form.detailed_summary
                .disbursements
                .line25_total_disbursements
                .column_a,
            40.0
        );
    }
}
