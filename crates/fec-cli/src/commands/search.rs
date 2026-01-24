/*!
 * Interactive TUI for searching FEC candidates and committees
 *
 * This module provides a terminal user interface (TUI) for searching through FEC candidate
 * and committee data using ratatui. It allows users to search by name and view detailed
 * information about campaigns and committees.
 *
 * ## Features
 *
 * - **Real-time dual search**: As you type, both candidate and committee results are fetched
 *   from the local SQLite bulk data cache and displayed instantly with query timing information.
 *   Both result sets are queried simultaneously.
 *
 * - **Tabbed interface**: Results are displayed in tabs showing "Candidates (N)" and "Committees (N)"
 *   where N is the count of results. The active tab is highlighted in green. Default tab is Candidates.
 *
 * - **Cycle selection**: Users can switch between election cycles (2000-present) to search
 *   candidates and committees in different election years. Default cycle is 2026.
 *
 * - **Focus management**: Three focusable panels (Search, Cycle, Results) with Tab/Shift+Tab
 *   navigation. Green borders indicate the currently focused panel.
 *
 * - **Keyboard navigation**:
 *   - Tab/Shift+Tab: Cycle through panels (Search → Cycle → Results → Search)
 *   - Ctrl+A: Switch to Candidates tab
 *   - Ctrl+B: Switch to Committees tab
 *   - ↑/↓: Navigate results or adjust cycle year (depending on focus)
 *   - Typing: Updates search input from any panel and refocuses Search
 *   - Enter: Select candidate/committee and display detailed information
 *   - PageUp/PageDown: Quick cycle year adjustment from any panel
 *   - q/Esc: Quit the application
 *
 * - **Candidate Results table**: Displays candidate information in a structured table with columns:
 *   - Candidate ID (e.g., P00003392)
 *   - Name (e.g., BIDEN, JOSEPH R JR)
 *   - Election Year
 *   - Office (H=House, S=Senate, P=President)
 *   - State/District (e.g., CA-12 for House, CA for Senate)
 *   - Principal Campaign Committee ID
 *
 * - **Committee Results table**: Displays committee information in a structured table with columns:
 *   - Committee ID (e.g., C00401224)
 *   - Name (e.g., BIDEN FOR PRESIDENT)
 *   - Type (committee type code)
 *   - Desig (designation, e.g., P for Principal)
 *   - Party (party affiliation)
 *   - Org (connected organization name)
 *   - Candidate (candidate ID if connected to a candidate)
 *
 * - **Detailed view**: When a candidate/committee is selected (Enter key), the TUI exits and displays
 *   detailed information about the selection.
 *
 * ## Data Source
 *
 * Searches are performed against the local bulk data cache maintained in `.bulk-data.db`.
 * The cache is automatically synced when searching a new cycle. Data comes from FEC's bulk
 * candidate and committee master files.
 *
 * ## Implementation Notes
 *
 * Future changes to this module should update this top-level comment if they modify:
 * - The focus/navigation system
 * - The search behavior or data displayed
 * - Keyboard shortcuts or interactions
 * - The layout or panels available
 * - The tab system
 */

