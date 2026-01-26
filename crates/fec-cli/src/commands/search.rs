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
    cache::bulk_opexp::OpExpSearchResult,
    cli::SearchArgs,
    sourcer::FilingSourcer,
    tui::candidate_detail::{render_candidate_detail, CandidateDetailAction, CandidateDetailState},
    tui::committee_detail::{render_committee_detail, CommitteeDetailAction, CommitteeDetailState},
    tui::filing_detail::{
        render_filing_detail, FilingDetail, FilingDetailAction, FilingDetailState,
    },
    tui::truncate_string,
    tui::HelpBar,
};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultsTab {
    Candidates,
    Committees,
    OpExp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewState {
    Search,
    CandidateDetail,
    CommitteeDetail,
    FilingDetail,
}

struct App {
    input: String,
    cycle: u16,
    candidate_results: Vec<CandidateSearchResult>,
    committee_results: Vec<CommitteeSearchResult>,
    opexp_results: Vec<OpExpSearchResult>,
    candidate_table_state: TableState,
    committee_table_state: TableState,
    opexp_table_state: TableState,
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
    filing_detail: Option<FilingDetail>,
    filing_detail_state: FilingDetailState,
    /// Tracks which detail view we came from when showing filing detail
    filing_detail_from: Option<ViewState>,
    /// Whether a search is currently in progress
    searching: bool,
    /// Spinner frame for loading animation
    spinner_frame: usize,
}

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

impl App {
    fn new(initial_cycle: u16, initial_query: String) -> Self {
        Self {
            input: initial_query,
            cycle: initial_cycle,
            candidate_results: Vec::new(),
            committee_results: Vec::new(),
            opexp_results: Vec::new(),
            candidate_table_state: TableState::default(),
            committee_table_state: TableState::default(),
            opexp_table_state: TableState::default(),
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
            filing_detail: None,
            filing_detail_state: FilingDetailState::new(),
            filing_detail_from: None,
            searching: false,
            spinner_frame: 0,
        }
    }

    fn advance_spinner(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
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
            self.opexp_results.clear();
            self.last_query_duration = None;
        } else {
            match sourcer.cache.open_bulk_data_database() {
                Ok(mut db) => {
                    // Always search candidates and committees in parallel
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

                    // Only search operating expenses when that tab is active
                    if self.active_tab == ResultsTab::OpExp {
                        match crate::cache::bulk_opexp::search_operating_expenses(
                            &mut db,
                            self.cycle,
                            &self.input,
                        ) {
                            Ok(results) => {
                                self.opexp_results = results;
                            }
                            Err(e) => {
                                eprintln!("OpExp search error: {:?}", e);
                                self.opexp_results.clear();
                            }
                        }
                    } else {
                        self.opexp_results.clear();
                    }

                    self.last_query_duration = Some(start.elapsed());

                    // Select first result in active tab
                    match self.active_tab {
                        ResultsTab::Candidates => {
                            if !self.candidate_results.is_empty() {
                                self.candidate_table_state.select(Some(0));
                            } else {
                                self.candidate_table_state.select(None);
                            }
                            self.committee_table_state.select(None);
                            self.opexp_table_state.select(None);
                        }
                        ResultsTab::Committees => {
                            if !self.committee_results.is_empty() {
                                self.committee_table_state.select(Some(0));
                            } else {
                                self.committee_table_state.select(None);
                            }
                            self.candidate_table_state.select(None);
                            self.opexp_table_state.select(None);
                        }
                        ResultsTab::OpExp => {
                            if !self.opexp_results.is_empty() {
                                self.opexp_table_state.select(Some(0));
                            } else {
                                self.opexp_table_state.select(None);
                            }
                            self.candidate_table_state.select(None);
                            self.committee_table_state.select(None);
                        }
                    }
                }
                Err(_) => {
                    self.candidate_results.clear();
                    self.committee_results.clear();
                    self.opexp_results.clear();
                    self.candidate_table_state.select(None);
                    self.committee_table_state.select(None);
                    self.opexp_table_state.select(None);
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

    fn opexp_next(&mut self) {
        if self.opexp_results.is_empty() {
            return;
        }
        let i = match self.opexp_table_state.selected() {
            Some(i) => {
                if i >= self.opexp_results.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.opexp_table_state.select(Some(i));
    }

    fn opexp_previous(&mut self) {
        if self.opexp_results.is_empty() {
            return;
        }
        let i = match self.opexp_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.opexp_results.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.opexp_table_state.select(Some(i));
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
                                self.candidate_detail_state = CandidateDetailState::new();
                                // Load linked committees
                                if let Ok(linkages) = crate::cache::bulk_candidate_committee_linkage::get_candidate_committee_linkages(
                                    &mut db,
                                    self.cycle,
                                    &candidate.candidate_id,
                                ) {
                                    self.candidate_detail_state.set_linked_committees(linkages);
                                }
                                self.candidate_detail = Some(detail);
                                self.view_state = ViewState::CandidateDetail;
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
            ResultsTab::OpExp => {
                // OpExp doesn't have a detail view yet
                // Could potentially navigate to committee detail for the committee_id
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
        self.filing_detail = None;
        self.filing_detail_state = FilingDetailState::new();
    }

    fn go_back_to_committee_detail(&mut self) {
        self.view_state = ViewState::CommitteeDetail;
        self.filing_detail = None;
        self.filing_detail_state = FilingDetailState::new();
        self.committee_detail_state.filing_detail_loading = false;
    }

    fn go_back_to_candidate_detail(&mut self) {
        self.view_state = ViewState::CandidateDetail;
        self.filing_detail = None;
        self.filing_detail_state = FilingDetailState::new();
        self.candidate_detail_state.filing_detail_loading = false;
    }

    fn show_committee_detail_by_id(&mut self, sourcer: &mut FilingSourcer, committee_id: &str) {
        if let Ok(mut db) = sourcer.cache.open_bulk_data_database() {
            if let Ok(Some(detail)) = crate::cache::bulk_committee::get_committee_detail(
                &mut db,
                self.cycle,
                committee_id,
            ) {
                self.committee_detail = Some(detail);
                self.committee_detail_state = CommitteeDetailState::new();
                self.view_state = ViewState::CommitteeDetail;
            }
        }
    }

    fn show_filing_detail_from_candidate(&mut self, sourcer: &mut FilingSourcer, filing_id: &str) {
        match sourcer.resolve_from_user_argument(filing_id) {
            Ok(filing) => {
                self.filing_detail = Some(FilingDetail::from(&filing));
                self.filing_detail_state = FilingDetailState::new();
                self.filing_detail_from = Some(ViewState::CandidateDetail);
                self.view_state = ViewState::FilingDetail;
                self.candidate_detail_state.filing_detail_loading = false;
            }
            Err(e) => {
                self.candidate_detail_state.filing_detail_loading = false;
                self.candidate_detail_state
                    .set_filings_error(format!("Error loading filing {}: {}", filing_id, e));
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

    /// Clear the entire input
    fn clear_input(&mut self) {
        self.input.clear();
        self.cursor_position = 0;
    }

    /// Delete from cursor to end of input
    fn delete_to_end(&mut self) {
        self.input.truncate(self.cursor_position);
    }

    /// Delete word backward from cursor (Ctrl+W, Ctrl+Backspace)
    fn delete_word_backward(&mut self) {
        if self.cursor_position == 0 {
            return;
        }

        let before_cursor = &self.input[..self.cursor_position];

        // Find the start of the word to delete
        // Skip trailing whitespace first
        let trimmed = before_cursor.trim_end();
        if trimmed.is_empty() {
            // Delete all whitespace
            self.input.drain(..self.cursor_position);
            self.cursor_position = 0;
            return;
        }

        // Find last word boundary (space, hyphen, etc.)
        let word_start = trimmed
            .rfind(|c: char| c.is_whitespace() || c == '-' || c == '_')
            .map(|i| i + 1)
            .unwrap_or(0);

        // Delete from word start to cursor
        self.input.drain(word_start..self.cursor_position);
        self.cursor_position = word_start;
    }

    fn show_filing_detail(&mut self, sourcer: &mut FilingSourcer, filing_id: &str) {
        // Try to resolve and show filing detail
        match sourcer.resolve_from_user_argument(filing_id) {
            Ok(filing) => {
                self.filing_detail = Some(FilingDetail::from(&filing));
                self.filing_detail_state = FilingDetailState::new();
                self.filing_detail_from = Some(ViewState::CommitteeDetail);
                self.view_state = ViewState::FilingDetail;
                self.committee_detail_state.filing_detail_loading = false;
            }
            Err(e) => {
                // Show error in filings table header
                self.committee_detail_state.filing_detail_loading = false;
                self.committee_detail_state
                    .set_filings_error(format!("Error loading filing {}: {}", filing_id, e));
            }
        }
    }

    fn show_filer_from_filing(&mut self, sourcer: &mut FilingSourcer, filer_id: &str) {
        // Determine if it's a committee or candidate based on first letter
        if filer_id.starts_with('C') {
            // Committee
            if let Ok(mut db) = sourcer.cache.open_bulk_data_database() {
                if let Ok(Some(detail)) = crate::cache::bulk_committee::get_committee_detail(
                    &mut db, self.cycle, filer_id,
                ) {
                    self.committee_detail = Some(detail);
                    self.committee_detail_state = CommitteeDetailState::new();
                    self.view_state = ViewState::CommitteeDetail;
                }
            }
        } else if filer_id.starts_with('H')
            || filer_id.starts_with('S')
            || filer_id.starts_with('P')
        {
            // Candidate
            if let Ok(mut db) = sourcer.cache.open_bulk_data_database() {
                if let Ok(Some(detail)) = crate::cache::bulk_candidates::get_candidate_detail(
                    &mut db, self.cycle, filer_id,
                ) {
                    self.candidate_detail = Some(detail);
                    self.candidate_detail_state = CandidateDetailState::new();
                    self.view_state = ViewState::CandidateDetail;
                }
            }
        }
    }
}

pub fn search(mut sourcer: FilingSourcer, args: &SearchArgs) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(args.cycle, args.query.clone());
    app.cursor_position = app.input.len();
    app.search(&mut sourcer)?;

    let res = run_app(&mut terminal, &mut app, &mut sourcer);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
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
    // Helper function to perform search with animated spinner
    let search_with_loading = |app: &mut App, terminal: &mut Terminal<B>, sourcer: &mut FilingSourcer| -> anyhow::Result<()> {
        // Show initial spinner frame
        app.searching = true;
        app.advance_spinner();
        terminal.draw(|f| ui(f, app)).unwrap();

        // Perform the search (blocking)
        app.search(sourcer)?;

        app.searching = false;
        Ok(())
    };

    loop {
        terminal.draw(|f| ui(f, app)).unwrap();

        if app.should_exit {
            return Ok(());
        }

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Ctrl+C exits immediately from any view
            if key.code == KeyCode::Char('c')
                && key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                return Ok(());
            }

            // Handle detail views first - they manage their own keypresses
            match app.view_state {
                ViewState::CandidateDetail => {
                    if let Some(ref candidate) = app.candidate_detail {
                        match app.candidate_detail_state.handle_key_event(key, candidate) {
                            CandidateDetailAction::Exit => app.go_back_to_search(),
                            CandidateDetailAction::ShowCommitteeDetail { committee_id } => {
                                app.show_committee_detail_by_id(sourcer, &committee_id);
                            }
                            CandidateDetailAction::FetchFilings => {
                                let candidate_id = candidate.candidate_id.clone();
                                // Render the loading state before blocking API call
                                terminal.draw(|f| ui(f, app)).unwrap();
                                app.candidate_detail_state
                                    .fetch_filings_for_candidate(&candidate_id);
                            }
                            CandidateDetailAction::ShowFilingDetail { filing_id } => {
                                // Render the loading state before blocking API call
                                terminal.draw(|f| ui(f, app)).unwrap();
                                app.show_filing_detail_from_candidate(sourcer, &filing_id);
                            }
                            CandidateDetailAction::FetchF1Affiliations { committee_id } => {
                                // Render the loading state before blocking API call
                                terminal.draw(|f| ui(f, app)).unwrap();
                                app.candidate_detail_state
                                    .fetch_f1_affiliations(&committee_id, sourcer);
                            }
                            CandidateDetailAction::None => {}
                        }
                    }
                    continue;
                }
                ViewState::CommitteeDetail => {
                    if let Some(ref committee) = app.committee_detail {
                        match app.committee_detail_state.handle_key_event(key, committee) {
                            CommitteeDetailAction::Exit => app.go_back_to_search(),
                            CommitteeDetailAction::OpenBrowser => {
                                let _ = committee.open_in_browser();
                            }
                            CommitteeDetailAction::ShowFilingDetail { filing_id } => {
                                // Render the loading state before blocking API call
                                terminal.draw(|f| ui(f, app)).unwrap();
                                app.show_filing_detail(sourcer, &filing_id);
                            }
                            CommitteeDetailAction::FetchFilings => {
                                let committee_id = committee.committee_id.clone();
                                // Render the loading state before blocking API call
                                terminal.draw(|f| ui(f, app)).unwrap();
                                app.committee_detail_state
                                    .fetch_filings_for_committee(&committee_id);
                            }
                            CommitteeDetailAction::None => {}
                        }
                    }
                    continue;
                }
                ViewState::FilingDetail => {
                    if let Some(ref filing) = app.filing_detail {
                        match app.filing_detail_state.handle_key_event(key, filing) {
                            FilingDetailAction::Exit => match app.filing_detail_from {
                                Some(ViewState::CandidateDetail) => {
                                    app.go_back_to_candidate_detail()
                                }
                                _ => app.go_back_to_committee_detail(),
                            },
                            FilingDetailAction::OpenBrowser => {
                                let _ = filing.open_in_browser();
                            }
                            FilingDetailAction::ShowFiler { filer_id } => {
                                app.show_filer_from_filing(sourcer, &filer_id);
                            }
                            FilingDetailAction::None => {}
                        }
                    }
                    continue;
                }
                ViewState::Search => {} // Fall through to search view handling
            }

            // Search view key handling
            match key.code {
                KeyCode::Esc => return Ok(()),
                KeyCode::Tab => {
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::SHIFT)
                    {
                        app.focus_previous();
                    } else {
                        app.focus_next();
                    }
                }
                KeyCode::BackTab => {
                    app.focus_previous();
                }
                KeyCode::Enter => {
                    app.select_current(sourcer)?;
                }
                KeyCode::Up => {
                    match app.focus {
                        FocusPanel::Search => {
                            // Switch focus to results and navigate
                            let has_results = match app.active_tab {
                                ResultsTab::Candidates => !app.candidate_results.is_empty(),
                                ResultsTab::Committees => !app.committee_results.is_empty(),
                                ResultsTab::OpExp => !app.opexp_results.is_empty(),
                            };
                            if has_results {
                                app.focus = FocusPanel::Results;
                                match app.active_tab {
                                    ResultsTab::Candidates => app.candidate_previous(),
                                    ResultsTab::Committees => app.committee_previous(),
                                    ResultsTab::OpExp => app.opexp_previous(),
                                }
                            }
                        }
                        FocusPanel::Cycle => {
                            app.cycle_next();
                            search_with_loading(app, terminal, sourcer)?;
                        }
                        FocusPanel::Results => match app.active_tab {
                            ResultsTab::Candidates => app.candidate_previous(),
                            ResultsTab::Committees => app.committee_previous(),
                            ResultsTab::OpExp => app.opexp_previous(),
                        },
                    }
                }
                KeyCode::Down => {
                    match app.focus {
                        FocusPanel::Search => {
                            // Switch focus to results and navigate
                            let has_results = match app.active_tab {
                                ResultsTab::Candidates => !app.candidate_results.is_empty(),
                                ResultsTab::Committees => !app.committee_results.is_empty(),
                                ResultsTab::OpExp => !app.opexp_results.is_empty(),
                            };
                            if has_results {
                                app.focus = FocusPanel::Results;
                                match app.active_tab {
                                    ResultsTab::Candidates => app.candidate_next(),
                                    ResultsTab::Committees => app.committee_next(),
                                    ResultsTab::OpExp => app.opexp_next(),
                                }
                            }
                        }
                        FocusPanel::Cycle => {
                            app.cycle_previous();
                            search_with_loading(app, terminal, sourcer)?;
                        }
                        FocusPanel::Results => match app.active_tab {
                            ResultsTab::Candidates => app.candidate_next(),
                            ResultsTab::Committees => app.committee_next(),
                            ResultsTab::OpExp => app.opexp_next(),
                        },
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
                    if app.focus == FocusPanel::Search {
                        // Ctrl+Backspace: delete word backward
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                        {
                            app.delete_word_backward();
                            search_with_loading(app, terminal, sourcer)?;
                        } else if app.cursor_position > 0 {
                            app.input.remove(app.cursor_position - 1);
                            app.cursor_position -= 1;
                            search_with_loading(app, terminal, sourcer)?;
                        }
                    }
                }
                KeyCode::Delete => {
                    if app.focus == FocusPanel::Search && app.cursor_position < app.input.len() {
                        app.input.remove(app.cursor_position);
                        search_with_loading(app, terminal, sourcer)?;
                    }
                }
                KeyCode::PageUp => {
                    app.cycle_next();
                    search_with_loading(app, terminal, sourcer)?;
                }
                KeyCode::PageDown => {
                    app.cycle_previous();
                    search_with_loading(app, terminal, sourcer)?;
                }
                KeyCode::Char(c) => {
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
                    // Ctrl+E to switch to OpExp tab
                    else if c == 'e'
                        && key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        app.active_tab = ResultsTab::OpExp;
                        // Search when switching to OpExp tab if input exists
                        if !app.input.is_empty() {
                            search_with_loading(app, terminal, sourcer)?;
                        }
                        // Ensure opexp has selection if results exist
                        if !app.opexp_results.is_empty()
                            && app.opexp_table_state.selected().is_none()
                        {
                            app.opexp_table_state.select(Some(0));
                        }
                    }
                    // Ctrl+U: clear entire input
                    else if c == 'u'
                        && key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        app.clear_input();
                        search_with_loading(app, terminal, sourcer)?;
                    }
                    // Ctrl+K: delete to end of line
                    else if c == 'k'
                        && key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        app.delete_to_end();
                        search_with_loading(app, terminal, sourcer)?;
                    }
                    // Ctrl+W: delete word backward (alternative to Ctrl+Backspace)
                    else if c == 'w'
                        && key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        app.delete_word_backward();
                        search_with_loading(app, terminal, sourcer)?;
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
                    else if c.is_alphanumeric() || c.is_whitespace() || c == '-' || c == '_' || c == '.' || c == ',' {
                        app.input.insert(app.cursor_position, c);
                        app.cursor_position += 1;
                        search_with_loading(app, terminal, sourcer)?;
                        app.focus = FocusPanel::Search;
                    }
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
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

fn render_search_bar(f: &mut Frame, app: &mut App, area: Rect) {
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

fn render_help_text(f: &mut Frame, _app: &mut App, area: Rect) {
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

fn render_opexp_results_table(f: &mut Frame, app: &mut App, area: Rect) {
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
        format!(
            "Operating Expenses ({} matches)",
            app.opexp_results.len()
        )
    };

    let opexp_table = Table::new(
        opexp_rows,
        [
            Constraint::Length(12),  // Committee
            Constraint::Length(30),  // Recipient
            Constraint::Length(15),  // City
            Constraint::Length(5),   // State
            Constraint::Length(10),  // Date
            Constraint::Length(12),  // Amount
            Constraint::Min(20),     // Purpose
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

fn render_search_view(f: &mut Frame, app: &mut App, area: Rect) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    fn create_test_app() -> App {
        let mut app = App::new(2024, "biden".to_string());
        // Add some sample results
        app.candidate_results = vec![
            CandidateSearchResult {
                candidate_id: "P00003392".to_string(),
                name: "BIDEN, JOSEPH R JR".to_string(),
                election_year: 2020,
                office: "P".to_string(),
                state: "".to_string(),
                district: "".to_string(),
                principal_campaign_committee: Some("C00703975".to_string()),
            },
        ];
        app.committee_results = vec![
            CommitteeSearchResult {
                committee_id: "C00703975".to_string(),
                name: "BIDEN FOR PRESIDENT".to_string(),
                committee_type: "P".to_string(),
                designation: "P".to_string(),
                party_affiliation: "DEM".to_string(),
                connected_org_name: "".to_string(),
                candidate_id: Some("P00003392".to_string()),
            },
        ];
        app.candidate_table_state.select(Some(0));
        app
    }

    #[test]
    fn test_render_search_view() {
        let mut app = create_test_app();
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal
            .draw(|f| render_search_view(f, &mut app, f.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_search_view_empty() {
        let mut app = App::new(2024, "".to_string());
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal
            .draw(|f| render_search_view(f, &mut app, f.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_help_text() {
        let mut app = App::new(2024, "".to_string());
        let mut terminal = Terminal::new(TestBackend::new(100, 2)).unwrap();
        terminal
            .draw(|f| render_help_text(f, &mut app, f.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_search_view_opexp() {
        let mut app = App::new(2024, "consulting".to_string());
        // Set active tab to OpExp
        app.active_tab = ResultsTab::OpExp;
        // Add some sample operating expense results
        app.opexp_results = vec![
            OpExpSearchResult {
                committee_id: "C00703975".to_string(),
                name: "ACME CONSULTING LLC".to_string(),
                city: "WASHINGTON".to_string(),
                state: "DC".to_string(),
                transaction_date: "2024-03-15".to_string(),
                transaction_amount: 5000.00,
                purpose: "STRATEGY CONSULTING".to_string(),
                filing_id: 123456,
            },
            OpExpSearchResult {
                committee_id: "C00703975".to_string(),
                name: "SMITH CONSULTING GROUP".to_string(),
                city: "NEW YORK".to_string(),
                state: "NY".to_string(),
                transaction_date: "2024-02-28".to_string(),
                transaction_amount: 3500.50,
                purpose: "MEDIA CONSULTING".to_string(),
                filing_id: 123457,
            },
        ];
        app.opexp_table_state.select(Some(0));
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal
            .draw(|f| render_search_view(f, &mut app, f.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_search_bar_with_spinner() {
        let mut app = App::new(2024, "biden".to_string());
        app.searching = true;
        app.spinner_frame = 2; // Use a specific frame for consistent testing
        let mut terminal = Terminal::new(TestBackend::new(100, 3)).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                render_search_bar(f, &mut app, area);
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
