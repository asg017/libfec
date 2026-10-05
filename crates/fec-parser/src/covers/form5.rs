//! Form 5: report of independent expenditures made and contributions received,
//! by persons other than political committees.

use crate::covers::fields::{amount, date, flag, person_name_or_legacy, text, text_or_empty, Data};
use crate::covers::{Address, PersonName};
use jiff::civil::Date;

/// "FEC FORM 5 - REPORT OF INDEPENDENT EXPENDITURES MADE AND CONTRIBUTIONS
/// RECEIVED", printed with the subtitle "To Be Used by Persons (Other than
/// Political Committees)"
/// ([fecfrm5.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5.pdf#page=1)).
/// Form types `F5N` (new) and `F5A` (amendment).
///
/// **Who files.** "Every person, group of persons or organization, other than
/// a political committee, that makes or contracts to make independent
/// expenditures aggregating in excess of $250 with respect to a given election
/// in a calendar year"; political committees use Form 3X Schedule E instead
/// (instructions revised 09/2013,
/// [fecfrm5i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=1)).
///
/// **Which report.** One record covers both kinds of Form 5 report (Line 4(a)):
/// quarterly/year-end reports, identified by [`Form5::report_code`], and 24- or
/// 48-hour reports, identified by [`Form5::report_type`]. The workbook rule is
/// "Either Report Code or 24/48-Hour Code is required" (FEC format workbook
/// v8.4, sheet F5, fields 18-19). A 48-hour report is due when independent
/// expenditures aggregate $10,000 or more with respect to an election "up to
/// and including the 20th day before an election"; a 24-hour report when they
/// aggregate $1,000 or more "after the twentieth day but more than 24 hours
/// before 12:01a.m. of the day of the election"
/// ([fecfrm5i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=1),
/// [p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=2)).
///
/// **Totals.** The record carries only the page-1 totals, Line 6 (total
/// contributions) and Line 7 (total independent expenditures); there is no
/// two-column summary. Itemized contributions and expenditures are on
/// Schedules 5-A and 5-E, whose totals are carried to Lines 6 and 7
/// ([fecfrm5.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5.pdf#page=2),
/// [p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5.pdf#page=3)).
///
/// # Versions
///
/// v8.1–8.5 is the layout documented here. v6.1–8.0 also carry
/// `qualified_nonprofit` and lack `original_amendment_date`. v3 and v5.x name
/// the filer in a single `committee_name` column, carry the election
/// (`report_pgi`, `election_date`, `election_state`), and give the person
/// completing the form as one caret-delimited name (`person_completing_name`);
/// all are read here where present (FEC format workbook v5.2, sheet F5).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "gleam", derive(fec_parser_macros::GleamType))]
pub struct Form5 {
    /// Form type as filed, e.g. `F5N`: the base form plus the
    /// amendment-indicator suffix (see [`crate::covers::base_form_type`]).
    /// Column `form_type` (FEC format workbook v8.4, sheet `F5`, field 1),
    /// which allows `F5N`, `F5A` and `F5T`.
    pub form_type: String,
    /// The filer's FEC identification number, Line 3 ("First time filers—leave
    /// this line blank",
    /// [fecfrm5i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=2)).
    /// Column `filer_committee_id_number` (FEC format workbook v8.4, sheet F5,
    /// field 2).
    pub filer_committee_id: String,
    /// Electronic-format code for the kind of filer: `IND` or `ORG` (no box on
    /// the paper form). The workbook ties occupation/employer to "If Entity =
    /// IND", and the paper form asks for those "for Individual Filers Only".
    /// Column `entity_type` (FEC format workbook v8.4, sheet F5, field 3).
    pub entity_type: Option<String>,
    /// Line 1(a), name of the organization or corporation filing. Required "If
    /// not Individual". Column `organization_name` (FEC format workbook v8.4,
    /// sheet F5, field 4); `committee_name` in v3/v5.x.
    pub organization_name: Option<String>,
    /// Line 1(a), name of the individual filing. Required "If not
    /// Organization". Columns `individual_last_name`, `individual_first_name`,
    /// `individual_middle_name`, `individual_prefix`, `individual_suffix` (FEC
    /// format workbook v8.4, sheet F5, fields 5-9).
    pub individual: PersonName,
    /// Line 1(b) "check if different than previously reported": the filer's
    /// address changed. Column `change_of_address` (FEC format workbook v8.4,
    /// sheet F5, field 10;
    /// [fecfrm5.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5.pdf#page=1)).
    pub change_of_address: bool,
    /// Lines 1(b)-(c), the filer's mailing address. Columns `street_1`,
    /// `street_2`, `city`, `state`, `zip_code` (FEC format workbook v8.4,
    /// sheet F5, fields 11-15).
    pub address: Address,
    /// Line 2, occupation, "for Individual Filers Only" ("the principal job
    /// title or position of an individual",
    /// [fecfrm5i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=1)).
    /// Column `individual_occupation` (FEC format workbook v8.4, sheet F5,
    /// field 16).
    pub individual_occupation: Option<String>,
    /// Line 2, name of employer, for individual filers. Column
    /// `individual_employer` (FEC format workbook v8.4, sheet F5, field 17).
    pub individual_employer: Option<String>,
    /// v6.1–8.0 and v3/v5.x only: "YES/NO (Qualified Non-Profit
    /// Corporation)", `Y` or `N` (FEC format workbook v8.0, sheet F5,
    /// field 16). Column `qualified_nonprofit`. Absent from v8.1+.
    pub qualified_nonprofit: Option<String>,
    /// Line 4(a), quarterly report code: `Q1`, `Q2`, `Q3`, `Q4` or `YE`.
    /// Blank on 24/48-hour reports. Column `report_code` (FEC format workbook
    /// v8.4, sheet F5, field 18). See [`Form5::report_code_label`].
    pub report_code: Option<String>,
    /// Line 4(a), 24/48-hour code: `24` or `48`. Blank on quarterly reports
    /// (the workbook flags it as "not needed" when a report code is present).
    /// Column `report_type` (FEC format workbook v8.4, sheet F5, field 19,
    /// "24HOUR 48HOUR CODE"). See [`Form5::report_type_label`].
    pub report_type: Option<String>,
    /// Line 4(b), on an amendment the date of the report it amends ("Use date
    /// of original report or of most recent amendment"). Column
    /// `original_amendment_date` (FEC format workbook v8.4, sheet F5,
    /// field 20). v8.1+ only.
    pub original_amendment_date: Option<Date>,
    /// Line 5, covering period start. "Coverage dates are not required on 24-
    /// and 48-hour reports"; filers that enter them anyway "may use the first
    /// and last dates on which the communication airs"
    /// ([fecfrm5i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=2)).
    /// Column `coverage_from_date` (FEC format workbook v8.4, sheet F5,
    /// field 21).
    pub coverage_from_date: Option<Date>,
    /// Line 5, covering period end. Column `coverage_through_date` (FEC format
    /// workbook v8.4, sheet F5, field 22).
    pub coverage_through_date: Option<Date>,
    /// v3/v5.x only: the workbook's "RPTPGI" election code, a letter plus
    /// year (values `C,G,P,R,S[CCYY]`, sample `P2006`), required when the
    /// report code is a pre-election one. Column `report_pgi` (FEC format
    /// workbook v5.2, sheet F5, field 14), or `election_code` in paper (`P1`)
    /// layouts.
    pub election_code: Option<String>,
    /// v3/v5.x only: "DATE (Of Election)". Column `election_date` (FEC format
    /// workbook v5.2, sheet F5, field 15).
    pub election_date: Option<Date>,
    /// v3/v5.x only: "STATE (Of Election)". Column `election_state` (FEC
    /// format workbook v5.2, sheet F5, field 16).
    pub election_state: Option<String>,
    /// Line 6, total contributions received during the reporting period,
    /// "including contributions of $200 or less that were not itemized on
    /// Schedule 5-A"
    /// ([fecfrm5i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=2)).
    /// Column `total_contribution` (FEC format workbook v8.4, sheet F5,
    /// field 23).
    pub total_contributions: f64,
    /// Line 7, "the total amount of independent expenditures made during this
    /// reporting period"
    /// ([fecfrm5i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=2)).
    /// Column `total_independent_expenditure` (FEC format workbook v8.4,
    /// sheet F5, field 24).
    pub total_independent_expenditures: f64,
    /// "TYPE OR PRINT NAME OF PERSON COMPLETING FORM": the person who signs
    /// the certification that the expenditures were not coordinated with any
    /// candidate, committee or party
    /// ([fecfrm5.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5.pdf#page=1)).
    /// Columns `person_completing_last_name`, `_first_name`, `_middle_name`,
    /// `_prefix`, `_suffix` (FEC format workbook v8.4, sheet F5, fields
    /// 25-29); the caret-delimited `person_completing_name` in v3/v5.x.
    pub person_completing: PersonName,
    /// Date signed. Column `date_signed` (FEC format workbook v8.4, sheet F5,
    /// field 30).
    pub date_signed: Option<Date>,
}

