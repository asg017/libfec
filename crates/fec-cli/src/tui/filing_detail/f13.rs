//! Form 13 (donations accepted for an inaugural committee) cover rendering.

use super::layout::{report_code_text, Columns, Doc};
use fec_parser::covers::Form13;

pub(super) fn append_f13_content_lines(d: &mut Doc, data: &Form13) {
    d.address("Address", &data.address, data.change_of_address);
    if let Some(ref code) = data.report_code {
        d.field("Report", report_code_text(code, data.report_code_label()));
    }
    d.field_opt(
        "Amends",
        data.amendment_date
            .map(|date| format!("filing dated {date}")),
    );

    d.blank();
    d.heading("CUMULATIVE TOTALS (FROM COMMITTEE'S INCEPTION)");
    d.table(Columns::One);
    d.amount(
        "5. Total donations accepted",
        data.line5_total_donations_accepted,
        false,
    );
    d.amount(
        "6. Total donations refunded",
        data.line6_total_donations_refunded,
        false,
    );
    d.amount("7. Net donations", data.line7_net_donations, true);
    d.blank();
}
