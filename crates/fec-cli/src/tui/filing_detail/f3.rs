use super::format_usd;
use fec_parser::covers::Form3;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub struct FilingDetailF3 {
    // Summary lines 6-14
    pub line6_total_contributions_no_loans: f64,
    pub line7_total_contribution_refunds: f64,
    pub line8_net_contributions: f64,
    pub line9_total_operating_expenditures: f64,
    pub line10_total_offset_to_operating_expenditures: f64,
    pub line11_net_operating_expenditures: f64,
    pub line12_cash_on_hand_close_of_period: f64,
    pub line13_debts_owed_to_committee: f64,
    pub line14_debts_owed_by_committee: f64,
    // From detailed summary
    pub cash_on_hand_beginning: f64,
    pub total_receipts_period: f64,
    pub total_disbursements_period: f64,
}

impl From<&Form3> for FilingDetailF3 {
    fn from(form: &Form3) -> Self {
        Self {
            line6_total_contributions_no_loans: form.summary.line6_total_contributions_no_loans,
            line7_total_contribution_refunds: form.summary.line7_total_contribution_refunds,
            line8_net_contributions: form.summary.line8_net_contributions,
            line9_total_operating_expenditures: form.summary.line9_total_operating_expenditures,
            line10_total_offset_to_operating_expenditures: form
                .summary
                .line10_total_offset_to_operating_expenditures,
            line11_net_operating_expenditures: form.summary.line11_net_operating_expenditures,
            line12_cash_on_hand_close_of_period: form.summary.line12_cash_on_hand_close_of_period,
            line13_debts_owed_to_committee: form.summary.line13_debts_owed_to_committee,
            line14_debts_owed_by_committee: form.summary.line14_debts_owed_by_committee,
            cash_on_hand_beginning: form.detailed_summary.cash_on_hand_beginning,
            total_receipts_period: form.detailed_summary.total_receipts_period,
            total_disbursements_period: form.detailed_summary.total_disbursements_period,
        }
    }
}

pub fn append_f3_content_lines(lines: &mut Vec<Line<'static>>, data: &FilingDetailF3) {
    let label_style = Style::default().fg(Color::White);

    let summary_items: Vec<(&str, f64)> = vec![
        (
            "6.  Total Contributions (No Loans)",
            data.line6_total_contributions_no_loans,
        ),
        (
            "7.  Total Contribution Refunds",
            data.line7_total_contribution_refunds,
        ),
        ("8.  Net Contributions", data.line8_net_contributions),
        (
            "9.  Total Operating Expenditures",
            data.line9_total_operating_expenditures,
        ),
        (
            "10. Total Offset to Operating Exp.",
            data.line10_total_offset_to_operating_expenditures,
        ),
        (
            "11. Net Operating Expenditures",
            data.line11_net_operating_expenditures,
        ),
        (
            "12. Cash on Hand - Close",
            data.line12_cash_on_hand_close_of_period,
        ),
        (
            "13. Debts Owed TO Committee",
            data.line13_debts_owed_to_committee,
        ),
        (
            "14. Debts Owed BY Committee",
            data.line14_debts_owed_by_committee,
        ),
    ];

    for (label, value) in &summary_items {
        lines.push(Line::from(vec![
            Span::styled(format!("{:<40}", label), label_style),
            Span::styled(format!("{:>16}", format_usd(*value)), label_style),
        ]));
    }

    lines.push(Line::from(""));

    // Cash flow with percentage change
    let cash_begin = data.cash_on_hand_beginning;
    let cash_end = data.line12_cash_on_hand_close_of_period;
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
            format!("+{:>15}", format_usd(data.total_receipts_period)),
            Style::default().fg(Color::Blue),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("{:<24}", "Expenditures"), label_style),
        Span::styled(
            format!("-{:>15}", format_usd(data.total_disbursements_period)),
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
