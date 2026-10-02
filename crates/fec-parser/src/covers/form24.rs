//! Form 24: 24/48-hour notice of independent expenditures.

use crate::covers::fields::{date, text, text_or_empty, Data};
use crate::covers::{Address, PersonName};
use jiff::civil::Date;

/// "FORM 24 - 24 / 48 HOUR NOTICE OF INDEPENDENT EXPENDITURE": the cover
/// record of a 24- or 48-hour report of independent expenditures (form types
/// `F24N` new, `F24A` amendment).
///
/// **There is no paper Form 24.** No blank form or instructions for it are in
/// the FEC document set, and no FEC PDF names "Form 24"; everything here comes
/// from the e-filing format workbook (FEC format workbook v8.4, sheet F24,
/// fields 1-16). The record is thin: who filed (committee name, address,
/// treasurer), whether this is a 24- or 48-hour report, the original filing
/// date when it is an amendment, and the signing date. It carries **no dollar
/// amounts**.
///
/// What the two report types mean, per the Schedule E instructions (revised
/// 05/2016): a political committee files a 48-hour report when independent
/// expenditures regarding an election aggregate $10,000 or more "up to and
/// including the 20th day before an election", and a 24-hour report when they
/// aggregate $1,000 or more "after the 20th day, but more than 24 hours before
/// 12:01 A.M. of the day of the election"; each report "must include all of
/// the information required on Schedule E"
/// ([fecfrm3xei.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xei.pdf#page=1)).
/// The same expenditures are reported again on Schedule E of the committee's
/// next regular report
/// ([nongui.pdf p82](https://www.fec.gov/resources/cms-content/documents/policy-guidance/nongui.pdf#page=82)),
/// so adding 24/48-hour amounts to regular-report amounts double-counts.
///
/// Persons other than political committees report the same kind of
/// expenditures on [`crate::covers::Form5`]
/// ([fecfrm5i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=1)).
///
/// # Versions
///
/// v8.0–8.5 carry all 16 fields. v6.1–7.0 lack `original_amendment_date`.
/// v3 and v5.x have no treasurer name columns (v5.2's single caret-delimited
/// "NAME/TREASURER (as signed)" field is unnamed in `fec-parser`'s mapping), so
/// [`Form24::treasurer`] is empty there; v3 also lacks `report_type`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Form24 {
    /// Form type as filed, e.g. `F24N`: the base form plus the
    /// amendment-indicator suffix (see [`crate::covers::base_form_type`]).
    /// Column `form_type` (FEC format workbook v8.4, sheet `F24`, field 1;
    /// values `F24+[N|A]`).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (FEC format workbook v8.4, sheet F24, field 2).
    pub filer_committee_id: String,
    /// Whether this is a 24-hour or 48-hour report: the code `24` or `48`.
    /// Required; a missing or wrongly coded value is an error. Column
    /// `report_type` (FEC format workbook v8.4, sheet F24, field 3,
    /// "REPORT TYPE {24/48 Hour}"). See [`Form24::report_type_label`].
    pub report_type: Option<String>,
    /// On an amendment (`F24A`), the date of the report being amended. The
    /// v8.4 workbook says "Use date of original report or of most recent
    /// amendment"; the v8.0 workbook said "Use the POSTed Filed on Date of
    /// Original (1st) F24 being amended". Required when the form type is
    /// `F24A`. Column `original_amendment_date` (FEC format workbook v8.4,
    /// sheet F24, field 4).
    pub original_amendment_date: Option<Date>,
    /// The filing committee's name. Column `committee_name` (FEC format
    /// workbook v8.4, sheet F24, field 5).
    pub committee_name: String,
    /// The committee's mailing address. Columns `street_1`, `street_2`,
    /// `city`, `state`, `zip_code` (FEC format workbook v8.4, sheet F24,
    /// fields 6-10).
    pub address: Address,
    /// The committee treasurer, the only person named on the record. Columns
    /// `treasurer_last_name`, `treasurer_first_name`, `treasurer_middle_name`,
    /// `treasurer_prefix`, `treasurer_suffix` (FEC format workbook v8.4,
    /// sheet F24, fields 11-15). Empty on v3/v5 filings.
    pub treasurer: PersonName,
    /// Date the report was signed. Column `date_signed` (FEC format workbook
    /// v8.4, sheet F24, field 16).
    pub date_signed: Option<Date>,
}

impl Form24 {
    pub fn from_data(data: &Data) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            report_type: text(data, "report_type"),
            original_amendment_date: date(data, "original_amendment_date"),
            committee_name: text_or_empty(data, "committee_name"),
            address: Address::from_prefixed(data, ""),
            treasurer: PersonName::from_data(data),
            date_signed: date(data, "date_signed"),
        })
    }

    /// True for an amended report (`F24A`); see [`Form24::form_type`].
    pub fn is_amendment(&self) -> bool {
        crate::covers::is_amendment_form_type(&self.form_type)
    }

    /// "24-Hour Report" or "48-Hour Report" for the codes `24` / `48`
    /// (FEC format workbook v8.4, sheet F24, field 3, values `24, 48`; the
    /// report names as the Schedule E instructions use them,
    /// [fecfrm3xei.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xei.pdf#page=1)).
    /// `None` for any other value.
    pub fn report_type_label(&self) -> Option<&'static str> {
        report_type_label(self.report_type.as_deref()?)
    }
}

/// Label for a 24/48-hour code (`24` / `48`) as used on Forms 24 and 5.
pub(crate) fn report_type_label(code: &str) -> Option<&'static str> {
    match code.trim() {
        "24" => Some("24-Hour Report"),
        "48" => Some("48-Hour Report"),
        _ => None,
    }
}
