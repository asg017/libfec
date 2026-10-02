//! FEC Form 1M, Notification of Multicandidate Status (`F1MN` / `F1MA`
//! cover records).
//!
//! Sources used throughout this module:
//!
//! - the blank form, [fecfrm1m.pdf](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1m.pdf#page=1)
//!   one page;
//! - its instructions, [fecfrm1mi.pdf](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1mi.pdf#page=1)
//!   (Revised 01/2004), one page;
//! - the FEC e-filing format workbook v8.4, sheet `F1M`, whose field numbers
//!   (1–71) are cited as "field N".

use crate::covers::fields::{date, person_name_or_legacy, text, text_or_empty, Data};
use crate::covers::{Address, PersonName};
use jiff::civil::Date;

/// FEC Form 1M, **Notification of Multicandidate Status**.
///
/// The form "discloses supplemental information that verifies the date on
/// which your committee became a multicandidate committee". To qualify, a
/// political committee must (a) be registered for at least 6 months, (b)
/// receive contributions from more than 50 persons, and (c) make
/// contributions to at least 5 Federal candidates — (c) does not apply to
/// State party committees. It is filed within ten days after satisfying the
/// three requirements
/// ([fecfrm1mi.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1mi.pdf#page=1)).
///
/// The treasurer certifies that one of two situations is correct and
/// completes Line 4 *or* Line 5
/// ([fecfrm1m.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1m.pdf#page=1)):
///
/// - **Line 4, status by affiliation** ([`Form1M::affiliation`]): the
///   committee qualified through its affiliation with an existing
///   multicandidate committee.
/// - **Line 5, status by qualification** ([`Form1M::qualification`]): five
///   candidates it contributed to, plus the 51st-contributor, registration
///   and qualification dates.
///
/// The workbook makes each group all-or-nothing ("Req if any Affil fields
/// used", "Req if any 51st Contrib fields used"; FEC format workbook v8.4,
/// sheet `F1M`, fields 10–12 and 63–65), so normally one of the two is
/// `None`. Real filings occasionally populate both; both are kept.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct Form1M {
    /// Form type as filed, e.g. `F1MN`: the base form plus the
    /// amendment-indicator suffix (see [`crate::covers::base_form_type`]).
    /// Column `form_type` (FEC format workbook v8.4, sheet `F1M`, field 1).
    pub form_type: String,
    /// Line 1(a) "Name of Committee in Full" (`committee_name`, field 3).
    pub committee_name: String,
    /// Line 1(b)–(c) mailing address (`street_1`, `street_2`, `city`, `state`,
    /// `zip_code`, fields 4–8).
    pub address: Address,
    /// Line 2 FEC identification number (`filer_committee_id_number`, field 2).
    pub filer_committee_id: String,
    /// Line 3 "Type of Committee (check one)": `X` = State party, `N` = other
    /// (`committee_type`, field 9). This is *not* the Form 1 committee type.
    /// See [`Form1M::committee_type_label`].
    pub committee_type: Option<String>,
    /// Line 4, status by affiliation (fields 10–12). `None` when all three
    /// columns are blank.
    pub affiliation: Option<Form1MAffiliation>,
    /// Line 5, status by qualification (fields 13–65). `None` when every
    /// candidate column and all three dates are blank.
    pub qualification: Option<Form1MQualification>,
    /// "Type or Print Name of Treasurer" (`treasurer_*`, fields 66–70; the
    /// single caret-delimited `treasurer_name` column of pre-v6 formats is
    /// split into parts).
    pub treasurer: PersonName,
    /// Date next to the treasurer's signature (`date_signed`, field 71).
    pub date_signed: Option<Date>,
}

/// Line 4, **status by affiliation**: "The committee submitted its Statement
/// of Organization (FEC FORM 1) on ___ and simultaneously qualified as a
/// multicandidate committee through its affiliation with: Committee Name: ___
/// FEC Identification Number: ___"
/// ([fecfrm1m.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1m.pdf#page=1)).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Form1MAffiliation {
    /// Date the committee's Form 1 was submitted
    /// (`affiliated_date_f1_filed`, field 10).
    pub date_form1_filed: Option<Date>,
    /// FEC ID of the affiliated multicandidate committee
    /// (`affiliated_committee_id_number`, field 11).
    pub committee_id: Option<String>,
    /// Name of the affiliated multicandidate committee
    /// (`affiliated_committee_name`, field 12).
    pub committee_name: Option<String>,
}

