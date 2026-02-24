//! Filing Detail TUI Component
//!
//! This module provides rendering functions for displaying detailed FEC filing information
//! within a ratatui application. It supports form-specific cover UIs for F1, F3, and F3P
//! while sharing common chrome (title, URL, metadata, help bar, yank popup, key handling).
//!
//! Keyboard shortcuts:
//! - Esc/q: Return to previous view
//! - c: View committee or candidate detail page
//! - o: Open filing in browser
//! - w: Open committee website (F1 forms only)
//! - y: Open copy popup to copy filing ID, filer ID, or filer name to clipboard
//! - j/k: Scroll up/down

pub mod f1;
pub mod f3;
pub mod f3p;

use crate::tui::{navigation_popup_help_line, HelpBar};
use crossterm::event::{KeyCode, KeyEvent};
use f1::FilingDetailF1;
use f3::FilingDetailF3;
use f3p::FilingDetailF3P;
use fec_parser::{covers::Cover, report_code_label};
use indicatif::HumanBytes;
use num_format::{Locale, ToFormattedString};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Action returned by handle_key_event indicating what the parent should do
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilingDetailAction {
    /// Key was handled internally, no action needed from parent
    None,
    /// User wants to exit/go back
    Exit,
    /// User pressed 'o' to open in browser
    OpenBrowser,
    /// User pressed 'c' to view committee/candidate detail
    ShowFiler { filer_id: String },
    /// User pressed 'w' to open committee website (F1)
    OpenWebsite { url: String },
}

pub enum FilingCoverContent {
    Form1(Box<FilingDetailF1>),
    Form3(FilingDetailF3),
    Form3P(FilingDetailF3P),
    Unknown,
}

/// Holds extracted filing information for TUI display
pub struct FilingDetail {
    pub filing_id: String,
    pub form_type: String,
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
    pub cover_content: FilingCoverContent,
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
        let (treasurer, signed_date, cover_content) =
            if let Some(ref cover) = filing.cover.cover_data {
                match cover {
                    Cover::Form1(form) => (
                        Some(form.treasurer.to_string()),
                        form.date_signed.map(|d| d.to_string()),
                        FilingCoverContent::Form1(Box::new(FilingDetailF1::from(form))),
                    ),
                    Cover::Form3(form) => (
                        Some(form.treasurer.to_string()),
                        Some(form.signed.to_string()),
                        FilingCoverContent::Form3(FilingDetailF3::from(form)),
                    ),
                    Cover::Form3P(form) => (
                        Some(form.treasurer.to_string()),
                        Some(form.signed.to_string()),
                        FilingCoverContent::Form3P(FilingDetailF3P::from(form)),
                    ),
                }
            } else {
                (None, None, FilingCoverContent::Unknown)
            };

        FilingDetail {
            filing_id: filing.filing_id.clone(),
            form_type: filing.cover.form_type.clone(),
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
            cover_content,
        }
    }
}

pub fn format_usd(amount: f64) -> String {
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

    pub fn get_yank_options(
        &self,
        filing: &FilingDetail,
    ) -> Vec<(YankOption, String, Option<String>)> {
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
    pub fn handle_key_event(&mut self, key: KeyEvent, filing: &FilingDetail) -> FilingDetailAction {
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
            KeyCode::Char('c') => FilingDetailAction::ShowFiler {
                filer_id: filing.filer_id.clone(),
            },
            KeyCode::Char('w') => {
                if let FilingCoverContent::Form1(ref data) = filing.cover_content {
                    if let Some(ref url) = data.committee_url {
                        if !url.is_empty() {
                            return FilingDetailAction::OpenWebsite { url: url.clone() };
                        }
                    }
                }
                FilingDetailAction::None
            }
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
        "FEC-{} {} {} by {} ({})",
        filing.filing_id, filing.form_type, report_label, filing.filer_name, filing.filer_id
    );
    let title = Paragraph::new(title_text).style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(title, area);
}

fn render_content(f: &mut Frame, filing: &FilingDetail, state: &FilingDetailState, area: Rect) {
    let mut lines: Vec<Line<'static>> = vec![];

    // Coverage period
    if let (Some(ref from), Some(ref through)) = (&filing.coverage_from, &filing.coverage_through) {
        lines.push(Line::from(vec![
            Span::styled(
                from.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" through "),
            Span::styled(
                through.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
    }

    // Treasurer and signed date
    if let (Some(ref treasurer), Some(ref signed)) = (&filing.treasurer, &filing.signed_date) {
        lines.push(Line::from(vec![
            Span::styled(
                "Signed by: ".to_string(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(treasurer.clone()),
            Span::raw(" on "),
            Span::raw(signed.clone()),
        ]));
        lines.push(Line::from(""));
    }

    // Form-specific content
    match &filing.cover_content {
        FilingCoverContent::Form1(data) => f1::append_f1_content_lines(&mut lines, data),
        FilingCoverContent::Form3(data) => f3::append_f3_content_lines(&mut lines, data),
        FilingCoverContent::Form3P(data) => f3p::append_f3p_content_lines(&mut lines, data),
        FilingCoverContent::Unknown => {}
    }

    // FEC URL
    let fec_url = format!(
        "https://docquery.fec.gov/cgi-bin/forms/{}/{}",
        filing.filer_id, filing.filing_id
    );
    lines.push(Line::from(vec![
        Span::styled(
            "URL: ".to_string(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            fec_url,
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::UNDERLINED),
        ),
    ]));
    lines.push(Line::from(""));

    // Metadata line
    lines.push(Line::from(vec![
        Span::styled(
            "Version: ".to_string(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("v{}", filing.fec_version)),
        Span::raw("  "),
        Span::styled(
            "Size: ".to_string(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(HumanBytes(filing.source_length as u64).to_string()),
    ]));

    lines.push(Line::from(vec![
        Span::styled(
            "Software: ".to_string(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "{} {}",
            filing.software_name, filing.software_version
        )),
    ]));

    // Optional metadata
    if let Some(ref report_id) = filing.report_id {
        lines.push(Line::from(vec![
            Span::styled(
                "Report ID: ".to_string(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "'{}' ({})",
                report_id,
                filing.report_number.as_deref().unwrap_or("")
            )),
        ]));
    }

    if let Some(ref comment) = filing.comment {
        lines.push(Line::from(vec![
            Span::styled(
                "Comment: ".to_string(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("'{}'", comment)),
        ]));
    }

    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((state.scroll_offset, 0));
    f.render_widget(content, area);
}

fn render_help_text(f: &mut Frame, filing: &FilingDetail, area: Rect) {
    let mut help = HelpBar::new()
        .keys(vec!["Esc", "q"], " back")
        .item("c", " filer")
        .item("o", " open");

    // Show 'w' key for F1 forms with a committee URL
    if let FilingCoverContent::Form1(ref data) = filing.cover_content {
        if data.committee_url.as_ref().is_some_and(|u| !u.is_empty()) {
            help = help.item("w", " website");
        }
    }

    help.item("y", " copy").item("j/k", " scroll").render(f, area);
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
            Constraint::Length(1), // Title
            Constraint::Min(10),   // Content
            Constraint::Length(2), // Help text
        ]);

    let [title_area, content_area, help_area] = area.layout(&layout);

    render_title(f, filing, title_area);
    render_content(f, filing, state, content_area);
    render_help_text(f, filing, help_area);

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