impl Form5 {
    pub fn from_data(data: &Data) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            entity_type: text(data, "entity_type"),
            organization_name: text(data, "organization_name")
                .or_else(|| text(data, "committee_name")),
            individual: PersonName::from_prefixed(data, "individual_"),
            change_of_address: flag(data, "change_of_address"),
            address: Address::from_prefixed(data, ""),
            individual_occupation: text(data, "individual_occupation"),
            individual_employer: text(data, "individual_employer"),
            qualified_nonprofit: text(data, "qualified_nonprofit"),
            report_code: text(data, "report_code"),
            report_type: text(data, "report_type"),
            original_amendment_date: date(data, "original_amendment_date"),
            coverage_from_date: date(data, "coverage_from_date"),
            coverage_through_date: date(data, "coverage_through_date"),
            election_code: text(data, "report_pgi").or_else(|| text(data, "election_code")),
            election_date: date(data, "election_date"),
            election_state: text(data, "election_state"),
            total_contributions: amount(data, "total_contribution"),
            total_independent_expenditures: amount(data, "total_independent_expenditure"),
            person_completing: person_name_or_legacy(
                data,
                "person_completing_",
                "person_completing_name",
            ),
            date_signed: date(data, "date_signed"),
        })
    }

    /// True for an amended report (`F5A`); see [`Form5::form_type`].
    pub fn is_amendment(&self) -> bool {
        crate::covers::is_amendment_form_type(&self.form_type)
    }

    /// True when the filer is an individual (`entity_type` `IND`).
    pub fn is_individual(&self) -> bool {
        self.entity_type
            .as_deref()
            .is_some_and(|e| e.eq_ignore_ascii_case("IND"))
    }

    /// The filer's name as printed on Line 1(a): the organization name, or the
    /// individual's name when there is none.
    pub fn filer_name(&self) -> String {
        match &self.organization_name {
            Some(name) if !self.is_individual() || self.individual.is_empty() => name.clone(),
            _ => self.individual.to_string(),
        }
    }

    /// The Line 4(a) box for a quarterly report code. The 09/2013 form prints
    /// four boxes: "April 15 Quarterly Report", "July 15 Quarterly Report",
    /// "October 15 Quarterly Report" and "January 31 Year-End Report"
    /// ([fecfrm5.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5.pdf#page=1));
    /// the workbook codes are `Q1, Q2, Q3, Q4, YE` (FEC format workbook v8.4,
    /// sheet F5, field 18). The workbook does not itself pair codes with
    /// boxes; the pairing here follows their order. `Q4` has no printed box
    /// and returns `None`, as does any other value.
    pub fn report_code_label(&self) -> Option<&'static str> {
        match self.report_code.as_deref()?.trim() {
            "Q1" => Some("April 15 Quarterly Report"),
            "Q2" => Some("July 15 Quarterly Report"),
            "Q3" => Some("October 15 Quarterly Report"),
            "YE" => Some("January 31 Year-End Report"),
            _ => None,
        }
    }

    /// "24-Hour Report" or "48-Hour Report", the Line 4(a) box labels
    /// ([fecfrm5.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5.pdf#page=1))
    /// for the codes `24` / `48` (FEC format workbook v8.4, sheet F5,
    /// field 19). `None` for any other value.
    pub fn report_type_label(&self) -> Option<&'static str> {
        crate::covers::form24::report_type_label(self.report_type.as_deref()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_caret_names() {
        let data: Data = [("person_completing_name", "Smith^Pat T.^Mr.^Jr.")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let name = person_name_or_legacy(&data, "person_completing_", "person_completing_name");
        assert_eq!(name.to_string(), "Mr. Pat T. Smith Jr.");

        let data: Data = [("person_completing_name", "JANE DOE")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let name = person_name_or_legacy(&data, "person_completing_", "person_completing_name");
        assert_eq!(name.last_name, "JANE DOE");

        let data: Data = [("custodian_last_name", "Doe^Jane^^")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let name = crate::covers::fields::person_name(&data, "custodian_");
        assert_eq!(name.to_string(), "Jane Doe");
    }
}
