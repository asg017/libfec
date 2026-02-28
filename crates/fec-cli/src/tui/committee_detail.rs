//! Committee Detail TUI Component
//!
//! This module provides rendering functions for displaying detailed committee information
//! within a ratatui application. It integrates with parent TUI apps (search, info) to
//! provide a seamless navigation experience.
//!
//! The detail view shows all available committee information including name, treasurer,
//! address, type, designation, party affiliation, and related data. When filings are loaded,
//! they appear in a smaller scrollable table below the committee info.
//!
//! Keyboard shortcuts:
//! - Esc/q: Return to previous view
//! - o: Open committee in browser
//! - y: Open copy popup to copy committee ID, candidate ID, or name to clipboard
//! - f: Fetch and display filings for this committee
//! - j/k: Navigate filings (when filings are loaded)
//! - Enter: View selected filing detail (when filings are loaded)
//!
//! Copy popup navigation:
//! - ↑/↓ or j/k: Navigate options
//! - Enter: Copy selected value to clipboard
//! - Esc: Cancel and close popup

use crate::cache::bulk::committee::CommitteeDetail;
use crate::cache::bulk::pac_summary::CommitteeFinancialSummary;
use crate::tui::filing_detail::format_usd;
use crate::tui::{navigation_popup_help_line, HelpBar};
use crossterm::event::{KeyCode, KeyEvent};
use fec_api::{Api, CandidateId, CommitteeId, FilingArgsBuilder};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
    Frame,
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

pub struct CommitteeDetailState {
    pub show_yank_popup: bool,
    pub yank_selected: usize,
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
    /// Financial summary from PAC summary bulk data
    pub financial_summary: Option<CommitteeFinancialSummary>,
}

impl CommitteeDetailState {
    pub fn new() -> Self {
        Self {
            show_yank_popup: false,
            yank_selected: 0,
            filings: Vec::new(),
            filings_table_state: TableState::default(),
            filings_loading: false,
            filings_error: None,
            filings_cycle: 2026,
            filing_detail_loading: false,
            financial_summary: None,
        }
    }