/// Line 5, **status by qualification**
/// ([fecfrm1m.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1m.pdf#page=1),
/// [fecfrm1mi.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1mi.pdf#page=1)).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Form1MQualification {
    /// Line 5(a) "Candidates: The committee has made contributions to the five
    /// (5) federal candidates listed below (ONLY State party committees may
    /// leave this blank.)": rows (i)–(v), from the `first_candidate_*` …
    /// `fifth_candidate_*` column groups (fields 13–62). Blank rows are
    /// omitted, so this has 0–5 entries in form order.
    pub candidates: Vec<Form1MCandidate>,
    /// Line 5(b) "Contributors: The committee received a contribution from its
    /// 51st contributor on" (`fifty_first_contributor_date`, field 63).
    pub fifty_first_contributor_date: Option<Date>,
    /// Line 5(c) "Registration: The committee has been registered for at least
    /// 6 months. FEC FORM 1 was submitted on" (`original_registration_date`,
    /// field 64).
    pub original_registration_date: Option<Date>,
    /// Line 5(d) "Qualification: The committee met the above requirements on"
    /// (`requirements_met_date`, field 65): the date it "satisfied its final
    /// requirement for multicandidate status"
    /// ([fecfrm1mi.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1mi.pdf#page=1)).
    pub requirements_met_date: Option<Date>,
}

/// One row (i)–(v) of the Line 5(a) candidate table: "Name", "Office
/// Sought", "State/District", "Date"
/// ([fecfrm1m.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1m.pdf#page=1)).
/// Column names below use `first_` for row (i); rows (ii)–(v) use `second_`
/// … `fifth_` (fields 13–22, 23–32, 33–42, 43–52, 53–62).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Form1MCandidate {
    /// Candidate's FEC ID (`first_candidate_id_number`, field 13). The paper
    /// table has no ID column. In practice some filers enter the candidate's
    /// committee ID (`C…`) here; it is kept as filed.
    pub candidate_id: Option<String>,
    /// "Name" (`first_candidate_last_name` … `first_candidate_suffix`,
    /// fields 14–18; the caret-delimited `first_candidate_name` of pre-v6
    /// formats is split into parts). Some filers enter a committee name here.
    pub name: PersonName,
    /// "Office Sought": `H`, `S` or `P` (`first_candidate_office`, field 19).
    /// See [`Form1MCandidate::office_label`].
    pub office: Option<String>,
    /// Candidate's state, for House and Senate (`first_candidate_state`,
    /// field 20).
    pub state: Option<String>,
    /// Candidate's district, for House (`first_candidate_district`, field 21).
    pub district: Option<String>,
    /// "Date" of the committee's contribution to this candidate
    /// (`first_candidate_contribution_date`, field 22).
    pub contribution_date: Option<Date>,
}

impl Form1MCandidate {
    fn from_prefixed(data: &Data, prefix: &str) -> Option<Self> {
        let c = Self {
            candidate_id: text(data, &format!("{prefix}id_number")),
            name: person_name_or_legacy(data, prefix, &format!("{prefix}name")),
            office: text(data, &format!("{prefix}office")),
            state: text(data, &format!("{prefix}state")),
            district: text(data, &format!("{prefix}district")),
            contribution_date: date(data, &format!("{prefix}contribution_date")),
        };
        let blank = c.candidate_id.is_none()
            && c.name.is_empty()
            && c.office.is_none()
            && c.state.is_none()
            && c.district.is_none()
            && c.contribution_date.is_none();
        (!blank).then_some(c)
    }

