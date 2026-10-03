//! Form 7 (communication costs by corporations and membership organizations)
//! cover rendering.

use super::layout::{code_with_label, report_code_text, Columns, Doc};
use fec_parser::covers::Form7;

pub(super) fn append_f7_content_lines(d: &mut Doc, data: &Form7) {
    d.address("Address", &data.address, false);
    if let Some(ref code) = data.organization_type {
        d.field(
            "Organization",
            code_with_label(code, data.organization_type_label()),
        );
    }
    if let Some(ref code) = data.report_code {
        d.field("Report", report_code_text(code, data.report_code_label()));
    }
    if data.election_date.is_some() || data.election_state.is_some() {
        let mut value = String::new();
        if let Some(date) = data.election_date {
            value.push_str(&date.to_string());
        }
        if let Some(ref state) = data.election_state {
            if !value.is_empty() {
                value.push(' ');
            }
            value.push_str(&format!("in {state}"));
        }
        d.field("Election", value);
    }
    d.field_opt("Signer title", data.person_designated_title.clone());

    d.blank();
    d.heading("SUMMARY OF COMMUNICATION COSTS");
    d.table(Columns::One);
    d.amount(
        "Total costs this period",
        data.total_communication_costs,
        true,
    );
    d.blank();
}
