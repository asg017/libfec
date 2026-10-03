//! Form 9 line items (electioneering communications): persons sharing or
//! exercising control (`F91`), Schedule 9-A donations received (`F92`),
//! Schedule 9-B disbursements and obligations (`F93`), and the federal
//! candidates each disbursement's communication identifies (`F94`).
//!
//! The FEC introduced Form 9 in format v5.0 for BCRA: "F92 is like a
//! Schedule A where Donations are listed and F93 is like a Schedule B …
//! F94 is used as an extention for an item listed on F93 … each one having a
//! Tran_ID back-reference to a specific F93" (FEC format specification v5.3,
//! `FEC_v530.rtf`, "F9"). In v5.x the donation and disbursement rows were
//! filed as `SAF92` / `SBF93` in the Schedule A and B layouts (FEC format
//! workbook v5.3, sheets `F92`, `F93`, field 1), so they type as
//! [`crate::itemizations::ScheduleA`] / Schedule B rows, not as these.

use jiff::civil::Date;

use crate::covers::fields::{amount, date, person_name, text, text_or_empty, Fields};
use crate::covers::{Address, PersonName};
use crate::itemizations::{text_any, CandidateRef, Entity};

/// One person listed under Form 9 Line 12, "Person(s) Sharing/Exercising
/// Control": "officers, directors, executive directors or their
/// equivalents, partners, and, in the case of unincorporated organizations,
/// owners of the entity or persons making disbursements" who "shared or
/// exercised control of making the disbursement/obligation for the
/// electioneering communication" (instructions revised 01/2018)
/// ([fecfrm9.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9.pdf#page=2),
/// [fecfrm9i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=2)).
/// The `F91` record (FEC format workbook v8.4, sheet `F91`, "FORM 9 / PERSONS
/// SHARING/EXERCISING CONTROL (FOR EACH PERSON)"); it has no amounts.
///
/// # Versions
///
/// v6.1–8.5 share the 15-field layout. v5.x has one combined name, read
/// from the `controller_last_name` column (FEC format workbook v5.3, sheet
/// `F91`, field 3, "IND/NAME") and split on the name delimiter, and the
/// transaction ID at the end. Paper layouts have no transaction ID but carry
/// an `image_number`.
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
pub struct Form9ControllingPerson {
    /// The row type as filed, `F91`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `F91`, field 1).
    pub form_type: String,
    /// The filer's FEC ID. Column `filer_committee_id_number` (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this row, unique for the life of the report;
    /// electronic-only. Column `transaction_id` (field 3).
    pub transaction_id: Option<String>,
    /// "(a) Name". Columns `controller_last_name` … `controller_suffix`
    /// (fields 4–8).
    pub name: PersonName,
    /// "(b) Address" and "(c) City, State and ZIP Code". Columns
    /// `controller_street_1` … `controller_zip_code` (fields 9–13).
    pub address: Address,
    /// "(d) Name of Employer or Principal Place of Business". Column
    /// `controller_employer` (field 14).
    pub employer: Option<String>,
    /// "(e) Occupation". Column `controller_occupation` (field 15).
    pub occupation: Option<String>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl Form9ControllingPerson {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id"),
            name: person_name(data, "controller_"),
            address: Address::from_prefixed(data, "controller_"),
            employer: text(data, "controller_employer"),
            occupation: text(data, "controller_occupation"),
            image_number: text(data, "image_number"),
        })
    }
}

