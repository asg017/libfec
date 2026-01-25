//! Filing Detail TUI Component
//!
//! This module provides rendering functions for displaying detailed FEC filing information
//! within a ratatui application. It integrates with parent TUI apps (info) to
//! provide a seamless navigation experience.
//!
//! The detail view shows all available filing information including form type, filer,
//! coverage period, summary data, and metadata.
//!
//! Keyboard shortcuts (handled by parent app):
//! - y: Open copy popup to copy filing ID, filer ID, or filer name to clipboard
//! - Esc: Return to previous view
//!
//! Copy popup navigation:
//! - up/down or j/k: Navigate options
//! - Enter: Copy selected value to clipboard
//! - Esc: Cancel and close popup

use crossterm::event::{KeyCode, KeyEvent};
use fec_parser::{covers::Cover, report_code_label};
use indicatif::HumanBytes;
use num_format::{Locale, ToFormattedString};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Action returned by handle_key_event indicating what the parent should do
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilingDetailAction {
    /// Key was handled internally, no action needed from parent
    None,
    /// User wants to exit/go back
    Exit,
    /// User pressed 'o' to open in browser
    OpenBrowser,
}

/// Holds extracted filing information for TUI display
pub struct FilingDetail {
    pub filing_id: String,
    pub report_code: Option<String>,
    pub filer_name: String,
    pub filer_id: String,
    pub coverage_from: Option<String>,
    pub coverage_through: Option<String>,
    pub fec_version: String,
    pub source_length: usize,
    pub software_name: String,
    pub software_version: String,
    pub report_id: Option<String>,
    pub report_number: Option<String>,
    pub comment: Option<String>,
    pub treasurer: Option<String>,
    pub signed_date: Option<String>,
    pub summary: Option<FilingSummary>,
}

/// Summary data for cover forms
pub struct FilingSummary {
    pub cash_on_hand_beginning: f64,
    pub total_receipts: f64,
    pub total_disbursements: f64,
    pub cash_on_hand_end: f64,
}

impl FilingDetail {
    pub fn fec_url(&self) -> String {
        format!(
            "https://docquery.fec.gov/cgi-bin/forms/{}/{}",
            self.filer_id, self.filing_id
        )
    }

    pub fn open_in_browser(&self) -> Result<(), std::io::Error> {
        open::that(self.fec_url())
    }
}

impl<R: std::io::Read> From<&fec_parser::Filing<R>> for FilingDetail {
    fn from(filing: &fec_parser::Filing<R>) -> Self {
        let (treasurer, signed_date, summary) = if let Some(ref cover) = filing.cover.cover_data {
            match cover {
                Cover::Form1(form) => (
                    Some(form.treasurer.to_string()),
                    form.date_signed.map(|d| d.to_string()),
                    None, // F1 is a registration form, no financial summary
                ),
                Cover::Form3(form) => (
                    Some(form.treasurer.to_string()),
                    Some(form.signed.to_string()),
                    Some(FilingSummary {
                        cash_on_hand_beginning: form.detailed_summary.cash_on_hand_beginning,
                        total_receipts: form.detailed_summary.total_receipts_period,
                        total_disbursements: form.detailed_summary.total_disbursements_period,
                        cash_on_hand_end: form.summary.line12_cash_on_hand_close_of_period,
                    }),
                ),
                Cover::Form3P(form) => (
                    Some(form.treasurer.to_string()),
                    Some(form.signed.to_string()),
                    Some(FilingSummary {
                        cash_on_hand_beginning: form.summary.line6_cash_on_hand_beginning_period,
                        total_receipts: form.summary.line7_total_receipts,
                        total_disbursements: form.summary.line9_total_disbursements,
                        cash_on_hand_end: form.summary.line10_cash_on_hand_end_period,
                    }),
                ),
            }
        } else {
            (None, None, None)
        };

        FilingDetail {
            filing_id: filing.filing_id.clone(),
            report_code: filing.cover.report_code.clone(),
            filer_name: filing.cover.filer_name.clone(),
            filer_id: filing.cover.filer_id.clone(),
            coverage_from: filing.cover.coverage_from_date.map(|d| d.to_string()),
            coverage_through: filing.cover.coverage_through_date.map(|d| d.to_string()),
            fec_version: filing.header.fec_version.clone(),
            source_length: filing.source_length,
            software_name: filing.header.software_name.clone(),
            software_version: filing.header.software_version.clone(),
            report_id: filing.header.report_id.clone(),
            report_number: filing.header.report_number.clone(),
            comment: filing.header.comment.clone(),
            treasurer,
            signed_date,
            summary,
        }
    }
}

