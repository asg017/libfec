//! Form 9 (24-hour notice of disbursements/obligations for electioneering
//! communications) cover rendering.

use super::f24::{
    amendment_text, amount_line, field_line, note_line, push_address, push_opt, push_person,
    push_report_banner, section_line,
};
use fec_parser::covers::Form9;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn append_f9_content_lines(lines: &mut Vec<Line<'static>>, form: &Form9) {
    push_report_banner(
        lines,
        "24-HOUR NOTICE".to_string(),
        Color::Red,
        "of Electioneering Communications",
        amendment_text(form.is_amendment(), form.original_amendment_date, "report"),
    );
    lines.push(Line::from(""));

    // Line 6: the communication.
    lines.push(section_line("Communication"));
    if let Some(title) = &form.communication_title {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<width$}", "Title", width = super::f24::LABEL_WIDTH),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(title.clone(), Style::default().add_modifier(Modifier::BOLD)),
        ]));
    }
    push_opt(
        lines,
        "Distributed",
        form.date_public_distribution.map(|d| d.to_string()),
    );
    let covered = match (form.coverage_from_date, form.coverage_through_date) {
        (Some(from), Some(through)) => Some(format!("{from} through {through}")),
        (Some(from), None) => Some(format!("from {from}")),
        (None, Some(through)) => Some(format!("through {through}")),
        (None, None) => None,
    };
    push_opt(lines, "Covered period", covered);
    lines.push(Line::from(""));

    // Lines 1-3 and 7: the filer.
    lines.push(section_line("Filer"));
    lines.push(field_line("Name", form.filer_name()));
    let filer_type = form.filer_code.as_deref().map(|code| {
        let label = form.filer_code_label().unwrap_or(code);
        match (&form.filer_code_description, label) {
            (Some(desc), _) => format!("{label}: {desc}"),
            (None, label) => label.to_string(),
        }
    });
    push_opt(lines, "Filer is", filer_type);
    push_opt(lines, "Entity type", form.entity_type.clone());
    lines.push(field_line("FEC ID", form.filer_committee_id.clone()));
    push_address(lines, "Address", &form.address);
    if form.change_of_address {
        lines.push(field_line("", "(address changed since last report)"));
    }
    push_opt(lines, "Occupation", form.individual_occupation.clone());
    push_opt(lines, "Employer", form.individual_employer.clone());
    push_opt(lines, "Qualified NPO", form.qualified_non_profit.clone());
    push_opt(
        lines,
        "Segregated acct",
        form.used_segregated_bank_account()
            .map(|yes| if yes { "Yes" } else { "No" }.to_string())
            .or_else(|| form.segregated_bank_account.clone()),
    );
    lines.push(Line::from(""));

    // Line 9: custodian of records.
    let c = &form.custodian;
    if !c.name.is_empty() || !c.address.is_empty() {
        lines.push(section_line("Custodian of Records"));
        push_person(lines, "Name", &c.name);
        push_address(lines, "Address", &c.address);
        push_opt(lines, "Employer", c.employer.clone());
        push_opt(lines, "Occupation", c.occupation.clone());
        lines.push(Line::from(""));
    }

    // Lines 10-11: totals.
    lines.push(section_line("Totals This Statement"));
    lines.push(amount_line("10. Total Donations", form.total_donations, 36));
    lines.push(amount_line(
        "11. Total Disbursements/Obligations",
        form.total_disbursements,
        36,
    ));
    lines.push(Line::from(""));

    lines.push(note_line(
        "Disbursements include contracts creating an obligation to make disbursements, not only payments made.",
    ));
    lines.push(Line::from(""));
}