    /// Set filings after fetch completes
    pub fn set_filings(&mut self, mut filings: Vec<FilingListItem>) {
        filings.sort_by(|a, b| b.receipt_date.cmp(&a.receipt_date));
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

    /// Set financial summary
    pub fn set_financial_summary(&mut self, summary: Option<CommitteeFinancialSummary>) {
        self.financial_summary = summary;
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

    /// Fetch filings for a committee from the FEC API
    pub fn fetch_filings_for_committee(&mut self, committee_id: &str) {
        let api_key = std::env::var("LIBFEC_API_KEY").unwrap_or_else(|_| "DEMO_KEY".to_string());
        let client = Api::new(api_key.as_str());

        let args = FilingArgsBuilder::default()
            .committees(vec![CommitteeId::new(committee_id).unwrap()])
            .candidates(Vec::<CandidateId>::new())
            .form_types(None)
            .report_types(None)
            .committee_types(None)
            .cycle(vec![self.filings_cycle])
            .min_receipt_date(None)
            .max_receipt_date(None)
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

    pub fn get_yank_options(
        &self,
        committee: &CommitteeDetail,
    ) -> Vec<(YankOption, String, Option<String>)> {
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

        // Handle keys
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => CommitteeDetailAction::Exit,
            KeyCode::Char('o') => CommitteeDetailAction::OpenBrowser,
            KeyCode::Char('y') => {
                self.show_yank_popup = true;
                CommitteeDetailAction::None
            }
            KeyCode::Char('f') => {
                self.filings_loading = true;
                CommitteeDetailAction::FetchFilings
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
        let form_type = value
            .get("form_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let report_type = value
            .get("report_type_full")
            .or_else(|| value.get("report_type"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let coverage_from = value
            .get("coverage_start_date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let coverage_through = value
            .get("coverage_end_date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let receipt_date = value
            .get("receipt_date")
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

fn filing_frequency_label(code: &str) -> Option<&'static str> {
    match code.chars().next()? {
        'A' => Some("Administratively terminated"),
        'D' => Some("Debt"),
        'M' => Some("Monthly"),
        'Q' => Some("Quarterly"),
        'T' => Some("Terminated"),
        'W' => Some("Waived"),
        _ => None,
    }
}

fn designation_label(committee: &CommitteeDetail) -> Option<String> {
    match committee.designation.chars().next()? {
        'A' => Some("Authorized by a candidate".to_string()),
        'B' => Some("Lobbyist/Registrant PAC".to_string()),
        'D' => Some("Leadership PAC".to_string()),
        'J' => Some("Joint fundraiser".to_string()),
        'P' => {
            let base = "Principal campaign committee";
            match (&committee.candidate_name, &committee.candidate_id) {
                (Some(name), Some(id)) => Some(format!("{base} for {name} ({id})")),
                (None, Some(id)) => Some(format!("{base} for {id}")),
                _ => Some(base.to_string()),
            }
        }
        'U' => Some("Unauthorized".to_string()),
        _ => None,
    }
}

fn party_color(code: &str) -> Color {
    match code {
        "DEM" | "DFL" | "DNL" | "D/C" => Color::Blue,
        "REP" => Color::Red,
        "LIB" => Color::Yellow,
        "GRE" | "GR" | "IGR" | "PG" | "DCG" | "DGR" => Color::Green,
        _ => Color::Gray,
    }
}

fn render_title(f: &mut Frame, committee: &CommitteeDetail, area: Rect) {
    let mut title_spans = vec![Span::styled(
        format!("{} ({})", committee.name, committee.committee_id),
        Style::default().add_modifier(Modifier::BOLD),
    )];

    if !committee.party_affiliation.is_empty() {
        title_spans.push(Span::raw(" "));
        title_spans.push(Span::styled(
            format!("[{}]", committee.party_affiliation),
            Style::default()
                .fg(party_color(&committee.party_affiliation))
                .add_modifier(Modifier::BOLD),
        ));
    }

    let mut lines = vec![Line::from(title_spans)];

    if let Some(label) = designation_label(committee) {
        lines.push(Line::from(label));
    }

    if let Some(label) = filing_frequency_label(&committee.filing_frequency) {
        lines.push(Line::from(format!("Files {label}")));
    }

    let title = Paragraph::new(lines);
    f.render_widget(title, area);
}

fn render_content(f: &mut Frame, committee: &CommitteeDetail, area: Rect) {
    let mut lines = vec![];

    // Address
    if !committee.address_street1.is_empty() || !committee.address_city.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "Address:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));

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

    if !committee.connected_org_name.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(
                "Connected Organization: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&committee.connected_org_name),
        ]));
    }

    let content = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(content, area);
}

fn render_help_text(f: &mut Frame, area: Rect, has_filings: bool) {
    let mut bar = HelpBar::new().keys(vec!["Esc", "q"], " back");
    if has_filings {
        bar = bar
            .item("j/k", " navigate")
            .item("Enter", " view filing")
            .item("y", " copy")
            .item("o", " open");
    } else {
        bar = bar
            .item("o", " open")
            .item("y", " copy")
            .item("f", " filings");
    }
    bar.render(f, area);
}

fn render_filings_table(
    f: &mut Frame,
    committee: &CommitteeDetail,
    state: &mut CommitteeDetailState,
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
        format!("Filings for {} (loading...)", committee.committee_id)
    } else if state.filing_detail_loading {
        format!(
            "Filings for {} - {} (loading filing detail...)",
            committee.committee_id, state.filings_cycle
        )
    } else if let Some(ref error) = state.filings_error {
        format!("Filings for {} (error: {})", committee.committee_id, error)
    } else {
        format!(
            "Filings for {} - {} ({} filings)",
            committee.committee_id,
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

fn render_yank_popup(
    f: &mut Frame,
    area: Rect,
    committee: &CommitteeDetail,
    state: &CommitteeDetailState,
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
    let options = state.get_yank_options(committee);
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

fn render_financial_summary(f: &mut Frame, summary: &CommitteeFinancialSummary, area: Rect) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let green = Style::default().fg(Color::Green);
    let amount_width = 18;
    let content_width: u16 = 1 + 28 + amount_width as u16; // pad + label + amount

    let mut lines = vec![];

    // Primary lines (always shown)
    lines.push(Line::from(vec![
        Span::styled(format!(" {:<28}", "Total Receipts"), dim),
        Span::styled(
            format!(
                "{:>w$}",
                format_usd(summary.total_receipts),
                w = amount_width
            ),
            green,
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!(" {:<28}", "Total Disbursements"), dim),
        Span::styled(
            format!(
                "{:>w$}",
                format_usd(summary.total_disbursements),
                w = amount_width
            ),
            green,
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!(" {:<28}", "Cash on Hand"), dim),
        Span::styled(
            format!(
                "{:>w$}",
                format_usd(summary.cash_on_hand_close),
                w = amount_width
            ),
            green,
        ),
    ]));

    // Secondary lines (shown if non-zero)
    let secondary = [
        ("Individual Contributions", summary.individual_contributions),
        (
            "Committee Contributions",
            summary.other_committee_contributions,
        ),
        (
            "Contributions to Cmtes",
            summary.contributions_to_other_committees,
        ),
        ("Independent Expenditures", summary.independent_expenditures),
        (
            "Transfers From Affiliates",
            summary.transfers_from_affiliates,
        ),
        ("Transfers To Affiliates", summary.transfers_to_affiliates),
        ("Debts Owed", summary.debts_owed_by),
    ];
    for (label, value) in secondary {
        if value != 0.0 {
            lines.push(Line::from(vec![
                Span::styled(format!(" {:<28}", label), dim),
                Span::styled(format!("{:>w$}", format_usd(value), w = amount_width), dim),
            ]));
        }
    }

    let title_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let title = if summary.coverage_end_date.is_empty() {
        "Financial Summary".to_string()
    } else {
        // Convert MM/DD/YYYY to YYYY-MM-DD if possible
        let date_str = if let Some((m, rest)) = summary.coverage_end_date.split_once('/') {
            if let Some((d, y)) = rest.split_once('/') {
                format!("{}-{}-{}", y, m, d)
            } else {
                summary.coverage_end_date.clone()
            }
        } else {
            summary.coverage_end_date.clone()
        };
        format!("Financial Summary (thru {})", date_str)
    };

    // Box width: content + 2 for borders, but at least wide enough for the title + borders
    let title_width = title.len() as u16 + 2; // +2 for border chars around title
    let box_width = (content_width + 2).max(title_width).min(area.width);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, title_style))
        .border_style(Style::default().fg(Color::DarkGray));

    // Constrain the area to box_width
    let constrained_area = Rect {
        x: area.x,
        y: area.y,
        width: box_width,
        height: area.height,
    };

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, constrained_area);
}

/// Compute how many rows the financial summary box needs (including border)
fn financial_summary_height(summary: &CommitteeFinancialSummary) -> u16 {
    let mut rows: u16 = 3; // 3 primary lines
    let secondary = [
        summary.individual_contributions,
        summary.other_committee_contributions,
        summary.contributions_to_other_committees,
        summary.independent_expenditures,
        summary.transfers_from_affiliates,
        summary.transfers_to_affiliates,
        summary.debts_owed_by,
    ];
    for v in secondary {
        if v != 0.0 {
            rows += 1;
        }
    }
    rows + 2 // +2 for top/bottom border
}

pub fn render_committee_detail(
    f: &mut Frame,
    area: Rect,
    committee: &CommitteeDetail,
    state: &mut CommitteeDetailState,
) {
    let has_filings = !state.filings.is_empty() || state.filings_loading;

    let fin_height = state
        .financial_summary
        .as_ref()
        .map(financial_summary_height)
        .unwrap_or(0);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),          // Title
            Constraint::Length(fin_height), // Financial summary box
            Constraint::Min(10),            // Content
            Constraint::Length(12),         // Filings table (smaller, scrollable)
            Constraint::Length(2),          // Help text
        ]);

    let [title_area, fin_area, content_area, filings_area, help_area] = area.layout(&layout);

    render_title(f, committee, title_area);
    if let Some(ref summary) = state.financial_summary {
        render_financial_summary(f, summary, fin_area);
    }
    render_content(f, committee, content_area);
    render_filings_table(f, committee, state, filings_area);
    render_help_text(f, help_area, has_filings);

    if state.show_yank_popup {
        render_yank_popup(f, area, committee, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    fn create_test_committee() -> CommitteeDetail {
        CommitteeDetail {
            committee_id: "C00401224".to_string(),
            name: "FRIENDS OF DEMOCRACY PAC".to_string(),
            treasurer_name: "SMITH, JOHN Q".to_string(),
            address_street1: "123 K Street NW".to_string(),
            address_street2: "Suite 400".to_string(),
            address_city: "Washington".to_string(),
            address_state: "DC".to_string(),
            address_zip: "20001".to_string(),
            designation: "B - Lobbyist/Registrant PAC".to_string(),
            committee_type: "Q - Qualified Non-Party (e.g. PAC)".to_string(),
            party_affiliation: String::new(),
            filing_frequency: "Q - Quarterly".to_string(),
            interest_group_category: String::new(),
            connected_org_name: "DEMOCRACY CORP".to_string(),
            candidate_id: None,
            candidate_name: None,
        }
    }

    fn create_test_committee_with_candidate() -> CommitteeDetail {
        CommitteeDetail {
            committee_id: "C00703975".to_string(),
            name: "DOE FOR CONGRESS".to_string(),
            treasurer_name: "DOE, JANE A".to_string(),
            address_street1: "456 Main Street".to_string(),
            address_street2: String::new(),
            address_city: "Los Angeles".to_string(),
            address_state: "CA".to_string(),
            address_zip: "90001".to_string(),
            designation: "P - Principal Campaign Committee".to_string(),
            committee_type: "H - House".to_string(),
            party_affiliation: "DEM".to_string(),
            filing_frequency: "Q - Quarterly".to_string(),
            interest_group_category: String::new(),
            connected_org_name: String::new(),
            candidate_id: Some("H4CA12345".to_string()),
            candidate_name: Some("DOE, JANE".to_string()),
        }
    }

    fn create_test_filings() -> Vec<FilingListItem> {
        vec![
            FilingListItem {
                filing_id: "FEC-1234567".to_string(),
                form_type: "F3XN".to_string(),
                report_type: Some("YEAR-END".to_string()),
                coverage_from: Some("2025-07-01".to_string()),
                coverage_through: Some("2025-12-31".to_string()),
                receipt_date: Some("2026-01-31".to_string()),
            },
            FilingListItem {
                filing_id: "FEC-1234566".to_string(),
                form_type: "F3XN".to_string(),
                report_type: Some("MID-YEAR".to_string()),
                coverage_from: Some("2025-01-01".to_string()),
                coverage_through: Some("2025-06-30".to_string()),
                receipt_date: Some("2025-07-31".to_string()),
            },
            FilingListItem {
                filing_id: "FEC-1234500".to_string(),
                form_type: "F99".to_string(),
                report_type: None,
                coverage_from: None,
                coverage_through: None,
                receipt_date: Some("2025-03-15".to_string()),
            },
        ]
    }

    #[test]
    fn test_committee_detail_basic() {
        let committee = create_test_committee();
        let mut state = CommitteeDetailState::new();
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
        terminal
            .draw(|f| render_committee_detail(f, f.area(), &committee, &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_committee_detail_with_candidate() {
        let committee = create_test_committee_with_candidate();
        let mut state = CommitteeDetailState::new();
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
        terminal
            .draw(|f| render_committee_detail(f, f.area(), &committee, &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_committee_detail_with_filings() {
        let committee = create_test_committee();
        let mut state = CommitteeDetailState::new();
        state.set_filings(create_test_filings());
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
        terminal
            .draw(|f| render_committee_detail(f, f.area(), &committee, &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_committee_detail_filings_loading() {
        let committee = create_test_committee();
        let mut state = CommitteeDetailState::new();
        state.filings_loading = true;
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
        terminal
            .draw(|f| render_committee_detail(f, f.area(), &committee, &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_committee_detail_filings_error() {
        let committee = create_test_committee();
        let mut state = CommitteeDetailState::new();
        state.set_filings_error("Connection timeout".to_string());
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
        terminal
            .draw(|f| render_committee_detail(f, f.area(), &committee, &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_committee_detail_with_yank_popup() {
        let committee = create_test_committee();
        let mut state = CommitteeDetailState::new();
        state.show_yank_popup = true;
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
        terminal
            .draw(|f| render_committee_detail(f, f.area(), &committee, &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_committee_detail_with_yank_popup_candidate() {
        let committee = create_test_committee_with_candidate();
        let mut state = CommitteeDetailState::new();
        state.show_yank_popup = true;
        state.yank_selected = 1; // Candidate ID selected
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
        terminal
            .draw(|f| render_committee_detail(f, f.area(), &committee, &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_committee_detail_wide_terminal() {
        let committee = create_test_committee();
        let mut state = CommitteeDetailState::new();
        state.set_filings(create_test_filings());
        let mut terminal = Terminal::new(TestBackend::new(120, 35)).unwrap();
        terminal
            .draw(|f| render_committee_detail(f, f.area(), &committee, &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    fn create_test_financial_summary() -> CommitteeFinancialSummary {
        CommitteeFinancialSummary {
            total_receipts: 2_500_000.00,
            total_disbursements: 1_800_000.50,
            cash_on_hand_close: 700_000.25,
            individual_contributions: 1_200_000.00,
            other_committee_contributions: 500_000.00,
            contributions_to_other_committees: 150_000.00,
            independent_expenditures: 300_000.00,
            transfers_from_affiliates: 100_000.00,
            transfers_to_affiliates: 50_000.00,
            debts_owed_by: 25_000.00,
            coverage_end_date: "12/31/2025".to_string(),
        }
    }

    #[test]
    fn test_committee_detail_with_financial_summary() {
        let committee = create_test_committee();
        let mut state = CommitteeDetailState::new();
        state.set_financial_summary(Some(create_test_financial_summary()));
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
        terminal
            .draw(|f| render_committee_detail(f, f.area(), &committee, &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_committee_detail_financial_summary_minimal() {
        let committee = create_test_committee();
        let mut state = CommitteeDetailState::new();
        state.set_financial_summary(Some(CommitteeFinancialSummary {
            total_receipts: 50_000.00,
            total_disbursements: 30_000.00,
            cash_on_hand_close: 20_000.00,
            individual_contributions: 0.0,
            other_committee_contributions: 0.0,
            contributions_to_other_committees: 0.0,
            independent_expenditures: 0.0,
            transfers_from_affiliates: 0.0,
            transfers_to_affiliates: 0.0,
            debts_owed_by: 0.0,
            coverage_end_date: String::new(),
        }));
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
        terminal
            .draw(|f| render_committee_detail(f, f.area(), &committee, &mut state))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
