/*!
 * Interactive TUI for searching FEC candidates
 *
 * This module provides a terminal user interface (TUI) for searching through FEC candidate data
 * using ratatui. It allows users to search candidates by name and view detailed information
 * about their campaigns.
 *
 * ## Features
 *
 * - **Real-time search**: As you type, candidate results are fetched from the local SQLite
 *   bulk data cache and displayed instantly with query timing information.
 *
 * - **Cycle selection**: Users can switch between election cycles (2000-present) to search
 *   candidates running in different election years. Default cycle is 2026.
 *
 * - **Focus management**: Three focusable panels (Search, Cycle, Results) with Tab/Shift+Tab
 *   navigation. Green borders indicate the currently focused panel.
 *
 * - **Keyboard navigation**:
 *   - Tab/Shift+Tab: Cycle through panels (Search → Cycle → Results → Search)
 *   - ↑/↓: Navigate results or adjust cycle year (depending on focus)
 *   - Typing: Updates search input from any panel and refocuses Search
 *   - Enter: Select candidate and display detailed information
 *   - PageUp/PageDown: Quick cycle year adjustment from any panel
 *   - q/Esc: Quit the application
 *
 * - **Results table**: Displays candidate information in a structured table with columns:
 *   - Candidate ID (e.g., P00003392)
 *   - Name (e.g., BIDEN, JOSEPH R JR)
 *   - Election Year
 *   - Office (H=House, S=Senate, P=President)
 *   - State/District (e.g., CA-12 for House, CA for Senate)
 *   - Principal Campaign Committee ID
 *
 * - **Detailed view**: When a candidate is selected (Enter key), the TUI exits and displays:
 *   - Candidate name and ID
 *   - Office they're running for with location
 *   - Election year
 *   - Principal campaign committee ID (if available)
 *
 * ## Data Source
 *
 * Searches are performed against the local bulk candidate data cache maintained in
 * `.bulk-data.db`. The cache is automatically synced when searching a new cycle.
 * Data comes from FEC's bulk candidate master files.
 *
 * ## Implementation Notes
 *
 * Future changes to this module should update this top-level comment if they modify:
 * - The focus/navigation system
 * - The search behavior or data displayed
 * - Keyboard shortcuts or interactions
 * - The layout or panels available
 */

use crate::{cache::bulk_candidates::CandidateSearchResult, cli::SearchArgs, sourcer::FilingSourcer};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table, TableState, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusPanel {
    Search,
    Cycle,
    Results,
}

impl FocusPanel {
    fn next(&self) -> Self {
        match self {
            FocusPanel::Search => FocusPanel::Cycle,
            FocusPanel::Cycle => FocusPanel::Results,
            FocusPanel::Results => FocusPanel::Search,
        }
    }

    fn previous(&self) -> Self {
        match self {
            FocusPanel::Search => FocusPanel::Results,
            FocusPanel::Cycle => FocusPanel::Search,
            FocusPanel::Results => FocusPanel::Cycle,
        }
    }
}

struct App {
    input: String,
    cycle: u16,
    results: Vec<CandidateSearchResult>,
    table_state: TableState,
    cursor_position: usize,
    last_query_duration: Option<Duration>,
    should_exit: bool,
    selected_candidate: Option<CandidateSearchResult>,
    focus: FocusPanel,
}

impl App {
    fn new(initial_cycle: u16, initial_query: String) -> Self {
        Self {
            input: initial_query,
            cycle: initial_cycle,
            results: Vec::new(),
            table_state: TableState::default(),
            cursor_position: 0,
            last_query_duration: None,
            should_exit: false,
            selected_candidate: None,
            focus: FocusPanel::Search,
        }
    }

    fn focus_next(&mut self) {
        self.focus = self.focus.next();
    }

    fn focus_previous(&mut self) {
        self.focus = self.focus.previous();
    }

