//! Form 5 (independent expenditures by persons other than political
//! committees) cover rendering.

use super::f24::{
    amendment_text, amount_line, field_line, note_line, push_address, push_opt, push_report_banner,
    section_line,
};
use fec_parser::covers::Form5;
use ratatui::{style::Color, text::Line};

pub fn append_f5_content_lines(lines: &mut Vec<Line<'static>>, form: &Form5) {
    // Line 4(a): type of report. 24/48-hour reports and quarterly reports
    // share this record; show whichever was filed.
    let amendment = amendment_text(form.is_amendment(), form.original_amendment_date);
    let is_hour_report = form.report_type.is_some();
    match (form.report_type.as_deref(), form.report_code.as_deref()) {
        (Some(code), _) => {
            let (banner, color) = match code {
                "24" => ("24-HOUR REPORT".to_string(), Color::Red),
                "48" => ("48-HOUR REPORT".to_string(), Color::Yellow),
                other => (format!("{other}-HOUR REPORT"), Color::Gray),
            };
            push_report_banner(
                lines,
                banner,
                color,
                "of Independent Expenditures",
                amendment,
            );
        }
        (None, Some(code)) => {
            let label = form.report_code_label().unwrap_or("Quarterly Report");
            push_report_banner(lines, code.to_string(), Color::Green, label, amendment);
        }
        (None, None) => {
            push_report_banner(
                lines,
                "FORM 5".to_string(),
                Color::Gray,
                "Report of Independent Expenditures",
                amendment,
            );
        }
    }
    if is_hour_report && (form.coverage_from_date.is_some() || form.coverage_through_date.is_some())
    {
        lines.push(note_line(
            "Coverage dates are optional on 24/48-hour reports and may be the dates the communication aired.",
        ));
    }
    lines.push(Line::from(""));

    // Lines 1-3: the filer.
    lines.push(section_line("Filer"));
    lines.push(field_line("Name", form.filer_name()));
    push_opt(lines, "Entity type", form.entity_type.clone());
    lines.push(field_line("FEC ID", form.filer_committee_id.clone()));
    push_address(lines, "Address", &form.address);
    if form.change_of_address {
        lines.push(field_line("", "(address changed since last report)"));
    }
    push_opt(lines, "Occupation", form.individual_occupation.clone());
    push_opt(lines, "Employer", form.individual_employer.clone());
    push_opt(
        lines,
        "Qualified NPO",
        form.qualified_nonprofit.as_deref().map(|v| match v {
            "Y" | "y" => "Yes".to_string(),
            "N" | "n" => "No".to_string(),
            other => other.to_string(),
        }),
    );
    lines.push(Line::from(""));

    // Legacy (v3/v5) filings name the election.
    if form.election_code.is_some() || form.election_date.is_some() || form.election_state.is_some()
    {
        lines.push(section_line("Election"));
        push_opt(lines, "Code", form.election_code.clone());
        push_opt(lines, "Date", form.election_date.map(|d| d.to_string()));
        push_opt(lines, "State", form.election_state.clone());
        lines.push(Line::from(""));
    }

    // Lines 6-7: the two totals.
    lines.push(section_line("Totals This Period"));
    lines.push(amount_line(
        "6. Total Contributions",
        form.total_contributions,
        36,
    ));
    lines.push(amount_line(
        "7. Total Independent Expenditures",
        form.total_independent_expenditures,
        36,
    ));
    lines.push(Line::from(""));

    lines.push(note_line(
        "Certified under penalty of perjury that the expenditures were not made in cooperation, consultation, or concert with any candidate, committee or party.",
    ));
    lines.push(Line::from(""));
}
