//! Candidate results table renderer
//!
//! This module handles rendering the candidate search results in a table format.

use super::app::{App, FocusPanel};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

/// Render the candidate results table
pub(crate) fn render_candidate_results_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let candidate_header = Row::new(vec![
        Cell::from("Candidate ID").style(header_style),
        Cell::from("Name").style(header_style),
        Cell::from("Year").style(header_style),
        Cell::from("Office").style(header_style),
        Cell::from("State/Dist").style(header_style),
        Cell::from("Committee").style(header_style),
    ])
    .height(1);

    let candidate_rows: Vec<Row> = app
        .candidate_results
        .iter()
        .map(|result| {
            let state_district = if result.office == "H"
                && !result.state.is_empty()
                && !result.district.is_empty()
            {
                format!(
                    "{}-{:02}",
                    result.state,
                    result.district.parse::<u8>().unwrap_or(0)
                )
            } else if !result.state.is_empty() {
                result.state.clone()
            } else {
                String::new()
            };

            let office = result.office.clone();
            let committee = result.principal_campaign_committee.as_deref().unwrap_or("");

            Row::new(vec![
                Cell::from(result.candidate_id.clone()).style(Style::default().fg(Color::Cyan)),
                Cell::from(result.name.clone()),
                Cell::from(result.election_year.to_string()),
                Cell::from(office),
                Cell::from(state_district),
                Cell::from(committee).style(Style::default().fg(Color::Green)),
            ])
        })
        .collect();

    let candidate_title = if app.candidate_results.is_empty() && !app.input.is_empty() {
        "Candidate Results (no matches)".to_string()
    } else if app.candidate_results.is_empty() {
        "Candidate Results (start typing to search)".to_string()
    } else if let Some(duration) = app.last_query_duration {
        format!(
            "Candidate Results ({} matches, {}ms)",
            app.candidate_results.len(),
            duration.as_millis()
        )
    } else {
        format!(
            "Candidate Results ({} matches)",
            app.candidate_results.len()
        )
    };

    let candidate_table = Table::new(
        candidate_rows,
        [
            Constraint::Length(13),
            Constraint::Max(50),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Length(11),
            Constraint::Length(11),
        ],
    )
    .header(candidate_header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(candidate_title)
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

    f.render_stateful_widget(candidate_table, area, &mut app.candidate_table_state);
}
