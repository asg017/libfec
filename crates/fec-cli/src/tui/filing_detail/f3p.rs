//! Form 3P (presidential authorized committee report) cover rendering.
//!
//! Same layout as Form 3 (`f3.rs`): identification, cash flow, then the
//! Summary and Detailed Summary pages, plus the primary expenditures by state.

use super::f3::CYCLE_COLUMNS;
use super::layout::{election_text, report_code_text, Columns, Doc};
use fec_parser::covers::Form3P;

pub struct FilingDetailF3P {
    pub form: Form3P,
}

impl From<&Form3P> for FilingDetailF3P {
    fn from(form: &Form3P) -> Self {
        Self { form: form.clone() }
    }
}

pub(super) fn append_f3p_content_lines(d: &mut Doc, data: &FilingDetailF3P) {
    let form = &data.form;
    render_identification(d, form);
    d.blank();

    let s = &form.summary;
    d.cash_flow(
        s.line6_cash_on_hand_beginning_period,
        s.line7_total_receipts,
        s.line9_total_disbursements,
        s.line10_cash_on_hand_end_period,
    );

    render_summary(d, form);
    d.blank();
    render_receipts(d, form);
    d.blank();
    render_disbursements(d, form);
    d.blank();
    render_state_allocations(d, form);
}

fn render_identification(d: &mut Doc, form: &Form3P) {
    d.address("Address", &form.address, form.change_of_address);
    let activity: Vec<&str> = [
        (form.activity_primary, "Primary"),
        (form.activity_general, "General"),
    ]
    .into_iter()
    .filter_map(|(checked, label)| checked.then_some(label))
    .collect();
    if !activity.is_empty() {
        d.field("Activity", activity.join(", "));
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

fn render_summary(d: &mut Doc, form: &Form3P) {
    let s = &form.summary;
    d.heading("SUMMARY");
    d.table(Columns::One);
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
        d.amount(label, value, bold);
    }
    d.caption("Net Election Cycle-to-Date:");
    d.amount(
        "14. Net Contributions",
        s.line14_net_contributions_other_than_loans,
        false,
    );
    d.amount(
        "15. Net Operating Expenditures",
        s.line15_net_operating_expenditures,
        false,
    );
}

fn render_receipts(d: &mut Doc, form: &Form3P) {
    let r = &form.detailed_summary.receipts;
    d.heading("I. RECEIPTS");
    d.table(CYCLE_COLUMNS);
    d.row("16. Federal Funds", &r.line16_federal_funds);
    d.caption("17. Contributions from:");
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
        d.row_ab(label, Some(row.column_a), Some(row.column_b), bold);
    }
    d.caption("19. Loans Received:");
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
        d.row_ab(label, Some(row.column_a), Some(row.column_b), bold);
    }
    d.caption("20. Offsets to Expenditures:");
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
        d.row_ab(label, Some(row.column_a), Some(row.column_b), bold);
    }
}

fn render_disbursements(d: &mut Doc, form: &Form3P) {
    let ds = &form.detailed_summary.disbursements;
    d.heading("II. DISBURSEMENTS");
    d.table(CYCLE_COLUMNS);
    let rows = [
        (
            "23. Operating Expenditures",
            &ds.line23_operating_expenditures,
            false,
        ),
        (
            "24. Transfers to Auth. Committees",
            &ds.line24_transfers_to_other_authorized_committees,
            false,
        ),
        (
            "25. Fundraising Disbursements",
            &ds.line25_fundraising_disbursements,
            false,
        ),
        (
            "26. Exempt Legal & Accounting",
            &ds.line26_exempt_legal_and_accounting_disbursements,
            false,
        ),
    ];
    for (label, row, bold) in rows {
        d.row_ab(label, Some(row.column_a), Some(row.column_b), bold);
    }
    d.caption("27. Loan Repayments Made:");
    let rows = [
        (
            "  (a) Of Candidate Loans",
            &ds.line27a_loan_repayments_candidate,
            false,
        ),
        (
            "  (b) Other Repayments",
            &ds.line27b_loan_repayments_other,
            false,
        ),
        (
            "  (c) Total Loan Repayments",
            &ds.line27c_loan_repayments_total,
            true,
        ),
    ];
    for (label, row, bold) in rows {
        d.row_ab(label, Some(row.column_a), Some(row.column_b), bold);
    }
    d.caption("28. Refunds of Contributions to:");
    let rows = [
        (
            "  (a) Individuals",
            &ds.line28a_refunds_to_individuals,
            false,
        ),
        (
            "  (b) Political Party Committees",
            &ds.line28b_refunds_to_political_party_committees,
            false,
        ),
        (
            "  (c) Other Political Committees",
            &ds.line28c_refunds_to_other_political_committees,
            false,
        ),
        ("  (d) Total Refunds", &ds.line28d_refunds_total, true),
        (
            "29. Other Disbursements",
            &ds.line29_other_disbursements,
            false,
        ),
        (
            "30. Total Disbursements",
            &ds.line30_total_disbursements,
            true,
        ),
    ];
    for (label, row, bold) in rows {
        d.row_ab(label, Some(row.column_a), Some(row.column_b), bold);
    }
    d.blank();
    d.heading("III. CONTRIBUTED ITEMS");
    d.table(Columns::One);
    d.amount(
        "31. Items on Hand to Be Liquidated",
        form.detailed_summary.line31_items_on_hand_to_be_liquidated,
        false,
    );
}

/// Pages 5–7: only states with a non-zero allocation, plus the totals row.
fn render_state_allocations(d: &mut Doc, form: &Form3P) {
    let alloc = &form.state_allocations;
    if alloc.is_empty() {
        return;
    }
    d.heading("PRIMARY EXPENDITURES BY STATE");
    d.table(Columns::Two("This Period", "To Date"));
    for state in &alloc.states {
        if state.allocation.column_a != 0.0 || state.allocation.column_b != 0.0 {
            d.row(state.state, &state.allocation);
        }
    }
    d.total("Totals", &alloc.totals);
    d.blank();
}
