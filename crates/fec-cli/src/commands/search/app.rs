//! Application state and business logic for the search TUI
//!
//! This module contains the core state management and business logic for the search interface.
//! It defines the App struct and all operations that modify application state.

use crate::{
    cache::bulk_candidates::{CandidateDetail, CandidateSearchResult},
    cache::bulk_committee::{CommitteeDetail, CommitteeSearchResult},
    cache::bulk_opexp::OpExpSearchResult,
    sourcer::FilingSourcer,
    tui::candidate_detail::CandidateDetailState,
    tui::committee_detail::CommitteeDetailState,
    tui::filing_detail::{FilingDetail, FilingDetailState},
};
use anyhow::Result;
use ratatui::widgets::TableState;
use std::time::{Duration, Instant};

/// Which panel currently has focus in the search view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusPanel {
    Search,
    Cycle,
    Results,
}

impl FocusPanel {
    pub(crate) fn next(&self) -> Self {
        match self {
            FocusPanel::Search => FocusPanel::Cycle,
            FocusPanel::Cycle => FocusPanel::Results,
            FocusPanel::Results => FocusPanel::Search,
        }
    }

    pub(crate) fn previous(&self) -> Self {
        match self {
            FocusPanel::Search => FocusPanel::Results,
            FocusPanel::Cycle => FocusPanel::Search,
            FocusPanel::Results => FocusPanel::Cycle,
        }
    }
}

/// Which results tab is currently active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResultsTab {
    Candidates,
    Committees,
    OpExp,
}

/// Which view is currently displayed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewState {
    Search,
    CandidateDetail,
    CommitteeDetail,
    FilingDetail,
}

/// Main application state for the search TUI
pub(crate) struct App {
    pub(crate) input: String,
    pub(crate) cycle: u16,
    pub(crate) candidate_results: Vec<CandidateSearchResult>,
    pub(crate) committee_results: Vec<CommitteeSearchResult>,
    pub(crate) opexp_results: Vec<OpExpSearchResult>,
    pub(crate) candidate_table_state: TableState,
    pub(crate) committee_table_state: TableState,
    pub(crate) opexp_table_state: TableState,
    pub(crate) cursor_position: usize,
    pub(crate) last_query_duration: Option<Duration>,
    pub(crate) should_exit: bool,
    pub(crate) focus: FocusPanel,
    pub(crate) active_tab: ResultsTab,
    pub(crate) view_state: ViewState,
    pub(crate) candidate_detail: Option<CandidateDetail>,
    pub(crate) committee_detail: Option<CommitteeDetail>,
    pub(crate) candidate_detail_state: CandidateDetailState,
    pub(crate) committee_detail_state: CommitteeDetailState,
    pub(crate) filing_detail: Option<FilingDetail>,
    pub(crate) filing_detail_state: FilingDetailState,
    /// Tracks which detail view we came from when showing filing detail
    pub(crate) filing_detail_from: Option<ViewState>,
    /// Whether a search is currently in progress
    pub(crate) searching: bool,
    /// Spinner frame for loading animation
    pub(crate) spinner_frame: usize,
}

/// Spinner animation frames using Braille patterns
pub(crate) const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

impl App {
    pub(crate) fn new(initial_cycle: u16, initial_query: String) -> Self {
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

    pub(crate) fn advance_spinner(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
    }

    pub(crate) fn focus_next(&mut self) {
        self.focus = self.focus.next();
    }

    pub(crate) fn focus_previous(&mut self) {
        self.focus = self.focus.previous();
    }

    pub(crate) fn search(&mut self, sourcer: &mut FilingSourcer) -> Result<()> {
        let start = Instant::now();

        if self.input.is_empty() {
            self.candidate_results.clear();
            self.committee_results.clear();
            self.opexp_results.clear();
            self.last_query_duration = None;
        } else {
            match sourcer.cache.open_bulk_data_database() {
                Ok(mut db) => {
                    // Check if input is a district query (e.g., "CA41", "IL09")
                    let candidate_results = if let Some((state, district)) =
                        crate::cache::bulk_candidates::parse_district_query(&self.input)
                    {
                        crate::cache::bulk_candidates::filter_candidates_by_district(
                            &mut db,
                            self.cycle,
                            &state,
                            &district,
                        )
                        .unwrap_or_default()
                    } else {
                        crate::cache::bulk_candidates::search_candidates(
                            &mut db,
                            self.cycle,
                            &self.input,
                        )
                        .unwrap_or_default()
                    };
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

    pub(crate) fn candidate_next(&mut self) {
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

    pub(crate) fn candidate_previous(&mut self) {
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

    pub(crate) fn committee_next(&mut self) {
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

    pub(crate) fn committee_previous(&mut self) {
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

    pub(crate) fn opexp_next(&mut self) {
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

    pub(crate) fn opexp_previous(&mut self) {
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

    pub(crate) fn select_current(&mut self, sourcer: &mut FilingSourcer) -> Result<()> {
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

    pub(crate) fn go_back_to_search(&mut self) {
        self.view_state = ViewState::Search;
        self.candidate_detail = None;
        self.committee_detail = None;
        self.candidate_detail_state = CandidateDetailState::new();
        self.committee_detail_state = CommitteeDetailState::new();
        self.filing_detail = None;
        self.filing_detail_state = FilingDetailState::new();
    }

    pub(crate) fn go_back_to_committee_detail(&mut self) {
        self.view_state = ViewState::CommitteeDetail;
        self.filing_detail = None;
        self.filing_detail_state = FilingDetailState::new();
        self.committee_detail_state.filing_detail_loading = false;
    }

    pub(crate) fn go_back_to_candidate_detail(&mut self) {
        self.view_state = ViewState::CandidateDetail;
        self.filing_detail = None;
        self.filing_detail_state = FilingDetailState::new();
        self.candidate_detail_state.filing_detail_loading = false;
    }

    pub(crate) fn show_committee_detail_by_id(
        &mut self,
        sourcer: &mut FilingSourcer,
        committee_id: &str,
    ) {
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

    pub(crate) fn show_filing_detail_from_candidate(
        &mut self,
        sourcer: &mut FilingSourcer,
        filing_id: &str,
    ) {
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

    pub(crate) fn cycle_next(&mut self) {
        self.cycle += 2;
    }

    pub(crate) fn cycle_previous(&mut self) {
        if self.cycle > 2000 {
            self.cycle -= 2;
        }
    }

    /// Clear the entire input
    pub(crate) fn clear_input(&mut self) {
        self.input.clear();
        self.cursor_position = 0;
    }

    /// Delete from cursor to end of input
    pub(crate) fn delete_to_end(&mut self) {
        self.input.truncate(self.cursor_position);
    }

    /// Delete word backward from cursor (Ctrl+W, Ctrl+Backspace)
    pub(crate) fn delete_word_backward(&mut self) {
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

    pub(crate) fn show_filing_detail(&mut self, sourcer: &mut FilingSourcer, filing_id: &str) {
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

    pub(crate) fn show_filer_from_filing(&mut self, sourcer: &mut FilingSourcer, filer_id: &str) {
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
