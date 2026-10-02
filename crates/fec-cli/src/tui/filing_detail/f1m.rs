//! Form 1M (Notification of Multicandidate Status) cover rendering for the
//! filing detail view.
//!
//! Renders straight from [`fec_parser::covers::Form1M`]: committee (Lines 1–3),
//! then whichever of Line 4 (status by affiliation) and Line 5 (status by
//! qualification) the filer completed.

use super::layout::{bold, code_with_label, dim, Doc};
use fec_parser::covers::{Form1M, Form1MAffiliation, Form1MQualification};
use ratatui::{
    style::{Color, Style},
    text::Span,
};

const ROMAN: [&str; 5] = ["(i)", "(ii)", "(iii)", "(iv)", "(v)"];
/// Indent of a Line 5(a) candidate's detail line, under the name.
const CANDIDATE_INDENT: usize = 6;

fn date_or_dash(date: Option<jiff::civil::Date>) -> String {
    date.map(|d| d.to_string())
        .unwrap_or_else(|| "—".to_string())
}

fn push_committee(d: &mut Doc, form: &Form1M) {
    d.heading("Committee");
    d.field("Name", form.committee_name.clone());
    if !form.filer_committee_id_number.is_empty() {
        d.field("FEC ID", form.filer_committee_id_number.clone());
    }
    d.address("Address", &form.address, false);
    if let Some(ref code) = form.committee_type {
        d.field(
            "Committee type",
            code_with_label(code, form.committee_type_label()),
        );
    }
    d.blank();
}

fn push_affiliation(d: &mut Doc, a: &Form1MAffiliation) {
    d.heading("Status by affiliation (Line 4)");
    d.field("Form 1 filed", date_or_dash(a.date_form1_filed));
    let mut name = a.committee_name.clone().unwrap_or_else(|| "—".to_string());
    if let Some(ref id) = a.committee_id {
        name.push_str(&format!(" ({id})"));
    }
    d.field("Affiliated with", name);
    d.blank();
}

fn push_qualification(d: &mut Doc, q: &Form1MQualification) {
    d.heading("Status by qualification (Line 5)");
    d.field_spans("Contributions to candidates", vec![dim("(5a)")]);
    if q.candidates.is_empty() {
        d.note("None listed (State party committees may leave this blank).");
    }
    for (i, c) in q.candidates.iter().enumerate() {
        let mut name = vec![bold(c.name.to_string())];
        if let Some(ref id) = c.candidate_id {
            name.push(Span::raw(format!(" ({id})")));
        }
        let roman = format!("{:<CANDIDATE_INDENT$}", ROMAN.get(i).copied().unwrap_or(""));
        d.wrapped(vec![Span::raw(roman)], CANDIDATE_INDENT, name);

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
            d.wrapped(
                vec![Span::raw(" ".repeat(CANDIDATE_INDENT))],
                CANDIDATE_INDENT,
                vec![Span::styled(
                    detail.join(" · "),
                    Style::default().fg(Color::Gray),
                )],
            );
        }
    }
    d.blank();
    d.field_spans(
        "51st contributor",
        vec![
            Span::raw(date_or_dash(q.fifty_first_contributor_date)),
            dim(" (5b)"),
        ],
    );
    d.field_spans(
        "Registered",
        vec![
            Span::raw(date_or_dash(q.original_registration_date)),
            dim(" (5c)"),
        ],
    );
    d.field_spans(
        "Qualified",
        vec![bold(date_or_dash(q.requirements_met_date)), dim(" (5d)")],
    );
    d.blank();
}

pub(super) fn append_f1m_content_lines(d: &mut Doc, form: &Form1M) {
    push_committee(d, form);
    if let Some(ref a) = form.affiliation {
        push_affiliation(d, a);
    }
    if let Some(ref q) = form.qualification {
        push_qualification(d, q);
    }
    if form.affiliation.is_none() && form.qualification.is_none() {
        d.note("Neither Line 4 nor Line 5 was completed.");
        d.blank();
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
