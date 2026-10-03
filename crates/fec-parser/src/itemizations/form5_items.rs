//! Form 5 line items: Schedule 5-A contributions received (`F56`) and
//! Schedule 5-E independent expenditures (`F57`).

use jiff::civil::Date;

use crate::covers::fields::{amount, amount_opt, date, text, text_or_empty, Fields};
use crate::covers::Address;
use crate::itemizations::{category_code_label, support_oppose_label, address_either, text_any, CandidateRef, Entity};

/// "SCHEDULE 5-A ITEMIZED RECEIPTS": one contribution a person other than a
/// political committee received "for the purpose of furthering the
/// independent expenditures" it reports on Form 5, itemized when over $200
/// (instructions revised 09/2013)
/// ([fecfrm5.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5.pdf#page=2),
/// [fecfrm5i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=2)).
/// The `F56` record (FEC format workbook v8.4, sheet `F56`, "FORM 5.6 - FOR
/// EACH CONTRIBUTION (SCHEDULE 5-A)").
///
/// The schedule's total is carried to Line 6 of Form 5, which also counts
/// unitemized contributions of $200 or less, so summing a filing's `F56`
/// rows can come out below the cover's total
/// ([fecfrm5i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=2)).
/// There is no aggregate column (FEC format workbook v8.4, sheet `F56`,
/// fields 1–20).
///
/// # Versions
///
/// v6.1–8.5 share the 20-field layout. v2–5.x (FEC format workbook v5.3,
/// sheet `F56`) have one combined `contributor_name` (split per
/// [`Entity::from_prefixed`]), the transaction ID at the end, and a
/// candidate (fields 15–19) and conduit (fields 20–25) that v6+ dropped; their
/// `amended_cd` is ignored ("Unused Field (it's defined, but ignored)", FEC
/// format specification v5.3, `FEC_v530.rtf`, "Amend Code"). v1 has no entity
/// type and no transaction ID (its `sequence_number` is not read). Paper
/// layouts have no transaction ID or entity type but carry an `image_number`.
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
pub struct Form5Contribution {
    /// The row type as filed, `F56`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `F56`, field 1).
    pub form_type: String,
    /// The filer's FEC ID. Column `filer_committee_id_number` (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transaction, "unique and UPPER CASE for the
    /// life of the report (original + all amendments)"; electronic-only.
    /// Column `transaction_id` (field 3).
    pub transaction_id: Option<String>,
    /// Who gave: entity type, organization or person, mailing address
    /// ("A. Full Name", "Mailing Address" on
    /// [fecfrm5.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5.pdf#page=2)).
    /// Columns `entity_type`, `contributor_organization_name`,
    /// `contributor_last_name` … `contributor_suffix`, `contributor_street_1`
    /// … `contributor_zip_code` (fields 4–15); v1–5.x `contributor_name`.
    pub contributor: Entity,
    /// "FEC ID number of contributing federal political committee"
    /// ([fecfrm5.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5.pdf#page=2)).
    /// Column `contributor_fec_id` (field 16).
    pub contributor_fec_id: Option<String>,
    /// "Date of Receipt". Column `contribution_date` (field 17).
    pub contribution_date: Option<Date>,
    /// "Amount of Each Receipt this Period". Column `contribution_amount`
    /// (field 18).
    pub contribution_amount: f64,
    /// "Name of Employer". Column `contributor_employer` (field 19).
    pub contributor_employer: Option<String>,
    /// "Occupation". Column `contributor_occupation` (field 20).
    pub contributor_occupation: Option<String>,
    /// v2–5.x only: "FEC CANDIDATE ID NUMBER", "CANDIDATE NAME",
    /// "CAN/OFFICE", "CAN/STATE", "CAN/DIST" (FEC format workbook v5.3, sheet
    /// `F56`, fields 15–19). The workbook does not say what role the
    /// candidate plays. Columns `candidate_id`, `candidate_name`,
    /// `candidate_office`, `candidate_state`, `candidate_district`.
    pub candidate: CandidateRef,
    /// v2–5.x only: "CONDUIT NAME" (FEC format workbook v5.3, sheet `F56`,
    /// field 20). Column `conduit_name`.
    pub conduit_name: Option<String>,
    /// v2–5.x only (fields 21–25). Columns `conduit_street_1` …
    /// `conduit_zip_code`.
    pub conduit_address: Address,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl Form5Contribution {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id"),
            contributor: Entity::from_prefixed(data, "contributor_", "contributor_name"),
            contributor_fec_id: text(data, "contributor_fec_id"),
            contribution_date: date(data, "contribution_date"),
            contribution_amount: amount(data, "contribution_amount"),
            contributor_employer: text(data, "contributor_employer"),
            contributor_occupation: text(data, "contributor_occupation"),
            candidate: CandidateRef::from_prefixed(data, "candidate_id", "candidate_"),
            conduit_name: text(data, "conduit_name"),
            conduit_address: address_either(data, "conduit_"),
            image_number: text(data, "image_number"),
        })
    }
}

