//! Schedule F: coordinated party expenditures.

use jiff::civil::Date;

use crate::covers::fields::{amount, amount_opt, date, flag, key, text, text_or_empty, Fields};
use crate::covers::Address;
use crate::itemizations::{address_either, text_any, CandidateRef, Entity};

/// "SCHEDULE F - ITEMIZED COORDINATED EXPENDITURES MADE BY POLITICAL PARTY
/// COMMITTEES OR DESIGNATED AGENT(S) ON BEHALF OF CANDIDATES FOR FEDERAL
/// OFFICE": one expenditure a party committee (or its designated agent)
/// makes on behalf of a candidate in the general election under the special
/// limits of 52 U.S.C. § 30116(d). These "are not contributions to the
/// candidate and are not contributions in-kind reported on Schedule B", and
/// do not apply in primary elections
/// ([fecfrm3xi.pdf p23](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=23)).
/// Filed with Form 3X by party committees only
/// ([fecfrm3xi.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=8));
/// the total goes to Line 25 of the Detailed Summary Page (same page). The
/// row type is always `SF`, with no line number (FEC format workbook v8.4,
/// sheet `Sch F`, field 1).
///
/// A national committee may spend through designated agents (state or
/// subordinate party committees), and a state committee may designate
/// subordinate committees
/// ([fecfrm3xi.pdf p23–24](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=23)):
/// see [`ScheduleF::filer_designated`], [`ScheduleF::designating_committee`]
/// and [`ScheduleF::subordinate_committee`]. Agents' expenditures reported
/// by the designating committee "should not be included in the reporting
/// committee's totals"
/// ([fecfrm3xi.pdf p24](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=24));
/// under the party guide's recommended method they are memo entries
/// ([partygui.pdf p130](https://www.fec.gov/resources/cms-content/documents/policy-guidance/partygui.pdf#page=130)).
///
/// # Versions
///
/// v8.0–8.5 have all 44 fields. v5.0–7.0 add `expenditure_purpose_code`,
/// and v5.0–6.3 `increased_limit`. v2–5.x have one combined `payee_name`
/// (and `payee_candidate_name`), split per [`Entity::from_prefixed`], and a
/// conduit; v2 and v3 have no category code, v2 no memo or back-reference
/// columns. v1 gives the
/// designating committee's address instead of a subordinate committee and
/// has no entity type. Paper layouts have no transaction IDs, entity type or
/// committee IDs but carry an `image_number`.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        module = "libfec_parser.itemizations",
        frozen,
        get_all,
        skip_from_py_object
    )
)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScheduleF {
    /// The row type as filed, `SF`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `Sch F`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transaction, unique for the life of the
    /// report; electronic-only. Column `transaction_id_number` (field 3).
    pub transaction_id: Option<String>,
    /// The `transaction_id` of a related record. Column
    /// `back_reference_tran_id_number` (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// The schedule of that related record. Column
    /// `back_reference_sched_name` (field 5).
    pub back_reference_schedule_name: Option<String>,
    /// "Has your committee been designated to make coordinated
    /// expenditures?": `Y` is `Some(true)`, `N` `Some(false)`, blank or any
    /// other value `None`. Column `coordinated_expenditures` (field 6,
    /// allowed values `Y,N`).
    pub filer_designated: Option<bool>,
    /// The committee that designated the filer ("If YES, name the
    /// designating committee"). Columns `designating_committee_id_number`
    /// (field 7, electronic-only), `designating_committee_name` (field 8);
    /// v1 also `designating_street_1` … `designating_zip_code`.
    pub designating_committee: ScheduleFCommittee,
    /// The subordinate committee, with its mailing address. Columns
    /// `subordinate_committee_id_number` (field 9, electronic-only),
    /// `subordinate_committee_name` (field 10), `subordinate_street_1` …
    /// `subordinate_zip_code` (fields 11–15). Absent in v1.
    pub subordinate_committee: ScheduleFCommittee,
    /// Who was paid: entity type, organization or person, mailing address.
    /// Columns `entity_type`, `payee_organization_name`, `payee_last_name` …
    /// `payee_suffix`, `payee_street_1` … `payee_zip_code` (fields 16–27);
    /// v1–5.x `payee_name`.
    pub payee: Entity,
    /// Date of the expenditure. Column `expenditure_date` (field 28).
    pub expenditure_date: Option<Date>,
    /// Amount of the expenditure. Column `expenditure_amount` (field 29).
    pub expenditure_amount: f64,
    /// The coordinated expenditures made on behalf of this candidate for the
    /// general election
    /// ([fecfrm3xi.pdf p23](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=23)).
    /// Column `aggregate_general_elec_expended` (field 30).
    pub aggregate_general_election_expended: Option<f64>,
    /// A coded purpose, v5.x–7.0 only (dropped in v8.0). Column
    /// `expenditure_purpose_code`.
    pub expenditure_purpose_code: Option<String>,
    /// Purpose of the expenditure; required when the aggregate is over $0.00
    /// (field 31). Column `expenditure_purpose_descrip`.
    pub expenditure_purpose_description: Option<String>,
    /// Category of disbursement, `001`–`012` per the workbook (field 32);
    /// the Schedule F instructions print only 001–007 and 011
    /// ([fecfrm3xi.pdf p23](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=23)).
    /// Kept raw: which source's meanings apply is an open question (fec-docs
    /// `wiki/Open-Questions.md`). Column `category_code`.
    pub category_code: Option<String>,
    /// FEC ID of the payee, when it is a committee (entity `CCM`, `COM`,
    /// `PAC`, `PTY`). Column `payee_committee_id_number` (field 33).
    pub payee_committee_fec_id: Option<String>,
    /// The federal candidate the expenditure supports ("Name of Federal
    /// Candidate Supported", "Office Sought", "State", "District"). Columns
    /// `payee_candidate_id_number` (field 34, electronic-only),
    /// `payee_candidate_last_name` … `payee_candidate_district` (fields
    /// 35–42); v1–5.x `payee_candidate_name`.
    pub candidate: CandidateRef,
    /// The conduit, v2–5.x only (dropped in v6.1). Column `conduit_name`.
    /// Its role is not in the sources read for this crate.
    pub conduit_name: Option<String>,
    /// Columns `conduit_street_1` … `conduit_zip_code`, v2–5.x only.
    pub conduit_address: Address,
    /// True for a memo entry, not counted in the Line 25 total; agents'
    /// expenditures reported by the designating committee are memo entries
    /// ([partygui.pdf p130](https://www.fec.gov/resources/cms-content/documents/policy-guidance/partygui.pdf#page=130)).
    /// Column `memo_code`, `X` when true (field 43).
    pub memo: bool,
    /// Column `memo_text_description` (field 44).
    pub memo_text: Option<String>,
    /// Column `increased_limit`, raw: v5.0–6.3 and paper layouts P1–P2.4
    /// only. Its meaning and values are not in the sources read for this
    /// crate.
    pub increased_limit: Option<String>,
    /// Column `24_hour_notice`, raw: paper layouts P1–P2.4 only. Its
    /// meaning and values are not in the sources read for this crate.
    pub notice_24_hour: Option<String>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

/// A committee named on Schedule F's designation block: the designating
/// committee or the subordinate committee (FEC format workbook v8.4, sheet
/// `Sch F`, fields 7–15).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        module = "libfec_parser.itemizations",
        frozen,
        get_all,
        skip_from_py_object
    )
)]
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct ScheduleFCommittee {
    /// The committee's FEC ID; electronic-only.
    pub fec_id: Option<String>,
    pub name: Option<String>,
    /// Given for the subordinate committee (v2+) and, in v1 only, the
    /// designating committee; empty otherwise.
    pub address: Address,
}

