//! Operating expenses results table renderer
//!
//! This module handles rendering the operating expenses search results in a table format.

use super::app::{App, FocusPanel};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

/// Render the operating expenses results table
pub(crate) fn render_opexp_results_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let opexp_header = Row::new(vec![
        Cell::from("Committee").style(header_style),
        Cell::from("Recipient").style(header_style),
        Cell::from("City").style(header_style),
        Cell::from("State").style(header_style),
        Cell::from("Date").style(header_style),
        Cell::from("Amount").style(header_style),
        Cell::from("Purpose").style(header_style),
    ])
    .height(1);

    let opexp_rows: Vec<Row> = app
        .opexp_results
        .iter()
        .map(|result| {
            let amount = format!("${:.2}", result.transaction_amount);
            Row::new(vec![
                Cell::from(result.committee_id.clone()).style(Style::default().fg(Color::Magenta)),
                Cell::from(result.name.clone()),
                Cell::from(result.city.clone()),
                Cell::from(result.state.clone()),
                Cell::from(result.transaction_date.clone()),
                Cell::from(amount).style(Style::default().fg(Color::Green)),
                Cell::from(result.purpose.clone()),
            ])
        })
        .collect();

    let opexp_title = if app.opexp_results.is_empty() && !app.input.is_empty() {
        "Operating Expenses (no matches)".to_string()
    } else if app.opexp_results.is_empty() {
        "Operating Expenses (start typing to search)".to_string()
    } else if let Some(duration) = app.last_query_duration {
        format!(
            "Operating Expenses ({} matches, {}ms)",
            app.opexp_results.len(),
            duration.as_millis()
        )
    } else {
        format!("Operating Expenses ({} matches)", app.opexp_results.len())
    };

    let opexp_table = Table::new(
        opexp_rows,
        [
            Constraint::Length(12), // Committee
            Constraint::Length(30), // Recipient
            Constraint::Length(15), // City
            Constraint::Length(5),  // State
            Constraint::Length(10), // Date
            Constraint::Length(12), // Amount
            Constraint::Min(20),    // Purpose
        ],
    )
    .header(opexp_header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(opexp_title)
            .border_style(if app.focus == FocusPanel::Results {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            }),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    f.render_stateful_widget(opexp_table, area, &mut app.opexp_table_state);
}
