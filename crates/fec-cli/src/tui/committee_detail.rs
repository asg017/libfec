/*!
 * Committee Detail TUI Component
 *
 * This module provides rendering functions for displaying detailed committee information
 * within a ratatui application. It integrates with parent TUI apps (search, info) to
 * provide a seamless navigation experience.
 *
 * The detail view shows all available committee information including name, treasurer,
 * address, type, designation, party affiliation, and related data.
 *
 * Keyboard shortcuts (handled by parent app):
 * - y: Open copy popup to copy committee ID, candidate ID, or name to clipboard
 * - Esc: Return to previous view
 *
 * Copy popup navigation:
 * - ↑/↓ or j/k: Navigate options
 * - Enter: Copy selected value to clipboard
 * - Esc: Cancel and close popup
 */

use crate::cache::bulk_committee::CommitteeDetail;
use crossterm::event::{KeyCode, KeyEvent};
use fec_api::{Api, FilingArgsBuilder};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
};

/// A filing item for display in the filings list
#[derive(Debug, Clone)]
pub struct FilingListItem {
    pub filing_id: String,
    pub form_type: String,
    pub report_type: Option<String>,
    pub coverage_from: Option<String>,
    pub coverage_through: Option<String>,
    pub receipt_date: Option<String>,
}

/// Action returned by handle_key_event indicating what the parent should do
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitteeDetailAction {
    /// Key was handled internally, no action needed from parent
    None,
    /// User wants to exit/go back
    Exit,
    /// User pressed 'o' to open in browser
    OpenBrowser,
    /// User pressed Enter on a filing - parent should show filing detail
    ShowFilingDetail { filing_id: String },
    /// User pressed 'f' to fetch filings - parent should call fetch_filings_for_committee
    FetchFilings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YankOption {
    CommitteeId,
    CandidateId,
    CommitteeName,
}

/// The current view mode within committee detail
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommitteeDetailViewMode {
    #[default]
    Info,
    Filings,
}

pub struct CommitteeDetailState {
    pub show_yank_popup: bool,
    pub yank_selected: usize,
    /// Current view mode
    pub view_mode: CommitteeDetailViewMode,
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
}

impl CommitteeDetailState {
    pub fn new() -> Self {
        Self {
            show_yank_popup: false,
            yank_selected: 0,
            view_mode: CommitteeDetailViewMode::Info,
            filings: Vec::new(),
            filings_table_state: TableState::default(),
            filings_loading: false,
            filings_error: None,
            filings_cycle: 2026,
            filing_detail_loading: false,
        }
    }

    /// Set filings after fetch completes
    pub fn set_filings(&mut self, filings: Vec<FilingListItem>) {
        self.filings = filings;
        self.filings_loading = false;
        self.filings_error = None;
        // View mode is already set to Filings when 'f' is pressed
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
            Some(i) => if i >= count - 1 { 0 } else { i + 1 },
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
            Some(i) => if i == 0 { count - 1 } else { i - 1 },
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
            self.filings_table_state.select(Some(self.filings.len() - 1));
        }
    }

    fn get_selected_filing(&self) -> Option<&FilingListItem> {
        self.filings_table_state.selected().and_then(|i| self.filings.get(i))
    }

    /// Fetch filings for a committee from the FEC API
    pub fn fetch_filings_for_committee(&mut self, committee_id: &str) {
        let api_key = std::env::var("LIBFEC_API_KEY").unwrap_or_else(|_| "DEMO_KEY".to_string());
        let client = Api::new(api_key.as_str());

        let args = FilingArgsBuilder::default()
            .committees(vec![committee_id.to_string()])
            .candidates(Vec::<String>::new())
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
                            .filter_map(|v| FilingListItem::from_api_value(v))
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

    pub fn get_yank_options(&self, committee: &CommitteeDetail) -> Vec<(YankOption, String, Option<String>)> {
        let mut options = vec![];

        options.push((
            YankOption::CommitteeId,
            "Committee ID".to_string(),
            Some(committee.committee_id.clone()),
        ));

        if let Some(ref candidate_id) = committee.candidate_id {
            options.push((
                YankOption::CandidateId,
                "Candidate ID".to_string(),
                Some(candidate_id.clone()),
            ));
        }

        options.push((
            YankOption::CommitteeName,
            "Committee Name".to_string(),
            Some(committee.name.clone()),
        ));

        options
    }

    pub fn copy_selected(&self, committee: &CommitteeDetail) {
        let options = self.get_yank_options(committee);
        if let Some((_, _, Some(value))) = options.get(self.yank_selected) {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(value);
            }
        }
    }

    pub fn yank_next(&mut self, committee: &CommitteeDetail) {
        let options_count = self.get_yank_options(committee).len();
        if self.yank_selected < options_count.saturating_sub(1) {
            self.yank_selected += 1;
        } else {
            self.yank_selected = 0;
        }
    }

    pub fn yank_previous(&mut self, committee: &CommitteeDetail) {
        let options_count = self.get_yank_options(committee).len();
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
        committee: &CommitteeDetail,
    ) -> CommitteeDetailAction {
        // Handle yank popup first
        if self.show_yank_popup {
            return match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('y') => {
                    self.show_yank_popup = false;
                    CommitteeDetailAction::None
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.yank_next(committee);
                    CommitteeDetailAction::None
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.yank_previous(committee);
                    CommitteeDetailAction::None
                }
                KeyCode::Enter => {
                    self.copy_selected(committee);
                    self.show_yank_popup = false;
                    CommitteeDetailAction::None
                }
                _ => CommitteeDetailAction::None,
            };
        }

        // Handle filings view
        if self.view_mode == CommitteeDetailViewMode::Filings {
            return match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.view_mode = CommitteeDetailViewMode::Info;
                    CommitteeDetailAction::None
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.filings_select_next();
                    CommitteeDetailAction::None
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.filings_select_previous();
                    CommitteeDetailAction::None
                }
                KeyCode::Char('g') => {
                    self.filings_select_first();
                    CommitteeDetailAction::None
                }
                KeyCode::Char('G') => {
                    self.filings_select_last();
                    CommitteeDetailAction::None
                }
                KeyCode::Enter => {
                    if let Some(filing_id) = self.get_selected_filing().map(|f| f.filing_id.clone()) {
                        self.filing_detail_loading = true;
                        self.filings_error = None; // Clear any previous error
                        CommitteeDetailAction::ShowFilingDetail { filing_id }
                    } else {
                        CommitteeDetailAction::None
                    }
                }
                KeyCode::Char('y') => {
                    self.show_yank_popup = true;
                    CommitteeDetailAction::None
                }
                _ => CommitteeDetailAction::None,
            };
        }

        // Handle info view (default)
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => CommitteeDetailAction::Exit,
            KeyCode::Char('o') => CommitteeDetailAction::OpenBrowser,
            KeyCode::Char('y') => {
                self.show_yank_popup = true;
                CommitteeDetailAction::None
            }
            KeyCode::Char('f') => {
                self.filings_loading = true;
                self.view_mode = CommitteeDetailViewMode::Filings;
                CommitteeDetailAction::FetchFilings
            }
            _ => CommitteeDetailAction::None,
        }
    }
}