fn format_usd(amount: f64) -> String {
    let rounded = (amount * 100.0).round() as i64;
    let dollars = rounded / 100;
    let cents = (rounded % 100).abs();
    format!("${}.{:02}", dollars.to_formatted_string(&Locale::en), cents)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YankOption {
    FilingId,
    FilerId,
    FilerName,
}

pub struct FilingDetailState {
    pub show_yank_popup: bool,
    pub yank_selected: usize,
    pub scroll_offset: u16,
}

impl FilingDetailState {
    pub fn new() -> Self {
        Self {
            show_yank_popup: false,
            yank_selected: 0,
            scroll_offset: 0,
        }
    }

    pub fn get_yank_options(&self, filing: &FilingDetail) -> Vec<(YankOption, String, Option<String>)> {
        vec![
            (
                YankOption::FilingId,
                "Filing ID".to_string(),
                Some(format!("FEC-{}", filing.filing_id)),
            ),
            (
                YankOption::FilerId,
                "Filer ID".to_string(),
                Some(filing.filer_id.clone()),
            ),
            (
                YankOption::FilerName,
                "Filer Name".to_string(),
                Some(filing.filer_name.clone()),
            ),
        ]
    }

    pub fn copy_selected(&self, filing: &FilingDetail) {
        let options = self.get_yank_options(filing);
        if let Some((_, _, Some(value))) = options.get(self.yank_selected) {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(value);
            }
        }
    }

    pub fn yank_next(&mut self, filing: &FilingDetail) {
        let options_count = self.get_yank_options(filing).len();
        if self.yank_selected < options_count.saturating_sub(1) {
            self.yank_selected += 1;
        } else {
            self.yank_selected = 0;
        }
    }

    pub fn yank_previous(&mut self, filing: &FilingDetail) {
        let options_count = self.get_yank_options(filing).len();
        if self.yank_selected > 0 {
            self.yank_selected -= 1;
        } else {
            self.yank_selected = options_count.saturating_sub(1);
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Handle a key event and return an action for the parent to perform
    pub fn handle_key_event(
        &mut self,
        key: KeyEvent,
        filing: &FilingDetail,
    ) -> FilingDetailAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                if self.show_yank_popup {
                    self.show_yank_popup = false;
                    FilingDetailAction::None
                } else {
                    FilingDetailAction::Exit
                }
            }
            KeyCode::Char('o') => FilingDetailAction::OpenBrowser,
            KeyCode::Char('y') => {
                if !self.show_yank_popup {
                    self.show_yank_popup = true;
                }
                FilingDetailAction::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.show_yank_popup {
                    self.yank_next(filing);
                } else {
                    self.scroll_down();
                }
                FilingDetailAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.show_yank_popup {
                    self.yank_previous(filing);
                } else {
                    self.scroll_up();
                }
                FilingDetailAction::None
            }
            KeyCode::Enter => {
                if self.show_yank_popup {
                    self.copy_selected(filing);
                    self.show_yank_popup = false;
                }
                FilingDetailAction::None
            }
            _ => FilingDetailAction::None,
        }
    }
}

impl Default for FilingDetailState {
    fn default() -> Self {
        Self::new()
    }
}

fn render_title(f: &mut Frame, filing: &FilingDetail, area: Rect) {
    let report_label = filing
        .report_code
        .as_ref()
        .map(|rc| report_code_label(rc.as_str()))
        .unwrap_or("");
    let title_text = format!(
        "FEC-{} {} by {} ({})",
        filing.filing_id, report_label, filing.filer_name, filing.filer_id
    );
    let title = Paragraph::new(title_text)
        .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(title, area);
}

