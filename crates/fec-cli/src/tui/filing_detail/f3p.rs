//! Form 3P (presidential authorized committee report) cover rendering.
//!
//! Reuses the two-column summary helpers from `f3.rs`.

use super::f3::{
    address_line, amount_line, cash_flow_lines, group_line, info_line, report_lines, row_line,
    section_header, ElectionInfo, CYCLE_COLUMNS,
};
use fec_parser::covers::Form3P;
use ratatui::text::Line;

pub struct FilingDetailF3P {
    pub form: Form3P,
}

impl From<&Form3P> for FilingDetailF3P {
    fn from(form: &Form3P) -> Self {
        Self { form: form.clone() }
    }
}

pub fn append_f3p_content_lines(
    lines: &mut Vec<Line<'static>>,
    data: &FilingDetailF3P,
    width: u16,
) {
    let form = &data.form;
    render_identification(lines, form);
    lines.push(Line::from(""));

    let s = &form.summary;
    cash_flow_lines(
        lines,
        s.line6_cash_on_hand_beginning_period,
        s.line7_total_receipts,
        s.line9_total_disbursements,
        s.line10_cash_on_hand_end_period,
    );

    render_summary(lines, form, width);
    lines.push(Line::from(""));
    render_receipts(lines, form, width);
    lines.push(Line::from(""));
    render_disbursements(lines, form, width);
    lines.push(Line::from(""));
    render_state_allocations(lines, form, width);
}

fn render_identification(lines: &mut Vec<Line<'static>>, form: &Form3P) {
    address_line(lines, &form.address, form.change_of_address);
    let activity: Vec<&str> = [
        (form.activity_primary, "Primary"),
        (form.activity_general, "General"),
    ]
    .into_iter()
    .filter_map(|(checked, label)| checked.then_some(label))
    .collect();
    if !activity.is_empty() {
        lines.push(info_line("Activity", activity.join(", ")));
    }
    report_lines(
        lines,
        ElectionInfo {
            report_code: form.report_code.as_deref(),
            election_code: form.election_code.as_deref(),
            election_code_label: form.election_code_label(),
            election_date: form.election_date,
            state_of_election: form.state_of_election.as_deref(),
        },
    );
}

fn render_summary(lines: &mut Vec<Line<'static>>, form: &Form3P, width: u16) {
    let s = &form.summary;
    lines.push(section_header("SUMMARY", width, None));
    let rows = [
        (
            "6. Cash on Hand, Beginning",
            s.line6_cash_on_hand_beginning_period,
            false,
        ),
        ("7. Total Receipts", s.line7_total_receipts, false),
        ("8. Subtotal", s.line8_subtotal, false),
        ("9. Total Disbursements", s.line9_total_disbursements, false),
        (
            "10. Cash on Hand, Close",
            s.line10_cash_on_hand_end_period,
            true,
        ),
        (
            "11. Debts Owed TO Committee",
            s.line11_debts_owed_to_committee,
            false,
        ),
        (
            "12. Debts Owed BY Committee",
            s.line12_debts_owed_by_committee,
            false,
        ),
        (
            "13. Exp. Subject to Limitation",
            s.line13_expenditures_subject_to_limits,
            false,
        ),
    ];
    for (label, value, bold) in rows {
        lines.extend(amount_line(label, value, width, bold));
    }
    lines.push(group_line("Net Election Cycle-to-Date:"));
    lines.extend(amount_line(
        "14. Net Contributions",
        s.line14_net_contributions_other_than_loans,
        width,
        false,
    ));
    lines.extend(amount_line(
        "15. Net Operating Expenditures",
        s.line15_net_operating_expenditures,
        width,
        false,
    ));
}