/// "SCHEDULE 9-A Donation(s) Received": one donation used to finance an
/// electioneering communication — "any gift, subscription, loan, advance or
/// deposit of money or anything of value" (instructions revised 01/2018)
/// ([fecfrm9.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9.pdf#page=3),
/// [fecfrm9i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=1)).
/// The `F92` record (FEC format workbook v8.4, sheet `F92`, "FORM 9 / SCHED
/// 9-A - FOR EACH DONATION RECEIVED"). The cover's `total_donations` is "=
/// Sum of F92 Donations" (sheet `F9`, field 38).
///
/// Which donors appear depends on the filer: generally each donor of
/// $1,000 or more since the first day of the preceding calendar year, but a
/// corporation or labor organization not paying from a segregated account
/// lists only donations made "for the purpose of furthering electioneering
/// communications" ([fecfrm9i.pdf p2–3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=2)).
///
/// # Versions
///
/// v6.1–8.5 share the 19-field layout. The v5.x `F92` layout is the
/// Schedule A one (see the module docs): a combined `contributor_name` until
/// v5.1 added the separate name columns, employer and occupation, and a
/// "TRANS CODE"; its election code and aggregate are marked "Not used" and
/// its fields 18–33 are space holders (FEC format workbook v5.3, sheet
/// `F92`), so they are not read. Paper layouts have no transaction ID,
/// back reference or entity type but carry a memo text and an
/// `image_number`.
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
pub struct Form9Donation {
    /// The row type as filed, `F92`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `F92`, field 1).
    pub form_type: String,
    /// The filer's FEC ID. Column `filer_committee_id_number` (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transaction, unique for the life of the
    /// report; electronic-only. Column `transaction_id` (field 3).
    pub transaction_id: Option<String>,
    /// "Reference to the Tran ID of a Related Record". Column
    /// `back_reference_tran_id_number` (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// The schedule of that record, `F92` or `F93`. Column
    /// `back_reference_sched_name` (field 5).
    pub back_reference_schedule_name: Option<String>,
    /// "Full Name of Donor" and "Mailing Address of Donor"
    /// ([fecfrm9.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9.pdf#page=3)),
    /// though the columns say contributor. Columns `entity_type`,
    /// `contributor_organization_name`, `contributor_last_name` …
    /// `contributor_suffix`, `contributor_street_1` … `contributor_zip_code`
    /// (fields 6–17); v5.0 `contributor_name`.
    pub donor: Entity,
    /// "Date of Receipt" (DATE RECEIVED). Column `contribution_date`
    /// (field 18).
    pub contribution_date: Option<Date>,
    /// "Amount" (AMOUNT RECEIVED). Column `contribution_amount` (field 19).
    pub contribution_amount: f64,
    /// v5.x only, "INDEMP", required "if Aggr > 200" (FEC format workbook
    /// v5.3, sheet `F92`, field 12); the v6+ record and the printed schedule
    /// have no employer. Column `contributor_employer`.
    pub donor_employer: Option<String>,
    /// v5.x only, "INDOCC" (field 13). Column `contributor_occupation`.
    pub donor_occupation: Option<String>,
    /// v5.x only, "TRANS CODE", sample `15`, values `15,15C,...` (FEC format
    /// workbook v5.3, sheet `F92`, field 17). Column
    /// `contribution_purpose_code`.
    pub transaction_code: Option<String>,
    /// Paper layouts only. Column `memo_text_description`.
    pub memo_text: Option<String>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl Form9Donation {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id"),
            back_reference_transaction_id: text(data, "back_reference_tran_id_number"),
            back_reference_schedule_name: text(data, "back_reference_sched_name"),
            donor: Entity::from_prefixed(data, "contributor_", "contributor_name"),
            contribution_date: date(data, "contribution_date"),
            contribution_amount: amount(data, "contribution_amount"),
            donor_employer: text(data, "contributor_employer"),
            donor_occupation: text(data, "contributor_occupation"),
            transaction_code: text(data, "contribution_purpose_code"),
            memo_text: text(data, "memo_text_description"),
            image_number: text(data, "image_number"),
        })
    }
}

