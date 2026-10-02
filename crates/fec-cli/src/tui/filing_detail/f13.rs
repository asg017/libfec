//! Form 13 (donations accepted for an inaugural committee) cover rendering.

use super::misc_helpers::{
    amount_line, code_with_label, field_line, note_line, push_address, section_line,
};
use fec_parser::covers::Form13;
use ratatui::text::Line;

pub fn append_f13_content_lines(lines: &mut Vec<Line<'static>>, data: &Form13) {
    push_address(lines, &data.address);
    if data.change_of_address {
        lines.push(note_line("Address changed since last report"));
    }
    if let Some(ref code) = data.report_code {
        lines.push(field_line(
            "Filing",
            code_with_label(code, data.report_code_label()),
        ));
    }
    if let Some(date) = data.amendment_date {
        lines.push(field_line("Amends", format!("filing dated {date}")));
    }

    lines.push(Line::from(""));
    lines.push(section_line(
        "CUMULATIVE TOTALS (FROM COMMITTEE'S INCEPTION)",
    ));
    lines.push(amount_line(
        "5. Total donations accepted",
        Some(data.line5_total_donations_accepted),
        false,
    ));
    lines.push(amount_line(
        "6. Total donations refunded",
        Some(data.line6_total_donations_refunded),
        false,
    ));
    lines.push(amount_line(
        "7. Net donations",
        Some(data.line7_net_donations),
        true,
    ));
    lines.push(Line::from(""));
}
