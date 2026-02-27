//! Candidate Detail TUI Component
//!
//! This module provides rendering functions for displaying detailed candidate information
//! within a ratatui application. It integrates with parent TUI apps (search, info) to
//! provide a seamless navigation experience.
//!
//! The detail view shows all available candidate information including name, party,
//! office, address, campaign committee, linked committees, and status.
//!
//! Keyboard shortcuts:
//! - Esc/q: Return to previous view
//! - c: View principal campaign committee (if available)
//! - a: Fetch F1 affiliations (affiliated committees and joint fund participants)
//! - y: Open copy popup to copy candidate ID, committee ID, or name to clipboard
//! - f: Fetch and display filings for this candidate
//! - j/k: Navigate filings (when filings are loaded)
//! - Enter: View selected filing detail (when filings are loaded)
//!
//! Copy popup navigation:
//! - ↑/↓ or j/k: Navigate options
//! - Enter: Copy selected value to clipboard
//! - Esc: Cancel and close popup

use crate::cache::bulk::candidate_committee_linkage::CommitteeLinkage;
use crate::cache::bulk::candidates::CandidateDetail;
use crate::tui::committee_detail::FilingListItem;
use crate::tui::{navigation_popup_help_line, HelpBar};
use crossterm::event::{KeyCode, KeyEvent};
use fec_api::{Api, CandidateId, CommitteeId, FilingArgsBuilder};
use fec_parser::mappings::column_names_for_field;
use indexmap::IndexMap;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

/// Information about affiliated committees and joint fund participants from F1S schedules
#[derive(Debug, Clone)]
pub struct F1Affiliation {
    /// The committee ID of the affiliated committee
    pub affiliated_committee_id: Option<String>,
    /// The name of the affiliated committee
    pub affiliated_committee_name: Option<String>,
    /// The joint fund participant committee name
    pub joint_fund_participant_name: Option<String>,
    /// The joint fund participant committee ID
    pub joint_fund_participant_id: Option<String>,
}

impl F1Affiliation {
    fn from_row_data(data: &IndexMap<String, String>) -> Self {
        Self {
            affiliated_committee_id: data
                .get("affiliated_committee_id_number")
                .cloned()
                .filter(|s| !s.is_empty()),
            affiliated_committee_name: data
                .get("affiliated_committee_name")
                .cloned()
                .filter(|s| !s.is_empty()),
            joint_fund_participant_name: data
                .get("joint_fund_participant_committee_name")
                .cloned()
                .filter(|s| !s.is_empty()),
            joint_fund_participant_id: data
                .get("joint_fund_participant_committee_id_number")
                .cloned()
                .filter(|s| !s.is_empty()),
        }
    }

    fn has_data(&self) -> bool {
        self.affiliated_committee_id.is_some()
            || self.affiliated_committee_name.is_some()
            || self.joint_fund_participant_name.is_some()
            || self.joint_fund_participant_id.is_some()
    }
}

