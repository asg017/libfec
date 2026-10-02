//! Form 7 (communication costs by corporations and membership organizations)
//! cover rendering.

use super::misc_helpers::{amount_line, code_with_label, field_line, push_address, section_line};
use fec_parser::covers::Form7;
use ratatui::text::Line;

pub fn append_f7_content_lines(lines: &mut Vec<Line<'static>>, data: &Form7) {
    push_address(lines, &data.address);
    if let Some(ref code) = data.organization_type {
        lines.push(field_line(
            "Organization",
            code_with_label(code, data.organization_type_label()),
        ));
    }
    if let Some(ref code) = data.report_code {
        lines.push(field_line(
            "Report",
            code_with_label(code, data.report_code_label()),
        ));
    }
    if data.election_date.is_some() || data.election_state.is_some() {
        let mut value = String::new();
        if let Some(date) = data.election_date {
            value.push_str(&date.to_string());
        }
        if let Some(ref state) = data.election_state {
            if !value.is_empty() {
                value.push(' ');
            }
            value.push_str(&format!("in {state}"));
        }
        lines.push(field_line("Election", value));
    }
    if let Some(ref title) = data.person_designated_title {
        lines.push(field_line("Signer title", title.clone()));
    }

    lines.push(Line::from(""));
    lines.push(section_line("SUMMARY OF COMMUNICATION COSTS"));
    lines.push(amount_line(
        "Total costs this period",
        Some(data.total_communication_costs),
        true,
    ));
    lines.push(Line::from(""));
}
