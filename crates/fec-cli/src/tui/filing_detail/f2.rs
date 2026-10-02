//! Form 2 (Statement of Candidacy) cover rendering.
//!
//! Rendered straight from [`fec_parser::covers::Form2`], grouped like the
//! paper form: the candidate (Lines 1–4), the office sought (Lines 5–6 and
//! the Line 7 election year), the principal campaign committee (Line 7) and
//! the first other authorized committee (Line 8).

use super::layout::{bold, code_with_label, Doc};
use fec_parser::covers::{Form2, Form2Committee};
use ratatui::text::Span;

fn push_committee(d: &mut Doc, committee: &Form2Committee) {
    let name = committee.name.clone().unwrap_or_else(|| "(no name)".into());
    let mut spans = vec![bold(name)];
    if let Some(ref id) = committee.id {
        spans.push(Span::raw(format!(" ({id})")));
    }
    d.field_spans("Name", spans);
    d.address("Address", &committee.address, false);
}

pub(super) fn append_f2_content_lines(d: &mut Doc, form: &Form2) {
    let kind = if form.is_amendment() {
        "Amended statement"
    } else {
        "New statement"
    };
    d.note(&format!("Statement of Candidacy - {kind}"));
    d.blank();

    // Candidate (Lines 1-4)
    d.heading("Candidate");
    d.field_spans("Name", vec![bold(form.candidate.to_string())]);
    if !form.candidate_id.is_empty() {
        d.field("Candidate ID", form.candidate_id.clone());
    }
    d.address("Address", &form.candidate_address, form.change_of_address);
    if let Some(ref party) = form.party_code {
        d.field("Party", code_with_label(party, form.party_label()));
    }
    d.field_opt(
        "Running mate",
        form.vice_president.as_ref().map(|vp| vp.to_string()),
    );
    d.blank();

    // Office sought (Lines 5-6, Line 7 election year)
    d.heading("Office Sought");
    if let Some(ref office) = form.office {
        d.field("Office", code_with_label(office, form.office_label()));
    }
    let mut race = vec![];
    if let Some(ref state) = form.office_state {
        race.push(state.clone());
    }
    if let Some(ref district) = form.district {
        race.push(format!("District {district}"));
    }
    if !race.is_empty() {
        d.field("State/District", race.join(", "));
    }
    d.field_opt("Election year", form.election_year.map(|y| y.to_string()));
    d.blank();

    // Principal campaign committee (Line 7)
    d.heading("Principal Campaign Committee");
    if form.principal_committee.is_empty() {
        d.note("None designated");
    } else {
        push_committee(d, &form.principal_committee);
    }
    d.blank();

    // Other authorized committee (Line 8)
    if let Some(ref committee) = form.authorized_committee {
        d.heading("Other Authorized Committee");
        push_committee(d, committee);
        d.note("Any further authorized committees are listed on F2S records.");
        d.blank();
    }

    // Declarations (legacy, format <= 6.3 only)
    if let Some(ref decl) = form.personal_funds_declaration {
        d.heading("Declarations");
        d.note("Intent to spend personal funds");
        d.table(super::layout::Columns::One);
        if let Some(primary) = decl.primary {
            d.amount("Primary", primary, false);
        }
        if let Some(general) = decl.general {
            d.amount("General", general, false);
        }
        d.blank();
    }
}