/// "SCHEDULE 5-E ITEMIZED INDEPENDENT EXPENDITURES": one independent
/// expenditure by a person other than a political committee, with the
/// payee, the federal candidate it supports or opposes, and the
/// calendar-year aggregate per election and office
/// ([fecfrm5.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5.pdf#page=3)).
/// The `F57` record (FEC format workbook v8.4, sheet `F57`, "FORM 5.7 - FOR
/// EACH INDEPENDENT EXPENDITURE MADE (SCHEDULE 5-E)").
///
/// Entries are itemized "once the total of independent expenditures made
/// exceeds $250 per election in a calendar year"; the schedule's total,
/// including unitemized expenditures, is carried to Line 7 of Form 5
/// (instructions revised 09/2013)
/// ([fecfrm5i.pdf p2–3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=2)).
/// For the 24- and 48-hour reports filers aggregate "including enforceable
/// contracts obligating funds for disbursement" as of the first date of
/// public dissemination, so a row can be an obligation rather than a payment
/// ([fecfrm5i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=2)).
///
/// # Versions
///
/// v8.1–8.5 and v8.0 have the 33-field layout; v6.1–7.0 add
/// `expenditure_purpose_code`. v2–5.x (FEC format workbook v5.3, sheet `F57`)
/// have combined `payee_name` and `candidate_name` columns (split per
/// [`Entity::from_prefixed`] and [`CandidateRef::from_prefixed`]), a conduit
/// (fields 25–30) dropped in v6; v5.x add at the end the category and
/// purpose codes, the aggregate and the election code (fields 33–37). v2's
/// `payee_candidate_*` columns (v5.x's "Unused field" 20–24) and every
/// legacy `amended_code` are not read. v1 has no entity type or transaction
/// ID. Paper layouts have no transaction ID, entity type, candidate ID or
/// payee committee ID but carry an `image_number`.
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
pub struct Form5Expenditure {
    /// The row type as filed, `F57`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `F57`, field 1).
    pub form_type: String,
    /// The filer's FEC ID. Column `filer_committee_id_number` (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transaction, unique for the life of the
    /// report; electronic-only. Column `transaction_id_number` (field 3).
    pub transaction_id: Option<String>,
    /// Who was paid: "Full Name (Last, First, Middle Initial) of Payee" and
    /// "Mailing Address"
    /// ([fecfrm5.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5.pdf#page=3)).
    /// Columns `entity_type`, `payee_organization_name`, `payee_last_name` …
    /// `payee_suffix`, `payee_street_1` … `payee_zip_code` (fields 4–15);
    /// v1–5.x `payee_name`.
    pub payee: Entity,
    /// The payee's FEC committee ID; electronic-only (no box on the printed
    /// schedule). Column `payee_cmtte_fec_id_number` (field 23).
    pub payee_committee_fec_id: Option<String>,
    /// "Disbursement For:" the election, a letter and a year (`P2024`; the
    /// workbook allows `G,P,O[CCYY]`). See [`Form5Expenditure::election_code_label`].
    /// Column `election_code` (field 16).
    pub election_code: Option<String>,
    /// Required when `election_code` is `O…` (Other). Column
    /// `election_other_description` (field 17).
    pub election_other_description: Option<String>,
    /// "Date of Public Distribution/Dissemination" — not a payment date
    /// ([fecfrm5.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5.pdf#page=3)).
    /// Column `dissemination_date` (field 18).
    pub dissemination_date: Option<Date>,
    /// "Amount". Column `expenditure_amount` (field 19).
    pub expenditure_amount: f64,
    /// "Calendar Year-To-Date Per Election for Office Sought": "the total
    /// amount expended in the aggregate during the calendar year, per
    /// election, per office sought" — a running aggregate, not an amount to
    /// sum ([fecfrm5i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=3)).
    /// Column `calendar_y_t_d_per_election_office` (field 20).
    pub calendar_ytd_per_election_office: Option<f64>,
    /// A coded purpose ("TRANS {Purpose} CODE", sample `24A, 24E`; FEC format
    /// workbook v5.3, sheet `F57`, field 34), v5.0–7.0 only. Column
    /// `expenditure_purpose_code`.
    pub expenditure_purpose_code: Option<String>,
    /// "Purpose of Expenditure". Column `expenditure_purpose_descrip`
    /// (field 21).
    pub expenditure_purpose_description: Option<String>,
    /// "Category/Type" code, `001`–`012` (field 22); "not intended to replace
    /// or to serve as a substitute for the 'purpose of disbursement'"
    /// ([fecfrm5i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5i.pdf#page=3)).
    /// See [`Form5Expenditure::category_code_label`]. Column `category_code`;
    /// from v5.0.
    pub category_code: Option<String>,
    /// `S` or `O`, the printed "Check One: Support / Oppose" (field 24). See
    /// [`Form5Expenditure::support_oppose_label`]. Column
    /// `support_oppose_code`.
    pub support_oppose_code: Option<String>,
    /// "Name of Federal Candidate Supported or Opposed by Expenditure",
    /// "Office Sought", "State", "District"
    /// ([fecfrm5.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm5.pdf#page=3)),
    /// plus the candidate's FEC ID (electronic-only). Columns
    /// `candidate_id_number`, `candidate_last_name` … `candidate_district`
    /// (fields 25–33); v1–5.x `candidate_name`.
    pub candidate: CandidateRef,
    /// v2–5.x only: "CONDUIT NAME" (FEC format workbook v5.3, sheet `F57`,
    /// field 25). Column `conduit_name`.
    pub conduit_name: Option<String>,
    /// v2–5.x only (fields 26–30). Columns `conduit_street_1` …
    /// `conduit_zip_code`.
    pub conduit_address: Address,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl Form5Expenditure {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text_any(data, &["transaction_id_number", "transaction_id"]),
            payee: Entity::from_prefixed(data, "payee_", "payee_name"),
            payee_committee_fec_id: text(data, "payee_cmtte_fec_id_number"),
            election_code: text(data, "election_code"),
            election_other_description: text(data, "election_other_description"),
            dissemination_date: date(data, "dissemination_date"),
            expenditure_amount: amount(data, "expenditure_amount"),
            calendar_ytd_per_election_office: amount_opt(
                data,
                "calendar_y_t_d_per_election_office",
            ),
            expenditure_purpose_code: text(data, "expenditure_purpose_code"),
            expenditure_purpose_description: text(data, "expenditure_purpose_descrip"),
            category_code: text(data, "category_code"),
            support_oppose_code: text(data, "support_oppose_code"),
            candidate: CandidateRef::from_prefixed(data, "candidate_id_number", "candidate_"),
            conduit_name: text(data, "conduit_name"),
            conduit_address: address_either(data, "conduit_"),
            image_number: text(data, "image_number"),
        })
    }

    /// "Primary", "General", … for `election_code`; see
    /// [`crate::covers::election_code_label`].
    pub fn election_code_label(&self) -> Option<&'static str> {
        crate::covers::election_code_label(self.election_code.as_deref()?)
    }

    /// "Support" or "Oppose" for `support_oppose_code` (the printed
    /// "Check One: Support Oppose"), `None` for other codes.
    pub fn support_oppose_label(&self) -> Option<&'static str> {
        support_oppose_label(self.support_oppose_code.as_deref()?)
    }

    /// The FEC's name for `category_code`, `001`–`012` (FEC format
    /// specification v8.4, `FEC_Format_v8.4.pdf` p16), `None` for other codes.
    pub fn category_code_label(&self) -> Option<&'static str> {
        category_code_label(self.category_code.as_deref()?)
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
    fn legacy_f57() {
        // v5.3 layout: combined payee and candidate names, conduit.
        let d = data(&[
            ("form_type", "F57"),
            ("filer_committee_id_number", "C90065431"),
            ("entity_type", "CCM"),
            ("payee_name", "Pat for Congress"),
            ("candidate_id_number", "H04MA3210"),
            ("candidate_name", "Smith^Pat T.^Mr.^Jr."),
            ("expenditure_amount", "1500"),
            ("conduit_name", "Conduit Co"),
            ("conduit_street_1", "1 Main St"),
            ("transaction_id_number", "56123456789-1234"),
            ("category_code", "004"),
            ("support_oppose_code", "S"),
        ]);
        let e = Form5Expenditure::from_data(&d).expect("typed");
        assert_eq!(
            e.payee.organization_name.as_deref(),
            Some("Pat for Congress")
        );
        assert_eq!(e.candidate.name.last_name, "Smith");
        assert_eq!(e.candidate.name.first_name, "Pat T.");
        assert_eq!(e.conduit_address.street_1.as_deref(), Some("1 Main St"));
        assert_eq!(e.transaction_id.as_deref(), Some("56123456789-1234"));
        assert_eq!(e.category_code_label(), Some("Advertising Expenses"));
        assert_eq!(e.support_oppose_label(), Some("Support"));
    }

    #[test]
    fn legacy_f56() {
        let d = data(&[
            ("form_type", "F56"),
            ("entity_type", "IND"),
            ("contributor_name", "Smith^Pat T.^Mr.^Jr."),
            ("contribution_amount", "250"),
            ("candidate_id", "H04MA3210"),
            ("transaction_id", "56123456789-1234"),
        ]);
        let c = Form5Contribution::from_data(&d).expect("typed");
        assert_eq!(c.contributor.name.last_name, "Smith");
        assert_eq!(c.candidate.fec_id.as_deref(), Some("H04MA3210"));
        assert_eq!(c.contribution_amount, 250.0);
    }

    #[test]
    fn labels() {
        assert_eq!(
            category_code_label("1"),
            Some("Administrative/Salary/Overhead Expenses")
        );
        assert_eq!(category_code_label("012"), Some("Donations"));
        assert_eq!(category_code_label("101"), Some("Expenses that are not Allocable"));
        assert_eq!(support_oppose_label(" o "), Some("Oppose"));
        assert_eq!(support_oppose_label("X"), None);
    }
}