impl ScheduleFCommittee {
    /// Read `{prefix}committee_id_number`, `{prefix}committee_name` and the
    /// `{prefix}street_1` … address.
    fn from_prefixed<F: Fields + ?Sized>(data: &F, prefix: &str) -> Self {
        Self {
            fec_id: text(data, &key(prefix, "committee_id_number")),
            name: text(data, &key(prefix, "committee_name")),
            address: Address::from_prefixed(data, prefix),
        }
    }

    /// True when no part of the committee is filled in.
    pub fn is_empty(&self) -> bool {
        self.fec_id.is_none() && self.name.is_none() && self.address == Address::default()
    }
}

impl ScheduleF {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text_any(data, &["transaction_id_number", "transaction_id"]),
            back_reference_transaction_id: text(data, "back_reference_tran_id_number"),
            back_reference_schedule_name: text(data, "back_reference_sched_name"),
            filer_designated: yes_no(data, "coordinated_expenditures"),
            designating_committee: ScheduleFCommittee::from_prefixed(data, "designating_"),
            subordinate_committee: ScheduleFCommittee::from_prefixed(data, "subordinate_"),
            payee: Entity::from_prefixed(data, "payee_", "payee_name"),
            expenditure_date: date(data, "expenditure_date"),
            expenditure_amount: amount(data, "expenditure_amount"),
            aggregate_general_election_expended: amount_opt(
                data,
                "aggregate_general_elec_expended",
            ),
            expenditure_purpose_code: text(data, "expenditure_purpose_code"),
            expenditure_purpose_description: text(data, "expenditure_purpose_descrip"),
            category_code: text(data, "category_code"),
            payee_committee_fec_id: text(data, "payee_committee_id_number"),
            candidate: CandidateRef::from_prefixed(
                data,
                "payee_candidate_id_number",
                "payee_candidate_",
            ),
            conduit_name: text(data, "conduit_name"),
            conduit_address: address_either(data, "conduit_"),
            memo: flag(data, "memo_code"),
            memo_text: text(data, "memo_text_description"),
            increased_limit: text(data, "increased_limit"),
            notice_24_hour: text(data, "24_hour_notice"),
            image_number: text(data, "image_number"),
        })
    }
}

/// A `Y`/`N` column: `Some(true)`, `Some(false)`, or `None` when blank or
/// anything else.
fn yes_no<F: Fields + ?Sized>(data: &F, key: &str) -> Option<bool> {
    match data.raw(key)?.trim() {
        y if y.eq_ignore_ascii_case("y") => Some(true),
        n if n.eq_ignore_ascii_case("n") => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(pairs: &[(&str, &str)]) -> crate::covers::fields::Data {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn v1_designating_committee_address() {
        let d = data(&[
            ("form_type", "SF"),
            ("coordinated_expenditures", "Y"),
            ("designating_committee_id_number", "C00000935"),
            ("designating_committee_name", "DCCC"),
            ("designating_city", "Washington"),
            ("payee_name", "Acme Media"),
            ("payee_candidate_name", "Smith^Jane"),
            ("expenditure_amount", "100"),
        ]);
        let sf = ScheduleF::from_data(&d).expect("typed");
        assert_eq!(sf.filer_designated, Some(true));
        assert_eq!(
            sf.designating_committee.address.city.as_deref(),
            Some("Washington")
        );
        assert!(sf.subordinate_committee.is_empty());
        assert_eq!(sf.payee.organization_name.as_deref(), Some("Acme Media"));
        assert_eq!(sf.candidate.name.last_name, "Smith");
        assert_eq!(sf.expenditure_amount, 100.0);
    }
}
