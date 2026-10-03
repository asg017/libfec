//! Form 7: Report of Communication Costs by Corporations and Membership
//! Organizations.

use crate::covers::fields::{amount, date, person_name_or_legacy, text, text_or_empty, Data};
use crate::covers::{Address, PersonName};
use jiff::civil::Date;
use serde::Serialize;

/// FEC Form 7, "Report of Communication Costs by Corporations and Membership
/// Organizations" — the `F7N` / `F7A` cover record
/// ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1)).
///
/// **What it is.** A corporation may communicate with its stockholders and
/// executive or administrative personnel, and a labor organization with its
/// members, on any subject, including express advocacy for or against a
/// federal candidate; the costs of such communications must be reported "if
/// those costs exceed $2,000 per election" (instructions revised 2/2001)
/// ([fecfrm7i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7i.pdf#page=1)).
/// This record is the cover page; each communication is an `F76` row (FEC
/// format workbook v8.4, sheets `F7`, `F76`).
///
/// **Who files.** "Every membership organization (including a labor
/// organization) or corporation" that makes such disbursements
/// ([fecfrm7i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7i.pdf#page=1)).
/// The filer is an organization, not a political committee, though the ID
/// column is still `filer_committee_id_number` (and the field is
/// [`Form7::filer_committee_id`], as on every other cover).
///
/// **When.** In a calendar year with a regularly scheduled general election:
/// quarterly (April 15, July 15, October 15, January 31) plus a 12 Day
/// Pre-General Election Report, starting with the first period in which costs
/// exceed $2,000 per election
/// ([fecfrm7i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7i.pdf#page=2)).
///
/// **Versions.** v6.1–v8.5 share one layout. The legacy v3/v5 layout has a
/// single caret-delimited `person_designated_name` column, split into the
/// parts of [`Form7::person_designated`].
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Form7 {
    /// Form type as filed, e.g. `F7N`: the base form plus the
    /// amendment-indicator suffix (see [`crate::covers::base_form_type`]).
    /// Column `form_type` (FEC format workbook v8.4, sheet `F7`, field 1).
    /// The suffix answers Line 4(b), "Is this Report an Amendment?"
    /// ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1)).
    pub form_type: String,
    /// Line 1(a), name of the organization (`organization_name`)
    /// ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1)).
    pub organization_name: String,
    /// Line 2, identification number assigned by the FEC
    /// (`filer_committee_id_number`)
    /// ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1)).
    pub filer_committee_id: String,
    /// Line 1(b)–(c), the organization's address (`street_1`, `street_2`,
    /// `city`, `state`, `zip_code`)
    /// ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1)).
    pub address: Address,
    /// Line 3, type of organization (`organization_type`): one of `C`, `T`,
    /// `L`, `M`, `V`, `W`. See [`Form7::organization_type_label`].
    pub organization_type: Option<String>,
    /// Line 4(a), type of report (`report_code`). See
    /// [`Form7::report_code_label`].
    pub report_code: Option<String>,
    /// Line 4(a), date of the general election a 12 Day Pre-General Election
    /// Report is for (`election_date`)
    /// ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1)).
    pub election_date: Option<Date>,
    /// Line 4(a), "in the State of" — the state of that election
    /// (`election_state`)
    /// ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1)).
    pub election_state: Option<String>,
    /// Line 5, first day of the period covered (`coverage_from_date`)
    /// ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1)).
    pub coverage_from_date: Option<Date>,
    /// Line 5, last day of the period covered (`coverage_through_date`).
    pub coverage_through_date: Option<Date>,
    /// "Total Communication Costs for This Period" (`total_costs`), defined as
    /// "= Sum of F76 Itemized Costs" (FEC format workbook v8.4, sheet `F7`,
    /// field 15)
    /// ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1)).
    pub total_communication_costs: f64,
    /// The person designated to sign the report
    /// (`person_designated_last_name`, `person_designated_first_name`,
    /// `person_designated_middle_name`, `person_designated_prefix`,
    /// `person_designated_suffix`; legacy `person_designated_name`)
    /// ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1)).
    pub person_designated: PersonName,
    /// That person's title, from "Signature and Title of Person Designated to
    /// Sign This Report" (`person_designated_title`)
    /// ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1)).
    pub person_designated_title: Option<String>,
    /// Date signed (`date_signed`).
    pub date_signed: Option<Date>,
}

impl Form7 {
    /// Build from a cover record's column map. `None` only if the record has
    /// no `organization_name` column at all.
    pub fn from_data(data: &Data) -> Option<Self> {
        data.get("organization_name")?;

        let person_designated =
            person_name_or_legacy(data, "person_designated_", "person_designated_name");

        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            organization_name: text_or_empty(data, "organization_name"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            address: Address::from_prefixed(data, ""),
            organization_type: text(data, "organization_type"),
            report_code: text(data, "report_code"),
            election_date: date(data, "election_date"),
            election_state: text(data, "election_state"),
            coverage_from_date: date(data, "coverage_from_date"),
            coverage_through_date: date(data, "coverage_through_date"),
            total_communication_costs: amount(data, "total_costs"),
            person_designated,
            person_designated_title: text(data, "person_designated_title"),
            date_signed: date(data, "date_signed"),
        })
    }

    /// True for an amended report (`F7A`); see [`Form7::form_type`].
    pub fn is_amendment(&self) -> bool {
        crate::covers::is_amendment_form_type(&self.form_type)
    }

    /// The FEC's description of [`Form7::organization_type`]: "C - Corporation
    /// T - Trade Association L - Labor Organization M - Membership
    /// Organization V - Cooperative W - Corporation w/o capital stock" (FEC
    /// format workbook v8.4, sheet `F7`, field 9), the same six boxes as Line 3
    /// of the form
    /// ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1)).
    pub fn organization_type_label(&self) -> Option<&'static str> {
        Some(match self.organization_type.as_deref()? {
            "C" => "Corporation",
            "T" => "Trade Association",
            "L" => "Labor Organization",
            "M" => "Membership Organization",
            "V" => "Cooperative",
            "W" => "Corporation w/o capital stock",
            _ => return None,
        })
    }

    /// The FEC's description of [`Form7::report_code`], for the codes Form 7
    /// accepts: `Q1; Q2; Q3; YE; 12G` (FEC e-filing specifications v8.4, p15,
    /// "Accepted Report Codes by Type of Filing"). Labels are the Line 4(a)
    /// boxes printed on the form
    /// ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1)).
    pub fn report_code_label(&self) -> Option<&'static str> {
        Some(match self.report_code.as_deref()? {
            "Q1" => "April 15 Quarterly Report",
            "Q2" => "July 15 Quarterly Report",
            "Q3" => "October 15 Quarterly Report",
            "YE" => "January 31 Year End Report",
            "12G" => "12 Day Pre-General Election Report",
            _ => return None,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn legacy_person_designated_name() {
        let d: Data = [
            ("organization_name", "ACME"),
            ("person_designated_name", "DOE^JANE"),
            ("organization_type", "W"),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        let form = Form7::from_data(&d).unwrap();
        assert_eq!(form.person_designated.to_string(), "JANE DOE");
        assert_eq!(
            form.organization_type_label(),
            Some("Corporation w/o capital stock")
        );
    }
}
