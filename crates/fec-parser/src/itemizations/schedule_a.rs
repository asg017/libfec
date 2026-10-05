//! Schedule A: itemized receipts.

use jiff::civil::Date;

use crate::covers::fields::{amount, amount_opt, date, flag, text, text_or_empty, Fields};
use crate::covers::Address;
use crate::itemizations::{address_either, text_any, CandidateRef, Entity};

/// "SCHEDULE A - ITEMIZED RECEIPTS": one receipt a committee itemizes, with
/// the source, the date and amount, and the source's aggregate to date
/// ([fecfrm3i.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=6),
/// [fecfrm3xi.pdf p10](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=10)).
/// Filed with Forms 3, 3X, 3P and 4 (FEC format workbook v8.4, sheet `Sch A`,
/// field 1); Form 3L's `SA3L` lobbyist-bundling rows are a different layout
/// and not read as a `ScheduleA`.
///
/// The summary-page line the receipt supports is carried in the row type:
/// `SA11AI` is line 11(a)(i) (see [`crate::itemizations::line_number`]).
/// A committee itemizes a person's receipts once they aggregate over $200 in
/// the election cycle (Forms 3, 3P) or calendar year (Form 3X)
/// ([fecfrm3i.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=4),
/// [fecfrm3xi.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=6)),
/// so summing a filing's Schedule A rows does not give its total receipts.
///
/// # Versions
///
/// v8.0–8.5 have all 45 fields. v6.x–7.0 add `contribution_purpose_code`.
/// v3–5.x have one combined `contributor_name` (and `donor_candidate_name`),
/// split per [`Entity::from_prefixed`]. Paper layouts have no transaction IDs
/// or entity type but carry an `image_number`.
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
#[cfg_attr(feature = "gleam", derive(fec_parser_macros::GleamType))]
pub struct ScheduleA {
    /// The row type as filed, `SA` plus the line number: `SA11AI`. Column
    /// `form_type` (FEC format workbook v8.4, sheet `Sch A`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transaction, required and electronic-only.
    /// Column `transaction_id` (field 3).
    pub transaction_id: Option<String>,
    /// The `transaction_id` of the record this one is a memo or child of
    /// (an earmark's conduit, a partnership's attribution, …). Column
    /// `back_reference_tran_id_number` (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// The schedule of that parent record (`SA11AI`, `SB23`, …). Column
    /// `back_reference_sched_name` (field 5).
    pub back_reference_schedule_name: Option<String>,
    /// Who gave: entity type, organization or person, mailing address.
    /// Columns `entity_type`, `contributor_organization_name`,
    /// `contributor_last_name` … `contributor_suffix`, `contributor_street_1`
    /// … `contributor_zip_code` (fields 6–17); v3–5.x `contributor_name`.
    pub contributor: Entity,
    /// The election the receipt is for: a letter and a year, `P2024`
    /// (see [`ScheduleA::election_code_label`]). On Form 3X used only for
    /// refunds and loan repayments from federal candidates
    /// ([fecfrm3xi.pdf p10](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=10)).
    /// Column `election_code` (field 18).
    pub election_code: Option<String>,
    /// Required when `election_code` is `O…` (Other). Column
    /// `election_other_description` (field 19).
    pub election_other_description: Option<String>,
    /// Date of receipt. Column `contribution_date` (field 20).
    pub contribution_date: Option<Date>,
    /// "Amount of Each Receipt this Period"; negative for a returned or
    /// bounced receipt itemized again
    /// ([fecfrm3i.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=7)).
    /// Column `contribution_amount` (field 21).
    pub contribution_amount: f64,
    /// The source's aggregate: cycle-to-date on Forms 3 and 3P, year-to-date
    /// on Form 3X (FEC format workbook v8.4, sheet `Sch A`, field 22, "F3 |
    /// F3P - Cycle to Date; F3X - YTD"). Column `contribution_aggregate`.
    pub contribution_aggregate: Option<f64>,
    /// A coded purpose, v5.x–7.0 only (dropped in v8.0). Column
    /// `contribution_purpose_code`.
    pub contribution_purpose_code: Option<String>,
    /// Free-text description of the receipt (earmarks, in-kind items, …).
    /// Column `contribution_purpose_descrip` (field 23).
    pub contribution_purpose_description: Option<String>,
    /// For individuals; required once the aggregate is over $200 (FEC
    /// format workbook v8.4, sheet `Sch A`, fields 24–25). Column
    /// `contributor_employer` (field 24).
    pub contributor_employer: Option<String>,
    /// Column `contributor_occupation` (field 25).
    pub contributor_occupation: Option<String>,
    /// FEC ID of a contributing committee (entity `CCM`, `PAC`, `PTY`).
    /// Column `donor_committee_fec_id` (field 26).
    pub donor_committee_fec_id: Option<String>,
    /// Column `donor_committee_name` (field 27).
    pub donor_committee_name: Option<String>,
    /// A contributing candidate (entity `CAN`, `CCM`). Columns
    /// `donor_candidate_fec_id`, `donor_candidate_last_name` …
    /// `donor_candidate_district` (fields 28–36).
    pub donor_candidate: CandidateRef,
    /// The conduit an earmarked contribution came through (ActBlue,
    /// WinRed, …). Column `conduit_name` (field 37).
    pub conduit_name: Option<String>,
    /// Columns `conduit_street1` … `conduit_zip_code` (fields 38–42).
    pub conduit_address: Address,
    /// True for a memo entry, which does not count toward the schedule's or
    /// summary page's totals
    /// ([partygui.pdf p213](https://www.fec.gov/resources/cms-content/documents/policy-guidance/partygui.pdf#page=213)).
    /// Column `memo_code`, `X` when true (field 43).
    pub memo: bool,
    /// Column `memo_text_description` (field 44).
    pub memo_text: Option<String>,
    /// A Schedule I or L system code tying the receipt to an account.
    /// Column `reference_code` (field 45).
    pub reference_code: Option<String>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl ScheduleA {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text_any(data, &["transaction_id", "transaction_id_number"]),
            back_reference_transaction_id: text(data, "back_reference_tran_id_number"),
            back_reference_schedule_name: text(data, "back_reference_sched_name"),
            contributor: Entity::from_prefixed(data, "contributor_", "contributor_name"),
            election_code: text(data, "election_code"),
            election_other_description: text(data, "election_other_description"),
            contribution_date: date(data, "contribution_date"),
            contribution_amount: amount(data, "contribution_amount"),
            contribution_aggregate: amount_opt(data, "contribution_aggregate"),
            contribution_purpose_code: text(data, "contribution_purpose_code"),
            contribution_purpose_description: text(data, "contribution_purpose_descrip"),
            contributor_employer: text(data, "contributor_employer"),
            contributor_occupation: text(data, "contributor_occupation"),
            donor_committee_fec_id: text(data, "donor_committee_fec_id"),
            donor_committee_name: text(data, "donor_committee_name"),
            donor_candidate: CandidateRef::from_prefixed(
                data,
                "donor_candidate_fec_id",
                "donor_candidate_",
            ),
            conduit_name: text(data, "conduit_name"),
            conduit_address: address_either(data, "conduit_"),
            memo: flag(data, "memo_code"),
            memo_text: text(data, "memo_text_description"),
            reference_code: text(data, "reference_code"),
            image_number: text(data, "image_number"),
        })
    }

    /// "Primary", "General", … for `election_code`; see
    /// [`crate::covers::election_code_label`].
    pub fn election_code_label(&self) -> Option<&'static str> {
        crate::covers::election_code_label(self.election_code.as_deref()?)
    }

    /// The summary-page line, `11AI`; see [`crate::itemizations::line_number`].
    pub fn line_number(&self) -> Option<&str> {
        crate::itemizations::line_number(&self.form_type)
    }
}
