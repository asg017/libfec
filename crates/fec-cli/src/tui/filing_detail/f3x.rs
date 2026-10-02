//! Form 3X (PAC / party committee report) cover rendering.
//!
//! Renders straight from [`fec_parser::covers::Form3X`]: page 1 identification,
//! a cash-flow block, the Summary Page (Lines 6–10) and the three sections of
//! the Detailed Summary Page (Lines 11–38), with Column A ("This Period") and
//! Column B ("Year-to-Date", i.e. calendar year-to-date) side by side.

use super::format_usd;
use fec_parser::covers::{DetailedSummaryRow, Form3X};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Width of each amount column, including one leading space.
const AMOUNT_WIDTH: usize = 15;
/// Widest the label column grows on wide terminals.
const MAX_LABEL_WIDTH: usize = 48;
/// Narrowest label column; below this, lines simply wrap.
const MIN_LABEL_WIDTH: usize = 24;

fn label_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

fn label_line(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(label.to_string(), label_style()),
        Span::raw(value),
    ])
}

fn section_header(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    ))
}

/// Pad or truncate (with `…`) `label` to exactly `width` characters.
fn fit(label: &str, width: usize) -> String {
    let len = label.chars().count();
    if len <= width {
        format!("{label:<width$}")
    } else {
        let mut s: String = label.chars().take(width.saturating_sub(1)).collect();
        s.push('…');
        s
    }
}

/// Lays out `label | This Period | Year-to-Date` rows for a given width.
struct Table {
    label_width: usize,
}

impl Table {
    fn new(width: u16) -> Self {
        let label_width = (width as usize)
            .saturating_sub(2 * AMOUNT_WIDTH)
            .clamp(MIN_LABEL_WIDTH, MAX_LABEL_WIDTH);
        Self { label_width }
    }

    fn headings(&self) -> Line<'static> {
        let style = Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::BOLD);
        Line::from(vec![
            Span::raw(" ".repeat(self.label_width)),
            Span::styled(format!("{:>AMOUNT_WIDTH$}", "This Period"), style),
            Span::styled(format!("{:>AMOUNT_WIDTH$}", "Year-to-Date"), style),
        ])
    }

    /// A row with optional values in each column (a column the form does
    /// not have for this line is left blank).
    fn row(&self, label: &str, a: Option<f64>, b: Option<f64>, total: bool) -> Line<'static> {
        let style = if total {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let cell = |v: Option<f64>| match v {
            Some(v) => format!("{:>AMOUNT_WIDTH$}", format_usd(v)),
            None => " ".repeat(AMOUNT_WIDTH),
        };
        Line::from(vec![
            Span::styled(fit(label, self.label_width), style),
            Span::styled(cell(a), style),
            Span::styled(cell(b), style),
        ])
    }

    fn two(&self, label: &str, r: &DetailedSummaryRow) -> Line<'static> {
        self.row(label, Some(r.column_a), Some(r.column_b), false)
    }

    fn total(&self, label: &str, r: &DetailedSummaryRow) -> Line<'static> {
        self.row(label, Some(r.column_a), Some(r.column_b), true)
    }

    /// A heading line inside a section that carries no amounts.
    fn caption(&self, label: &str) -> Line<'static> {
        Line::from(Span::raw(
            fit(label, self.label_width).trim_end().to_string(),
        ))
    }
}

pub fn append_f3x_content_lines(lines: &mut Vec<Line<'static>>, data: &Form3X, width: u16) {
    append_identification(lines, data);
    append_cash_flow(lines, data);

    let t = Table::new(width);
    append_summary_page(lines, data, &t);
    append_receipts(lines, data, &t);
    append_disbursements(lines, data, &t);
    append_net(lines, data, &t);
}

/// Page 1: committee, address, report type, election, multicandidate status.
fn append_identification(lines: &mut Vec<Line<'static>>, data: &Form3X) {
    let mut committee = format!("{} ({})", data.committee_name, data.filer_committee_id);
    if data.qualified_committee {
        committee.push_str(" - multicandidate committee");
    }
    lines.push(label_line("Committee: ", committee));

    if !data.address.is_empty() {
        let mut addr = data.address.one_line();
        if data.change_of_address {
            addr.push_str(" (changed)");
        }
        lines.push(label_line("Address: ", addr));
    }

    if let Some(code) = &data.report_code {
        let mut report = match data.report_code_label() {
            Some(label) => format!("{label} ({code})"),
            None => code.clone(),
        };
        report.push_str(if data.is_amendment() {
            ", amended"
        } else {
            ", new"
        });
        lines.push(label_line("Report: ", report));
    }

    if data.election_code.is_some() || data.date_of_election.is_some() {
        let mut parts = vec![];
        if let Some(label) = data.election_code_label() {
            parts.push(format!("{label} election"));
        } else {
            parts.push("Election".to_string());
        }
        if let Some(d) = data.date_of_election {
            parts.push(format!("on {d}"));
        }
        if let Some(state) = &data.state_of_election {
            parts.push(format!("in {state}"));
        }
        if let Some(code) = &data.election_code {
            parts.push(format!("({code})"));
        }
        lines.push(label_line("Election: ", parts.join(" ")));
    }
    lines.push(Line::from(""));
}

