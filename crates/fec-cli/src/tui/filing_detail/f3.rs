//! Form 3 (House/Senate authorized committee report) cover rendering:
//! identification, a cash-flow block, then the Summary and Detailed Summary
//! pages with Column A "This Period" and Column B "Election Cycle-to-Date".

use super::layout::{election_text, report_code_text, Columns, Doc};
use fec_parser::covers::Form3;

pub struct FilingDetailF3 {
    pub form: Form3,
}

impl From<&Form3> for FilingDetailF3 {
    fn from(form: &Form3) -> Self {
        Self { form: form.clone() }
    }
}

/// Column headings of the Form 3/3P summary pages: Column A "This Period",
/// Column B "Election Cycle-to-Date".
pub(super) const CYCLE_COLUMNS: Columns = Columns::Two("This Period", "Cycle-to-Date");

pub(super) fn append_f3_content_lines(d: &mut Doc, data: &FilingDetailF3) {
    let form = &data.form;
    render_identification(d, form);
    d.blank();

    let cash = &form.detailed_summary.cash_summary;
    d.cash_flow(
        cash.line23_cash_on_hand_beginning,
        cash.line24_total_receipts,
        cash.line26_total_disbursements,
        form.summary.line8_cash_on_hand_close_of_period,
    );

    render_summary(d, form);
    d.blank();
    render_receipts(d, form);
    d.blank();
    render_disbursements(d, form);
    d.blank();
    render_cash_summary(d, form);
    d.blank();
}

fn render_identification(d: &mut Doc, form: &Form3) {
    d.address("Address", &form.address, form.change_of_address);
    match (&form.election_state, &form.election_district) {
        (Some(state), Some(district)) => d.field("State/District", format!("{state}-{district}")),
        (Some(state), None) => d.field("State", state.clone()),
        _ => {}
    }
    if let Some(ref code) = form.report_code {
        d.field("Report", report_code_text(code, None));
    }
    d.field_opt(
        "Election",
        election_text(
            form.election_code.as_deref(),
            form.election_code_label(),
            form.election_date,
            form.state_of_election.as_deref(),
        ),
    );
}

fn render_summary(d: &mut Doc, form: &Form3) {
    let s = &form.summary;
    d.heading("SUMMARY");
    d.table(CYCLE_COLUMNS);
    d.caption("6. Net Contributions (other than loans)");
    d.row("  (a) Total Contributions", &s.line6a_total_contributions);
    d.row(
        "  (b) Contribution Refunds",
        &s.line6b_total_contribution_refunds,
    );
    d.total("  (c) Net Contributions", &s.line6c_net_contributions);
    d.caption("7. Net Operating Expenditures");
    d.row(
        "  (a) Operating Expenditures",
        &s.line7a_total_operating_expenditures,
    );
    d.row(
        "  (b) Offsets to Operating Exp.",
        &s.line7b_total_offsets_to_operating_expenditures,
    );
    d.total(
        "  (c) Net Operating Expenditures",
        &s.line7c_net_operating_expenditures,
    );
    d.amount(
        "8. Cash on Hand at Close",
        s.line8_cash_on_hand_close_of_period,
        true,
    );
    d.amount(
        "9. Debts Owed TO Committee",
        s.line9_debts_owed_to_committee,
        false,
    );
    d.amount(
        "10. Debts Owed BY Committee",
        s.line10_debts_owed_by_committee,
        false,
    );
}