impl Default for CommitteeDetailState {
    fn default() -> Self {
        Self::new()
    }
}

impl FilingListItem {
    /// Create a FilingListItem from an API response JSON value
    pub fn from_api_value(value: &serde_json::Value) -> Option<Self> {
        let filing_id = value.get("fec_file_id")?.as_str()?.to_string();
        let form_type = value.get("form_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let report_type = value.get("report_type_full")
            .or_else(|| value.get("report_type"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let coverage_from = value.get("coverage_start_date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let coverage_through = value.get("coverage_end_date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let receipt_date = value.get("receipt_date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Some(FilingListItem {
            filing_id,
            form_type,
            report_type,
            coverage_from,
            coverage_through,
            receipt_date,
        })
    }
}


fn render_title(f: &mut Frame, committee: &CommitteeDetail, area: Rect) {
    let title_text = format!("{} ({})", committee.name, committee.committee_id);
    let title = Paragraph::new(title_text)
        .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(title, area);
}

fn render_content(f: &mut Frame, committee: &CommitteeDetail, area: Rect) {
    let mut lines = vec![];

    // Basic info
    lines.push(Line::from(vec![
        Span::styled("Committee ID: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(&committee.committee_id),
    ]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("Name: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(&committee.name),
    ]));
    lines.push(Line::from(""));

    if !committee.treasurer_name.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Treasurer: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(&committee.treasurer_name),
        ]));
        lines.push(Line::from(""));
    }

    // Address
    if !committee.address_street1.is_empty() || !committee.address_city.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Address:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));

        if !committee.address_street1.is_empty() {
            lines.push(Line::from(format!("  {}", committee.address_street1)));
        }
        if !committee.address_street2.is_empty() {
            lines.push(Line::from(format!("  {}", committee.address_street2)));
        }

        let mut city_line = String::from("  ");
        if !committee.address_city.is_empty() {
            city_line.push_str(&committee.address_city);
        }
        if !committee.address_state.is_empty() {
            if !committee.address_city.is_empty() {
                city_line.push_str(", ");
            }
            city_line.push_str(&committee.address_state);
        }
        if !committee.address_zip.is_empty() {
            city_line.push(' ');
            city_line.push_str(&committee.address_zip);
        }
        if city_line.len() > 2 {
            lines.push(Line::from(city_line));
        }
        lines.push(Line::from(""));
    }

    // Committee details
    if !committee.committee_type.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Committee Type: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(&committee.committee_type),
        ]));
    }

    if !committee.designation.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Designation: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(&committee.designation),
        ]));
    }

    if !committee.party_affiliation.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Party Affiliation: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(&committee.party_affiliation),
        ]));
    }

    if !committee.filing_frequency.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Filing Frequency: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(&committee.filing_frequency),
        ]));
    }

    if !committee.interest_group_category.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Interest Group Category: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(&committee.interest_group_category),
        ]));
    }

    if !committee.connected_org_name.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Connected Organization: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(&committee.connected_org_name),
        ]));
    }

    if let Some(ref candidate_id) = committee.candidate_id {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Candidate ID: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(candidate_id, Style::default().fg(Color::Cyan)),
        ]));
    }

    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false });
    f.render_widget(content, area);
}

