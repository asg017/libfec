//! Form 6 (48-hour notice of contributions/loans received) cover rendering.

use super::layout::{amendment_text, bold, Doc};
use fec_parser::covers::Form6;
use ratatui::style::Color;

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

pub(super) fn append_f6_content_lines(d: &mut Doc, form: &Form6) {
    d.banner(
        "48-HOUR NOTICE",
        Color::Yellow,
        "of Contributions/Loans Received",
        amendment_text(form.is_amendment(), form.original_amendment_date, "notice"),
    );
    d.note(
        "Contributions and loans of $1,000 or more received after the 20th day, but more than 48 hours, before the election.",
    );
    d.blank();

    // Lines 2-3: candidate and office sought.
    d.heading("Candidate");
    let name = form.candidate.name.to_string();
    if !name.is_empty() {
        d.field_spans("Name", vec![bold(name)]);
    }
    d.field_opt("Candidate ID", form.candidate.candidate_id.clone());
    d.field_opt("Office sought", office_sought(form));
    d.blank();

    // Lines 1 and 4: the committee.
    d.heading("Committee");
    d.field("Name", form.committee_name.clone());
    d.field("FEC ID", form.filer_committee_id.clone());
    d.address("Address", &form.address, false);
    d.blank();

    d.note(
        "Contributions are itemized on this notice's F65 rows and itemized again in the first report filed after the election.",
    );
    d.blank();
}