/// "SCHEDULE 9-B Disbursement(s) Made or Obligation(s)": one disbursement
/// made or contract executed for an electioneering communication,
/// "including each disbursement made or contract executed prior to
/// exceeding the $10,000 threshold" (instructions revised 01/2018)
/// ([fecfrm9.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9.pdf#page=4),
/// [fecfrm9i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=3)).
/// The `F93` record (FEC format workbook v8.4, sheet `F93`, "FORM 9 / SCHED
/// 9-B - FOR EACH DISBURSEMENT MADE"). The cover's `total_disbursements` is
/// "= Sum of F93 Disbursements" (sheet `F9`, field 39). The candidates the
/// communication identifies are [`Form9Candidate`] (`F94`) rows that
/// back-reference this one's `transaction_id`.
///
/// # Versions
///
/// v8.0–8.5 have the 25-field layout; v6.1–7.0 add
/// `expenditure_purpose_code`. The v5.x `F93` layout is the Schedule B one
/// (see the module docs): a combined `payee_name` until v5.1 added the
/// separate name columns, a purpose code, a category code (fields 10, 36)
/// and the transaction ID near the end; its fields 16–30 are space holders
/// (FEC format workbook v5.3, sheet `F93`), so the beneficiary and conduit
/// columns the layout names there are not read. Paper layouts have no back
/// reference, entity type or election code but carry a memo text (P1–P3.1)
/// and an `image_number`.
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
pub struct Form9Disbursement {
    /// The row type as filed, `F93`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `F93`, field 1).
    pub form_type: String,
    /// The filer's FEC ID. Column `filer_committee_id_number` (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transaction, unique for the life of the
    /// report. Column `transaction_id` (field 3); v5.x
    /// `transaction_id_number`.
    pub transaction_id: Option<String>,
    /// "Reference to the Tran ID of a Related Record". Column
    /// `back_reference_tran_id_number` (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// The schedule of that record, `F92` or `F93`. Column
    /// `back_reference_sched_name` (field 5).
    pub back_reference_schedule_name: Option<String>,
    /// "Full Name (Last, First, Middle Initial) of Payee" and "Mailing
    /// Address of Payee"
    /// ([fecfrm9.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9.pdf#page=4)).
    /// Columns `entity_type`, `payee_organization_name`, `payee_last_name` …
    /// `payee_suffix`, `payee_street_1` … `payee_zip_code` (fields 6–17);
    /// v5.0 `payee_name`.
    pub payee: Entity,
    /// "Disbursement/Obligation For:" the election, a letter and a year
    /// (`G,P,O[YYYY]`). See [`Form9Disbursement::election_code_label`].
    /// Column `election_code` (field 18).
    pub election_code: Option<String>,
    /// "Other (specify)", required when `election_code` is `O…`. Column
    /// `election_other_description` (field 19).
    pub election_other_description: Option<String>,
    /// "Date of Disbursement or Obligation". Column `expenditure_date`
    /// (field 20).
    pub expenditure_date: Option<Date>,
    /// "Amount". Column `expenditure_amount` (field 21).
    pub expenditure_amount: f64,
    /// v5.0–7.0 only: "TRANS {Purpose} CODE" (FEC format workbook v5.3,
    /// sheet `F93`, field 10). Column `expenditure_purpose_code`.
    pub expenditure_purpose_code: Option<String>,
    /// "Purpose of Disbursement (Including title(s) of communication(s))":
    /// the specific purpose (radio ad, television ad, …) and "the title of
    /// the communication as named by the media vendor or producer"
    /// ([fecfrm9i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=3)).
    /// Column `expenditure_purpose_descrip` (field 22).
    pub expenditure_purpose_description: Option<String>,
    /// v5.x only: "CATEGORY CODE", `001`–`010` in the v5.3 workbook (sheet
    /// `F93`, field 36). Column `category_code`.
    pub category_code: Option<String>,
    /// "Name of Employer", required if the payee is an individual or
    /// candidate. Column `payee_employer` (field 23).
    pub payee_employer: Option<String>,
    /// "Occupation", same rule. Column `payee_occupation` (field 24).
    pub payee_occupation: Option<String>,
    /// "Communication Date", which "in most instances … will be the Date of
    /// Public Distribution"
    /// ([fecfrm9i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=3)).
    /// Column `communication_date` (field 25).
    pub communication_date: Option<Date>,
    /// Paper layouts P1–P3.1 only. Column `memo_text_description`.
    pub memo_text: Option<String>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl Form9Disbursement {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text_any(data, &["transaction_id", "transaction_id_number"]),
            back_reference_transaction_id: text(data, "back_reference_tran_id_number"),
            back_reference_schedule_name: text(data, "back_reference_sched_name"),
            payee: Entity::from_prefixed(data, "payee_", "payee_name"),
            election_code: text(data, "election_code"),
            election_other_description: text(data, "election_other_description"),
            expenditure_date: date(data, "expenditure_date"),
            expenditure_amount: amount(data, "expenditure_amount"),
            expenditure_purpose_code: text(data, "expenditure_purpose_code"),
            expenditure_purpose_description: text(data, "expenditure_purpose_descrip"),
            category_code: text(data, "category_code"),
            payee_employer: text(data, "payee_employer"),
            payee_occupation: text(data, "payee_occupation"),
            communication_date: date(data, "communication_date"),
            memo_text: text(data, "memo_text_description"),
            image_number: text(data, "image_number"),
        })
    }

    /// "Primary", "General", … for `election_code`; see
    /// [`crate::covers::election_code_label`].
    pub fn election_code_label(&self) -> Option<&'static str> {
        crate::covers::election_code_label(self.election_code.as_deref()?)
    }
}

