//! Form 3 (House/Senate authorized committee report) cover rendering.
//!
//! Also hosts the two-column summary helpers shared with Form 3P (`f3p.rs`):
//! both forms print Column A "This Period" and Column B "Election
//! Cycle-to-Date" side by side.

use super::format_usd;
use fec_parser::covers::{DetailedSummaryRow, Form3};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub struct FilingDetailF3 {
    pub form: Form3,
}

impl From<&Form3> for FilingDetailF3 {
    fn from(form: &Form3) -> Self {
        Self { form: form.clone() }
    }
}

/// Width of each amount column, enough for `$999,999,999.99`.
const AMOUNT_WIDTH: usize = 16;

/// Label width for a two-column summary table rendered `width` columns wide.
fn label_width(width: u16) -> usize {
    (width as usize)
        .saturating_sub(2 * (AMOUNT_WIDTH + 1))
        .clamp(16, 44)
}

fn fit(label: &str, width: usize) -> String {
    if label.chars().count() <= width {
        format!("{label:<width$}")
    } else {
        let mut s: String = label.chars().take(width.saturating_sub(1)).collect();
        s.push('…');
        s
    }
}

fn amount_span(value: f64) -> Span<'static> {
    let style = if value == 0.0 {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };
    Span::styled(format!(" {:>AMOUNT_WIDTH$}", format_usd(value)), style)
}

fn label_style(bold: bool) -> Style {
    if bold {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    }
}

/// `Label: value` page-1 line.
pub(super) fn info_line(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(value),
    ])
}

/// Column headings of the Form 3/3P summary pages: Column A "This Period",
/// Column B "Election Cycle-to-Date".
pub(super) const CYCLE_COLUMNS: Option<(&str, &str)> = Some(("This Period", "Cycle-to-Date"));

/// A section heading, followed by column headings when given, e.g.
/// `I. RECEIPTS        This Period   Cycle-to-Date`.
pub(super) fn section_header(
    title: &str,
    width: u16,
    columns: Option<(&str, &str)>,
) -> Line<'static> {
    let heading = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let mut spans = vec![Span::styled(fit(title, label_width(width)), heading)];
    if let Some((a, b)) = columns {
        spans.push(Span::styled(format!(" {a:>AMOUNT_WIDTH$}"), heading));
        spans.push(Span::styled(format!(" {b:>AMOUNT_WIDTH$}"), heading));
    }
    Line::from(spans)
}

/// A line-group heading with no amounts, e.g. `11. Contributions from:`.
pub(super) fn group_line(label: &str) -> Line<'static> {
    Line::from(Span::styled(label.to_string(), label_style(false)))
}

/// Label span for a summary line. A label too long for the label column goes
/// on its own line, with the amounts on the next (`(label line, padded label)`).
fn label_spans(label: &str, width: u16, bold: bool) -> (Option<Line<'static>>, Span<'static>) {
    let lw = label_width(width);
    if label.chars().count() <= lw {
        (None, Span::styled(fit(label, lw), label_style(bold)))
    } else {
        (
            Some(Line::from(Span::styled(
                label.to_string(),
                label_style(bold),
            ))),
            Span::raw(" ".repeat(lw)),
        )
    }
}

/// One two-column line (two terminal lines if the label must wrap); `bold`
/// for totals.
pub(super) fn row_line(
    label: &str,
    row: &DetailedSummaryRow,
    width: u16,
    bold: bool,
) -> Vec<Line<'static>> {
    let (label_line, label) = label_spans(label, width, bold);
    label_line
        .into_iter()
        .chain([Line::from(vec![
            label,
            amount_span(row.column_a),
            amount_span(row.column_b),
        ])])
        .collect()
}

/// One single-amount line, aligned under the "This Period" column.
pub(super) fn amount_line(label: &str, value: f64, width: u16, bold: bool) -> Vec<Line<'static>> {
    let (label_line, label) = label_spans(label, width, bold);
    label_line
        .into_iter()
        .chain([Line::from(vec![label, amount_span(value)])])
        .collect()
}

