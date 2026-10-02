//! Form 3L (bundled contributions by lobbyists/registrants) cover rendering.

use super::layout::{report_code_text, Columns, Doc};
use fec_parser::covers::Form3L;

pub(super) fn append_f3l_content_lines(d: &mut Doc, data: &Form3L) {
    d.address("Address", &data.address, data.change_of_address);

    // Line 4: candidate committees only.
    if let Some(ref state) = data.election_state {
        let value = match data.election_district.as_deref() {
            Some(district) => format!("{state}, district {district}"),
            None => state.clone(),
        };
        d.field("Running in", value);
    }

    if let Some(ref code) = data.report_code {
        d.field("Report", report_code_text(code, data.report_code_label()));
    }

    // Line 5(c)/(d): pre-/post-election reports.
    if data.election_date.is_some() || data.election_held_in_state.is_some() {
        let mut value = String::new();
        if let Some(date) = data.election_date {
            value.push_str(&date.to_string());
        }
        if let Some(ref state) = data.election_held_in_state {
            if !value.is_empty() {
                value.push(' ');
            }
            value.push_str(&format!("in {state}"));
        }
        d.field("Election", value);
    }

    // Line 6(b): which semi-annual period, when boxed.
    let semi_annual = match (
        data.semi_annual_january_june,
        data.semi_annual_july_december,
    ) {
        (true, true) => Some("January 1 - June 30 and July 1 - December 31"),
        (true, false) => Some("January 1 - June 30"),
        (false, true) => Some("July 1 - December 31"),
        (false, false) => None,
    };
    d.field_opt("Semi-annual", semi_annual);

    d.blank();
    d.heading("7. TOTAL REPORTABLE BUNDLED CONTRIBUTIONS");
    d.table(Columns::One);
    d.amount(
        "(a) Qtr/Monthly/Pre/Post",
        data.line7a_quarterly_monthly_bundled_contributions,
        true,
    );
    if data.covers_semi_annual_period() || data.line7b_semi_annual_bundled_contributions.is_some() {
        d.row_ab(
            "(b) Semi-annual period",
            data.line7b_semi_annual_bundled_contributions,
            None,
            true,
        );
        d.note("(a) and (b) are overlapping periods; do not add them.");
    }
    d.blank();
}
