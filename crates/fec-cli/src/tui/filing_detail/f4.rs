//! Form 4 (convention / host committee) cover rendering: identification, then
//! the two-column Summary and Detailed Summary pages.

use super::misc_helpers::{
    code_with_label, field_line, push_address, row_line, section_line, two_column_header,
    two_column_line,
};
use fec_parser::covers::{Form4, Form4ItemizedLine, Form4LoanLine};
use ratatui::text::Line;

/// Form 4's column headings: Column A "This Period", Column B "Calendar
/// Year-to-Date" (fecfrm4.pdf p1-2).
const COLUMN_A: &str = "This Period";
const COLUMN_B: &str = "Calendar YTD";

pub fn append_f4_content_lines(lines: &mut Vec<Line<'static>>, data: &Form4) {
    push_address(lines, &data.address);
    if let Some(ref code) = data.committee_type {
        let mut value = code_with_label(code, data.committee_type_label());
        if let Some(ref description) = data.committee_type_description {
            value.push_str(&format!(": {description}"));
        }
        lines.push(field_line("Type", value));
    }
    if let Some(ref code) = data.report_code {
        lines.push(field_line(
            "Report",
            code_with_label(code, data.report_code_label()),
        ));
    }
    lines.push(Line::from(""));

    append_summary(lines, data);
    append_detailed_summary(lines, data);
}

fn append_summary(lines: &mut Vec<Line<'static>>, data: &Form4) {
    let s = &data.summary;

    lines.push(section_line("SECTION A - CASH BALANCE SUMMARY"));
    lines.push(two_column_header(COLUMN_A, COLUMN_B));
    let jan_1 = match s.line6a_year.as_deref() {
        Some(year) => format!("6(a) Cash on hand Jan 1, {year}"),
        None => "6(a) Cash on hand Jan 1".to_string(),
    };
    lines.push(two_column_line(
        &jan_1,
        None,
        Some(s.line6a_cash_on_hand_january_1),
        false,
    ));
    lines.push(two_column_line(
        "6(b) Cash on hand, beginning",
        Some(s.line6b_cash_on_hand_beginning_period),
        None,
        false,
    ));
    lines.push(row_line(
        "6(c) Total receipts",
        &s.line6c_total_receipts,
        false,
    ));
    lines.push(row_line("6(d) Subtotal", &s.line6d_subtotal, false));
    lines.push(row_line(
        "7.   Total disbursements",
        &s.line7_total_disbursements,
        false,
    ));
    lines.push(row_line(
        "8.   Cash on hand at close",
        &s.line8_cash_on_hand_close_of_period,
        true,
    ));
    lines.push(two_column_line(
        "9.   Debts owed TO committee",
        Some(s.line9_debts_owed_to_committee),
        None,
        false,
    ));
    lines.push(two_column_line(
        "10.  Debts owed BY committee",
        Some(s.line10_debts_owed_by_committee),
        None,
        false,
    ));
    lines.push(Line::from(""));

    lines.push(section_line(
        "SECTION B - EXPENDITURES SUBJECT TO LIMITATION",
    ));
    lines.push(two_column_header(COLUMN_A, COLUMN_B));
    lines.push(row_line(
        "11.  Convention expenditures",
        &s.line11_convention_expenditures,
        false,
    ));
    lines.push(row_line(
        "12.  Refunds re: convention",
        &s.line12_convention_refunds,
        false,
    ));
    lines.push(row_line(
        "12(a) Subject to limitation",
        &s.line12a_expenditures_subject_to_limitation,
        false,
    ));
    lines.push(row_line(
        "12(b) Prior years, subject",
        &s.line12b_prior_years_expenditures_subject_to_limitation,
        false,
    ));
    lines.push(two_column_line(
        "12(c) Total subject to limit",
        None,
        Some(s.line12c_total_expenditures_subject_to_limitation),
        true,
    ));
    lines.push(Line::from(""));
}

fn append_detailed_summary(lines: &mut Vec<Line<'static>>, data: &Form4) {
    let r = &data.detailed_summary.receipts;
    let d = &data.detailed_summary.disbursements;

    lines.push(section_line("DETAILED SUMMARY - RECEIPTS"));
    lines.push(two_column_header(COLUMN_A, COLUMN_B));
    lines.push(row_line(
        "13.  Federal funds",
        &r.line13_federal_funds,
        false,
    ));
    push_itemized(lines, "14.  Contributions", &r.line14_contributions);
    lines.push(row_line(
        "15.  Transfers from affiliated",
        &r.line15_transfers_from_affiliated_committees,
        false,
    ));
    push_loans(
        lines,
        "16.  Loans received",
        "(a) Loans received",
        "(b) Repayments received",
        &r.line16_loans_received,
    );
    push_itemized(
        lines,
        "17.  Refunds re: convention",
        &r.line17_convention_refunds,
    );
    push_itemized(lines, "18.  Other refunds", &r.line18_other_refunds);
    push_itemized(lines, "19.  Other income", &r.line19_other_income);
    lines.push(row_line(
        "20.  TOTAL RECEIPTS",
        &r.line20_total_receipts,
        true,
    ));
    lines.push(Line::from(""));

    lines.push(section_line("DETAILED SUMMARY - DISBURSEMENTS"));
    lines.push(two_column_header(COLUMN_A, COLUMN_B));
    push_itemized(
        lines,
        "21.  Convention expenditures",
        &d.line21_convention_expenditures,
    );
    lines.push(row_line(
        "22.  Transfers to affiliated",
        &d.line22_transfers_to_affiliated_committees,
        false,
    ));
    push_loans(
        lines,
        "23.  Loans made",
        "(a) Loans made",
        "(b) Repayments made",
        &d.line23_loans_made,
    );
    push_itemized(
        lines,
        "24.  Other disbursements",
        &d.line24_other_disbursements,
    );
    lines.push(row_line(
        "25.  TOTAL DISBURSEMENTS",
        &d.line25_total_disbursements,
        true,
    ));
    lines.push(Line::from(""));
}

/// A heading line, then (a)/(b) in Column A only and the (c) subtotal in both.
fn push_itemized(lines: &mut Vec<Line<'static>>, heading: &str, line: &Form4ItemizedLine) {
    lines.push(Line::from(heading.to_string()));
    lines.push(two_column_line(
        "      (a) Itemized",
        Some(line.itemized),
        None,
        false,
    ));
    lines.push(two_column_line(
        "      (b) Unitemized",
        Some(line.unitemized),
        None,
        false,
    ));
    lines.push(row_line("      (c) Subtotal", &line.subtotal, false));
}

fn push_loans(
    lines: &mut Vec<Line<'static>>,
    heading: &str,
    label_a: &str,
    label_b: &str,
    line: &Form4LoanLine,
) {
    lines.push(Line::from(heading.to_string()));
    lines.push(two_column_line(
        &format!("      {label_a}"),
        Some(line.loans),
        None,
        false,
    ));
    lines.push(two_column_line(
        &format!("      {label_b}"),
        Some(line.loan_repayments),
        None,
        false,
    ));
    lines.push(row_line("      (c) Subtotal", &line.subtotal, false));
}
