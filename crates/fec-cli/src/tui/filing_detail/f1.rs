//! Form 1 (Statement of Organization) cover rendering for the filing detail view.
//!
//! Renders straight from [`fec_parser::covers::Form1`], one section per part of
//! the paper form: committee (Lines 1–3), type & designation and candidate
//! (Line 5), affiliated/connected organization (Line 6), custodian (Line 7),
//! treasurer and designated agent (Line 8) and banks (Line 9).
//!
//! Also home to small line-building helpers shared with the Form 1M renderer.

use fec_parser::covers::{Address, Form1, Form1Contact};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Width of the label column, so values line up.
pub(super) const LABEL_WIDTH: usize = 18;

/// A bold, colored section heading.
pub(super) fn section_header(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    ))
}

pub(super) fn label_span(label: &str) -> Span<'static> {
    Span::styled(
        format!("{:<width$}", format!("{label}: "), width = LABEL_WIDTH),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
}

/// `Label:          value`
pub(super) fn field_line(label: &str, value: impl Into<String>) -> Line<'static> {
    Line::from(vec![label_span(label), Span::raw(value.into())])
}

fn changed_span() -> Span<'static> {
    Span::styled(
        "  (changed)".to_string(),
        Style::default().fg(Color::Magenta),
    )
}

/// `Label:          value  (changed)`, for the "(Check if … is changed)" boxes.
fn field_line_changed(label: &str, value: impl Into<String>, changed: bool) -> Line<'static> {
    let mut spans = vec![label_span(label), Span::raw(value.into())];
    if changed {
        spans.push(changed_span());
    }
    Line::from(spans)
}

/// A field whose value is a link.
fn link_line(label: &str, url: &str, changed: bool) -> Line<'static> {
    let mut spans = vec![
        label_span(label),
        Span::styled(
            url.to_string(),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::UNDERLINED),
        ),
    ];
    if changed {
        spans.push(changed_span());
    }
    Line::from(spans)
}

/// A dimmed explanatory note.
pub(super) fn note_line(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    ))
}

/// Push an `Address:` line unless the address is blank.
pub(super) fn push_address(lines: &mut Vec<Line<'static>>, label: &str, address: &Address) {
    if !address.is_empty() {
        lines.push(field_line(label, address.one_line()));
    }
}

/// `(405) 826-6448` for ten-digit numbers, otherwise the value as filed.
fn format_phone(phone: &str) -> String {
    let phone = phone.trim();
    if phone.len() == 10 && phone.chars().all(|c| c.is_ascii_digit()) {
        format!("({}) {}-{}", &phone[0..3], &phone[3..6], &phone[6..])
    } else {
        phone.to_string()
    }
}

/// `"code (label)"`, or just the code when there is no sourced label.
pub(super) fn code_with_label(code: &str, label: Option<&str>) -> String {
    match label {
        Some(label) => format!("{code} ({label})"),
        None => code.to_string(),
    }
}

fn push_contact(lines: &mut Vec<Line<'static>>, title: &str, contact: &Form1Contact) {
    lines.push(section_header(title));
    let name = contact.name.to_string();
    lines.push(field_line(
        "Name",
        if name.is_empty() {
            "—".to_string()
        } else {
            name
        },
    ));
    if let Some(ref t) = contact.title {
        lines.push(field_line("Title", t.clone()));
    }
    push_address(lines, "Address", &contact.address);
    if let Some(ref phone) = contact.telephone {
        lines.push(field_line("Phone", format_phone(phone)));
    }
    lines.push(Line::from(""));
}

fn push_committee(lines: &mut Vec<Line<'static>>, form: &Form1) {
    lines.push(section_header("Committee"));
    lines.push(field_line_changed(
        "Name",
        form.committee_name.clone(),
        form.change_of_committee_name,
    ));
    if !form.filer_committee_id_number.is_empty() {
        lines.push(field_line("FEC ID", form.filer_committee_id_number.clone()));
    }
    if !form.address.is_empty() || form.change_of_address {
        lines.push(field_line_changed(
            "Address",
            form.address.one_line(),
            form.change_of_address,
        ));
    }
    if let Some(ref email) = form.committee_email {
        lines.push(field_line_changed(
            "Email",
            email.clone(),
            form.change_of_committee_email,
        ));
    }
    if let Some(ref url) = form.committee_url {
        lines.push(link_line("Website", url, form.change_of_committee_url));
    }
    if let Some(date) = form.effective_date {
        lines.push(field_line("Effective date", date.to_string()));
    }
    lines.push(Line::from(""));
}

