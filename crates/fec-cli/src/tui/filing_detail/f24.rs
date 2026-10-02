//! Form 24 (24/48-hour notice of independent expenditures) cover rendering,
//! plus small line-building helpers shared by the other notice-style covers
//! (Forms 5, 6 and 9).

use fec_parser::covers::{Address, Form24, PersonName};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Width of the label column in `field_line`.
pub(super) const LABEL_WIDTH: usize = 16;

/// A section heading, e.g. "Committee".
pub(super) fn section_line(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    ))
}

/// An indented `label  value` row with the label padded to [`LABEL_WIDTH`].
pub(super) fn field_line(label: &str, value: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:<width$}", label, width = LABEL_WIDTH),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(value.into()),
    ])
}

/// Push a `field_line` only when `value` is present.
pub(super) fn push_opt(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    value: Option<impl Into<String>>,
) {
    if let Some(value) = value {
        lines.push(field_line(label, value));
    }
}

/// Push an address row unless the address is blank.
pub(super) fn push_address(lines: &mut Vec<Line<'static>>, label: &str, address: &Address) {
    if !address.is_empty() {
        lines.push(field_line(label, address.one_line()));
    }
}

/// Push a person row unless the name is blank.
pub(super) fn push_person(lines: &mut Vec<Line<'static>>, label: &str, name: &PersonName) {
    if !name.is_empty() {
        lines.push(field_line(label, name.to_string()));
    }
}

/// A form-line total: `label` padded to `width`, then the amount right-aligned.
pub(super) fn amount_line(label: &str, amount: f64, width: usize) -> Line<'static> {
    Line::from(vec![
        Span::raw(format!("  {:<width$}", label, width = width)),
        Span::styled(
            format!("{:>16}", super::format_usd(amount)),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ])
}

/// A dimmed explanatory line.
pub(super) fn note_line(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    ))
}

/// The prominent "24-HOUR REPORT" / "48-HOUR REPORT" banner shared by
/// Forms 24 and 5, followed by the new/amendment status.
pub(super) fn push_report_banner(
    lines: &mut Vec<Line<'static>>,
    banner: String,
    color: Color,
    subtitle: &str,
    amendment: Option<String>,
) {
    lines.push(Line::from(vec![
        Span::styled(
            format!(" {banner} "),
            Style::default()
                .fg(Color::Black)
                .bg(color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            subtitle.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ]));
    if let Some(amendment) = amendment {
        lines.push(Line::from(Span::styled(
            amendment,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));
    }
}

/// Amendment status text for a cover whose form type ends in `A`.
/// `noun` is what the form calls the amended filing ("report", "notice").
pub(super) fn amendment_text(
    is_amendment: bool,
    original: Option<jiff::civil::Date>,
    noun: &str,
) -> Option<String> {
    match (is_amendment, original) {
        (true, Some(date)) => Some(format!("AMENDMENT of the {noun} filed {date}")),
        (true, None) => Some("AMENDMENT".to_string()),
        (false, _) => None,
    }
}

pub fn append_f24_content_lines(lines: &mut Vec<Line<'static>>, form: &Form24) {
    let (banner, color, when) = match form.report_type.as_deref() {
        Some("24") => (
            "24-HOUR REPORT".to_string(),
            Color::Red,
            Some("Independent expenditures aggregating $1,000 or more after the 20th day, but more than 24 hours, before the election."),
        ),
        Some("48") => (
            "48-HOUR REPORT".to_string(),
            Color::Yellow,
            Some("Independent expenditures aggregating $10,000 or more up to and including the 20th day before the election."),
        ),
        Some(other) => (format!("REPORT TYPE {other}"), Color::Gray, None),
        None => ("24/48-HOUR REPORT".to_string(), Color::Gray, None),
    };
    push_report_banner(
        lines,
        banner,
        color,
        "of Independent Expenditures",
        amendment_text(form.is_amendment(), form.original_amendment_date, "report"),
    );
    if let Some(when) = when {
        lines.push(note_line(when));
    }
    lines.push(Line::from(""));

    lines.push(section_line("Committee"));
    lines.push(field_line("Name", form.committee_name.clone()));
    lines.push(field_line("FEC ID", form.filer_committee_id.clone()));
    push_address(lines, "Address", &form.address);
    push_person(lines, "Treasurer", &form.treasurer);
    lines.push(Line::from(""));

    lines.push(note_line(
        "Each report includes the information required on Schedule E; the same expenditures are reported again on the committee's next regular report.",
    ));
    lines.push(Line::from(""));
}