fn render_receipts(d: &mut Doc, form: &Form3) {
    let r = &form.detailed_summary.receipts;
    d.heading("I. RECEIPTS");
    d.table(CYCLE_COLUMNS);
    d.caption("11. Contributions from:");
    let rows = [
        (
            "  (a)(i) Individuals, Itemized",
            &r.line11a_i_contributions_from_individuals_itemized,
            false,
        ),
        (
            "  (a)(ii) Individuals, Unitemized",
            &r.line11a_ii_contributions_from_individuals_unitemized,
            false,
        ),
        (
            "  (a)(iii) Individuals, Total",
            &r.line11a_iii_contributions_from_individuals_total,
            false,
        ),
        (
            "  (b) Political Party Committees",
            &r.line11b_political_party_committees,
            false,
        ),
        (
            "  (c) Other Political Committees",
            &r.line11c_other_political_committees_pacs,
            false,
        ),
        ("  (d) The Candidate", &r.line11d_the_candidate, false),
        (
            "  (e) Total Contributions",
            &r.line11e_total_contributions,
            true,
        ),
        (
            "12. Transfers from Auth. Committees",
            &r.line12_transfers_from_authorized,
            false,
        ),
    ];
    for (label, row, bold) in rows {
        d.row_ab(label, Some(row.column_a), Some(row.column_b), bold);
    }
    d.caption("13. Loans:");
    let rows = [
        (
            "  (a) Made/Guaranteed by Candidate",
            &r.line13a_loans_from_candidate,
            false,
        ),
        ("  (b) All Other Loans", &r.line13b_other_loans, false),
        ("  (c) Total Loans", &r.line13c_total_loans, true),
        (
            "14. Offsets to Operating Exp.",
            &r.line14_offset_to_operating_expenditures,
            false,
        ),
        ("15. Other Receipts", &r.line15_other_receipts, false),
        ("16. Total Receipts", &r.line16_total_receipts, true),
    ];
    for (label, row, bold) in rows {
        d.row_ab(label, Some(row.column_a), Some(row.column_b), bold);
    }
}

fn render_disbursements(d: &mut Doc, form: &Form3) {
    let ds = &form.detailed_summary.disbursements;
    d.heading("II. DISBURSEMENTS");
    d.table(CYCLE_COLUMNS);
    d.row(
        "17. Operating Expenditures",
        &ds.line17_operating_expenditures,
    );
    d.row(
        "18. Transfers to Auth. Committees",
        &ds.line18_transfers_to_authorized,
    );
    d.caption("19. Loan Repayments:");
    let rows = [
        (
            "  (a) Of Candidate Loans",
            &ds.line19a_candidate_loan_repayments,
            false,
        ),
        (
            "  (b) Of All Other Loans",
            &ds.line19b_other_loan_repayments,
            false,
        ),
        (
            "  (c) Total Loan Repayments",
            &ds.line19c_total_loan_repayments,
            true,
        ),
    ];
    for (label, row, bold) in rows {
        d.row_ab(label, Some(row.column_a), Some(row.column_b), bold);
    }
    d.caption("20. Refunds of Contributions to:");
    let rows = [
        (
            "  (a) Individuals",
            &ds.line20a_refunds_to_individuals,
            false,
        ),
        (
            "  (b) Political Party Committees",
            &ds.line20b_refunds_to_party_committees,
            false,
        ),
        (
            "  (c) Other Political Committees",
            &ds.line20c_refunds_to_other_committees,
            false,
        ),
        ("  (d) Total Refunds", &ds.line20d_total_refunds, true),
        (
            "21. Other Disbursements",
            &ds.line21_other_disbursements,
            false,
        ),
        (
            "22. Total Disbursements",
            &ds.line22_total_disbursements,
            true,
        ),
    ];
    for (label, row, bold) in rows {
        d.row_ab(label, Some(row.column_a), Some(row.column_b), bold);
    }
}

fn render_cash_summary(d: &mut Doc, form: &Form3) {
    let c = &form.detailed_summary.cash_summary;
    d.heading("III. CASH SUMMARY");
    d.table(Columns::One);
    let rows = [
        (
            "23. Cash on Hand, Beginning",
            c.line23_cash_on_hand_beginning,
            false,
        ),
        ("24. Total Receipts", c.line24_total_receipts, false),
        ("25. Subtotal", c.line25_subtotal, false),
        (
            "26. Total Disbursements",
            c.line26_total_disbursements,
            false,
        ),
        ("27. Cash on Hand, Close", c.line27_cash_on_hand_close, true),
    ];
    for (label, value, bold) in rows {
        d.amount(label, value, bold);
    }
}