use crate::{
    cache::bulk_candidates::{CandidateDetail, CandidateSearchResult},
    cache::bulk_committee::{CommitteeDetail, CommitteeSearchResult},
    cli::SearchArgs,
    sourcer::FilingSourcer,
    tui::candidate_detail::{render_candidate_detail, CandidateDetailState},
    tui::committee_detail::{render_committee_detail, CommitteeDetailState},
};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};
use std::time::{Duration, Instant};
use std::io;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultsTab {
    Candidates,
    Committees,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewState {
    Search,
    CandidateDetail,
    CommitteeDetail,
}

struct App {
    input: String,
    cycle: u16,
    candidate_results: Vec<CandidateSearchResult>,
    committee_results: Vec<CommitteeSearchResult>,
    candidate_table_state: TableState,
    committee_table_state: TableState,
    cursor_position: usize,
    last_query_duration: Option<Duration>,
    should_exit: bool,
    focus: FocusPanel,
    active_tab: ResultsTab,
    view_state: ViewState,
    candidate_detail: Option<CandidateDetail>,
    committee_detail: Option<CommitteeDetail>,
    candidate_detail_state: CandidateDetailState,
    committee_detail_state: CommitteeDetailState,
}

impl App {
    fn new(initial_cycle: u16, initial_query: String) -> Self {
        Self {
            input: initial_query,
            cycle: initial_cycle,
            candidate_results: Vec::new(),
            committee_results: Vec::new(),
            candidate_table_state: TableState::default(),
            committee_table_state: TableState::default(),
            cursor_position: 0,
            last_query_duration: None,
            should_exit: false,
            focus: FocusPanel::Search,
            active_tab: ResultsTab::Candidates,
            view_state: ViewState::Search,
            candidate_detail: None,
            committee_detail: None,
            candidate_detail_state: CandidateDetailState::new(),
            committee_detail_state: CommitteeDetailState::new(),
        }
    }

    fn focus_next(&mut self) {
        self.focus = self.focus.next();
    }

    fn focus_previous(&mut self) {
        self.focus = self.focus.previous();
    }

    fn search(&mut self, sourcer: &mut FilingSourcer) -> Result<()> {
        let start = Instant::now();

        if self.input.is_empty() {
            self.candidate_results.clear();
            self.committee_results.clear();
            self.last_query_duration = None;
        } else {
            match sourcer.cache.open_bulk_data_database() {
                Ok(mut db) => {
                    let candidate_results = crate::cache::bulk_candidates::search_candidates(
                        &mut db,
                        self.cycle,
                        &self.input,
                    )
                    .unwrap_or_default();
                    let committee_results = crate::cache::bulk_committee::search_committees(
                        &mut db,
                        self.cycle,
                        &self.input,
                    )
                    .unwrap_or_default();

                    self.candidate_results = candidate_results;
                    self.committee_results = committee_results;
                    self.last_query_duration = Some(start.elapsed());

                    // Select first candidate if available, otherwise first committee
                    if !self.candidate_results.is_empty() {
                        self.candidate_table_state.select(Some(0));
                        self.committee_table_state.select(None);
                    } else if !self.committee_results.is_empty() {
                        self.candidate_table_state.select(None);
                        self.committee_table_state.select(Some(0));
                    } else {
                        self.candidate_table_state.select(None);
                        self.committee_table_state.select(None);
                    }
                }
                Err(_) => {
                    self.candidate_results.clear();
                    self.committee_results.clear();
                    self.candidate_table_state.select(None);
                    self.committee_table_state.select(None);
                    self.last_query_duration = None;
                }
            }
        }
        Ok(())
    }

    fn candidate_next(&mut self) {
        if self.candidate_results.is_empty() {
            return;
        }
        let i = match self.candidate_table_state.selected() {
            Some(i) => {
                if i >= self.candidate_results.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.candidate_table_state.select(Some(i));
    }

    fn candidate_previous(&mut self) {
        if self.candidate_results.is_empty() {
            return;
        }
        let i = match self.candidate_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.candidate_results.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.candidate_table_state.select(Some(i));
    }

    fn committee_next(&mut self) {
        if self.committee_results.is_empty() {
            return;
        }
        let i = match self.committee_table_state.selected() {
            Some(i) => {
                if i >= self.committee_results.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.committee_table_state.select(Some(i));
    }

    fn committee_previous(&mut self) {
        if self.committee_results.is_empty() {
            return;
        }
        let i = match self.committee_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.committee_results.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.committee_table_state.select(Some(i));
    }

    fn select_current(&mut self, sourcer: &mut FilingSourcer) -> Result<()> {
        match self.active_tab {
            ResultsTab::Candidates => {
                if let Some(selected_idx) = self.candidate_table_state.selected() {
                    if let Some(candidate) = self.candidate_results.get(selected_idx) {
                        // Load full candidate detail
                        if let Ok(mut db) = sourcer.cache.open_bulk_data_database() {
                            if let Ok(Some(detail)) =
                                crate::cache::bulk_candidates::get_candidate_detail(
                                    &mut db,
                                    self.cycle,
                                    &candidate.candidate_id,
                                )
                            {
                                self.candidate_detail = Some(detail);
                                self.view_state = ViewState::CandidateDetail;
                                self.candidate_detail_state = CandidateDetailState::new();
                            }
                        }
                    }
                }
            }
            ResultsTab::Committees => {
                if let Some(selected_idx) = self.committee_table_state.selected() {
                    if let Some(committee) = self.committee_results.get(selected_idx) {
                        // Load full committee detail
                        if let Ok(mut db) = sourcer.cache.open_bulk_data_database() {
                            if let Ok(Some(detail)) =
                                crate::cache::bulk_committee::get_committee_detail(
                                    &mut db,
                                    self.cycle,
                                    &committee.committee_id,
                                )
                            {
                                self.committee_detail = Some(detail);
                                self.view_state = ViewState::CommitteeDetail;
                                self.committee_detail_state = CommitteeDetailState::new();
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn go_back_to_search(&mut self) {
        self.view_state = ViewState::Search;
        self.candidate_detail = None;
        self.committee_detail = None;
        self.candidate_detail_state = CandidateDetailState::new();
        self.committee_detail_state = CommitteeDetailState::new();
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
    app.search(&mut sourcer)?;

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
                KeyCode::Esc => match app.view_state {
                    ViewState::Search => return Ok(()),
                    ViewState::CandidateDetail => {
                        if app.candidate_detail_state.show_yank_popup {
                            app.candidate_detail_state.show_yank_popup = false;
                        } else {
                            app.go_back_to_search();
                        }
                    }
                    ViewState::CommitteeDetail => {
                        if app.committee_detail_state.show_yank_popup {
                            app.committee_detail_state.show_yank_popup = false;
                        } else {
                            app.go_back_to_search();
                        }
                    }
                },
                KeyCode::Tab => {
                    if app.view_state == ViewState::Search {
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::SHIFT)
                        {
                            app.focus_previous();
                        } else {
                            app.focus_next();
                        }
                    }
                }
                KeyCode::BackTab => {
                    if app.view_state == ViewState::Search {
                        app.focus_previous();
                    }
                }
                KeyCode::Enter => match app.view_state {
                    ViewState::Search => {
                        app.select_current(sourcer)?;
                    }
                    ViewState::CandidateDetail => {
                        if app.candidate_detail_state.show_yank_popup {
                            if let Some(ref candidate) = app.candidate_detail {
                                app.candidate_detail_state.copy_selected(candidate);
                                app.candidate_detail_state.show_yank_popup = false;
                            }
                        }
                    }
                    ViewState::CommitteeDetail => {
                        if app.committee_detail_state.show_yank_popup {
                            if let Some(ref committee) = app.committee_detail {
                                app.committee_detail_state.copy_selected(committee);
                                app.committee_detail_state.show_yank_popup = false;
                            }
                        }
                    }
                },
                KeyCode::Up => {
                    match app.view_state {
                        ViewState::Search => {
                            match app.focus {
                                FocusPanel::Search => {
                                    // Switch focus to results and navigate
                                    let has_results = match app.active_tab {
                                        ResultsTab::Candidates => !app.candidate_results.is_empty(),
                                        ResultsTab::Committees => !app.committee_results.is_empty(),
                                    };
                                    if has_results {
                                        app.focus = FocusPanel::Results;
                                        match app.active_tab {
                                            ResultsTab::Candidates => app.candidate_previous(),
                                            ResultsTab::Committees => app.committee_previous(),
                                        }
                                    }
                                }
                                FocusPanel::Cycle => {
                                    app.cycle_next();
                                    app.search(sourcer)?;
                                }
                                FocusPanel::Results => match app.active_tab {
                                    ResultsTab::Candidates => app.candidate_previous(),
                                    ResultsTab::Committees => app.committee_previous(),
                                },
                            }
                        }
                        ViewState::CandidateDetail => {
                            if app.candidate_detail_state.show_yank_popup {
                                if let Some(ref candidate) = app.candidate_detail {
                                    app.candidate_detail_state.yank_previous(candidate);
                                }
                            }
                        }
                        ViewState::CommitteeDetail => {
                            if app.committee_detail_state.show_yank_popup {
                                if let Some(ref committee) = app.committee_detail {
                                    app.committee_detail_state.yank_previous(committee);
                                }
                            }
                        }
                    }
                }
                KeyCode::Down => {
                    match app.view_state {
                        ViewState::Search => {
                            match app.focus {
                                FocusPanel::Search => {
                                    // Switch focus to results and navigate
                                    let has_results = match app.active_tab {
                                        ResultsTab::Candidates => !app.candidate_results.is_empty(),
                                        ResultsTab::Committees => !app.committee_results.is_empty(),
                                    };
                                    if has_results {
                                        app.focus = FocusPanel::Results;
                                        match app.active_tab {
                                            ResultsTab::Candidates => app.candidate_next(),
                                            ResultsTab::Committees => app.committee_next(),
                                        }
                                    }
                                }
                                FocusPanel::Cycle => {
                                    app.cycle_previous();
                                    app.search(sourcer)?;
                                }
                                FocusPanel::Results => match app.active_tab {
                                    ResultsTab::Candidates => app.candidate_next(),
                                    ResultsTab::Committees => app.committee_next(),
                                },
                            }
                        }
                        ViewState::CandidateDetail => {
                            if app.candidate_detail_state.show_yank_popup {
                                if let Some(ref candidate) = app.candidate_detail {
                                    app.candidate_detail_state.yank_next(candidate);
                                }
                            }
                        }
                        ViewState::CommitteeDetail => {
                            if app.committee_detail_state.show_yank_popup {
                                if let Some(ref committee) = app.committee_detail {
                                    app.committee_detail_state.yank_next(committee);
                                }
                            }
                        }
                    }
                }
                KeyCode::Left => {
                    if app.view_state == ViewState::Search
                        && app.focus == FocusPanel::Search
                        && app.cursor_position > 0
                    {
                        app.cursor_position -= 1;
                    }
                }
                KeyCode::Right => {
                    if app.view_state == ViewState::Search
                        && app.focus == FocusPanel::Search
                        && app.cursor_position < app.input.len()
                    {
                        app.cursor_position += 1;
                    }
                }
                KeyCode::Home => {
                    if app.view_state == ViewState::Search && app.focus == FocusPanel::Search {
                        app.cursor_position = 0;
                    }
                }
                KeyCode::End => {
                    if app.view_state == ViewState::Search && app.focus == FocusPanel::Search {
                        app.cursor_position = app.input.len();
                    }
                }
                KeyCode::Backspace => {
                    if app.view_state == ViewState::Search
                        && app.focus == FocusPanel::Search
                        && app.cursor_position > 0
                    {
                        app.input.remove(app.cursor_position - 1);
                        app.cursor_position -= 1;
                        app.search(sourcer)?;
                    }
                }
                KeyCode::Delete => {
                    if app.view_state == ViewState::Search
                        && app.focus == FocusPanel::Search
                        && app.cursor_position < app.input.len()
                    {
                        app.input.remove(app.cursor_position);
                        app.search(sourcer)?;
                    }
                }
                KeyCode::PageUp => {
                    if app.view_state == ViewState::Search {
                        app.cycle_next();
                        app.search(sourcer)?;
                    }
                }
                KeyCode::PageDown => {
                    if app.view_state == ViewState::Search {
                        app.cycle_previous();
                        app.search(sourcer)?;
                    }
                }
                KeyCode::Char(c) => {
                    match app.view_state {
                        ViewState::Search => {
                            // Ctrl+A to switch to Candidates tab
                            if c == 'a'
                                && key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL)
                            {
                                app.active_tab = ResultsTab::Candidates;
                                // Ensure candidate has selection if results exist
                                if !app.candidate_results.is_empty()
                                    && app.candidate_table_state.selected().is_none()
                                {
                                    app.candidate_table_state.select(Some(0));
                                }
                            }
                            // Ctrl+B to switch to Committees tab
                            else if c == 'b'
                                && key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL)
                            {
                                app.active_tab = ResultsTab::Committees;
                                // Ensure committee has selection if results exist
                                if !app.committee_results.is_empty()
                                    && app.committee_table_state.selected().is_none()
                                {
                                    app.committee_table_state.select(Some(0));
                                }
                            }
                            // Ctrl+C quit
                            else if c == 'c'
                                && key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL)
                            {
                                return Ok(());
                            }
                            // Allow typing alphanumeric characters to update search from any focus
                            else if c.is_alphanumeric()
                                || c.is_whitespace()
                                || c == '-'
                                || c == '_'
                            {
                                app.input.insert(app.cursor_position, c);
                                app.cursor_position += 1;
                                app.search(sourcer)?;
                                app.focus = FocusPanel::Search;
                            }
                        }
                        ViewState::CandidateDetail => {
                            if c == 'y' && !app.candidate_detail_state.show_yank_popup {
                                app.candidate_detail_state.show_yank_popup = true;
                            } else if c == 'j' && app.candidate_detail_state.show_yank_popup {
                                if let Some(ref candidate) = app.candidate_detail {
                                    app.candidate_detail_state.yank_next(candidate);
                                }
                            } else if c == 'k' && app.candidate_detail_state.show_yank_popup {
                                if let Some(ref candidate) = app.candidate_detail {
                                    app.candidate_detail_state.yank_previous(candidate);
                                }
                            }
                        }
                        ViewState::CommitteeDetail => {
                            if c == 'y' && !app.committee_detail_state.show_yank_popup {
                                app.committee_detail_state.show_yank_popup = true;
                            } else if c == 'j' && app.committee_detail_state.show_yank_popup {
                                if let Some(ref committee) = app.committee_detail {
                                    app.committee_detail_state.yank_next(committee);
                                }
                            } else if c == 'k' && app.committee_detail_state.show_yank_popup {
                                if let Some(ref committee) = app.committee_detail {
                                    app.committee_detail_state.yank_previous(committee);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    match app.view_state {
        ViewState::Search => render_search_view(f, app),
        ViewState::CandidateDetail => {
            if let Some(ref candidate) = app.candidate_detail {
                render_candidate_detail(f, f.area(), candidate, &app.candidate_detail_state);
            }
        }
        ViewState::CommitteeDetail => {
            if let Some(ref committee) = app.committee_detail {
                render_committee_detail(f, f.area(), committee, &app.committee_detail_state);
            }
        }
    }
}

fn render_search_bar(f: &mut Frame, app: &mut App, area: Rect) {
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title("Search")
        .border_style(if app.focus == FocusPanel::Search {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        });

    let input_with_prompt = format!("❯ {}", app.input);
    let input_text = Paragraph::new(input_with_prompt).block(input_block);
    f.render_widget(input_text, area);

    if app.focus == FocusPanel::Search {
        f.set_cursor_position((area.x + app.cursor_position as u16 + 3, area.y + 1));
    }
}

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

fn render_tabs(f: &mut Frame, app: &mut App, area: Rect) {
    let candidate_tab_text = format!(" Candidates ({}) ", app.candidate_results.len());
    let committee_tab_text = format!(" Committees ({}) ", app.committee_results.len());
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
    ]);

    let tabs_widget = Paragraph::new(tabs).block(Block::default().borders(Borders::NONE));
    f.render_widget(tabs_widget, area);
}

fn render_help_text(f: &mut Frame, app: &mut App, area: Rect) {
    //let help_text = "Tab: focus | ctrl + a: Candidates | ctrl + b: Committees | Enter: select | Esc: quit | ↑/↓: navigate";
    let shortcut_style = Style::default().bold().fg(Color::White);
    let descrip_style = Style::default().fg(Color::DarkGray);
    let help_text = Line::from(vec![
        Span::styled("Esc", shortcut_style),
        Span::styled(" quit  ", descrip_style),
        Span::styled("Tab", shortcut_style),
        Span::styled(" focus  ", descrip_style),
        Span::styled("Enter", shortcut_style),
        Span::styled(" select  ", descrip_style),
        Span::styled("↑/↓", shortcut_style),
        Span::styled(" navigate ", descrip_style),
        Span::styled("⌃a", shortcut_style),
        Span::styled(" Candidates  ", descrip_style),
        Span::styled("⌃b", shortcut_style),
        Span::styled(" Committees  ", descrip_style),
    ]);
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(help, area);
}

fn render_candidate_results_table(f: &mut Frame, app: &mut App, area: Rect) {
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
            } else if result.office == "S" && !result.state.is_empty() {
                result.state.clone()
            } else if !result.state.is_empty() {
                result.state.clone()
            } else {
                String::new()
            };

            let office = result.office.clone();
            let committee = result
                .principal_campaign_committee
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

    let candidate_title = if app.candidate_results.is_empty() && !app.input.is_empty() {
        "Candidate Results (no matches)".to_string()
    } else if app.candidate_results.is_empty() {
        "Candidate Results (start typing to search)".to_string()
    } else {
        if let Some(duration) = app.last_query_duration {
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
        }
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
            .title("Results")
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

    f.render_stateful_widget(candidate_table, area, &mut app.candidate_table_state);
}

fn render_committee_results_table(f: &mut Frame, app: &mut App, area: Rect) {
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
            let candidate = result
                .candidate_id
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("");

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
    } else {
        if let Some(duration) = app.last_query_duration {
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
        }
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
    .highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    f.render_stateful_widget(committee_table, area, &mut app.committee_table_state);
}

fn render_search_view(f: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input and cycle selection
            Constraint::Length(1), // Tabs
            Constraint::Min(1),    // Results table
            Constraint::Length(2), // Help text
        ]);
    
    let [top_bar, tabs, results_table, help_text] = f.area().layout(&layout);
    
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
    }
    render_help_text(f, app, help_text);
}