    fn search_candidates(&mut self, sourcer: &mut FilingSourcer) -> Result<()> {
        let start = Instant::now();

        if self.input.is_empty() {
            self.results.clear();
            self.last_query_duration = None;
        } else {
            match sourcer
                .cache
                .open_bulk_data_database()
                .and_then(|mut db| {
                    crate::cache::bulk_candidates::search_candidates(&mut db, self.cycle, &self.input)
                })
            {
                Ok(results) => {
                    self.results = results;
                    self.last_query_duration = Some(start.elapsed());
                    if !self.results.is_empty() {
                        self.table_state.select(Some(0));
                    } else {
                        self.table_state.select(None);
                    }
                }
                Err(_) => {
                    self.results.clear();
                    self.table_state.select(None);
                    self.last_query_duration = None;
                }
            }
        }
        Ok(())
    }

    fn next(&mut self) {
        if self.results.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.results.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.results.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.results.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn select_current(&mut self) {
        if let Some(selected_idx) = self.table_state.selected() {
            if let Some(candidate) = self.results.get(selected_idx) {
                self.selected_candidate = Some(candidate.clone());
                self.should_exit = true;
            }
        }
    }

    fn cycle_next(&mut self) {
        self.cycle += 2;
    }

    fn cycle_previous(&mut self) {
        if self.cycle > 2000 {
            self.cycle -= 2;
        }
    }
}

pub fn search(mut sourcer: FilingSourcer, args: &SearchArgs) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(args.cycle, args.query.clone());
    app.cursor_position = app.input.len();
    app.search_candidates(&mut sourcer)?;

