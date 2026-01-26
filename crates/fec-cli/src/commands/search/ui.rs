//! UI coordination and chrome widgets for the search TUI
//!
//! This module contains the main UI layout coordination and reusable UI widgets
//! like breadcrumbs, search bar, tabs, and help text.

use super::app::{App, FocusPanel, ResultsTab, ViewState, SPINNER_FRAMES};
use super::candidates_table::render_candidate_results_table;
use super::committees_table::render_committee_results_table;
use super::opexp_table::render_opexp_results_table;
use crate::tui::{
    candidate_detail::render_candidate_detail, committee_detail::render_committee_detail,
    filing_detail::render_filing_detail, truncate_string, HelpBar,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Top-level UI router - renders breadcrumb and delegates to view-specific renderers
pub(crate) fn ui(f: &mut Frame, app: &mut App) {
    // Layout with breadcrumb at the top
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Breadcrumb
            Constraint::Min(1),    // Main content
        ]);

    let [breadcrumb_area, content_area] = f.area().layout(&layout);

    // Render breadcrumb
    render_breadcrumb(f, app, breadcrumb_area);

    // Render main content
    match app.view_state {
        ViewState::Search => render_search_view(f, app, content_area),
        ViewState::CandidateDetail => {
            if let Some(ref candidate) = app.candidate_detail {
                render_candidate_detail(
                    f,
                    content_area,
                    candidate,
                    &mut app.candidate_detail_state,
                );
            }
        }
        ViewState::CommitteeDetail => {
            if let Some(ref committee) = app.committee_detail {
                render_committee_detail(
                    f,
                    content_area,
                    committee,
                    &mut app.committee_detail_state,
                );
            }
        }
        ViewState::FilingDetail => {
            if let Some(ref filing) = app.filing_detail {
                render_filing_detail(f, content_area, filing, &app.filing_detail_state);
            }
        }
    }
}

/// Render the breadcrumb navigation at the top of the screen
fn render_breadcrumb(f: &mut Frame, app: &App, area: Rect) {
    let breadcrumb_text = match app.view_state {
        ViewState::Search => "Search".to_string(),
        ViewState::CandidateDetail => {
            if let Some(ref candidate) = app.candidate_detail {
                let name = truncate_string(&candidate.name, 40);
                format!("Search / {}", name)
            } else {
                "Search / Candidate".to_string()
            }
        }
        ViewState::CommitteeDetail => {
            if let Some(ref committee) = app.committee_detail {
                let name = truncate_string(&committee.name, 40);
                format!("Search / {}", name)
            } else {
                "Search / Committee".to_string()
            }
        }
        ViewState::FilingDetail => {
            if let Some(ref filing) = app.filing_detail {
                if let Some(ref committee) = app.committee_detail {
                    let name = truncate_string(&committee.name, 30);
                    format!("Search / {} / {}", name, filing.filing_id)
                } else {
                    format!("Search / {}", filing.filing_id)
                }
            } else {
                "Search / Filing".to_string()
            }
        }
    };

    let breadcrumb = Paragraph::new(breadcrumb_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(breadcrumb, area);
}

/// Render the main search view with all components
pub(crate) fn render_search_view(f: &mut Frame, app: &mut App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input and cycle selection
            Constraint::Length(1), // Tabs
            Constraint::Min(1),    // Results table
            Constraint::Length(2), // Help text
        ]);

    let [top_bar, tabs, results_table, help_text] = area.layout(&layout);

    let top_bar_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(15)]);

    let [search_bar, cycle_selection] = top_bar.layout(&top_bar_layout);

    render_search_bar(f, app, search_bar);
    render_cycle_selection(f, app, cycle_selection);
    render_tabs(f, app, tabs);

    match app.active_tab {
        ResultsTab::Candidates => render_candidate_results_table(f, app, results_table),
        ResultsTab::Committees => render_committee_results_table(f, app, results_table),
        ResultsTab::OpExp => render_opexp_results_table(f, app, results_table),
    }
    render_help_text(f, app, help_text);
}

/// Render the search input box with optional spinner
pub(crate) fn render_search_bar(f: &mut Frame, app: &mut App, area: Rect) {
    let title = if app.searching {
        format!("Search {}", SPINNER_FRAMES[app.spinner_frame])
    } else {
        "Search".to_string()
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(if app.focus == FocusPanel::Search {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        });

    let input_with_prompt = format!("❯ {}", app.input);
    let input_text = Paragraph::new(input_with_prompt).block(input_block);
    f.render_widget(input_text, area);

    if app.focus == FocusPanel::Search && !app.searching {
        f.set_cursor_position((area.x + app.cursor_position as u16 + 3, area.y + 1));
    }
}

/// Render the cycle selection widget
fn render_cycle_selection(f: &mut Frame, app: &mut App, area: Rect) {
    let cycle_text = format!("Cycle: {}", app.cycle);
    let cycle_widget = Paragraph::new(cycle_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(
            if app.focus == FocusPanel::Cycle {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            },
        ));
    f.render_widget(cycle_widget, area);
}

/// Render the tab bar showing Candidates, Committees, and Operating Expenses tabs
fn render_tabs(f: &mut Frame, app: &mut App, area: Rect) {
    let candidate_tab_text = format!(" Candidates ({}) ", app.candidate_results.len());
    let committee_tab_text = format!(" Committees ({}) ", app.committee_results.len());
    // Show (?) for OpExp when not active, since we don't search it until the tab is selected
    let opexp_tab_text = if app.active_tab == ResultsTab::OpExp {
        format!(" Operating Expenses ({}) ", app.opexp_results.len())
    } else {
        " Operating Expenses (?) ".to_string()
    };
    let tabs = Line::from(vec![
        Span::styled(
            candidate_tab_text,
            if app.active_tab == ResultsTab::Candidates {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            },
        ),
        if app.active_tab != ResultsTab::Candidates {
            Span::styled(" ⌃a", Style::default().fg(Color::DarkGray))
        } else {
            Span::raw("   ")
        },
        Span::raw(" "),
        Span::styled(
            committee_tab_text,
            if app.active_tab == ResultsTab::Committees {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            },
        ),
        if app.active_tab != ResultsTab::Committees {
            Span::styled(" ⌃b", Style::default().fg(Color::DarkGray))
        } else {
            Span::raw("   ")
        },
        Span::raw(" "),
        Span::styled(
            opexp_tab_text,
            if app.active_tab == ResultsTab::OpExp {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            },
        ),
        if app.active_tab != ResultsTab::OpExp {
            Span::styled(" ⌃e", Style::default().fg(Color::DarkGray))
        } else {
            Span::raw("   ")
        },
    ]);

    let tabs_widget = Paragraph::new(tabs).block(Block::default().borders(Borders::NONE));
    f.render_widget(tabs_widget, area);
}

/// Render the help text showing keyboard shortcuts
pub(crate) fn render_help_text(f: &mut Frame, _app: &mut App, area: Rect) {
    HelpBar::new()
        .item("Esc", " quit")
        .item("Tab", " focus")
        .item("Enter", " select")
        .item("↑/↓", " navigate")
        .item("⌃a", " Candidates")
        .item("⌃b", " Committees")
        .item("⌃e", " Expenses")
        .render(f, area);
}
