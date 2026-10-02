//! Form 3X (PAC / party committee report) cover rendering.
//!
//! Renders straight from [`fec_parser::covers::Form3X`]: page 1 identification,
//! a cash-flow block, the Summary Page (Lines 6–10) and the three sections of
//! the Detailed Summary Page (Lines 11–38), with Column A ("This Period") and
//! Column B ("Year-to-Date", i.e. calendar year-to-date) side by side.

use super::layout::{bold, election_text, report_code_text, Columns, Doc};
use fec_parser::covers::Form3X;
use ratatui::text::Span;

/// Form 3X's column headings: Column A "This Period", Column B "Calendar
/// Year-to-Date".
const COLUMNS: Columns = Columns::Two("This Period", "Year-to-Date");

pub(super) fn append_f3x_content_lines(d: &mut Doc, data: &Form3X) {
    append_identification(d, data);
    let s = &data.summary;
    d.cash_flow(
        s.line6b_cash_on_hand_beginning_period,
        s.line6c_total_receipts.column_a,
        s.line7_total_disbursements.column_a,
        s.line8_cash_on_hand_close_of_period.column_a,
    );
    append_summary_page(d, data);
    append_receipts(d, data);
    append_disbursements(d, data);
    append_net(d, data);
}

/// Page 1: committee, address, report type, election, multicandidate status.
fn append_identification(d: &mut Doc, data: &Form3X) {
    let mut committee = vec![
        bold(data.committee_name.clone()),
        Span::raw(format!(" ({})", data.filer_committee_id)),
    ];
    if data.qualified_committee {
        committee.push(Span::raw(" - multicandidate committee"));
    }
    d.field_spans("Committee", committee);
    d.address("Address", &data.address, data.change_of_address);
    if let Some(code) = &data.report_code {
        d.field("Report", report_code_text(code, data.report_code_label()));
    }
    d.field_opt(
        "Election",
        election_text(
            data.election_code.as_deref(),
            data.election_code_label(),
            data.date_of_election,
            data.state_of_election.as_deref(),
        ),
    );
    d.blank();
}

/// Page 2, Summary Page (Lines 6–10).
fn append_summary_page(d: &mut Doc, data: &Form3X) {
    let s = &data.summary;
    d.heading("Summary Page");
    d.table(COLUMNS);
    let jan_1 = match s.line6a_year {
        Some(year) => format!("6(a) Cash on Hand January 1, {year}"),
        None => "6(a) Cash on Hand January 1".to_string(),
    };
    d.row_ab(&jan_1, None, Some(s.line6a_cash_on_hand_jan_1), false);
    d.row_ab(
        "6(b) Cash on Hand at Beginning of Period",
        Some(s.line6b_cash_on_hand_beginning_period),
        None,
        false,
    );
    d.row("6(c) Total Receipts", &s.line6c_total_receipts);
    d.row("6(d) Subtotal", &s.line6d_subtotal);
    d.row("7.   Total Disbursements", &s.line7_total_disbursements);
    d.total(
        "8.   Cash on Hand at Close of Period",
        &s.line8_cash_on_hand_close_of_period,
    );
    d.row_ab(
        "9.   Debts Owed TO the Committee",
        Some(s.line9_debts_owed_to_committee),
        None,
        false,
    );
    d.row_ab(
        "10.  Debts Owed BY the Committee",
        Some(s.line10_debts_owed_by_committee),
        None,
        false,
    );
    d.blank();
}

/// Detailed Summary Page, Section I (Lines 11–20).
fn append_receipts(d: &mut Doc, data: &Form3X) {
    let r = &data.detailed_summary.receipts;
    d.heading("Detailed Summary - I. Receipts");
    d.table(COLUMNS);
    d.caption("11.  Contributions (other than loans) from:");
    d.row(
        "  (a)(i)   Individuals, itemized",
        &r.line11a_i_individuals_itemized,
    );
    d.row(
        "  (a)(ii)  Individuals, unitemized",
        &r.line11a_ii_individuals_unitemized,
    );
    d.row(
        "  (a)(iii) Individuals, total",
        &r.line11a_iii_individuals_total,
    );
    d.row(
        "  (b) Political party committees",
        &r.line11b_political_party_committees,
    );
    d.row(
        "  (c) Other political committees",
        &r.line11c_other_political_committees,
    );
    d.total("  (d) Total contributions", &r.line11d_total_contributions);
    d.row(
        "12.  Transfers from affiliated/party cmtes",
        &r.line12_transfers_from_affiliated,
    );
    d.row("13.  All loans received", &r.line13_loans_received);
    d.row(
        "14.  Loan repayments received",
        &r.line14_loan_repayments_received,
    );
    d.row(
        "15.  Offsets to operating expenditures",
        &r.line15_offsets_to_operating_expenditures,
    );
    d.row(
        "16.  Refunds of contributions made",
        &r.line16_refunds_of_federal_contributions,
    );
    d.row(
        "17.  Other federal receipts",
        &r.line17_other_federal_receipts,
    );
    d.caption("18.  Transfers from nonfederal/Levin:");
    d.row(
        "  (a) Nonfederal account (H3)",
        &r.line18a_transfers_from_nonfederal_account,
    );
    d.row(
        "  (b) Levin funds (H5)",
        &r.line18b_transfers_from_levin_funds,
    );
    d.row(
        "  (c) Total transfers",
        &r.line18c_total_nonfederal_transfers,
    );
    d.total("19.  Total receipts", &r.line19_total_receipts);
    d.total(
        "20.  Total federal receipts",
        &r.line20_total_federal_receipts,
    );
    d.blank();
}