fn push_type(lines: &mut Vec<Line<'static>>, form: &Form1) {
    lines.push(section_header("Type & designation"));
    let committee_type = match (
        form.committee_type.as_deref(),
        form.committee_type_line(),
        form.committee_type_label(),
    ) {
        (Some(code), Some(line), Some(label)) => format!("{code} · {line} {label}"),
        (Some(code), _, _) => code.to_string(),
        (None, _, _) => "—".to_string(),
    };
    lines.push(field_line("Committee type", committee_type));
    if let Some(ref org) = form.organization_type {
        lines.push(field_line(
            "Connected org",
            code_with_label(org, form.organization_type_label()),
        ));
    }
    if form.party_code.is_some() || form.party_type.is_some() {
        let mut parts = vec![];
        if let Some(ref code) = form.party_code {
            parts.push(code_with_label(code, form.party_code_label()));
        }
        if let Some(ref ptype) = form.party_type {
            parts.push(code_with_label(ptype, form.party_type_label()));
        }
        lines.push(field_line("Party", parts.join(" · ")));
    }
    let mut also = vec![];
    if form.pac_flags.is_lobbyist_registrant_pac() {
        also.push("Lobbyist/Registrant PAC");
    }
    if form.pac_flags.leadership_pac {
        also.push("Leadership PAC");
    }
    if !also.is_empty() {
        lines.push(field_line("Also", also.join(", ")));
    }
    lines.push(Line::from(""));
}

fn push_candidate(lines: &mut Vec<Line<'static>>, form: &Form1) {
    let Some(ref c) = form.candidate else {
        return;
    };
    lines.push(section_header("Candidate"));
    let mut name_spans = vec![
        label_span("Name"),
        Span::styled(c.full_name(), Style::default().add_modifier(Modifier::BOLD)),
    ];
    if let Some(ref id) = c.candidate_id {
        name_spans.push(Span::raw(format!(" ({id})")));
    }
    lines.push(Line::from(name_spans));

    let mut office = vec![];
    if let Some(ref o) = c.office {
        office.push(c.office_label().unwrap_or(o).to_string());
    }
    if let Some(ref state) = c.state {
        office.push(state.clone());
    }
    // Only House races have districts; Senate/President filings carry "00".
    if let Some(ref district) = c.district {
        if c.office.as_deref() == Some("H") {
            office.push(format!("District {district}"));
        }
    }
    if !office.is_empty() {
        lines.push(field_line("Office", office.join(" · ")));
    }
    lines.push(Line::from(""));
}

fn push_affiliated(lines: &mut Vec<Line<'static>>, form: &Form1) {
    let Some(ref a) = form.affiliated else {
        return;
    };
    lines.push(section_header("Affiliated / connected organization"));
    let mut name = a.display_name();
    if let Some(id) = a.committee_id.as_ref().or(a.candidate_id.as_ref()) {
        name.push_str(&format!(" ({id})"));
    }
    lines.push(field_line("Name", name));
    if let Some(ref code) = a.relationship_code {
        lines.push(field_line(
            "Relationship",
            code_with_label(code, a.relationship_label()),
        ));
    }
    push_address(lines, "Address", &a.address);
    lines.push(Line::from(""));
}

fn push_banks(lines: &mut Vec<Line<'static>>, form: &Form1) {
    if form.banks.is_empty() {
        return;
    }
    lines.push(section_header("Banks / depositories"));
    for (i, bank) in form.banks.iter().enumerate() {
        lines.push(field_line(
            &format!("Bank {}", i + 1),
            bank.name.clone().unwrap_or_else(|| "—".to_string()),
        ));
        push_address(lines, "Address", &bank.address);
    }
    lines.push(Line::from(""));
}

pub fn append_f1_content_lines(lines: &mut Vec<Line<'static>>, form: &Form1) {
    push_committee(lines, form);
    push_type(lines, form);
    push_candidate(lines, form);
    push_affiliated(lines, form);
    if let Some(ref custodian) = form.custodian {
        push_contact(lines, "Custodian of records", custodian);
    }
    push_contact(lines, "Treasurer", &form.treasurer);
    if let Some(ref agent) = form.agent {
        push_contact(lines, "Designated agent", agent);
    }
    push_banks(lines, form);
    lines.push(note_line(
        "Further affiliates, agents and banks may be filed on F1S records.",
    ));
    lines.push(Line::from(""));
}

#[cfg(test)]
mod tests {
    use super::super::tests::render_fixture;
    use insta::assert_snapshot;

    #[test]
    fn filing_detail_f1_candidate_committee() {
        assert_snapshot!(render_fixture("F1N_1906351.fec", 100, 70));
    }

    #[test]
    fn filing_detail_f1_ssf() {
        assert_snapshot!(render_fixture("F1A_1914988.fec", 100, 70));
    }

    #[test]
    fn filing_detail_f1_leadership_pac_narrow() {
        assert_snapshot!(render_fixture("F1A_1917499.fec", 60, 90));
    }

    #[test]
    fn format_phone() {
        assert_eq!(super::format_phone("4058266448"), "(405) 826-6448");
        assert_eq!(super::format_phone("x123"), "x123");
    }
}
