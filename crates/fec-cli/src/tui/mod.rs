//! TUI (Terminal User Interface) module for displaying FEC data
//!
//! This module contains reusable TUI components for displaying FEC data in an
//! interactive terminal interface using ratatui.

pub mod candidate_detail;
pub mod committee_detail;
pub mod filing_detail;

use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// A single item in the help bar (key binding + label)
pub struct HelpItem<'a> {
    /// Keys to display (multiple keys are joined with "/")
    pub keys: Vec<&'a str>,
    /// Label describing what the key does
    pub label: &'a str,
}

impl<'a> HelpItem<'a> {
    /// Create a help item with a single key
    pub fn new(key: &'a str, label: &'a str) -> Self {
        Self {
            keys: vec![key],
            label,
        }
    }

    /// Create a help item with multiple keys (displayed as "key1/key2")
    pub fn keys(keys: Vec<&'a str>, label: &'a str) -> Self {
        Self { keys, label }
    }
}

/// Builder for creating help bar content
pub struct HelpBar<'a> {
    items: Vec<HelpBarEntry<'a>>,
}

enum HelpBarEntry<'a> {
    Item(HelpItem<'a>),
    /// Plain text like "↑/↓ or j/k" displayed in gray
    Text(&'a str),
}

impl<'a> HelpBar<'a> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Add a key binding with a label
    pub fn item(mut self, key: &'a str, label: &'a str) -> Self {
        self.items
            .push(HelpBarEntry::Item(HelpItem::new(key, label)));
        self
    }

    /// Add multiple keys that do the same thing (displayed as "key1/key2 label")
    pub fn keys(mut self, keys: Vec<&'a str>, label: &'a str) -> Self {
        self.items
            .push(HelpBarEntry::Item(HelpItem::keys(keys, label)));
        self
    }

    /// Add plain gray text (useful for "↑/↓ or j/k" style navigation hints)
    pub fn text(mut self, text: &'a str) -> Self {
        self.items.push(HelpBarEntry::Text(text));
        self
    }

    /// Build into a Line for inline use (e.g., in popups)
    pub fn into_line(self) -> Line<'a> {
        let key_style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(Color::DarkGray);

        let mut spans = Vec::new();

        for (i, entry) in self.items.into_iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("  ", label_style));
            }

            match entry {
                HelpBarEntry::Item(item) => {
                    // Render keys with "/" separator
                    for (j, key) in item.keys.iter().enumerate() {
                        if j > 0 {
                            spans.push(Span::styled("/", label_style));
                        }
                        spans.push(Span::styled(*key, key_style));
                    }
                    spans.push(Span::styled(item.label, label_style));
                }
                HelpBarEntry::Text(text) => {
                    spans.push(Span::styled(text, label_style));
                }
            }
        }

        Line::from(spans)
    }

    /// Render as a help bar with top border (for bottom of detail views)
    pub fn render(self, f: &mut Frame, area: Rect) {
        let help = Paragraph::new(self.into_line())
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        f.render_widget(help, area);
    }
}

impl Default for HelpBar<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Common help bar for navigation popups (copy to clipboard dialogs)
pub fn navigation_popup_help_line() -> Line<'static> {
    HelpBar::new()
        .text("↑/↓ or j/k navigate")
        .item("Enter", " copy")
        .item("Esc", " cancel")
        .into_line()
}

/// Normalize an FEC candidate name from "LAST, FIRST MIDDLE" to "First Middle Last"
///
/// Splits on the first comma, swaps parts, and title-cases each word.
/// Hyphenated segments are title-cased independently (e.g., "OCASIO-CORTEZ" → "Ocasio-Cortez").
/// Names without a comma are just title-cased (e.g., "ACTBLUE" → "Actblue").
pub fn normalize_candidate_name(name: &str) -> String {
    fn title_case_word(word: &str) -> String {
        word.split('-')
            .map(|segment| {
                let mut chars = segment.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        let upper: String = first.to_uppercase().collect();
                        let lower: String = chars.as_str().to_lowercase();
                        format!("{}{}", upper, lower)
                    }
                }
            })
            .collect::<Vec<_>>()
            .join("-")
    }

    let (first_part, last_part) = match name.split_once(',') {
        Some((last, first)) => (first.trim(), last.trim()),
        None => {
            return name
                .split_whitespace()
                .map(title_case_word)
                .collect::<Vec<_>>()
                .join(" ");
        }
    };

    let mut words: Vec<String> = first_part.split_whitespace().map(title_case_word).collect();
    words.extend(last_part.split_whitespace().map(title_case_word));
    words.join(" ")
}

/// Truncate a string to max_len characters, adding "..." if truncated
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

/// Helper function to create a centered rect using certain percentage of available rect
pub fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