    /// `H` House, `S` Senate, `P` President: the workbook's `H,S,P` codes
    /// (FEC format workbook v8.4, sheet `F1M`, field 19), named as on the Form
    /// 1 "Office Sought" boxes; see [`crate::covers::Form1Candidate::office_label`].
    pub fn office_label(&self) -> Option<&'static str> {
        self.office.as_deref().and_then(crate::covers::office_label)
    }
}

impl Form1M {
    /// Build from an `F1MN`/`F1MA` cover record. Returns `None` only when the
    /// record has no `committee_name` column at all.
    pub fn from_data(data: &Data) -> Option<Self> {
        if !data.contains_key("committee_name") {
            return None;
        }

        let affiliation = Form1MAffiliation {
            date_form1_filed: date(data, "affiliated_date_f1_filed"),
            committee_id: text(data, "affiliated_committee_id_number"),
            committee_name: text(data, "affiliated_committee_name"),
        };
        let affiliation = (affiliation.date_form1_filed.is_some()
            || affiliation.committee_id.is_some()
            || affiliation.committee_name.is_some())
        .then_some(affiliation);

        let qualification = Form1MQualification {
            candidates: ["first", "second", "third", "fourth", "fifth"]
                .into_iter()
                .filter_map(|n| Form1MCandidate::from_prefixed(data, &format!("{n}_candidate_")))
                .collect(),
            fifty_first_contributor_date: date(data, "fifty_first_contributor_date"),
            original_registration_date: date(data, "original_registration_date"),
            requirements_met_date: date(data, "requirements_met_date"),
        };
        let qualification = (!qualification.candidates.is_empty()
            || qualification.fifty_first_contributor_date.is_some()
            || qualification.original_registration_date.is_some()
            || qualification.requirements_met_date.is_some())
        .then_some(qualification);

        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            committee_name: text_or_empty(data, "committee_name"),
            address: Address::from_prefixed(data, ""),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            committee_type: text(data, "committee_type"),
            affiliation,
            qualification,
            treasurer: person_name_or_legacy(data, "treasurer_", "treasurer_name"),
            date_signed: date(data, "date_signed"),
        })
    }

    /// True for an amended notification (`F1MA`); see [`Form1M::form_type`].
    pub fn is_amendment(&self) -> bool {
        crate::covers::is_amendment_form_type(&self.form_type)
    }

    /// The workbook's description of [`Form1M::committee_type`]: `X` "State
    /// Pty", `N` "Other" (FEC format workbook v8.4, sheet `F1M`, field 9),
    /// rendered as the form's check boxes "State Party" / "Other"
    /// ([fecfrm1m.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1m.pdf#page=1)).
    /// National and local party committees, nonconnected committees and
    /// separate segregated funds check "Other"
    /// ([fecfrm1mi.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1mi.pdf#page=1)).
    pub fn committee_type_label(&self) -> Option<&'static str> {
        match self
            .committee_type
            .as_deref()?
            .to_ascii_uppercase()
            .as_str()
        {
            "X" => Some("State Party"),
            "N" => Some("Other"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(pairs: &[(&str, &str)]) -> Data {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn affiliation_only() {
        let d = data(&[
            ("committee_name", "Some PAC"),
            ("committee_type", "N"),
            ("affiliated_date_f1_filed", "20250801"),
            ("affiliated_committee_id_number", "C00574970"),
            ("first_candidate_last_name", ""),
        ]);
        let f = Form1M::from_data(&d).unwrap();
        assert!(f.qualification.is_none());
        assert_eq!(
            f.affiliation.as_ref().unwrap().committee_id.as_deref(),
            Some("C00574970")
        );
        assert_eq!(f.committee_type_label(), Some("Other"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn legacy_candidate_name() {
        let d = data(&[
            ("committee_name", "Old PAC"),
            ("third_candidate_name", "SMITH^JOHN"),
            ("third_candidate_office", "S"),
        ]);
        let q = Form1M::from_data(&d).unwrap().qualification.unwrap();
        assert_eq!(q.candidates.len(), 1);
        assert_eq!(q.candidates[0].name.to_string(), "JOHN SMITH");
        assert_eq!(q.candidates[0].office_label(), Some("Senate"));
    }
}
