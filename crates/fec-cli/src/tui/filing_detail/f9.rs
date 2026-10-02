//! Form 9 (24-hour notice of disbursements/obligations for electioneering
//! communications) cover rendering.

use super::layout::{amendment_text, bold, Columns, Doc};
use fec_parser::covers::Form9;
use ratatui::style::Color;

pub(super) fn append_f9_content_lines(d: &mut Doc, form: &Form9) {
    d.banner(
        "24-HOUR NOTICE",
        Color::Red,
        "of Electioneering Communications",
        amendment_text(form.is_amendment(), form.original_amendment_date, "report"),
    );
    d.blank();

    // Line 6: the communication.
    d.heading("Communication");
    if let Some(title) = &form.communication_title {
        d.field_spans("Title", vec![bold(title.clone())]);
    }
    d.field_opt(
        "Distributed",
        form.date_public_distribution.map(|date| date.to_string()),
    );
    let covered = match (form.coverage_from_date, form.coverage_through_date) {
        (Some(from), Some(through)) => Some(format!("{from} through {through}")),
        (Some(from), None) => Some(format!("from {from}")),
        (None, Some(through)) => Some(format!("through {through}")),
        (None, None) => None,
    };
    d.field_opt("Covered period", covered);
    d.blank();

    // Lines 1-3 and 7: the filer.
    d.heading("Filer");
    d.field("Name", form.filer_name());
    let filer_type = form.filer_code.as_deref().map(|code| {
        let label = form.filer_code_label().unwrap_or(code);
        match (&form.filer_code_description, label) {
            (Some(desc), _) => format!("{label}: {desc}"),
            (None, label) => label.to_string(),
        }
    });
    d.field_opt("Filer is", filer_type);
    d.field_opt("Entity type", form.entity_type.clone());
    d.field("FEC ID", form.filer_committee_id.clone());
    d.address("Address", &form.address, form.change_of_address);
    d.field_opt("Occupation", form.individual_occupation.clone());
    d.field_opt("Employer", form.individual_employer.clone());
    d.field_opt("Qualified NPO", form.qualified_non_profit.clone());
    d.field_opt(
        "Segregated acct",
        form.used_segregated_bank_account()
            .map(|yes| if yes { "Yes" } else { "No" }.to_string())
            .or_else(|| form.segregated_bank_account.clone()),
    );
    d.blank();

    // Line 9: custodian of records.
    let c = &form.custodian;
    if !c.name.is_empty() || !c.address.is_empty() {
        d.heading("Custodian of Records");
        d.person("Name", &c.name);
        d.address("Address", &c.address, false);
        d.field_opt("Employer", c.employer.clone());
        d.field_opt("Occupation", c.occupation.clone());
        d.blank();
    }

    // Lines 10-11: totals.
    d.heading("Totals This Statement");
    d.table(Columns::One);
    d.amount("10. Total Donations", form.total_donations, true);
    d.amount(
        "11. Total Disbursements/Obligations",
        form.total_disbursements,
        true,
    );
    d.blank();

    d.note(
        "Disbursements include contracts creating an obligation to make disbursements, not only payments made.",
    );
    d.blank();
}
