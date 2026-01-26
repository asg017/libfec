//! Committee results table renderer
//!
//! This module handles rendering the committee search results in a table format.

use super::app::{App, FocusPanel};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

/// Render the committee results table
pub(crate) fn render_committee_results_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let committee_header = Row::new(vec![
        Cell::from("Committee ID").style(header_style),
        Cell::from("Name").style(header_style),
        Cell::from("Type").style(header_style),
        Cell::from("Desig").style(header_style),
        Cell::from("Party").style(header_style),
        Cell::from("Org").style(header_style),
        Cell::from("Candidate").style(header_style),
    ])
    .height(1);

    let committee_rows: Vec<Row> = app
        .committee_results
        .iter()
        .map(|result| {
            let candidate = result.candidate_id.as_deref().unwrap_or("");

            Row::new(vec![
                Cell::from(result.committee_id.clone()).style(Style::default().fg(Color::Magenta)),
                Cell::from(result.name.clone()),
                Cell::from(result.committee_type.clone()),
                Cell::from(result.designation.clone()),
                Cell::from(result.party_affiliation.clone()),
                Cell::from(result.connected_org_name.clone()),
                Cell::from(candidate).style(Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();

    let committee_title = if app.committee_results.is_empty() && !app.input.is_empty() {
        "Committee Results (no matches)".to_string()
    } else if app.committee_results.is_empty() {
        "Committee Results (start typing to search)".to_string()
    } else if let Some(duration) = app.last_query_duration {
        format!(
            "Committee Results ({} matches, {}ms)",
            app.committee_results.len(),
            duration.as_millis()
        )
    } else {
        format!(
            "Committee Results ({} matches)",
            app.committee_results.len()
        )
    };

    let committee_table = Table::new(
        committee_rows,
        [
            Constraint::Length(12),
            Constraint::Length(50),
            Constraint::Length(5),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(15),
            Constraint::Length(12),
        ],
    )
    .header(committee_header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(committee_title)
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

    f.render_stateful_widget(committee_table, area, &mut app.committee_table_state);
}
