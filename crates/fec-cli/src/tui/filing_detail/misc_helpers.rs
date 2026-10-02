//! Line builders shared by the Form 3L, 4, 7 and 13 cover renderers.

use super::format_usd;
use fec_parser::covers::{Address, DetailedSummaryRow};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Width of the label column in [`field_line`].
const FIELD_LABEL_WIDTH: usize = 14;
/// Width of the label column in [`amount_line`] and [`two_column_line`].
const AMOUNT_LABEL_WIDTH: usize = 30;
/// Width of each money column.
const AMOUNT_WIDTH: usize = 15;

fn label_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

/// A bold section heading, e.g. "SUMMARY PAGE".
pub fn section_line(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

/// `Label:       value` with the label in a fixed-width column.
pub fn field_line(label: &str, value: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{:<width$}", format!("{label}:"), width = FIELD_LABEL_WIDTH),
            label_style(),
        ),
        Span::raw(value.into()),
    ])
}

/// `Address:` line, skipped when the address is blank.
pub fn push_address(lines: &mut Vec<Line<'static>>, address: &Address) {
    if !address.is_empty() {
        lines.push(field_line("Address", address.one_line()));
    }
}

/// `"CODE (label)"`, or just the code when no label is sourced.
pub fn code_with_label(code: &str, label: Option<&str>) -> String {
    match label {
        Some(label) => format!("{code} ({label})"),
        None => code.to_string(),
    }
}

/// One money line: label, then a right-aligned amount. `None` renders as `-`.
pub fn amount_line(label: &str, amount: Option<f64>, bold: bool) -> Line<'static> {
    let style = if bold {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::styled(
            format!("{:<width$}", label, width = AMOUNT_LABEL_WIDTH),
            style,
        ),
        Span::styled(money_cell(amount), style),
    ])
}

/// Column headings over [`two_column_line`] rows.
pub fn two_column_header(column_a: &str, column_b: &str) -> Line<'static> {
    let style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    Line::from(vec![
        Span::raw(" ".repeat(AMOUNT_LABEL_WIDTH)),
        Span::styled(format!("{:>width$}", column_a, width = AMOUNT_WIDTH), style),
        Span::styled(format!("{:>width$}", column_b, width = AMOUNT_WIDTH), style),
    ])
}

/// One two-column line. `None` in either column renders as `-` (the box does
/// not exist on the form or in the record).
pub fn two_column_line(
    label: &str,
    column_a: Option<f64>,
    column_b: Option<f64>,
    bold: bool,
) -> Line<'static> {
    let style = if bold {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::styled(
            format!("{:<width$}", label, width = AMOUNT_LABEL_WIDTH),
            style,
        ),
        Span::styled(money_cell(column_a), style),
        Span::styled(money_cell(column_b), style),
    ])
}

/// A [`DetailedSummaryRow`] as a [`two_column_line`].
pub fn row_line(label: &str, row: &DetailedSummaryRow, bold: bool) -> Line<'static> {
    two_column_line(label, Some(row.column_a), Some(row.column_b), bold)
}

/// A dim explanatory note.
pub fn note_line(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    ))
}

fn money_cell(amount: Option<f64>) -> String {
    let text = amount.map(format_usd).unwrap_or_else(|| "-".to_string());
    format!("{:>width$}", text, width = AMOUNT_WIDTH)
}
