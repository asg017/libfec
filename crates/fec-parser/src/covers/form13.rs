//! Form 13: Report of Donations Accepted for Inaugural Committee.

use crate::covers::fields::{amount, date, flag, text, text_or_empty, Data};
use crate::covers::{Address, PersonName};
use jiff::civil::Date;
use serde::Serialize;

/// FEC Form 13, "Report of Donations Accepted for Inaugural Committee" — the
/// `F13N` / `F13A` cover record
/// ([fecfrm13.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13.pdf#page=1)).
///
/// **Who files.** An inaugural committee — "the committee appointed by the
/// President-elect to be in charge of the Presidential inaugural ceremony and
/// functions … and that has filed the letter required by 11 CFR 104.21(b)" —
/// through "the chairperson or other officer identified in the inaugural
/// committee's letter-filing"
/// ([fecfrm13i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=1)).
///
/// **When.** By the 90th day after the inaugural ceremony (report code
/// `90D`); any supplement (`90S`) by the 90th day after the previous filing
/// ([fecfrm13i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=1)).
///
/// **Totals are cumulative.** Lines 5–7 are headed "Cumulative Total (From
/// Committee's Inception)"
/// ([fecfrm13.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13.pdf#page=1)),
/// so on a `90S` supplement they cover every Schedule 13-A/13-B filed so far,
/// not just this filing's `F132`/`F133` rows
/// ([fecfrm13i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=1)).
///
/// **Versions.** All layouts (v5.2 onwards, and the paper `P3.x` layouts) use
/// the same column names; v5.2–5.3 only order `change_of_address` after the
/// address.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "gleam", derive(fec_parser_macros::GleamType))]
pub struct Form13 {
    /// Form type as filed, e.g. `F13N`: the base form plus the
    /// amendment-indicator suffix (see [`crate::covers::base_form_type`]).
    /// Column `form_type` (FEC format workbook v8.4, sheet `F13`, field 1).
    pub form_type: String,
    /// Line 1, name of the inaugural committee (`committee_name`)
    /// ([fecfrm13.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13.pdf#page=1)).
    pub committee_name: String,
    /// Line 2, FEC identification number, "provided by the Commission in
    /// response to the letter-filing required by 11 CFR 104.21(b)"
    /// (`filer_committee_id_number`)
    /// ([fecfrm13i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=1)).
    pub filer_committee_id: String,
    /// Line 1, mailing address (`street_1`, `street_2`, `city`, `state`,
    /// `zip_code`)
    /// ([fecfrm13.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13.pdf#page=1)).
    pub address: Address,
    /// The "Check if different than previously reported" box beside the
    /// address (`change_of_address`; `X` = yes)
    /// (FEC format workbook v8.4, sheet `F13`, field 4).
    pub change_of_address: bool,
    /// Line 3a, type of filing (`report_code`): `90D` for the first report,
    /// `90S` for a supplement. See [`Form13::report_code_label`].
    pub report_code: Option<String>,
    /// Line 3b, on an amendment, "the filing date of the report or supplement
    /// to which the amendment relates" (`amendment_date`); required when the
    /// form type is `F13A`
    /// ([fecfrm13i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=1);
    /// FEC format workbook v8.4, sheet `F13`, field 11).
    pub original_amendment_date: Option<Date>,
    /// Line 4, first date of financial activity covered
    /// (`coverage_from_date`). A report starts with the date of appointment by
    /// the President-elect; a supplement, the day after the previous closing
    /// date; an amendment repeats the original filing's period
    /// ([fecfrm13i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=1)).
    pub coverage_from_date: Option<Date>,
    /// Line 4, closing date of the report — "a date 15 days or less from the
    /// date the report is filed" (`coverage_through_date`)
    /// ([fecfrm13i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=1)).
    pub coverage_through_date: Option<Date>,
    /// Line 5, total donations accepted: "the total donations itemized on all
    /// Schedules 13-A filed since the committee's inception"
    /// (`total_donations_accepted`)
    /// ([fecfrm13i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=1)).
    pub line5_total_donations_accepted: f64,
    /// Line 6, total donations refunded: the sum of refunds itemized on all
    /// Schedules 13-B since the committee's inception
    /// (`total_donations_refunded`)
    /// ([fecfrm13i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=1)).
    pub line6_total_donations_refunded: f64,
    /// Line 7, net donations: Line 5 − Line 6 (`net_donations`)
    /// ([fecfrm13i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=1);
    /// FEC format workbook v8.4, sheet `F13`, field 16).
    pub line7_net_donations: f64,
    /// The officer designated in the letter-filing to sign this report
    /// (`designated_last_name`, `designated_first_name`,
    /// `designated_middle_name`, `designated_prefix`, `designated_suffix`).
    /// The paper form asks for a title too, but the record has no title
    /// column
    /// ([fecfrm13i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=1);
    /// FEC format workbook v8.4, sheet `F13`, fields 17–21).
    pub designated_officer: PersonName,
    /// Date signed (`date_signed`).
    pub date_signed: Option<Date>,
}

impl Form13 {
    /// Build from a cover record's column map. `None` only if the record has
    /// no `committee_name` column at all.
    pub fn from_data(data: &Data) -> Option<Self> {
        data.get("committee_name")?;
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            committee_name: text_or_empty(data, "committee_name"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            address: Address::from_prefixed(data, ""),
            change_of_address: flag(data, "change_of_address"),
            report_code: text(data, "report_code"),
            original_amendment_date: date(data, "amendment_date"),
            coverage_from_date: date(data, "coverage_from_date"),
            coverage_through_date: date(data, "coverage_through_date"),
            line5_total_donations_accepted: amount(data, "total_donations_accepted"),
            line6_total_donations_refunded: amount(data, "total_donations_refunded"),
            line7_net_donations: amount(data, "net_donations"),
            designated_officer: PersonName::from_prefixed(data, "designated_"),
            date_signed: date(data, "date_signed"),
        })
    }

    /// True for an amended report (`F13A`); see [`Form13::form_type`].
    pub fn is_amendment(&self) -> bool {
        crate::covers::is_amendment_form_type(&self.form_type)
    }

    /// The FEC's description of [`Form13::report_code`]: "90D = 90 Day Post
    /// Presidential Inauguration Report", "90S = Supplement to 90 Day Post
    /// Presidential Inauguration" (FEC format workbook v8.4, sheet `F13`,
    /// comments on field 10).
    pub fn report_code_label(&self) -> Option<&'static str> {
        Some(match self.report_code.as_deref()? {
            "90D" => "90 Day Post Presidential Inauguration Report",
            "90S" => "Supplement to 90 Day Post Presidential Inauguration",
            _ => return None,
        })
    }
}
