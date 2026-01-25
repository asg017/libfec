//! Candidate Detail TUI Component
//!
//! This module provides rendering functions for displaying detailed candidate information
//! within a ratatui application. It integrates with parent TUI apps (search, info) to
//! provide a seamless navigation experience.
//!
//! The detail view shows all available candidate information including name, party,
//! office, address, campaign committee, and status.
//!
//! Keyboard shortcuts (handled by parent app):
//! - y: Open copy popup to copy candidate ID, committee ID, or name to clipboard
//! - Esc: Return to previous view
//!
//! Copy popup navigation:
//! - ↑/↓ or j/k: Navigate options
//! - Enter: Copy selected value to clipboard
//! - Esc: Cancel and close popup

use crate::cache::bulk_candidates::CandidateDetail;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Action returned by handle_key_event indicating what the parent should do
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateDetailAction {
    /// Key was handled internally, no action needed from parent
    None,
    /// User wants to exit/go back
    Exit,
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
}

impl CandidateDetailState {
    pub fn new() -> Self {
        Self {
            show_yank_popup: false,
            yank_selected: 0,
        }
    }

    pub fn get_yank_options(&self, candidate: &CandidateDetail) -> Vec<(YankOption, String, Option<String>)> {
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
        match key.code {
            KeyCode::Esc => {
                if self.show_yank_popup {
                    self.show_yank_popup = false;
                    CandidateDetailAction::None
                } else {
                    CandidateDetailAction::Exit
                }
            }
            KeyCode::Char('y') => {
                if !self.show_yank_popup {
                    self.show_yank_popup = true;
                }
                CandidateDetailAction::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.show_yank_popup {
                    self.yank_next(candidate);
                }
                CandidateDetailAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.show_yank_popup {
                    self.yank_previous(candidate);
                }
                CandidateDetailAction::None
            }
            KeyCode::Enter => {
                if self.show_yank_popup {
                    self.copy_selected(candidate);
                    self.show_yank_popup = false;
                }
                CandidateDetailAction::None
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
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)))
        .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(title, area);
}

fn render_content(f: &mut Frame, candidate: &CandidateDetail, area: Rect) {
    let content_block = Block::default()
        .borders(Borders::ALL)
        .title("Candidate Information")
        .border_style(Style::default().fg(Color::Green));

    let inner_area = content_block.inner(area);
    f.render_widget(content_block, area);

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Candidate ID: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(&candidate.candidate_id),
        ]),
        Line::from(""),
    ];

    lines.push(Line::from(vec![
        Span::styled("Name: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(&candidate.name),
    ]));
    lines.push(Line::from(""));

    // Office and location
    if !candidate.office.is_empty() {
        let office_desc = match candidate.office.as_str() {
            "H" => {
                if !candidate.state.is_empty() && !candidate.district.is_empty() {
                    format!("U.S. House ({}-{:02})",
                        candidate.state,
                        candidate.district.parse::<u8>().unwrap_or(0))
                } else {
                    "U.S. House".to_string()
                }
            }
            "S" => {
                if !candidate.state.is_empty() {
                    format!("U.S. Senate ({})", candidate.state)
                } else {
                    "U.S. Senate".to_string()
                }
            }
            "P" => "President".to_string(),
            _ => candidate.office.clone(),
        };

        lines.push(Line::from(vec![
            Span::styled("Office: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(office_desc),
        ]));
    }

    if candidate.election_year > 0 {
        lines.push(Line::from(vec![
            Span::styled("Election Year: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(candidate.election_year.to_string()),
        ]));
    }
    lines.push(Line::from(""));

    // Party and status
    if !candidate.party_affiliation.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Party: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(&candidate.party_affiliation),
        ]));
    }

    if !candidate.incumbent_challenger_status.is_empty() {
        let status_desc = match candidate.incumbent_challenger_status.as_str() {
            "I" => "Incumbent",
            "C" => "Challenger",
            "O" => "Open Seat",
            _ => &candidate.incumbent_challenger_status,
        };
        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(status_desc),
        ]));
    }

    if !candidate.status.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Candidate Status: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(&candidate.status),
        ]));
    }
    lines.push(Line::from(""));

    // Campaign committee
    if let Some(ref pcc) = candidate.principal_campaign_committee {
        lines.push(Line::from(vec![
            Span::styled("Principal Campaign Committee: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(pcc, Style::default().fg(Color::Magenta)),
        ]));
        lines.push(Line::from(""));
    }

    // Address
    if !candidate.address_street1.is_empty() || !candidate.address_city.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Address:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));

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

    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false });
    f.render_widget(content, inner_area);
}

fn render_help_text(f: &mut Frame, area: Rect) {
    let help_line = Line::from(vec![
        Span::styled("Esc", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" back  ", Style::default().fg(Color::DarkGray)),
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

pub fn render_candidate_detail(
    f: &mut Frame,
    area: Rect,
    candidate: &CandidateDetail,
    state: &CandidateDetailState,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(10),    // Content
            Constraint::Length(2),  // Help text
        ]);

    let [title_area, content_area, help_area] = area.layout(&layout);

    render_title(f, candidate, title_area);
    render_content(f, candidate, content_area);
    render_help_text(f, help_area);

    if state.show_yank_popup {
        render_yank_popup(f, area, candidate, state);
    }
}

fn render_yank_popup(f: &mut Frame, area: Rect, candidate: &CandidateDetail, state: &CandidateDetailState) {
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
