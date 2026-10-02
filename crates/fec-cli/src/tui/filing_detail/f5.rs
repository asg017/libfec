//! Form 5 (independent expenditures by persons other than political
//! committees) cover rendering.

use super::layout::{amendment_text, Columns, Doc};
use fec_parser::covers::Form5;
use ratatui::style::Color;

pub(super) fn append_f5_content_lines(d: &mut Doc, form: &Form5) {
    // Line 4(a): type of report. 24/48-hour reports and quarterly reports
    // share this record; show whichever was filed.
    let amendment = amendment_text(form.is_amendment(), form.original_amendment_date, "report");
    let is_hour_report = form.report_type.is_some();
    match (form.report_type.as_deref(), form.report_code.as_deref()) {
        (Some(code), _) => {
            let (banner, color) = match code {
                "24" => ("24-HOUR REPORT".to_string(), Color::Red),
                "48" => ("48-HOUR REPORT".to_string(), Color::Yellow),
                other => (format!("{other}-HOUR REPORT"), Color::Gray),
            };
            d.banner(&banner, color, "of Independent Expenditures", amendment);
        }
        (None, Some(code)) => {
            let label = form.report_code_label().unwrap_or("Quarterly Report");
            d.banner(code, Color::Green, label, amendment);
        }
        (None, None) => {
            d.banner(
                "FORM 5",
                Color::Gray,
                "Report of Independent Expenditures",
                amendment,
            );
        }
    }
    if is_hour_report && (form.coverage_from_date.is_some() || form.coverage_through_date.is_some())
    {
        d.note(
            "Coverage dates are optional on 24/48-hour reports and may be the dates the communication aired.",
        );
    }
    d.blank();

    // Lines 1-3: the filer.
    d.heading("Filer");
    d.field("Name", form.filer_name());
    d.field_opt("Entity type", form.entity_type.clone());
    d.field("FEC ID", form.filer_committee_id.clone());
    d.address("Address", &form.address, form.change_of_address);
    d.field_opt("Occupation", form.individual_occupation.clone());
    d.field_opt("Employer", form.individual_employer.clone());
    d.field_opt(
        "Qualified NPO",
        form.qualified_nonprofit.as_deref().map(|v| match v {
            "Y" | "y" => "Yes".to_string(),
            "N" | "n" => "No".to_string(),
            other => other.to_string(),
        }),
    );
    d.blank();

    // Legacy (v3/v5) filings name the election.
    if form.election_code.is_some() || form.election_date.is_some() || form.election_state.is_some()
    {
        d.heading("Election");
        d.field_opt("Code", form.election_code.clone());
        d.field_opt("Date", form.election_date.map(|d| d.to_string()));
        d.field_opt("State", form.election_state.clone());
        d.blank();
    }

    // Lines 6-7: the two totals.
    d.heading("Totals This Period");
    d.table(Columns::One);
    d.amount("6. Total Contributions", form.total_contributions, true);
    d.amount(
        "7. Total Independent Expenditures",
        form.total_independent_expenditures,
        true,
    );
    d.blank();

    d.note(
        "Certified under penalty of perjury that the expenditures were not made in cooperation, consultation, or concert with any candidate, committee or party.",
    );
    d.blank();
}
