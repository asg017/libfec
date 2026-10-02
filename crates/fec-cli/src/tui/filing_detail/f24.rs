//! Form 24 (24/48-hour notice of independent expenditures) cover rendering.

use super::layout::{amendment_text, Doc};
use fec_parser::covers::Form24;
use ratatui::style::Color;

pub(super) fn append_f24_content_lines(d: &mut Doc, form: &Form24) {
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
    d.banner(
        &banner,
        color,
        "of Independent Expenditures",
        amendment_text(form.is_amendment(), form.original_amendment_date, "report"),
    );
    if let Some(when) = when {
        d.note(when);
    }
    d.blank();

    d.heading("Committee");
    d.field("Name", form.committee_name.clone());
    d.field("FEC ID", form.filer_committee_id.clone());
    d.address("Address", &form.address, false);
    d.person("Treasurer", &form.treasurer);
    d.blank();

    d.note(
        "Each report includes the information required on Schedule E; the same expenditures are reported again on the committee's next regular report.",
    );
    d.blank();
}