/// Cash on hand start → receipts → disbursements → end, for this period.
fn append_cash_flow(lines: &mut Vec<Line<'static>>, data: &Form3X) {
    let s = &data.summary;
    let label_style = Style::default().fg(Color::White);
    let cash_begin = s.line6b_cash_on_hand_beginning_period;
    let cash_end = s.line8_cash_on_hand_close_of_period.column_a;
    let pct_change = if cash_begin == 0.0 {
        100.0
    } else {
        ((cash_end - cash_begin) / cash_begin) * 100.0
    };
    let amount_change = cash_end - cash_begin;
    let (pct_color, pct_sign) = if pct_change >= 0.0 {
        (Color::Green, "+")
    } else {
        (Color::Red, "")
    };

    lines.push(Line::from(vec![
        Span::styled(format!("{:<24}", "Cash on Hand - Start"), label_style),
        Span::styled(format!("{:>16}", format_usd(cash_begin)), label_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("{:<24}", "Receipts"), label_style),
        Span::styled(
            format!("+{:>15}", format_usd(s.line6c_total_receipts.column_a)),
            Style::default().fg(Color::Blue),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("{:<24}", "Disbursements"), label_style),
        Span::styled(
            format!("-{:>15}", format_usd(s.line7_total_disbursements.column_a)),
            Style::default().fg(Color::Red),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("{:<24}", "Cash on Hand - End"), label_style),
        Span::styled(
            format!("{:>16}", format_usd(cash_end)),
            Style::default().fg(Color::White).bold(),
        ),
        Span::styled(
            format!(
                " {}{}, {}{:.0}%",
                pct_sign,
                format_usd(amount_change),
                pct_sign,
                pct_change
            ),
            Style::default().fg(pct_color).add_modifier(Modifier::DIM),
        ),
    ]));
    lines.push(Line::from(""));
}

/// Page 2, Summary Page (Lines 6–10).
fn append_summary_page(lines: &mut Vec<Line<'static>>, data: &Form3X, t: &Table) {
    let s = &data.summary;
    lines.push(section_header("Summary Page"));
    lines.push(t.headings());
    let jan_1 = match s.line6a_year {
        Some(year) => format!("6(a) Cash on Hand January 1, {year}"),
        None => "6(a) Cash on Hand January 1".to_string(),
    };
    lines.push(t.row(&jan_1, None, Some(s.line6a_cash_on_hand_jan_1), false));
    lines.push(t.row(
        "6(b) Cash on Hand at Beginning of Period",
        Some(s.line6b_cash_on_hand_beginning_period),
        None,
        false,
    ));
    lines.push(t.two("6(c) Total Receipts", &s.line6c_total_receipts));
    lines.push(t.two("6(d) Subtotal", &s.line6d_subtotal));
    lines.push(t.two("7.   Total Disbursements", &s.line7_total_disbursements));
    lines.push(t.total(
        "8.   Cash on Hand at Close of Period",
        &s.line8_cash_on_hand_close_of_period,
    ));
    lines.push(t.row(
        "9.   Debts Owed TO the Committee",
        Some(s.line9_debts_owed_to_committee),
        None,
        false,
    ));
    lines.push(t.row(
        "10.  Debts Owed BY the Committee",
        Some(s.line10_debts_owed_by_committee),
        None,
        false,
    ));
    lines.push(Line::from(""));
}

/// Detailed Summary Page, Section I (Lines 11–20).
fn append_receipts(lines: &mut Vec<Line<'static>>, data: &Form3X, t: &Table) {
    let r = &data.detailed_summary.receipts;
    lines.push(section_header("Detailed Summary - I. Receipts"));
    lines.push(t.headings());
    lines.push(t.caption("11.  Contributions (other than loans) from:"));
    lines.push(t.two(
        "  (a)(i)   Individuals, itemized",
        &r.line11a_i_individuals_itemized,
    ));
    lines.push(t.two(
        "  (a)(ii)  Individuals, unitemized",
        &r.line11a_ii_individuals_unitemized,
    ));
    lines.push(t.two(
        "  (a)(iii) Individuals, total",
        &r.line11a_iii_individuals_total,
    ));
    lines.push(t.two(
        "  (b) Political party committees",
        &r.line11b_political_party_committees,
    ));
    lines.push(t.two(
        "  (c) Other political committees",
        &r.line11c_other_political_committees,
    ));
    lines.push(t.total("  (d) Total contributions", &r.line11d_total_contributions));
    lines.push(t.two(
        "12.  Transfers from affiliated/party cmtes",
        &r.line12_transfers_from_affiliated,
    ));
    lines.push(t.two("13.  All loans received", &r.line13_loans_received));
    lines.push(t.two(
        "14.  Loan repayments received",
        &r.line14_loan_repayments_received,
    ));
    lines.push(t.two(
        "15.  Offsets to operating expenditures",
        &r.line15_offsets_to_operating_expenditures,
    ));
    lines.push(t.two(
        "16.  Refunds of contributions made",
        &r.line16_refunds_of_federal_contributions,
    ));
    lines.push(t.two(
        "17.  Other federal receipts",
        &r.line17_other_federal_receipts,
    ));
    lines.push(t.caption("18.  Transfers from nonfederal/Levin:"));
    lines.push(t.two(
        "  (a) Nonfederal account (H3)",
        &r.line18a_transfers_from_nonfederal_account,
    ));
    lines.push(t.two(
        "  (b) Levin funds (H5)",
        &r.line18b_transfers_from_levin_funds,
    ));
    lines.push(t.two(
        "  (c) Total transfers",
        &r.line18c_total_nonfederal_transfers,
    ));
    lines.push(t.total("19.  Total receipts", &r.line19_total_receipts));
    lines.push(t.total(
        "20.  Total federal receipts",
        &r.line20_total_federal_receipts,
    ));
    lines.push(Line::from(""));
}

