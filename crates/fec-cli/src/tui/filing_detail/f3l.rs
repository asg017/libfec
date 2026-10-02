//! Form 3L (bundled contributions by lobbyists/registrants) cover rendering.

use super::misc_helpers::{
    amount_line, code_with_label, field_line, note_line, push_address, section_line,
};
use fec_parser::covers::Form3L;
use ratatui::text::Line;

pub fn append_f3l_content_lines(lines: &mut Vec<Line<'static>>, data: &Form3L) {
    push_address(lines, &data.address);
    if data.change_of_address {
        lines.push(note_line("Address changed since last report"));
    }

    // Line 4: candidate committees only.
    if let Some(ref state) = data.election_state {
        let value = match data.election_district.as_deref() {
            Some(district) => format!("{state}, district {district}"),
            None => state.clone(),
        };
        lines.push(field_line("Running in", value));
    }

    if let Some(ref code) = data.report_code {
        lines.push(field_line(
            "Report",
            code_with_label(code, data.report_code_label()),
        ));
    }

    // Line 5(c)/(d): pre-/post-election reports.
    if data.election_date.is_some() || data.election_held_in_state.is_some() {
        let mut value = String::new();
        if let Some(date) = data.election_date {
            value.push_str(&date.to_string());
        }
        if let Some(ref state) = data.election_held_in_state {
            if !value.is_empty() {
                value.push(' ');
            }
            value.push_str(&format!("in {state}"));
        }
        lines.push(field_line("Election", value));
    }

    // Line 6(b): which semi-annual period, when boxed.
    let semi_annual = match (
        data.semi_annual_january_june,
        data.semi_annual_july_december,
    ) {
        (true, true) => Some("January 1 - June 30 and July 1 - December 31"),
        (true, false) => Some("January 1 - June 30"),
        (false, true) => Some("July 1 - December 31"),
        (false, false) => None,
    };
    if let Some(period) = semi_annual {
        lines.push(field_line("Semi-annual", period));
    }

    lines.push(Line::from(""));
    lines.push(section_line("7. TOTAL REPORTABLE BUNDLED CONTRIBUTIONS"));
    lines.push(amount_line(
        "(a) Qtr/Monthly/Pre/Post",
        Some(data.line7a_quarterly_monthly_bundled_contributions),
        true,
    ));
    if data.covers_semi_annual_period() || data.line7b_semi_annual_bundled_contributions.is_some() {
        lines.push(amount_line(
            "(b) Semi-annual period",
            data.line7b_semi_annual_bundled_contributions,
            true,
        ));
        lines.push(note_line(
            "(a) and (b) are overlapping periods; do not add them.",
        ));
    }
    lines.push(Line::from(""));
}
