//! Form 6 (48-hour notice of contributions/loans received) cover rendering.

use super::f24::{
    amendment_text, field_line, note_line, push_address, push_opt, push_report_banner, section_line,
};
use fec_parser::covers::Form6;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// "Senate - TX" / "House - CA-22" / "President".
fn office_sought(form: &Form6) -> Option<String> {
    let c = &form.candidate;
    let office = c
        .office_label()
        .map(str::to_string)
        .or_else(|| c.office.clone());
    let district = c
        .district
        .as_deref()
        .filter(|d| !d.is_empty() && *d != "00" && c.office.as_deref() == Some("H"));
    let place = match (c.state.as_deref(), district) {
        (Some(state), Some(district)) => Some(format!("{state}-{district}")),
        (Some(state), None) => Some(state.to_string()),
        (None, Some(district)) => Some(format!("District {district}")),
        (None, None) => None,
    };
    match (office, place) {
        (Some(office), Some(place)) => Some(format!("{office} - {place}")),
        (Some(office), None) => Some(office),
        (None, Some(place)) => Some(place),
        (None, None) => None,
    }
}

pub fn append_f6_content_lines(lines: &mut Vec<Line<'static>>, form: &Form6) {
    push_report_banner(
        lines,
        "48-HOUR NOTICE".to_string(),
        Color::Yellow,
        "of Contributions/Loans Received",
        amendment_text(form.is_amendment(), form.original_amendment_date, "notice"),
    );
    lines.push(note_line(
        "Contributions and loans of $1,000 or more received after the 20th day, but more than 48 hours, before the election.",
    ));
    lines.push(Line::from(""));

    // Lines 2-3: candidate and office sought.
    lines.push(section_line("Candidate"));
    let name = form.candidate.name.to_string();
    if !name.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<width$}", "Name", width = super::f24::LABEL_WIDTH),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(name, Style::default().add_modifier(Modifier::BOLD)),
        ]));
    }
    push_opt(lines, "Candidate ID", form.candidate.candidate_id.clone());
    push_opt(lines, "Office sought", office_sought(form));
    lines.push(Line::from(""));

    // Lines 1 and 4: the committee.
    lines.push(section_line("Committee"));
    lines.push(field_line("Name", form.committee_name.clone()));
    lines.push(field_line("FEC ID", form.filer_committee_id.clone()));
    push_address(lines, "Address", &form.address);
    lines.push(Line::from(""));

    lines.push(note_line(
        "Contributions are itemized on this notice's F65 rows and itemized again in the first report filed after the election.",
    ));
    lines.push(Line::from(""));
}