/// Detailed Summary Page, Section II (Lines 21–32).
fn append_disbursements(lines: &mut Vec<Line<'static>>, data: &Form3X, t: &Table) {
    let d = &data.detailed_summary.disbursements;
    lines.push(section_header("Detailed Summary - II. Disbursements"));
    lines.push(t.headings());
    lines.push(t.caption("21.  Operating expenditures:"));
    lines.push(t.two(
        "  (a)(i)  Shared, federal share (H4)",
        &d.line21a_i_shared_operating_federal_share,
    ));
    lines.push(t.two(
        "  (a)(ii) Shared, nonfederal share (H4)",
        &d.line21a_ii_shared_operating_nonfederal_share,
    ));
    lines.push(t.two(
        "  (b) Other federal operating",
        &d.line21b_other_federal_operating_expenditures,
    ));
    lines.push(t.total(
        "  (c) Total operating expenditures",
        &d.line21c_total_operating_expenditures,
    ));
    lines.push(t.two(
        "22.  Transfers to affiliated/party cmtes",
        &d.line22_transfers_to_affiliated,
    ));
    lines.push(t.two(
        "23.  Contributions to federal cands/cmtes",
        &d.line23_contributions_to_federal_candidates,
    ));
    lines.push(t.two(
        "24.  Independent expenditures",
        &d.line24_independent_expenditures,
    ));
    lines.push(t.two(
        "25.  Coordinated party expenditures",
        &d.line25_coordinated_party_expenditures,
    ));
    lines.push(t.two("26.  Loan repayments made", &d.line26_loan_repayments_made));
    lines.push(t.two("27.  Loans made", &d.line27_loans_made));
    lines.push(t.caption("28.  Refunds of contributions to:"));
    lines.push(t.two(
        "  (a) Individuals/persons",
        &d.line28a_refunds_to_individuals,
    ));
    lines.push(t.two(
        "  (b) Political party committees",
        &d.line28b_refunds_to_party_committees,
    ));
    lines.push(t.two(
        "  (c) Other political committees",
        &d.line28c_refunds_to_other_committees,
    ));
    lines.push(t.total(
        "  (d) Total contribution refunds",
        &d.line28d_total_contribution_refunds,
    ));
    lines.push(t.two("29.  Other disbursements", &d.line29_other_disbursements));
    lines.push(t.caption("30.  Federal election activity:"));
    lines.push(t.two(
        "  (a)(i)  Shared, federal share (H6)",
        &d.line30a_i_fea_federal_share,
    ));
    lines.push(t.two(
        "  (a)(ii) Shared, Levin share (H6)",
        &d.line30a_ii_fea_levin_share,
    ));
    lines.push(t.two(
        "  (b) Paid entirely with federal funds",
        &d.line30b_fea_all_federal,
    ));
    lines.push(t.total(
        "  (c) Total federal election activity",
        &d.line30c_fea_total,
    ));
    lines.push(t.total("31.  Total disbursements", &d.line31_total_disbursements));
    lines.push(t.total(
        "32.  Total federal disbursements",
        &d.line32_total_federal_disbursements,
    ));
    lines.push(Line::from(""));
}

/// Detailed Summary Page, Section III (Lines 33–38).
fn append_net(lines: &mut Vec<Line<'static>>, data: &Form3X, t: &Table) {
    let n = &data.detailed_summary.net;
    lines.push(section_header(
        "Detailed Summary - III. Net Contributions/Operating Expenditures",
    ));
    lines.push(t.headings());
    lines.push(t.two("33.  Total contributions", &n.line33_total_contributions));
    lines.push(t.two(
        "34.  Total contribution refunds",
        &n.line34_total_contribution_refunds,
    ));
    lines.push(t.total("35.  Net contributions", &n.line35_net_contributions));
    lines.push(t.two(
        "36.  Total federal operating expenditures",
        &n.line36_total_federal_operating_expenditures,
    ));
    lines.push(t.two(
        "37.  Offsets to operating expenditures",
        &n.line37_offsets_to_operating_expenditures,
    ));
    lines.push(t.total(
        "38.  Net operating expenditures",
        &n.line38_net_operating_expenditures,
    ));
    lines.push(Line::from(""));
}
