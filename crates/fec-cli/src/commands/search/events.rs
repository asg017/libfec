//! Event handling and keyboard input processing for the search TUI
//!
//! This module contains the main event loop and keyboard input routing logic.

use super::app::{App, FocusPanel, ResultsTab, ViewState};
use super::ui::ui;
use crate::{
    sourcer::{Contest, FilingSourcer},
    tui::candidate_detail::CandidateDetailAction,
    tui::committee_detail::CommitteeDetailAction,
    tui::filing_detail::FilingDetailAction,
};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::Terminal;

/// Main event loop for the search TUI
pub(crate) fn run_app<B: ratatui::backend::Backend<Error: Send + Sync + 'static>>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    sourcer: &mut FilingSourcer,
) -> anyhow::Result<()> {
    // Helper function to perform search with animated spinner
    let search_with_loading = |app: &mut App,
                               terminal: &mut Terminal<B>,
                               sourcer: &mut FilingSourcer|
     -> anyhow::Result<()> {
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
                            CandidateDetailAction::OpenFecPage => {
                                let url = format!(
                                    "https://www.fec.gov/data/candidate/{}/",
                                    candidate.candidate_id
                                );
                                let _ = open::that(&url);
                            }
                            CandidateDetailAction::ShowContest {
                                office,
                                state,
                                district,
                            } => {
                                let contest = match office.as_str() {
                                    "P" => Some(Contest::President),
                                    "S" => Some(Contest::Senate { state }),
                                    "H" => Some(Contest::House { state, district }),
                                    _ => None,
                                };
                                if let Some(contest) = contest {
                                    let _ = crate::commands::contest::run_contest_tui(
                                        terminal, sourcer, &contest, app.cycle,
                                    );
                                }
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
                            FilingDetailAction::OpenWebsite { url } => {
                                let _ = open::that(&url);
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
                    else if c.is_alphanumeric()
                        || c.is_whitespace()
                        || c == '-'
                        || c == '_'
                        || c == '.'
                        || c == ','
                    {
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
