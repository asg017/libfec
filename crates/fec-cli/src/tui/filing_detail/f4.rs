//! Form 4 (convention / host committee) cover rendering: identification, then
//! the two-column Summary and Detailed Summary pages.

use super::layout::{code_with_label, report_code_text, Columns, Doc};
use fec_parser::covers::{Form4, Form4ItemizedLine, Form4LoanLine};

/// Form 4's column headings: Column A "This Period", Column B "Calendar
/// Year-to-Date" (fecfrm4.pdf p1-2).
const COLUMNS: Columns = Columns::Two("This Period", "Calendar YTD");

pub(super) fn append_f4_content_lines(d: &mut Doc, data: &Form4) {
    d.address("Address", &data.address, false);
    if let Some(ref code) = data.committee_type {
        let mut value = code_with_label(code, data.committee_type_label());
        if let Some(ref description) = data.committee_type_description {
            value.push_str(&format!(": {description}"));
        }
        d.field("Type", value);
    }
    if let Some(ref code) = data.report_code {
        d.field("Report", report_code_text(code, data.report_code_label()));
    }
    d.blank();

    append_summary(d, data);
    append_detailed_summary(d, data);
}

fn append_summary(d: &mut Doc, data: &Form4) {
    let s = &data.summary;

    d.heading("SECTION A - CASH BALANCE SUMMARY");
    d.table(COLUMNS);
    let jan_1 = match s.line6a_year {
        Some(year) => format!("6(a) Cash on hand Jan 1, {year}"),
        None => "6(a) Cash on hand Jan 1".to_string(),
    };
    d.row_ab(&jan_1, None, Some(s.line6a_cash_on_hand_jan_1), false);
    d.row_ab(
        "6(b) Cash on hand, beginning",
        Some(s.line6b_cash_on_hand_beginning_period),
        None,
        false,
    );
    d.row("6(c) Total receipts", &s.line6c_total_receipts);
    d.row("6(d) Subtotal", &s.line6d_subtotal);
    d.row("7.   Total disbursements", &s.line7_total_disbursements);
    d.total(
        "8.   Cash on hand at close",
        &s.line8_cash_on_hand_close_of_period,
    );
    d.row_ab(
        "9.   Debts owed TO committee",
        Some(s.line9_debts_owed_to_committee),
        None,
        false,
    );
    d.row_ab(
        "10.  Debts owed BY committee",
        Some(s.line10_debts_owed_by_committee),
        None,
        false,
    );
    d.blank();

    d.heading("SECTION B - EXPENDITURES SUBJECT TO LIMITATION");
    d.table(COLUMNS);
    d.row(
        "11.  Convention expenditures",
        &s.line11_convention_expenditures,
    );
    d.row("12.  Refunds re: convention", &s.line12_convention_refunds);
    d.row(
        "12(a) Subject to limitation",
        &s.line12a_expenditures_subject_to_limitation,
    );
    d.row(
        "12(b) Prior years, subject",
        &s.line12b_prior_years_expenditures_subject_to_limitation,
    );
    d.row_ab(
        "12(c) Total subject to limit",
        None,
        Some(s.line12c_total_expenditures_subject_to_limitation),
        true,
    );
    d.blank();
}

fn append_detailed_summary(d: &mut Doc, data: &Form4) {
    let r = &data.detailed_summary.receipts;
    let ds = &data.detailed_summary.disbursements;

    d.heading("DETAILED SUMMARY - RECEIPTS");
    d.table(COLUMNS);
    d.row("13.  Federal funds", &r.line13_federal_funds);
    push_itemized(d, "14.  Contributions", &r.line14_contributions);
    d.row(
        "15.  Transfers from affiliated",
        &r.line15_transfers_from_affiliated_committees,
    );
    push_loans(
        d,
        "16.  Loans received",
        "(a) Loans received",
        "(b) Repayments received",
        &r.line16_loans_received,
    );
    push_itemized(
        d,
        "17.  Refunds re: convention",
        &r.line17_convention_refunds,
    );
    push_itemized(d, "18.  Other refunds", &r.line18_other_refunds);
    push_itemized(d, "19.  Other income", &r.line19_other_income);
    d.total("20.  TOTAL RECEIPTS", &r.line20_total_receipts);
    d.blank();

    d.heading("DETAILED SUMMARY - DISBURSEMENTS");
    d.table(COLUMNS);
    push_itemized(
        d,
        "21.  Convention expenditures",
        &ds.line21_convention_expenditures,
    );
    d.row(
        "22.  Transfers to affiliated",
        &ds.line22_transfers_to_affiliated_committees,
    );
    push_loans(
        d,
        "23.  Loans made",
        "(a) Loans made",
        "(b) Repayments made",
        &ds.line23_loans_made,
    );
    push_itemized(
        d,
        "24.  Other disbursements",
        &ds.line24_other_disbursements,
    );
    d.total("25.  TOTAL DISBURSEMENTS", &ds.line25_total_disbursements);
    d.blank();
}

/// A heading line, then (a)/(b) in Column A only and the (c) subtotal in both.
fn push_itemized(d: &mut Doc, heading: &str, line: &Form4ItemizedLine) {
    d.caption(heading);
    d.row_ab("  (a) Itemized", Some(line.itemized), None, false);
    d.row_ab("  (b) Unitemized", Some(line.unitemized), None, false);
    d.row("  (c) Subtotal", &line.subtotal);
}

fn push_loans(d: &mut Doc, heading: &str, label_a: &str, label_b: &str, line: &Form4LoanLine) {
    d.caption(heading);
    d.row_ab(&format!("  {label_a}"), Some(line.loans), None, false);
    d.row_ab(
        &format!("  {label_b}"),
        Some(line.loan_repayments),
        None,
        false,
    );
    d.row("  (c) Subtotal", &line.subtotal);
}