/// Action returned by handle_key_event indicating what the parent should do
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateDetailAction {
    /// Key was handled internally, no action needed from parent
    None,
    /// User wants to exit/go back
    Exit,
    /// User pressed 'c' to view the principal campaign committee
    ShowCommitteeDetail { committee_id: String },
    /// User pressed 'f' to fetch filings - parent should call fetch_filings_for_candidate
    FetchFilings,
    /// User pressed Enter on a filing - parent should show filing detail
    ShowFilingDetail { filing_id: String },
    /// User pressed 'a' to fetch F1 affiliations - parent should call fetch_f1_affiliations
    FetchF1Affiliations { committee_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YankOption {
    CandidateId,
    CommitteeId,
    CandidateName,
}

pub struct CandidateDetailState {
    pub show_yank_popup: bool,
    pub yank_selected: usize,
    /// Linked committees (populated when detail view is shown)
    pub linked_committees: Vec<CommitteeLinkage>,
    /// Filings list (populated when 'f' is pressed)
    pub filings: Vec<FilingListItem>,
    /// Table state for filings list
    pub filings_table_state: TableState,
    /// Whether filings are currently loading
    pub filings_loading: bool,
    /// Error message if filings fetch failed
    pub filings_error: Option<String>,
    /// The cycle used for fetching filings
    pub filings_cycle: u16,
    /// Whether a filing detail is currently being loaded
    pub filing_detail_loading: bool,
    /// F1 affiliations (affiliated committees and joint fund participants)
    pub f1_affiliations: Vec<F1Affiliation>,
    /// Whether F1 affiliations are currently loading
    pub f1_loading: bool,
    /// Error message if F1 fetch failed
    pub f1_error: Option<String>,
    /// The filing ID of the most recent F1
    pub f1_filing_id: Option<String>,
}

impl CandidateDetailState {
    pub fn new() -> Self {
        Self {
            show_yank_popup: false,
            yank_selected: 0,
            linked_committees: Vec::new(),
            filings: Vec::new(),
            filings_table_state: TableState::default(),
            filings_loading: false,
            filings_error: None,
            filings_cycle: 2026,
            filing_detail_loading: false,
            f1_affiliations: Vec::new(),
            f1_loading: false,
            f1_error: None,
            f1_filing_id: None,
        }
    }

    /// Set linked committees
    pub fn set_linked_committees(&mut self, linkages: Vec<CommitteeLinkage>) {
        self.linked_committees = linkages;
    }

    /// Set filings after fetch completes
    pub fn set_filings(&mut self, filings: Vec<FilingListItem>) {
        self.filings = filings;
        self.filings_loading = false;
        self.filings_error = None;
        if !self.filings.is_empty() {
            self.filings_table_state.select(Some(0));
        }
    }

    /// Set filings error
    pub fn set_filings_error(&mut self, error: String) {
        self.filings_loading = false;
        self.filings_error = Some(error);
    }

    fn filings_select_next(&mut self) {
        let count = self.filings.len();
        if count == 0 {
            return;
        }
        let i = match self.filings_table_state.selected() {
            Some(i) => {
                if i >= count - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.filings_table_state.select(Some(i));
    }

    fn filings_select_previous(&mut self) {
        let count = self.filings.len();
        if count == 0 {
            return;
        }
        let i = match self.filings_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.filings_table_state.select(Some(i));
    }

    fn filings_select_first(&mut self) {
        if !self.filings.is_empty() {
            self.filings_table_state.select(Some(0));
        }
    }

    fn filings_select_last(&mut self) {
        if !self.filings.is_empty() {
            self.filings_table_state
                .select(Some(self.filings.len() - 1));
        }
    }

    fn get_selected_filing(&self) -> Option<&FilingListItem> {
        self.filings_table_state
            .selected()
            .and_then(|i| self.filings.get(i))
    }

    /// Fetch filings for a candidate from the FEC API
    pub fn fetch_filings_for_candidate(&mut self, candidate_id: &str) {
        let api_key = std::env::var("LIBFEC_API_KEY").unwrap_or_else(|_| "DEMO_KEY".to_string());
        let client = Api::new(api_key.as_str());

        let args = FilingArgsBuilder::default()
            .committees(Vec::<CommitteeId>::new())
            .candidates(vec![CandidateId::new(candidate_id).unwrap()])
            .form_types(None)
            .report_types(None)
            .committee_types(None)
            .cycle(vec![self.filings_cycle])
            .build();

        match args {
            Ok(filing_args) => {
                let url = client.filings_url(filing_args);
                match fec_api::api_request(&url.0) {
                    Ok(response) => {
                        let filings: Vec<FilingListItem> = response
                            .result_items
                            .iter()
                            .filter_map(FilingListItem::from_api_value)
                            .collect();
                        self.set_filings(filings);
                    }
                    Err(e) => {
                        self.set_filings_error(e.to_string());
                    }
                }
            }
            Err(e) => {
                self.set_filings_error(e.to_string());
            }
        }
    }

    /// Fetch the most recent F1 filing for a committee and parse F1S itemizations
    pub fn fetch_f1_affiliations(
        &mut self,
        committee_id: &str,
        sourcer: &crate::sourcer::FilingSourcer,
    ) {
        self.f1_loading = true;
        self.f1_error = None;
        self.f1_affiliations.clear();
        self.f1_filing_id = None;

        let api_key = std::env::var("LIBFEC_API_KEY").unwrap_or_else(|_| "DEMO_KEY".to_string());
        let client = Api::new(api_key.as_str());

        // Fetch F1 filings for the committee
        let args = FilingArgsBuilder::default()
            .committees(vec![CommitteeId::new(committee_id).unwrap()])
            .candidates(Vec::<CandidateId>::new())
            .form_types(Some(vec!["F1".to_string()]))
            .report_types(None)
            .committee_types(None)
            .cycle(vec![self.filings_cycle])
            .build();

        match args {
            Ok(filing_args) => {
                let url = client.filings_url(filing_args);
                match fec_api::api_request(&url.0) {
                    Ok(response) => {
                        // Find the most recent F1 filing
                        let f1_filing = response
                            .result_items
                            .iter()
                            .filter_map(FilingListItem::from_api_value)
                            .find(|f| f.form_type.starts_with("F1"));

                        if let Some(filing_item) = f1_filing {
                            self.f1_filing_id = Some(filing_item.filing_id.clone());
                            // Now fetch and parse the actual filing using FilingSourcer
                            self.parse_f1_filing(&filing_item.filing_id, sourcer);
                        } else {
                            self.f1_loading = false;
                        }
                    }
                    Err(e) => {
                        self.f1_loading = false;
                        self.f1_error = Some(e.to_string());
                    }
                }
            }
            Err(e) => {
                self.f1_loading = false;
                self.f1_error = Some(e.to_string());
            }
        }
    }

    /// Parse F1S itemizations from a filing using FilingSourcer
    fn parse_f1_filing(&mut self, filing_id: &str, sourcer: &crate::sourcer::FilingSourcer) {
        match sourcer.resolve_from_user_argument(filing_id) {
            Ok(mut filing) => {
                let fec_version = filing.header.fec_version.clone();

                // Iterate through rows looking for F1S schedule rows
                while let Some(row_result) = filing.next_row() {
                    if let Ok(row) = row_result {
                        if row.row_type.to_uppercase().starts_with("F1S") {
                            // Parse the F1S row
                            if let Ok(columns) = column_names_for_field(&row.row_type, &fec_version)
                            {
                                let mut data = IndexMap::new();
                                for (column_name, field) in columns.iter().zip(row.record.iter()) {
                                    data.insert(column_name.to_owned(), field.to_owned());
                                }
                                let affiliation = F1Affiliation::from_row_data(&data);
                                if affiliation.has_data() {
                                    self.f1_affiliations.push(affiliation);
                                }
                            }
                        }
                    }
                }
                self.f1_loading = false;
            }
            Err(e) => {
                self.f1_loading = false;
                self.f1_error = Some(format!("Failed to fetch filing: {}", e));
            }
        }
    }

    pub fn get_yank_options(
        &self,
        candidate: &CandidateDetail,
    ) -> Vec<(YankOption, String, Option<String>)> {
        let mut options = vec![];

        options.push((
            YankOption::CandidateId,
            "Candidate ID".to_string(),
            Some(candidate.candidate_id.clone()),
        ));

        if let Some(ref committee_id) = candidate.principal_campaign_committee {
            options.push((
                YankOption::CommitteeId,
                "Committee ID".to_string(),
                Some(committee_id.clone()),
            ));
        }

        options.push((
            YankOption::CandidateName,
            "Candidate Name".to_string(),
            Some(candidate.name.clone()),
        ));

        options
    }

    pub fn copy_selected(&self, candidate: &CandidateDetail) {
        let options = self.get_yank_options(candidate);
        if let Some((_, _, Some(value))) = options.get(self.yank_selected) {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(value);
            }
        }
    }

    pub fn yank_next(&mut self, candidate: &CandidateDetail) {
        let options_count = self.get_yank_options(candidate).len();
        if self.yank_selected < options_count.saturating_sub(1) {
            self.yank_selected += 1;
        } else {
            self.yank_selected = 0;
        }
    }

    pub fn yank_previous(&mut self, candidate: &CandidateDetail) {
        let options_count = self.get_yank_options(candidate).len();
        if self.yank_selected > 0 {
            self.yank_selected -= 1;
        } else {
            self.yank_selected = options_count.saturating_sub(1);
        }
    }

    /// Handle a key event and return an action for the parent to perform
    pub fn handle_key_event(
        &mut self,
        key: KeyEvent,
        candidate: &CandidateDetail,
    ) -> CandidateDetailAction {
        // Handle yank popup first
        if self.show_yank_popup {
            return match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('y') => {
                    self.show_yank_popup = false;
                    CandidateDetailAction::None
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.yank_next(candidate);
                    CandidateDetailAction::None
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.yank_previous(candidate);
                    CandidateDetailAction::None
                }
                KeyCode::Enter => {
                    self.copy_selected(candidate);
                    self.show_yank_popup = false;
                    CandidateDetailAction::None
                }
                _ => CandidateDetailAction::None,
            };
        }

        // Handle keys
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => CandidateDetailAction::Exit,
            KeyCode::Char('c') => {
                if let Some(ref committee_id) = candidate.principal_campaign_committee {
                    CandidateDetailAction::ShowCommitteeDetail {
                        committee_id: committee_id.clone(),
                    }
                } else {
                    CandidateDetailAction::None
                }
            }
            KeyCode::Char('y') => {
                self.show_yank_popup = true;
                CandidateDetailAction::None
            }
            KeyCode::Char('f') => {
                self.filings_loading = true;
                CandidateDetailAction::FetchFilings
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.filings_select_next();
                CandidateDetailAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.filings_select_previous();
                CandidateDetailAction::None
            }
            KeyCode::Char('g') => {
                self.filings_select_first();
                CandidateDetailAction::None
            }
            KeyCode::Char('G') => {
                self.filings_select_last();
                CandidateDetailAction::None
            }
            KeyCode::Enter => {
                if let Some(filing_id) = self.get_selected_filing().map(|f| f.filing_id.clone()) {
                    self.filing_detail_loading = true;
                    self.filings_error = None;
                    CandidateDetailAction::ShowFilingDetail { filing_id }
                } else {
                    CandidateDetailAction::None
                }
            }
            KeyCode::Char('a') => {
                if let Some(ref committee_id) = candidate.principal_campaign_committee {
                    self.f1_loading = true;
                    CandidateDetailAction::FetchF1Affiliations {
                        committee_id: committee_id.clone(),
                    }
                } else {
                    CandidateDetailAction::None
                }
            }
            _ => CandidateDetailAction::None,
        }
    }
}

impl Default for CandidateDetailState {
    fn default() -> Self {
        Self::new()
    }
}

fn render_title(f: &mut Frame, candidate: &CandidateDetail, area: Rect) {
    let title_text = format!("{} ({})", candidate.name, candidate.candidate_id);
    let title = Paragraph::new(title_text)
        .block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(Color::Green)),
        )
        .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(title, area);
}