/// One federal candidate "clearly identified in the communication" a
/// Schedule 9-B disbursement paid for, "including the office sought and the
/// election that the disbursement/obligation is made for" (instructions
/// revised 01/2018)
/// ([fecfrm9i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=3)).
/// On paper these are the repeated candidate blocks of a 9-B entry
/// ([fecfrm9.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9.pdf#page=4));
/// electronically each is an `F94` row (FEC format workbook v8.4, sheet
/// `F94`, "FORM 94 - FEDERAL CANDIDATE LIST FOR FORM 93 TRANSACTIONS") whose
/// back reference names the [`Form9Disbursement`]. It has no amount.
///
/// # Versions
///
/// v6.1–8.5 share the 16-field layout. v5.x has one combined
/// `candidate_name` (split per [`CandidateRef::from_prefixed`]) and the
/// transaction ID and back reference at the end (FEC format workbook v5.3,
/// sheet `F94`). Paper layouts have no transaction ID, back-reference
/// schedule or candidate ID but carry an `image_number`.
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
pub struct Form9Candidate {
    /// The row type as filed, `F94`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `F94`, field 1).
    pub form_type: String,
    /// The filer's FEC ID. Column `filer_committee_id_number` (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this row, unique for the life of the report.
    /// Column `transaction_id` (field 3).
    pub transaction_id: Option<String>,
    /// The `transaction_id` of the [`Form9Disbursement`] this candidate
    /// belongs to. Column `back_reference_tran_id_number` (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// Always `F93`. Column `back_reference_sched_name` (field 5).
    pub back_reference_schedule_name: Option<String>,
    /// "Name of Federal Candidate", "Office Sought", "State", "District",
    /// plus the candidate's FEC ID (electronic-only). Columns
    /// `candidate_id_number`, `candidate_last_name` … `candidate_district`
    /// (fields 6–14); v5.x `candidate_name`.
    pub candidate: CandidateRef,
    /// "Disbursement/Obligation For:" the election, a letter and a year
    /// (`G,P,O[YYYY]`). See [`Form9Candidate::election_code_label`]. Column
    /// `election_code` (field 15).
    pub election_code: Option<String>,
    /// "Other (specify)", required when `election_code` is `O…`. Column
    /// `election_other_description` (field 16).
    pub election_other_description: Option<String>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl Form9Candidate {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id"),
            back_reference_transaction_id: text(data, "back_reference_tran_id_number"),
            back_reference_schedule_name: text(data, "back_reference_sched_name"),
            candidate: CandidateRef::from_prefixed(data, "candidate_id_number", "candidate_"),
            election_code: text(data, "election_code"),
            election_other_description: text(data, "election_other_description"),
            image_number: text(data, "image_number"),
        })
    }

    /// "Primary", "General", … for `election_code`; see
    /// [`crate::covers::election_code_label`].
    pub fn election_code_label(&self) -> Option<&'static str> {
        crate::covers::election_code_label(self.election_code.as_deref()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::covers::fields::Data;

    fn data(pairs: &[(&str, &str)]) -> Data {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn legacy_controller_name() {
        // v5.x: one combined name in the `controller_last_name` column.
        let d = data(&[
            ("form_type", "F91"),
            ("controller_last_name", "Doe^Jane^Ms.^"),
        ]);
        let p = Form9ControllingPerson::from_data(&d).expect("typed");
        assert_eq!(
            (p.name.last_name.as_str(), p.name.first_name.as_str()),
            ("Doe", "Jane")
        );
    }

    #[test]
    fn legacy_candidate() {
        let d = data(&[
            ("form_type", "F94"),
            ("candidate_id_number", "H04MA3210"),
            ("candidate_name", "Smith^Pat"),
            ("candidate_office", "H"),
            ("back_reference_tran_id_number", "B1"),
        ]);
        let c = Form9Candidate::from_data(&d).expect("typed");
        assert_eq!(c.candidate.name.last_name, "Smith");
        assert_eq!(c.candidate.office_label(), Some("House"));
        assert_eq!(c.back_reference_transaction_id.as_deref(), Some("B1"));
    }
}