fn render_receipts(lines: &mut Vec<Line<'static>>, form: &Form3P, width: u16) {
    let r = &form.detailed_summary.receipts;
    lines.push(section_header("I. RECEIPTS", width, CYCLE_COLUMNS));
    lines.extend(row_line(
        "16. Federal Funds",
        &r.line16_federal_funds,
        width,
        false,
    ));
    lines.push(group_line("17. Contributions from:"));
    let rows = [
        (
            "  (a)(i) Individuals, Itemized",
            &r.line17a_i_contributions_from_individuals_itemized,
            false,
        ),
        (
            "  (a)(ii) Individuals, Unitemized",
            &r.line17a_ii_contributions_from_individuals_unitemized,
            false,
        ),
        (
            "  (a)(iii) Individuals, Total",
            &r.line17a_iii_contributions_from_individuals_total,
            false,
        ),
        (
            "  (b) Political Party Committees",
            &r.line17b_political_party_committees,
            false,
        ),
        (
            "  (c) Other Political Committees",
            &r.line17c_other_political_committees,
            false,
        ),
        ("  (d) The Candidate", &r.line17d_the_candidate, false),
        (
            "  (e) Total Contributions",
            &r.line17e_total_contributions,
            true,
        ),
        (
            "18. Transfers from Auth. Committees",
            &r.line18_transfers_from_other_authorized_committee,
            false,
        ),
    ];
    for (label, row, bold) in rows {
        lines.extend(row_line(label, row, width, bold));
    }
    lines.push(group_line("19. Loans Received:"));
    let rows = [
        (
            "  (a) From/Guaranteed by Candidate",
            &r.line19a_loans_received_from_or_guaranteed_by_candidate,
            false,
        ),
        ("  (b) Other Loans", &r.line19b_other_loans, false),
        ("  (c) Total Loans", &r.line19c_total_loans, true),
    ];
    for (label, row, bold) in rows {
        lines.extend(row_line(label, row, width, bold));
    }
    lines.push(group_line("20. Offsets to Expenditures:"));
    let rows = [
        (
            "  (a) Operating",
            &r.line20a_offsets_to_expenditures_operating,
            false,
        ),
        (
            "  (b) Fundraising",
            &r.line20b_offsets_to_expenditures_fundraising,
            false,
        ),
        (
            "  (c) Legal and Accounting",
            &r.line20c_offsets_to_expenditures_legal_and_accounting,
            false,
        ),
        (
            "  (d) Total Offsets",
            &r.line20d_offsets_to_expenditures_total,
            true,
        ),
        ("21. Other Receipts", &r.line21_other_receipts, false),
        ("22. Total Receipts", &r.line22_total_receipts, true),
    ];
    for (label, row, bold) in rows {
        lines.extend(row_line(label, row, width, bold));
    }
}

fn render_disbursements(lines: &mut Vec<Line<'static>>, form: &Form3P, width: u16) {
    let d = &form.detailed_summary.disbursements;
    lines.push(section_header("II. DISBURSEMENTS", width, CYCLE_COLUMNS));
    let rows = [
        (
            "23. Operating Expenditures",
            &d.line23_operating_expenditures,
            false,
        ),
        (
            "24. Transfers to Auth. Committees",
            &d.line24_transfers_to_other_authorized_committees,
            false,
        ),
        (
            "25. Fundraising Disbursements",
            &d.line25_fundraising_disbursements,
            false,
        ),
        (
            "26. Exempt Legal & Accounting",
            &d.line26_exempt_legal_and_accounting_disbursements,
            false,
        ),
    ];
    for (label, row, bold) in rows {
        lines.extend(row_line(label, row, width, bold));
    }
    lines.push(group_line("27. Loan Repayments Made:"));
    let rows = [
        (
            "  (a) Of Candidate Loans",
            &d.line27a_loan_repayments_candidate,
            false,
        ),
        (
            "  (b) Other Repayments",
            &d.line27b_loan_repayments_other,
            false,
        ),
        (
            "  (c) Total Loan Repayments",
            &d.line27c_loan_repayments_total,
            true,
        ),
    ];
    for (label, row, bold) in rows {
        lines.extend(row_line(label, row, width, bold));
    }
    lines.push(group_line("28. Refunds of Contributions to:"));
    let rows = [
        (
            "  (a) Individuals",
            &d.line28a_refunds_to_individuals,
            false,
        ),
        (
            "  (b) Political Party Committees",
            &d.line28b_refunds_to_political_party_committees,
            false,
        ),
        (
            "  (c) Other Political Committees",
            &d.line28c_refunds_to_other_political_committees,
            false,
        ),
        ("  (d) Total Refunds", &d.line28d_refunds_total, true),
        (
            "29. Other Disbursements",
            &d.line29_other_disbursements,
            false,
        ),
        (
            "30. Total Disbursements",
            &d.line30_total_disbursements,
            true,
        ),
    ];
    for (label, row, bold) in rows {
        lines.extend(row_line(label, row, width, bold));
    }
    lines.push(Line::from(""));
    lines.push(section_header("III. CONTRIBUTED ITEMS", width, None));
    lines.extend(amount_line(
        "31. Items on Hand to Be Liquidated",
        form.detailed_summary.line31_items_on_hand_to_be_liquidated,
        width,
        false,
    ));
}

/// Pages 5–7: only states with a non-zero allocation, plus the totals row.
fn render_state_allocations(lines: &mut Vec<Line<'static>>, form: &Form3P, width: u16) {
    let alloc = &form.state_allocations;
    if alloc.is_empty() {
        return;
    }
    lines.push(section_header(
        "PRIMARY EXPENDITURES BY STATE",
        width,
        Some(("This Period", "To Date")),
    ));
    for state in &alloc.states {
        if state.allocation.column_a != 0.0 || state.allocation.column_b != 0.0 {
            lines.extend(row_line(state.state, &state.allocation, width, false));
        }
    }
    lines.extend(row_line("Totals", &alloc.totals, width, true));
    lines.push(Line::from(""));
}
