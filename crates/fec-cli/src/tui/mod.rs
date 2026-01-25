//! TUI (Terminal User Interface) module for displaying FEC data
//!
//! This module contains reusable TUI components for displaying FEC data in an
//! interactive terminal interface using ratatui.

pub mod candidate_detail;
pub mod committee_detail;
pub mod filing_detail;

use ratatui::layout::{Constraint, Flex, Layout, Rect};

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