/// Start / receipts / expenditures / end cash-flow block with the change in
/// cash on hand.
pub(super) fn cash_flow_lines(
    lines: &mut Vec<Line<'static>>,
    cash_begin: f64,
    receipts: f64,
    disbursements: f64,
    cash_end: f64,
) {
    let label_style = Style::default().fg(Color::White);
    let pct_change = if cash_begin == 0.0 {
        100.0
    } else {
        ((cash_end - cash_begin) / cash_begin) * 100.0
    };
    let amount_change = cash_end - cash_begin;
    let pct_color = if pct_change >= 0.0 {
        Color::Green
    } else {
        Color::Red
    };
    let pct_sign = if pct_change >= 0.0 { "+" } else { "" };

    lines.push(Line::from(vec![
        Span::styled(format!("{:<24}", "Cash on Hand - Start"), label_style),
        Span::styled(format!("{:>16}", format_usd(cash_begin)), label_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("{:<24}", "Receipts"), label_style),
        Span::styled(
            format!("+{:>15}", format_usd(receipts)),
            Style::default().fg(Color::Blue),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("{:<24}", "Expenditures"), label_style),
        Span::styled(
            format!("-{:>15}", format_usd(disbursements)),
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

/// Page-1 election details shared by Forms 3 and 3P.
pub(super) struct ElectionInfo<'a> {
    pub report_code: Option<&'a str>,
    pub election_code: Option<&'a str>,
    pub election_code_label: Option<&'a str>,
    pub election_date: Option<jiff::civil::Date>,
    pub state_of_election: Option<&'a str>,
}

/// "Report: 30G Post-General" plus, for election reports, "Election: General
/// (G2024) on 2024-11-05 in CA".
pub(super) fn report_lines(lines: &mut Vec<Line<'static>>, info: ElectionInfo) {
    if let Some(code) = info.report_code {
        lines.push(info_line(
            "Report",
            format!("{code} {}", fec_parser::report_code_label(code)),
        ));
    }
    let mut election = vec![];
    if let Some(code) = info.election_code {
        election.push(match info.election_code_label {
            Some(label) => format!("{label} ({code})"),
            None => code.to_string(),
        });
    }
    if let Some(date) = info.election_date {
        election.push(format!("on {date}"));
    }
    if let Some(state) = info.state_of_election {
        election.push(format!("in {state}"));
    }
    if !election.is_empty() {
        lines.push(info_line("Election", election.join(" ")));
    }
}

/// "Address: …", flagged when the change-of-address box is checked.
pub(super) fn address_line(
    lines: &mut Vec<Line<'static>>,
    address: &fec_parser::covers::Address,
    changed: bool,
) {
    if !address.is_empty() {
        let mut value = address.one_line();
        if changed {
            value.push_str(" (changed)");
        }
        lines.push(info_line("Address", value));
    }
}

pub fn append_f3_content_lines(lines: &mut Vec<Line<'static>>, data: &FilingDetailF3, width: u16) {
    let form = &data.form;
    render_identification(lines, form);
    lines.push(Line::from(""));

    let cash = &form.detailed_summary.cash_summary;
    cash_flow_lines(
        lines,
        cash.line23_cash_on_hand_beginning,
        cash.line24_total_receipts,
        cash.line26_total_disbursements,
        form.summary.line8_cash_on_hand_close_of_period,
    );

    render_summary(lines, form, width);
    lines.push(Line::from(""));
    render_receipts(lines, form, width);
    lines.push(Line::from(""));
    render_disbursements(lines, form, width);
    lines.push(Line::from(""));
    render_cash_summary(lines, form, width);
    lines.push(Line::from(""));
}

fn render_identification(lines: &mut Vec<Line<'static>>, form: &Form3) {
    address_line(lines, &form.address, form.change_of_address);
    match (&form.election_state, &form.election_district) {
        (Some(state), Some(district)) => {
            lines.push(info_line("State/District", format!("{state}-{district}")))
        }
        (Some(state), None) => lines.push(info_line("State", state.clone())),
        _ => {}
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

fn render_summary(lines: &mut Vec<Line<'static>>, form: &Form3, width: u16) {
    let s = &form.summary;
    lines.push(section_header("SUMMARY", width, CYCLE_COLUMNS));
    lines.push(group_line("6. Net Contributions (other than loans)"));
    lines.extend(row_line(
        "  (a) Total Contributions",
        &s.line6a_total_contributions,
        width,
        false,
    ));
    lines.extend(row_line(
        "  (b) Contribution Refunds",
        &s.line6b_total_contribution_refunds,
        width,
        false,
    ));
    lines.extend(row_line(
        "  (c) Net Contributions",
        &s.line6c_net_contributions,
        width,
        true,
    ));
    lines.push(group_line("7. Net Operating Expenditures"));
    lines.extend(row_line(
        "  (a) Operating Expenditures",
        &s.line7a_total_operating_expenditures,
        width,
        false,
    ));
    lines.extend(row_line(
        "  (b) Offsets to Operating Exp.",
        &s.line7b_total_offsets_to_operating_expenditures,
        width,
        false,
    ));
    lines.extend(row_line(
        "  (c) Net Operating Expenditures",
        &s.line7c_net_operating_expenditures,
        width,
        true,
    ));
    lines.extend(amount_line(
        "8. Cash on Hand at Close",
        s.line8_cash_on_hand_close_of_period,
        width,
        true,
    ));
    lines.extend(amount_line(
        "9. Debts Owed TO Committee",
        s.line9_debts_owed_to_committee,
        width,
        false,
    ));
    lines.extend(amount_line(
        "10. Debts Owed BY Committee",
        s.line10_debts_owed_by_committee,
        width,
        false,
    ));
}

fn render_receipts(lines: &mut Vec<Line<'static>>, form: &Form3, width: u16) {
    let r = &form.detailed_summary.receipts;
    lines.push(section_header("I. RECEIPTS", width, CYCLE_COLUMNS));
    lines.push(group_line("11. Contributions from:"));
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
        lines.extend(row_line(label, row, width, bold));
    }
    lines.push(group_line("13. Loans:"));
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
        lines.extend(row_line(label, row, width, bold));
    }
}

fn render_disbursements(lines: &mut Vec<Line<'static>>, form: &Form3, width: u16) {
    let d = &form.detailed_summary.disbursements;
    lines.push(section_header("II. DISBURSEMENTS", width, CYCLE_COLUMNS));
    lines.extend(row_line(
        "17. Operating Expenditures",
        &d.line17_operating_expenditures,
        width,
        false,
    ));
    lines.extend(row_line(
        "18. Transfers to Auth. Committees",
        &d.line18_transfers_to_authorized,
        width,
        false,
    ));
    lines.push(group_line("19. Loan Repayments:"));
    let rows = [
        (
            "  (a) Of Candidate Loans",
            &d.line19a_candidate_loan_repayments,
            false,
        ),
        (
            "  (b) Of All Other Loans",
            &d.line19b_other_loan_repayments,
            false,
        ),
        (
            "  (c) Total Loan Repayments",
            &d.line19c_total_loan_repayments,
            true,
        ),
    ];
    for (label, row, bold) in rows {
        lines.extend(row_line(label, row, width, bold));
    }
    lines.push(group_line("20. Refunds of Contributions to:"));
    let rows = [
        (
            "  (a) Individuals",
            &d.line20a_refunds_to_individuals,
            false,
        ),
        (
            "  (b) Political Party Committees",
            &d.line20b_refunds_to_party_committees,
            false,
        ),
        (
            "  (c) Other Political Committees",
            &d.line20c_refunds_to_other_committees,
            false,
        ),
        ("  (d) Total Refunds", &d.line20d_total_refunds, true),
        (
            "21. Other Disbursements",
            &d.line21_other_disbursements,
            false,
        ),
        (
            "22. Total Disbursements",
            &d.line22_total_disbursements,
            true,
        ),
    ];
    for (label, row, bold) in rows {
        lines.extend(row_line(label, row, width, bold));
    }
}

fn render_cash_summary(lines: &mut Vec<Line<'static>>, form: &Form3, width: u16) {
    let c = &form.detailed_summary.cash_summary;
    lines.push(section_header("III. CASH SUMMARY", width, None));
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
        lines.extend(amount_line(label, value, width, bold));
    }
}
