//! Form 1M (Notification of Multicandidate Status) cover rendering for the
//! filing detail view.
//!
//! Renders straight from [`fec_parser::covers::Form1M`]: committee (Lines 1–3),
//! then whichever of Line 4 (status by affiliation) and Line 5 (status by
//! qualification) the filer completed.

use super::f1::{code_with_label, field_line, label_span, note_line, push_address, section_header};
use fec_parser::covers::{Form1M, Form1MAffiliation, Form1MQualification};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

const ROMAN: [&str; 5] = ["(i)", "(ii)", "(iii)", "(iv)", "(v)"];

fn date_or_dash(date: Option<jiff::civil::Date>) -> String {
    date.map(|d| d.to_string())
        .unwrap_or_else(|| "—".to_string())
}

fn push_committee(lines: &mut Vec<Line<'static>>, form: &Form1M) {
    lines.push(section_header("Committee"));
    lines.push(field_line("Name", form.committee_name.clone()));
    if !form.filer_committee_id_number.is_empty() {
        lines.push(field_line("FEC ID", form.filer_committee_id_number.clone()));
    }
    push_address(lines, "Address", &form.address);
    if let Some(ref code) = form.committee_type {
        lines.push(field_line(
            "Committee type",
            code_with_label(code, form.committee_type_label()),
        ));
    }
    lines.push(Line::from(""));
}

fn push_affiliation(lines: &mut Vec<Line<'static>>, a: &Form1MAffiliation) {
    lines.push(section_header("Status by affiliation (Line 4)"));
    lines.push(field_line("Form 1 filed", date_or_dash(a.date_form1_filed)));
    let mut name = a.committee_name.clone().unwrap_or_else(|| "—".to_string());
    if let Some(ref id) = a.committee_id {
        name.push_str(&format!(" ({id})"));
    }
    lines.push(field_line("Affiliated with", name));
    lines.push(Line::from(""));
}

fn push_qualification(lines: &mut Vec<Line<'static>>, q: &Form1MQualification) {
    lines.push(section_header("Status by qualification (Line 5)"));
    lines.push(Line::from(Span::styled(
        "Contributions to candidates (5a)".to_string(),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    if q.candidates.is_empty() {
        lines.push(note_line(
            "  None listed (State party committees may leave this blank).",
        ));
    }
    for (i, c) in q.candidates.iter().enumerate() {
        let mut spans = vec![
            Span::raw(format!("  {:<6}", ROMAN.get(i).copied().unwrap_or(""))),
            Span::styled(
                c.name.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];
        if let Some(ref id) = c.candidate_id {
            spans.push(Span::raw(format!(" ({id})")));
        }
        lines.push(Line::from(spans));

        let mut detail = vec![];
        if let Some(ref office) = c.office {
            detail.push(c.office_label().unwrap_or(office).to_string());
        }
        match (&c.state, &c.district) {
            (Some(state), Some(district)) if c.office.as_deref() == Some("H") => {
                detail.push(format!("{state}-{district}"))
            }
            (Some(state), _) => detail.push(state.clone()),
            _ => {}
        }
        if let Some(date) = c.contribution_date {
            detail.push(format!("contributed {date}"));
        }
        if !detail.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("        {}", detail.join(" · ")),
                Style::default().fg(Color::Gray),
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        label_span("51st contributor"),
        Span::raw(date_or_dash(q.fifty_first_contributor_date)),
        Span::styled("  (5b)", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(vec![
        label_span("Registered"),
        Span::raw(date_or_dash(q.original_registration_date)),
        Span::styled("  (5c)", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(vec![
        label_span("Qualified"),
        Span::styled(
            date_or_dash(q.requirements_met_date),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled("  (5d)", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(""));
}

pub fn append_f1m_content_lines(lines: &mut Vec<Line<'static>>, form: &Form1M) {
    push_committee(lines, form);
    if let Some(ref a) = form.affiliation {
        push_affiliation(lines, a);
    }
    if let Some(ref q) = form.qualification {
        push_qualification(lines, q);
    }
    if form.affiliation.is_none() && form.qualification.is_none() {
        lines.push(note_line("Neither Line 4 nor Line 5 was completed."));
        lines.push(Line::from(""));
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::render_fixture;
    use insta::assert_snapshot;

    #[test]
    fn filing_detail_f1m_qualification() {
        assert_snapshot!(render_fixture("F1MN_1917288.fec", 100, 50));
    }

    #[test]
    fn filing_detail_f1m_qualification_narrow() {
        assert_snapshot!(render_fixture("F1MN_1917288.fec", 60, 40));
    }

    #[test]
    fn filing_detail_f1m_affiliation() {
        assert_snapshot!(render_fixture("F1MN_1924609.fec", 100, 30));
    }
}