/// Detailed Summary Page, Section II (Lines 21–32).
fn append_disbursements(d: &mut Doc, data: &Form3X) {
    let ds = &data.detailed_summary.disbursements;
    d.heading("Detailed Summary - II. Disbursements");
    d.table(COLUMNS);
    d.caption("21.  Operating expenditures:");
    d.row(
        "  (a)(i)  Shared, federal share (H4)",
        &ds.line21a_i_shared_operating_federal_share,
    );
    d.row(
        "  (a)(ii) Shared, nonfederal share (H4)",
        &ds.line21a_ii_shared_operating_nonfederal_share,
    );
    d.row(
        "  (b) Other federal operating",
        &ds.line21b_other_federal_operating_expenditures,
    );
    d.total(
        "  (c) Total operating expenditures",
        &ds.line21c_total_operating_expenditures,
    );
    d.row(
        "22.  Transfers to affiliated/party cmtes",
        &ds.line22_transfers_to_affiliated,
    );
    d.row(
        "23.  Contributions to federal cands/cmtes",
        &ds.line23_contributions_to_federal_candidates,
    );
    d.row(
        "24.  Independent expenditures",
        &ds.line24_independent_expenditures,
    );
    d.row(
        "25.  Coordinated party expenditures",
        &ds.line25_coordinated_party_expenditures,
    );
    d.row("26.  Loan repayments made", &ds.line26_loan_repayments_made);
    d.row("27.  Loans made", &ds.line27_loans_made);
    d.caption("28.  Refunds of contributions to:");
    d.row(
        "  (a) Individuals/persons",
        &ds.line28a_refunds_to_individuals,
    );
    d.row(
        "  (b) Political party committees",
        &ds.line28b_refunds_to_party_committees,
    );
    d.row(
        "  (c) Other political committees",
        &ds.line28c_refunds_to_other_committees,
    );
    d.total(
        "  (d) Total contribution refunds",
        &ds.line28d_total_contribution_refunds,
    );
    d.row("29.  Other disbursements", &ds.line29_other_disbursements);
    d.caption("30.  Federal election activity:");
    d.row(
        "  (a)(i)  Shared, federal share (H6)",
        &ds.line30a_i_fea_federal_share,
    );
    d.row(
        "  (a)(ii) Shared, Levin share (H6)",
        &ds.line30a_ii_fea_levin_share,
    );
    d.row(
        "  (b) Paid entirely with federal funds",
        &ds.line30b_fea_all_federal,
    );
    d.total(
        "  (c) Total federal election activity",
        &ds.line30c_fea_total,
    );
    d.total("31.  Total disbursements", &ds.line31_total_disbursements);
    d.total(
        "32.  Total federal disbursements",
        &ds.line32_total_federal_disbursements,
    );
    d.blank();
}

/// Detailed Summary Page, Section III (Lines 33–38).
fn append_net(d: &mut Doc, data: &Form3X) {
    let n = &data.detailed_summary.net;
    d.heading("Detailed Summary - III. Net Contributions/Operating Expenditures");
    d.table(COLUMNS);
    d.row("33.  Total contributions", &n.line33_total_contributions);
    d.row(
        "34.  Total contribution refunds",
        &n.line34_total_contribution_refunds,
    );
    d.total("35.  Net contributions", &n.line35_net_contributions);
    d.row(
        "36.  Total federal operating expenditures",
        &n.line36_total_federal_operating_expenditures,
    );
    d.row(
        "37.  Offsets to operating expenditures",
        &n.line37_offsets_to_operating_expenditures,
    );
    d.total(
        "38.  Net operating expenditures",
        &n.line38_net_operating_expenditures,
    );
    d.blank();
}
