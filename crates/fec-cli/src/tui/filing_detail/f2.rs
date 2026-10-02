//! Form 2 (Statement of Candidacy) cover rendering.
//!
//! Rendered straight from [`fec_parser::covers::Form2`], grouped like the
//! paper form: the candidate (Lines 1–4), the office sought (Lines 5–6 and
//! the Line 7 election year), the principal campaign committee (Line 7) and
//! the first other authorized committee (Line 8).

use fec_parser::covers::{Address, Form2, Form2Committee};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

const LABEL_WIDTH: usize = 15;

pub(crate) fn section_line(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    ))
}

/// An indented `Label          value` line with the value given as spans.
pub(crate) fn field_spans(label: &str, value: Vec<Span<'static>>) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("  {:<width$}", label, width = LABEL_WIDTH),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )];
    spans.extend(value);
    Line::from(spans)
}

pub(crate) fn field_line(label: &str, value: String) -> Line<'static> {
    field_spans(label, vec![Span::raw(value)])
}

pub(crate) fn note_line(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {text}"),
        Style::default().add_modifier(Modifier::DIM),
    ))
}

/// `"CODE (Label)"` when a label is known, else just the code.
pub(crate) fn code_with_label(code: &str, label: Option<&str>) -> String {
    match label {
        Some(label) => format!("{code} ({label})"),
        None => code.to_string(),
    }
}

/// Push an address as two aligned lines: street, then city/state/ZIP, so a
/// narrow view does not wrap it under the label column.
pub(crate) fn push_address(lines: &mut Vec<Line<'static>>, address: &Address, suffix: &str) {
    if address.is_empty() {
        return;
    }
    let street = [address.street_1.as_deref(), address.street_2.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");
    let locality = Address {
        street_1: None,
        street_2: None,
        ..address.clone()
    }
    .one_line();
    let mut parts = [street, locality]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if let Some(last) = parts.last_mut() {
        last.push_str(suffix);
    }
    for (i, part) in parts.into_iter().enumerate() {
        lines.push(field_line(if i == 0 { "Address" } else { "" }, part));
    }
}

fn push_committee(lines: &mut Vec<Line<'static>>, committee: &Form2Committee) {
    let name = committee.name.clone().unwrap_or_else(|| "(no name)".into());
    let mut spans = vec![Span::styled(
        name,
        Style::default().add_modifier(Modifier::BOLD),
    )];
    if let Some(ref id) = committee.id {
        spans.push(Span::raw(format!(" ({id})")));
    }
    lines.push(field_spans("Name", spans));
    push_address(lines, &committee.address, "");
}

pub fn append_f2_content_lines(lines: &mut Vec<Line<'static>>, form: &Form2) {
    let kind = if form.is_amendment() {
        "Amended statement"
    } else {
        "New statement"
    };
    lines.push(note_line(&format!("Statement of Candidacy - {kind}")));
    lines.push(Line::from(""));

    // Candidate (Lines 1-4)
    lines.push(section_line("Candidate"));
    lines.push(field_spans(
        "Name",
        vec![Span::styled(
            form.candidate.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )],
    ));
    if !form.candidate_id.is_empty() {
        lines.push(field_line("Candidate ID", form.candidate_id.clone()));
    }
    let changed = if form.change_of_address {
        " (address changed)"
    } else {
        ""
    };
    push_address(lines, &form.candidate_address, changed);
    if let Some(ref party) = form.party_code {
        lines.push(field_line(
            "Party",
            code_with_label(party, form.party_label()),
        ));
    }
    if let Some(ref vp) = form.vice_president {
        lines.push(field_line("Running mate", vp.to_string()));
    }
    lines.push(Line::from(""));

    // Office sought (Lines 5-6, Line 7 election year)
    lines.push(section_line("Office Sought"));
    if let Some(ref office) = form.office {
        lines.push(field_line(
            "Office",
            match form.office_label() {
                Some(label) => format!("{label} ({office})"),
                None => office.clone(),
            },
        ));
    }
    let mut race = vec![];
    if let Some(ref state) = form.office_state {
        race.push(state.clone());
    }
    if let Some(ref district) = form.district {
        race.push(format!("District {district}"));
    }
    if !race.is_empty() {
        lines.push(field_line("State/District", race.join(", ")));
    }
    if let Some(year) = form.election_year {
        lines.push(field_line("Election year", year.to_string()));
    }
    lines.push(Line::from(""));

    // Principal campaign committee (Line 7)
    lines.push(section_line("Principal Campaign Committee"));
    if form.principal_committee.is_empty() {
        lines.push(note_line("None designated"));
    } else {
        push_committee(lines, &form.principal_committee);
    }
    lines.push(Line::from(""));

    // Other authorized committee (Line 8)
    if let Some(ref committee) = form.authorized_committee {
        lines.push(section_line("Other Authorized Committee"));
        push_committee(lines, committee);
        lines.push(note_line(
            "Any further authorized committees are listed on F2S records.",
        ));
        lines.push(Line::from(""));
    }

    // Declarations (legacy, format <= 6.3 only)
    if let Some(ref decl) = form.personal_funds_declaration {
        lines.push(section_line("Declarations"));
        lines.push(note_line("Intent to spend personal funds"));
        if let Some(primary) = decl.primary {
            lines.push(field_line("Primary", super::format_usd(primary)));
        }
        if let Some(general) = decl.general {
            lines.push(field_line("General", super::format_usd(general)));
        }
        lines.push(Line::from(""));
    }
}