fn render_content(f: &mut Frame, filing: &FilingDetail, state: &FilingDetailState, area: Rect) {
    let mut lines = vec![];

    // Coverage period
    if let (Some(ref from), Some(ref through)) = (&filing.coverage_from, &filing.coverage_through) {
        lines.push(Line::from(vec![
            Span::styled(from, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" through "),
            Span::styled(through, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
    }

    // Treasurer and signed date (for F3P forms)
    if let (Some(ref treasurer), Some(ref signed)) = (&filing.treasurer, &filing.signed_date) {
        lines.push(Line::from(vec![
            Span::styled("Signed by: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(treasurer),
            Span::raw(" on "),
            Span::raw(signed),
        ]));
        lines.push(Line::from(""));
    }

    // Summary section (for F3P forms) - simple table format
    if let Some(ref summary) = filing.summary {
        // Calculate percentage change
        let pct_change = if summary.cash_on_hand_beginning == 0.0 {
            100.0
        } else {
            ((summary.cash_on_hand_end - summary.cash_on_hand_beginning) / summary.cash_on_hand_beginning) * 100.0
        };
        let amount_change = summary.cash_on_hand_end - summary.cash_on_hand_beginning;
        let pct_color = if pct_change >= 0.0 { Color::Green } else { Color::Red };
        let pct_sign = if pct_change >= 0.0 { "+" } else { "" };

        // Cash on Hand - Start
        lines.push(Line::from(vec![
            Span::styled(format!("{:<24}", "Cash on Hand - Start"), Style::default().fg(Color::White)),
            Span::styled(format!("{:>16}", format_usd(summary.cash_on_hand_beginning)), Style::default().fg(Color::White)),
        ]));

        // Receipts
        lines.push(Line::from(vec![
            Span::styled(format!("{:<24}", "Receipts"), Style::default().fg(Color::White)),
            Span::styled(format!("+{:>15}", format_usd(summary.total_receipts)), Style::default().fg(Color::Blue)),
        ]));

        // Expenditures
        lines.push(Line::from(vec![
            Span::styled(format!("{:<24}", "Expenditures"), Style::default().fg(Color::White)),
            Span::styled(format!("-{:>15}", format_usd(summary.total_disbursements)), Style::default().fg(Color::Red)),
        ]));

        // Cash on Hand - End (with percentage change)
        lines.push(Line::from(vec![
            Span::styled(format!("{:<24}", "Cash on Hand - End"), Style::default().fg(Color::White)),
            Span::styled(format!("{:>16}", format_usd(summary.cash_on_hand_end)), Style::default().fg(Color::White).bold()),
            Span::styled(format!(" {}{}, {}{:.0}%", pct_sign, format_usd(amount_change), pct_sign, pct_change), Style::default().fg(pct_color).add_modifier(Modifier::DIM)),
        ]));
        lines.push(Line::from(""));
    }

    // FEC URL
    let fec_url = format!(
        "https://docquery.fec.gov/cgi-bin/forms/{}/{}",
        filing.filer_id, filing.filing_id
    );
    lines.push(Line::from(vec![
        Span::styled("URL: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(&fec_url, Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED)),
    ]));
    lines.push(Line::from(""));

    // Metadata line
    lines.push(Line::from(vec![
        Span::styled("Version: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(format!("v{}", filing.fec_version)),
        Span::raw("  "),
        Span::styled("Size: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(HumanBytes(filing.source_length as u64).to_string()),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Software: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(format!("{} {}", filing.software_name, filing.software_version)),
    ]));

    // Optional metadata
    if let Some(ref report_id) = filing.report_id {
        lines.push(Line::from(vec![
            Span::styled("Report ID: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(format!("'{}' ({})", report_id, filing.report_number.as_deref().unwrap_or(""))),
        ]));
    }

    if let Some(ref comment) = filing.comment {
        lines.push(Line::from(vec![
            Span::styled("Comment: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(format!("'{}'", comment)),
        ]));
    }

    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((state.scroll_offset, 0));
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
        Span::styled("j/k", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" scroll", Style::default().fg(Color::DarkGray)),
    ]);
    let help = Paragraph::new(help_line)
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::TOP).border_style(
            Style::default().fg(Color::DarkGray)
        ));
    f.render_widget(help, area);
}

pub fn render_filing_detail(
    f: &mut Frame,
    area: Rect,
    filing: &FilingDetail,
    state: &FilingDetailState,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Title
            Constraint::Min(10),    // Content
            Constraint::Length(2),  // Help text
        ]);

    let [title_area, content_area, help_area] = area.layout(&layout);

    render_title(f, filing, title_area);
    render_content(f, filing, state, content_area);
    render_help_text(f, help_area);

    if state.show_yank_popup {
        render_yank_popup(f, area, filing, state);
    }
}

fn render_yank_popup(f: &mut Frame, area: Rect, filing: &FilingDetail, state: &FilingDetailState) {
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
    let options = state.get_yank_options(filing);
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
        Span::styled("up/down", Style::default().fg(Color::DarkGray)),
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
