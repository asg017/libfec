/**
 * Candidate Detail TUI Component
 *
 * This module provides rendering functions for displaying detailed candidate information
 * within a ratatui application. It integrates with parent TUI apps (search, info) to
 * provide a seamless navigation experience.
 *
 * The detail view shows all available candidate information including name, party,
 * office, address, campaign committee, and status.
 *
 * Keyboard shortcuts (handled by parent app):
 * - y: Open copy popup to copy candidate ID, committee ID, or name to clipboard
 * - Esc: Return to previous view
 *
 * Copy popup navigation:
 * - ↑/↓ or j/k: Navigate options
 * - Enter: Copy selected value to clipboard
 * - Esc: Cancel and close popup
 */

use crate::cache::bulk_candidates::CandidateDetail;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

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
}

impl Default for CandidateDetailState {
    fn default() -> Self {
        Self::new()
    }
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
        ])
        .split(area);

    // Title
    let title_text = format!("{} ({})", candidate.name, candidate.candidate_id);
    let title = Paragraph::new(title_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)))
        .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(title, layout[0]);

    // Content area
    let content_block = Block::default()
        .borders(Borders::ALL)
        .title("Candidate Information")
        .border_style(Style::default().fg(Color::Green));

    let inner_area = content_block.inner(layout[1]);
    f.render_widget(content_block, layout[1]);

    // Build detailed information lines
    let mut lines = vec![];

    // Basic info
    lines.push(Line::from(vec![
        Span::styled("Candidate ID: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(&candidate.candidate_id),
    ]));
    lines.push(Line::from(""));

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

    // Help text
    let help_text = Line::from(vec![
        Span::styled("Esc", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" back  ", Style::default().fg(Color::DarkGray)),
        Span::styled("y", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" copy", Style::default().fg(Color::DarkGray)),
    ]);
    let help = Paragraph::new(help_text)
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::TOP).border_style(
            Style::default().fg(Color::DarkGray)
        ));
    f.render_widget(help, layout[2]);

    // Render yank popup if active
    if state.show_yank_popup {
        render_yank_popup(f, area, candidate, state);
    }
}

/// Helper function to create a centered rect using certain percentage of available rect
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

fn render_yank_popup(f: &mut Frame, area: Rect, candidate: &CandidateDetail, state: &CandidateDetailState) {
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