fn render_content(
    f: &mut Frame,
    candidate: &CandidateDetail,
    state: &CandidateDetailState,
    area: Rect,
) {
    let mut lines = vec![];
    let dim = Style::default().add_modifier(Modifier::DIM);
    let election_line = match candidate.office.as_str() {
        "H" => Line::from(vec![
            Span::styled(
                format!("{}{}", candidate.state, candidate.district),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(" candidate in ", dim),
            Span::styled(
                candidate.election_year.to_string(),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(", running as a ", dim),
            match candidate.party_affiliation.as_str() {
                "DEM" => Span::styled("Democrat", Style::default().fg(Color::Rgb(0, 0, 255))),
                "REP" => Span::styled("⬤Republican", Style::default().fg(Color::Rgb(255, 0, 0))),
                other => Span::styled(other, Style::default().fg(Color::Yellow)),
            },
        ]),
        "S" => Line::from(vec![
            Span::styled(candidate.state.as_str(), Style::default().fg(Color::Cyan)),
            Span::styled(" candidate in ", dim),
            Span::styled(
                format!("{} Senate race", candidate.election_year),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(", running as a ", dim),
            match candidate.party_affiliation.as_str() {
                "DEM" => Span::styled("Democrat", Style::default().fg(Color::Rgb(0, 0, 255))),
                "REP" => Span::styled("⬤Republican", Style::default().fg(Color::Rgb(255, 0, 0))),
                other => Span::styled(other, Style::default().fg(Color::Yellow)),
            },
        ]),
        "P" => Line::from(vec![
            Span::styled("Presidential candidate", Style::default().fg(Color::Cyan)),
            Span::styled(" in ", dim),
            Span::styled(
                candidate.election_year.to_string(),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(", running as a ", dim),
            match candidate.party_affiliation.as_str() {
                "DEM" => Span::styled("Democrat", Style::default().fg(Color::Rgb(0, 0, 255))),
                "REP" => Span::styled("⬤Republican", Style::default().fg(Color::Rgb(255, 0, 0))),
                other => Span::styled(other, Style::default().fg(Color::Yellow)),
            },
        ]),
        _ => Line::from(""),
    };
    lines.push(election_line);

    // Principal campaign committee
    if let Some(ref pcc) = candidate.principal_campaign_committee {
        lines.push(Line::from(vec![
            Span::styled(
                "Principal Campaign Committee: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(pcc, Style::default().fg(Color::Magenta)),
            Span::styled(" (press 'c' to view)", Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(""));
    }

    // F1 Affiliations (from most recent F1 filing)
    if state.f1_loading {
        lines.push(Line::from(vec![
            Span::styled(
                "F1 Affiliations: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Loading...", Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(""));
    } else if let Some(ref error) = state.f1_error {
        lines.push(Line::from(vec![
            Span::styled(
                "F1 Affiliations: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("Error: {}", error), Style::default().fg(Color::Red)),
        ]));
        lines.push(Line::from(""));
    } else if !state.f1_affiliations.is_empty() {
        let title = if let Some(ref filing_id) = state.f1_filing_id {
            format!("F1 Affiliations (FEC-{}):", filing_id)
        } else {
            "F1 Affiliations:".to_string()
        };
        lines.push(Line::from(vec![Span::styled(
            title,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
        for affiliation in &state.f1_affiliations {
            // Show joint fund participant if present
            if let (Some(ref name), Some(ref id)) = (
                &affiliation.joint_fund_participant_name,
                &affiliation.joint_fund_participant_id,
            ) {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Joint Fund: ", Style::default().fg(Color::Cyan)),
                    Span::raw(name),
                    Span::styled(format!(" ({})", id), Style::default().fg(Color::DarkGray)),
                ]));
            } else if let Some(ref name) = affiliation.joint_fund_participant_name {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Joint Fund: ", Style::default().fg(Color::Cyan)),
                    Span::raw(name),
                ]));
            }
            // Show affiliated committee if present
            if let (Some(ref name), Some(ref id)) = (
                &affiliation.affiliated_committee_name,
                &affiliation.affiliated_committee_id,
            ) {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Affiliated: ", Style::default().fg(Color::Magenta)),
                    Span::raw(name),
                    Span::styled(format!(" ({})", id), Style::default().fg(Color::DarkGray)),
                ]));
            } else if let Some(ref name) = affiliation.affiliated_committee_name {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Affiliated: ", Style::default().fg(Color::Magenta)),
                    Span::raw(name),
                ]));
            } else if let Some(ref id) = affiliation.affiliated_committee_id {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Affiliated: ", Style::default().fg(Color::Magenta)),
                    Span::styled(id, Style::default().fg(Color::Cyan)),
                ]));
            }
        }
        lines.push(Line::from(""));
    }

    // Linked committees
    if state.linked_committees.len() > 1 {
        lines.push(Line::from(vec![Span::styled(
            "Linked Committees:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));
        for linkage in &state.linked_committees {
            // Skip the principal campaign committee since we already show it
            if candidate.principal_campaign_committee.as_ref() == Some(&linkage.committee_id) {
                continue;
            }
            let designation = linkage.designation_description();
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(&linkage.committee_id, Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!(" - {}", designation),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Address
    if !candidate.address_street1.is_empty() || !candidate.address_city.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "Address:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));

        if !candidate.address_street1.is_empty() {
            lines.push(Line::from(format!("  {}", candidate.address_street1)));
        }
        if !candidate.address_street2.is_empty() {
            lines.push(Line::from(format!("  {}", candidate.address_street2)));
        }

        let mut city_line = String::from("  ");
        if !candidate.address_city.is_empty() {
            city_line.push_str(&candidate.address_city);
        }
        if !candidate.address_state.is_empty() {
            if !candidate.address_city.is_empty() {
                city_line.push_str(", ");
            }
            city_line.push_str(&candidate.address_state);
        }
        if !candidate.address_zip.is_empty() {
            city_line.push(' ');
            city_line.push_str(&candidate.address_zip);
        }
        if city_line.len() > 2 {
            lines.push(Line::from(city_line));
        }
    }

    let content = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(content, area);
}

fn render_filings_table(
    f: &mut Frame,
    candidate: &CandidateDetail,
    state: &mut CandidateDetailState,
    area: Rect,
) {
    let header_row = Row::new(vec![
        Cell::from("Filing ID").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Form").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Report").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Coverage").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Received").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ])
    .height(1);

    let rows: Vec<Row> = if state.filings.is_empty() && !state.filings_loading {
        // Show prompt when there are no filings
        vec![Row::new(vec![Cell::from(Span::styled(
            "Press 'f' to fetch filings",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ))
        .style(Style::default().fg(Color::DarkGray))])]
    } else {
        state
            .filings
            .iter()
            .map(|filing| {
                let coverage = match (&filing.coverage_from, &filing.coverage_through) {
                    (Some(from), Some(through)) => format!("{} - {}", from, through),
                    _ => "-".to_string(),
                };

                // Color code by form type
                let form_color = match filing.form_type.as_str() {
                    f if f.starts_with("F3P") => Color::Magenta,
                    f if f.starts_with("F3X") => Color::Cyan,
                    f if f.starts_with("F3") => Color::Green,
                    f if f.starts_with("F1") => Color::Yellow,
                    f if f.starts_with("F2") => Color::Blue,
                    f if f.starts_with("F99") => Color::Gray,
                    _ => Color::White,
                };

                Row::new(vec![
                    Cell::from(filing.filing_id.clone()).style(Style::default().fg(Color::Cyan)),
                    Cell::from(filing.form_type.clone()).style(Style::default().fg(form_color)),
                    Cell::from(
                        filing
                            .report_type
                            .clone()
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                    Cell::from(coverage),
                    Cell::from(
                        filing
                            .receipt_date
                            .clone()
                            .unwrap_or_else(|| "-".to_string()),
                    )
                    .style(Style::default().fg(Color::DarkGray)),
                ])
            })
            .collect()
    };

    let table_title = if state.filings_loading {
        format!("Filings for {} (loading...)", candidate.candidate_id)
    } else if state.filing_detail_loading {
        format!(
            "Filings for {} - {} (loading filing detail...)",
            candidate.candidate_id, state.filings_cycle
        )
    } else if let Some(ref error) = state.filings_error {
        format!("Filings for {} (error: {})", candidate.candidate_id, error)
    } else {
        format!(
            "Filings for {} - {} ({} filings)",
            candidate.candidate_id,
            state.filings_cycle,
            state.filings.len()
        )
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(14), // Filing ID
            Constraint::Length(8),  // Form
            Constraint::Length(20), // Report
            Constraint::Min(20),    // Coverage
            Constraint::Length(12), // Received
        ],
    )
    .header(header_row)
    .block(Block::default().borders(Borders::ALL).title(table_title))
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, area, &mut state.filings_table_state);
}

fn render_help_text(f: &mut Frame, area: Rect, has_filings: bool, has_pcc: bool) {
    let mut bar = HelpBar::new().keys(vec!["Esc", "q"], " back");
    if has_pcc {
        bar = bar.item("c", " committee").item("a", " F1 affiliations");
    }
    if has_filings {
        bar = bar
            .item("j/k", " navigate")
            .item("Enter", " view filing")
            .item("y", " copy");
    } else {
        bar = bar.item("y", " copy").item("f", " filings");
    }
    bar.render(f, area);
}

pub fn render_candidate_detail(
    f: &mut Frame,
    area: Rect,
    candidate: &CandidateDetail,
    state: &mut CandidateDetailState,
) {
    let has_filings = !state.filings.is_empty() || state.filings_loading;
    let has_pcc = candidate.principal_campaign_committee.is_some();

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Title
            Constraint::Min(10),    // Content
            Constraint::Length(12), // Filings table
            Constraint::Length(2),  // Help text
        ]);

    let [title_area, content_area, filings_area, help_area] = area.layout(&layout);

    render_title(f, candidate, title_area);
    render_content(f, candidate, state, content_area);
    render_filings_table(f, candidate, state, filings_area);
    render_help_text(f, help_area, has_filings, has_pcc);

    if state.show_yank_popup {
        render_yank_popup(f, area, candidate, state);
    }
}

fn render_yank_popup(
    f: &mut Frame,
    area: Rect,
    candidate: &CandidateDetail,
    state: &CandidateDetailState,
) {
    let popup_area = super::popup_area(area, 50, 40);

    // Clear the background
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Copy to Clipboard")
        .border_style(Style::default().fg(Color::Green));

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Build the options list
    let options = state.get_yank_options(candidate);
    let mut lines = vec![];

    lines.push(Line::from(Span::styled(
        "Select what to copy:",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (idx, (_, label, value)) in options.iter().enumerate() {
        let is_selected = idx == state.yank_selected;

        let prefix = if is_selected { ">> " } else { "   " };
        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        if let Some(val) = value {
            let line_text = format!("{}{}: {}", prefix, label, val);
            lines.push(Line::from(Span::styled(line_text, style)));
        }
    }

    lines.push(Line::from(""));
    lines.push(navigation_popup_help_line());

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::bulk::candidates::CandidateDetail;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    fn create_test_candidate() -> CandidateDetail {
        CandidateDetail {
            candidate_id: "P00000001".to_string(),
            name: "SMITH, JOHN Q".to_string(),
            party_affiliation: "DEM".to_string(),
            election_year: 2024,
            state: "US".to_string(),
            office: "P".to_string(),
            district: "00".to_string(),
            incumbent_challenger_status: "C".to_string(),
            status: "C".to_string(),
            principal_campaign_committee: Some("C00123456".to_string()),
            address_street1: "123 Campaign Trail".to_string(),
            address_street2: "Suite 100".to_string(),
            address_city: "Washington".to_string(),
            address_state: "DC".to_string(),
            address_zip: "20001".to_string(),
        }
    }

    fn create_house_candidate() -> CandidateDetail {
        CandidateDetail {
            candidate_id: "H4CA12345".to_string(),
            name: "DOE, JANE A".to_string(),
            party_affiliation: "REP".to_string(),
            election_year: 2024,
            state: "CA".to_string(),
            office: "H".to_string(),
            district: "12".to_string(),
            incumbent_challenger_status: "I".to_string(),
            status: "C".to_string(),
            principal_campaign_committee: Some("C00654321".to_string()),
            address_street1: "456 Main Street".to_string(),
            address_street2: String::new(),
            address_city: "Los Angeles".to_string(),
            address_state: "CA".to_string(),
            address_zip: "90001".to_string(),
        }
    }

    #[test]
    fn test_candidate_detail_basic() {
        let candidate = create_test_candidate();
        let mut state = CandidateDetailState::new();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|f| render_candidate_detail(f, f.area(), &candidate, &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_candidate_detail_house() {
        let candidate = create_house_candidate();
        let mut state = CandidateDetailState::new();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|f| render_candidate_detail(f, f.area(), &candidate, &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_candidate_detail_with_yank_popup() {
        let candidate = create_test_candidate();
        let mut state = CandidateDetailState::new();
        state.show_yank_popup = true;
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|f| render_candidate_detail(f, f.area(), &candidate, &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_candidate_detail_wide_terminal() {
        let candidate = create_test_candidate();
        let mut state = CandidateDetailState::new();
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal
            .draw(|f| render_candidate_detail(f, f.area(), &candidate, &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
