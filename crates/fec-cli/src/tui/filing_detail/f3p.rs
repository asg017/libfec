use super::format_usd;
use fec_parser::covers::Form3P;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub struct FilingDetailF3P {
    pub line6_cash_on_hand_beginning_period: f64,
    pub line7_total_receipts: f64,
    pub line8_subtotal: f64,
    pub line9_total_disbursements: f64,
    pub line10_cash_on_hand_end_period: f64,
    pub line11_debts_owed_to_committee: f64,
    pub line12_debts_owed_by_committee: f64,
    pub line13_expenditures_subject_to_limits: f64,
    pub line14_net_contributions_other_than_loans: f64,
    pub line15_net_operating_expenditures: f64,
}

impl From<&Form3P> for FilingDetailF3P {
    fn from(form: &Form3P) -> Self {
        Self {
            line6_cash_on_hand_beginning_period: form.summary.line6_cash_on_hand_beginning_period,
            line7_total_receipts: form.summary.line7_total_receipts,
            line8_subtotal: form.summary.line8_subtotal,
            line9_total_disbursements: form.summary.line9_total_disbursements,
            line10_cash_on_hand_end_period: form.summary.line10_cash_on_hand_end_period,
            line11_debts_owed_to_committee: form.summary.line11_debts_owed_to_committee,
            line12_debts_owed_by_committee: form.summary.line12_debts_owed_by_committee,
            line13_expenditures_subject_to_limits: form
                .summary
                .line13_expenditures_subject_to_limits,
            line14_net_contributions_other_than_loans: form
                .summary
                .line14_net_contributions_other_than_loans,
            line15_net_operating_expenditures: form.summary.line15_net_operating_expenditures,
        }
    }
}

pub fn append_f3p_content_lines(lines: &mut Vec<Line<'static>>, data: &FilingDetailF3P) {
    let label_style = Style::default().fg(Color::White);

    let summary_items: Vec<(&str, f64)> = vec![
        (
            "6.  Cash on Hand - Beginning",
            data.line6_cash_on_hand_beginning_period,
        ),
        ("7.  Total Receipts", data.line7_total_receipts),
        ("8.  Subtotal (6 + 7)", data.line8_subtotal),
        ("9.  Total Disbursements", data.line9_total_disbursements),
        (
            "10. Cash on Hand - Close",
            data.line10_cash_on_hand_end_period,
        ),
        (
            "11. Debts Owed TO Committee",
            data.line11_debts_owed_to_committee,
        ),
        (
            "12. Debts Owed BY Committee",
            data.line12_debts_owed_by_committee,
        ),
        (
            "13. Expenditures Subject to Limits",
            data.line13_expenditures_subject_to_limits,
        ),
        (
            "14. Net Contributions (Not Loans)",
            data.line14_net_contributions_other_than_loans,
        ),
        (
            "15. Net Operating Expenditures",
            data.line15_net_operating_expenditures,
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
    let cash_begin = data.line6_cash_on_hand_beginning_period;
    let cash_end = data.line10_cash_on_hand_end_period;
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
            format!("+{:>15}", format_usd(data.line7_total_receipts)),
            Style::default().fg(Color::Blue),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("{:<24}", "Expenditures"), label_style),
        Span::styled(
            format!("-{:>15}", format_usd(data.line9_total_disbursements)),
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
