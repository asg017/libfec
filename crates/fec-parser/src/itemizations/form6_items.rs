//! Form 6 line items: the contributions listed on a 48-hour notice (`F65`).

use jiff::civil::Date;

use crate::covers::fields::{amount, date, text, text_or_empty, Fields};
use crate::covers::Address;
use crate::itemizations::{address_either, CandidateRef, Entity};

/// One contribution on a Form 6 "48-Hour Notice of Contributions/Loans
/// Received": the lettered entries (A, B, C, …) on the face of the form,
/// each with full name, mailing address, occupation, employer, date and
/// amount ([fecfrm6.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm6.pdf#page=1)).
/// The `F65` record (FEC format workbook v8.4, sheet `F65`, "FORM 6.5 - FOR
/// EACH CONTRIBUTOR").
///
/// Rows include loans, loan guarantees and endorsements, advances, and a
/// candidate's draws on personal credit cards, not only contributions, and
/// nothing on the record tells them apart; the same contributions are
/// itemized "a second time in the first report filed after the election", so
/// they overlap with the committee's next regular report (instructions
/// revised 03/2016)
/// ([fecfrm6i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm6i.pdf#page=1)).
/// Employer and occupation are asked only of individuals; for "any other
/// person (including contributions from political committees)" the notice
/// gives name, address, date and amount
/// ([fecfrm6i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm6i.pdf#page=1)).
///
/// # Versions
///
/// v6.1–8.5 share the 20-field layout. v2–5.x (FEC format workbook v5.3,
/// sheet `F65`) have one combined `contributor_name` (split per
/// [`Entity::from_prefixed`]), the transaction ID at the end, and a
/// candidate (fields 15–19) and conduit (fields 20–25) that v6+ dropped;
/// their `amended_cd` is ignored, as the v5.3 specification says
/// (`FEC_v530.rtf`, "Amend Code … Unused Field (it's defined, but
/// ignored)"). v1 has no entity type and no transaction ID (its
/// `sequence_number` is not read). Paper layouts have no transaction ID,
/// entity type or contributor FEC ID but carry an `image_number`.
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
#[cfg_attr(feature = "columnar", derive(fec_parser_macros::Columnar))]
pub struct Form6Contribution {
    /// The row type as filed, `F65`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `F65`, field 1).
    pub form_type: String,
    /// The filing candidate committee's FEC ID ("4. FEC Identification
    /// Number"). Column `filer_committee_id_number` (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transaction, unique for the life of the
    /// report; electronic-only. Column `transaction_id` (field 3).
    pub transaction_id: Option<String>,
    /// Who gave or lent: entity type, organization or person ("Full Name"),
    /// "Mailing Address". Columns `entity_type`,
    /// `contributor_organization_name`, `contributor_last_name` …
    /// `contributor_suffix`, `contributor_street_1` … `contributor_zip_code`
    /// (fields 4–15); v1–5.x `contributor_name`.
    pub contributor: Entity,
    /// A contributing committee's FEC ID ("If CCM,PAC..."); electronic-only.
    /// Column `contributor_fec_id` (field 16).
    pub contributor_fec_id: Option<String>,
    /// "Date (month, day, year)" of receipt. Column `contribution_date`
    /// (field 17).
    pub contribution_date: Option<Date>,
    /// "Amount". Column `contribution_amount` (field 18).
    pub contribution_amount: f64,
    /// "Name of Employer", for individuals. Column `contributor_employer`
    /// (field 19).
    pub contributor_employer: Option<String>,
    /// "Occupation", for individuals. Column `contributor_occupation`
    /// (field 20).
    pub contributor_occupation: Option<String>,
    /// v2–5.x only: "FEC CANDIDATE ID NUMBER", "CANDIDATE NAME",
    /// "CAN/OFFICE", "CAN/STATE", "CAN/DIST" (FEC format workbook v5.3, sheet
    /// `F65`, fields 15–19). The workbook does not say what role the
    /// candidate plays. Columns `candidate_id`, `candidate_name`,
    /// `candidate_office`, `candidate_state`, `candidate_district`.
    pub candidate: CandidateRef,
    /// v2–5.x only: "CONDUIT NAME" (FEC format workbook v5.3, sheet `F65`,
    /// field 20). Column `conduit_name`.
    pub conduit_name: Option<String>,
    /// v2–5.x only (fields 21–25). Columns `conduit_street_1` …
    /// `conduit_zip_code`.
    pub conduit_address: Address,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl Form6Contribution {
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