    let res = run_app(&mut terminal, &mut app, &mut sourcer);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {err:?}");
        return Err(err);
    }

    if let Some(candidate) = app.selected_candidate {
        println!("\n{} ({})", candidate.name, candidate.candidate_id);

        let office_desc = match candidate.office.as_str() {
            "H" => format!("U.S. House ({}{})", candidate.state,
                if !candidate.district.is_empty() { format!("-{:02}", candidate.district.parse::<u8>().unwrap_or(0)) } else { String::new() }),
            "S" => format!("U.S. Senate ({})", candidate.state),
            "P" => "President".to_string(),
            _ => candidate.office.clone(),
        };

        let location = if candidate.office == "H" && !candidate.state.is_empty() && !candidate.district.is_empty() {
            format!("{}-{:02}", candidate.state, candidate.district.parse::<u8>().unwrap_or(0))
        } else if candidate.office == "S" && !candidate.state.is_empty() {
            candidate.state.clone()
        } else {
            String::new()
        };

        if !location.is_empty() {
            println!("Running for {} in {} in {}", office_desc, location, candidate.election_year);
        } else {
            println!("Running for {} in {}", office_desc, candidate.election_year);
        }

        if let Some(committee_id) = candidate.principal_campaign_committee {
            println!("Principal campaign committee: {}", committee_id);
        }
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    sourcer: &mut FilingSourcer,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app)).unwrap();

        if app.should_exit {
            return Ok(());
        }

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                KeyCode::Tab => {
                    if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                        app.focus_previous();
                    } else {
                        app.focus_next();
                    }
                }
                KeyCode::BackTab => {
                    app.focus_previous();
                }
                KeyCode::Enter => {
                    app.select_current();
                }
                KeyCode::Up => {
                    match app.focus {
                        FocusPanel::Search => {
                            // Switch focus to results and navigate
                            if !app.results.is_empty() {
                                app.focus = FocusPanel::Results;
                                app.previous();
                            }
                        }
                        FocusPanel::Cycle => {
                            app.cycle_next();
                            app.search_candidates(sourcer)?;
                        }
                        FocusPanel::Results => {
                            app.previous();
                        }
                    }
                }
                KeyCode::Down => {
                    match app.focus {
                        FocusPanel::Search => {
                            // Switch focus to results and navigate
                            if !app.results.is_empty() {
                                app.focus = FocusPanel::Results;
                                app.next();
                            }
                        }
                        FocusPanel::Cycle => {
                            app.cycle_previous();
                            app.search_candidates(sourcer)?;
                        }
                        FocusPanel::Results => {
                            app.next();
                        }
                    }
                }
                KeyCode::Left => {
                    if app.focus == FocusPanel::Search && app.cursor_position > 0 {
                        app.cursor_position -= 1;
                    }
                }
                KeyCode::Right => {
                    if app.focus == FocusPanel::Search && app.cursor_position < app.input.len() {
                        app.cursor_position += 1;
                    }
                }
                KeyCode::Home => {
                    if app.focus == FocusPanel::Search {
                        app.cursor_position = 0;
                    }
                }
                KeyCode::End => {
                    if app.focus == FocusPanel::Search {
                        app.cursor_position = app.input.len();
                    }
                }
                KeyCode::Backspace => {
                    if app.focus == FocusPanel::Search && app.cursor_position > 0 {
                        app.input.remove(app.cursor_position - 1);
                        app.cursor_position -= 1;
                        app.search_candidates(sourcer)?;
                    }
                }
                KeyCode::Delete => {
                    if app.focus == FocusPanel::Search && app.cursor_position < app.input.len() {
                        app.input.remove(app.cursor_position);
                        app.search_candidates(sourcer)?;
                    }
                }
                KeyCode::PageUp => {
                    app.cycle_next();
                    app.search_candidates(sourcer)?;
                }
                KeyCode::PageDown => {
                    app.cycle_previous();
                    app.search_candidates(sourcer)?;
                }
                KeyCode::Char(c) => {
                    // Allow typing alphanumeric characters to update search from any focus
                    if c.is_alphanumeric() || c.is_whitespace() || c == '-' || c == '_' {
                        app.input.insert(app.cursor_position, c);
                        app.cursor_position += 1;
                        app.search_candidates(sourcer)?;
                        app.focus = FocusPanel::Search;
                    }
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(f.area());
    let inner_header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(20)])
        .split(layout[0]);
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title("Search Candidates")
        .border_style(if app.focus == FocusPanel::Search {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        });

    let input_text = Paragraph::new(app.input.as_str())
        .block(input_block);
    f.render_widget(input_text, inner_header_chunks[0]);

    if app.focus == FocusPanel::Search {
        f.set_cursor_position((inner_header_chunks[0].x + app.cursor_position as u16 + 1, inner_header_chunks[0].y + 1));
    }

    let cycle_text = format!("Cycle: {}", app.cycle);
    let cycle_widget = Paragraph::new(cycle_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(if app.focus == FocusPanel::Cycle {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            }));
    f.render_widget(cycle_widget, inner_header_chunks[1]);

    let header = Row::new(vec![
        Cell::from("Candidate ID").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Name").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Year").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Office").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("State/Dist").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Committee").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ])
    .height(1);

    let rows: Vec<Row> = app
        .results
        .iter()
        .map(|result| {
            let state_district = if result.office == "H" && !result.state.is_empty() && !result.district.is_empty() {
                format!("{}-{:02}", result.state, result.district.parse::<u8>().unwrap_or(0))
            } else if result.office == "S" && !result.state.is_empty() {
                result.state.clone()
            } else if !result.state.is_empty() {
                result.state.clone()
            } else {
                String::new()
            };

            let office = result.office.clone();
            let committee = result.principal_campaign_committee
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("");

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

    let results_title = if app.results.is_empty() && !app.input.is_empty() {
        if let Some(duration) = app.last_query_duration {
            format!("Results (no matches found, {}ms)", duration.as_millis())
        } else {
            "Results (no matches found)".to_string()
        }
    } else if app.results.is_empty() {
        "Results (start typing to search)".to_string()
    } else {
        if let Some(duration) = app.last_query_duration {
            format!("Results ({} matches, {}ms)", app.results.len(), duration.as_millis())
        } else {
            format!("Results ({} matches)", app.results.len())
        }
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(13),
            Constraint::Min(20),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Length(11),
            Constraint::Length(11),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(results_title)
            .border_style(if app.focus == FocusPanel::Results {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            }),
    )
    .highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, layout[1], &mut app.table_state);

    let help_text = "Tab/Shift+Tab: switch focus | Enter: select | q/Esc: quit | ↑/↓: navigate/cycle | Type: search";
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, layout[2]);
}