fn render_help_text(f: &mut Frame, area: Rect) {
    let help_line = Line::from(vec![
        Span::styled("Esc", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("/", Style::default().fg(Color::DarkGray)),
        Span::styled("q", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" back  ", Style::default().fg(Color::DarkGray)),
        Span::styled("o", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" open  ", Style::default().fg(Color::DarkGray)),
        Span::styled("y", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" copy  ", Style::default().fg(Color::DarkGray)),
        Span::styled("f", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" filings", Style::default().fg(Color::DarkGray)),
    ]);
    let help = Paragraph::new(help_line)
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::TOP).border_style(
            Style::default().fg(Color::DarkGray)
        ));
    f.render_widget(help, area);
}

fn render_filings_help_text(f: &mut Frame, area: Rect) {
    let help_line = Line::from(vec![
        Span::styled("Esc", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("/", Style::default().fg(Color::DarkGray)),
        Span::styled("q", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" back  ", Style::default().fg(Color::DarkGray)),
        Span::styled("j/k", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" navigate  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" view filing  ", Style::default().fg(Color::DarkGray)),
        Span::styled("y", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" copy", Style::default().fg(Color::DarkGray)),
    ]);
    let help = Paragraph::new(help_line)
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::TOP).border_style(
            Style::default().fg(Color::DarkGray)
        ));
    f.render_widget(help, area);
}

fn render_filings_table(f: &mut Frame, committee: &CommitteeDetail, state: &mut CommitteeDetailState, area: Rect) {
    let header_row = Row::new(vec![
        Cell::from("Filing ID").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Form").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Report").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Coverage").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Received").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ])
    .height(1);

    let rows: Vec<Row> = state
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
                Cell::from(format!("{}", filing.filing_id)).style(Style::default().fg(Color::Cyan)),
                Cell::from(filing.form_type.clone()).style(Style::default().fg(form_color)),
                Cell::from(filing.report_type.clone().unwrap_or_else(|| "-".to_string())),
                Cell::from(coverage),
                Cell::from(filing.receipt_date.clone().unwrap_or_else(|| "-".to_string())).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let table_title = if state.filings_loading {
        format!("Filings for {} (loading...)", committee.committee_id)
    } else if state.filing_detail_loading {
        format!("Filings for {} - {} (loading filing detail...)", committee.committee_id, state.filings_cycle)
    } else if let Some(ref error) = state.filings_error {
        format!("Filings for {} (error: {})", committee.committee_id, error)
    } else {
        format!("Filings for {} - {} ({} filings)", committee.committee_id, state.filings_cycle, state.filings.len())
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

/// Helper function to create a centered rect using certain percentage of available rect
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

fn render_yank_popup(f: &mut Frame, area: Rect, committee: &CommitteeDetail, state: &CommitteeDetailState) {
    let popup_area = popup_area(area, 50, 40);

    // Clear the background
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Copy to Clipboard")
        .border_style(Style::default().fg(Color::Green));

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Build the options list
    let options = state.get_yank_options(committee);
    let mut lines = vec![];

    lines.push(Line::from(Span::styled(
        "Select what to copy:",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
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
    lines.push(Line::from(vec![
        Span::styled("↑/↓", Style::default().fg(Color::DarkGray)),
        Span::styled(" or ", Style::default().fg(Color::DarkGray)),
        Span::styled("j/k", Style::default().fg(Color::DarkGray)),
        Span::styled(" navigate  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" copy  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner_area);
}


pub fn render_committee_detail(
    f: &mut Frame,
    area: Rect,
    committee: &CommitteeDetail,
    state: &mut CommitteeDetailState,
) {
    match state.view_mode {
        CommitteeDetailViewMode::Info => {
            render_info_view(f, area, committee, state);
        }
        CommitteeDetailViewMode::Filings => {
            render_filings_view(f, area, committee, state);
        }
    }
}

fn render_info_view(
    f: &mut Frame,
    area: Rect,
    committee: &CommitteeDetail,
    state: &CommitteeDetailState,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Title
            Constraint::Min(10),    // Content
            Constraint::Length(2),  // Help text
        ]);

    let [title_area, content_area, help_area] = area.layout(&layout);

    render_title(f, committee, title_area);
    render_content(f, committee, content_area);
    render_help_text(f, help_area);

    if state.show_yank_popup {
        render_yank_popup(f, area, committee, state);
    }
}

fn render_filings_view(
    f: &mut Frame,
    area: Rect,
    committee: &CommitteeDetail,
    state: &mut CommitteeDetailState,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Title
            Constraint::Min(10),    // Filings table
            Constraint::Length(2),  // Help text
        ]);

    let [title_area, table_area, help_area] = area.layout(&layout);

    render_title(f, committee, title_area);
    render_filings_table(f, committee, state, table_area);
    render_filings_help_text(f, help_area);

    if state.show_yank_popup {
        render_yank_popup(f, area, committee, state);
    }
}
