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
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Action returned by handle_key_event indicating what the parent should do
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitteeDetailAction {
    /// Key was handled internally, no action needed from parent
    None,
    /// User wants to exit/go back
    Exit,
    /// User pressed 'o' to open in browser
    OpenBrowser,
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
}

impl CommitteeDetailState {
    pub fn new() -> Self {
        Self {
            show_yank_popup: false,
            yank_selected: 0,
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
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                if self.show_yank_popup {
                    self.show_yank_popup = false;
                    CommitteeDetailAction::None
                } else {
                    CommitteeDetailAction::Exit
                }
            }
            KeyCode::Char('o') => CommitteeDetailAction::OpenBrowser,
            KeyCode::Char('y') => {
                if !self.show_yank_popup {
                    self.show_yank_popup = true;
                }
                CommitteeDetailAction::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.show_yank_popup {
                    self.yank_next(committee);
                }
                CommitteeDetailAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.show_yank_popup {
                    self.yank_previous(committee);
                }
                CommitteeDetailAction::None
            }
            KeyCode::Enter => {
                if self.show_yank_popup {
                    self.copy_selected(committee);
                    self.show_yank_popup = false;
                }
                CommitteeDetailAction::None
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
        Span::styled(" copy", Style::default().fg(Color::DarkGray)),
    ]);
    let help = Paragraph::new(help_line)
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::TOP).border_style(
            Style::default().fg(Color::DarkGray)
        ));
    f.render_widget(help, area);
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
