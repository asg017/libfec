//! Schedule H4: disbursements for allocated federal/nonfederal activity.

use jiff::civil::Date;

use crate::covers::fields::{amount, amount_opt, date, flag, text, text_or_empty, Fields};
use crate::itemizations::Entity;

/// "SCHEDULE H4 - DISBURSEMENTS FOR ALLOCATED FEDERAL/NONFEDERAL ACTIVITY":
/// one payment for activity a committee splits between its federal and
/// nonfederal accounts, with the total, the federal share and the
/// nonfederal share
/// ([fecfrm3xi.pdf p29](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=29),
/// [p30](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=30)).
/// Supports Form 3X Line 21(a): the period's federal and nonfederal shares go
/// to Lines 21(a)(i) and 21(a)(ii), and every such disbursement is itemized
/// regardless of amount
/// ([fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
///
/// The shares come from the Schedule H1 ratio (administrative, generic voter
/// drive, exempt, party-only public communications) or the Schedule H2 ratio
/// of the named event (fundraising, direct candidate support)
/// ([fecfrm3xi.pdf p30](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=30)).
/// A payment covering several activity types is a memo entry followed by a
/// breakdown by type (same page), so leave out `memo` rows when summing; for
/// direct candidate support the federal share is also disclosed on Schedule
/// B, E or F (same page).
///
/// # Versions
///
/// v8.0–8.5 have all 33 fields; v6.1–7.0 add `expenditure_purpose_code`.
/// v2–5.x have one combined `payee_name`, split per [`Entity::from_prefixed`],
/// and also carry candidate, committee and conduit columns that are not read
/// (blank in every corpus row); v5.0–5.1 have two administrative columns,
/// either of which sets `administrative_activity`. Paper layouts have no
/// transaction IDs or entity type but an `image_number`.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        module = "libfec.itemizations",
        frozen,
        get_all,
        skip_from_py_object
    )
)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScheduleH4 {
    /// The row type as filed, `H4`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `Sch H4`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transaction, unique for the life of the
    /// report; electronic-only. Column `transaction_id_number` (field 3).
    pub transaction_id: Option<String>,
    /// The `transaction_id` of the record this one is a memo or child of
    /// (the parent of a breakdown, a Schedule B payment, …). Column
    /// `back_reference_tran_id_number` (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// The schedule of that parent record (`H4`, `SB21B`, …). Column
    /// `back_reference_sched_name` (field 5).
    pub back_reference_schedule_name: Option<String>,
    /// Who was paid: entity type, organization or person, mailing address.
    /// Columns `entity_type`, `payee_organization_name`, `payee_last_name`
    /// … `payee_suffix`, `payee_street_1` … `payee_zip_code` (fields 6–17);
    /// v2–5.x `payee_name`.
    pub payee: Entity,
    /// "Activity or Event Identifier": for fundraising and direct candidate
    /// support, the Schedule H2 event name
    /// ([fecfrm3xi.pdf p30](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=30)).
    /// Column `account_identifier` (field 18).
    pub activity_event_identifier: Option<String>,
    /// Date of the disbursement. Column `expenditure_date` (field 19).
    pub expenditure_date: Option<Date>,
    /// The whole payment, federal plus nonfederal. Column `total_amount`
    /// (field 20, "TOTAL FED-NONFED AMOUNT").
    pub total_amount: f64,
    /// The federal account's share. Column `federal_share` (field 21).
    pub federal_share: f64,
    /// The nonfederal account's share. Column `nonfederal_share` (field 22).
    pub nonfederal_share: f64,
    /// Year-to-date total for the activity type or, for fundraising and
    /// direct candidate support, for the event; not a per-payee aggregate
    /// ([fecfrm3xi.pdf p30](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=30)).
    /// Column `event_year_to_date` (field 23).
    pub event_year_to_date: Option<f64>,
    /// A coded purpose, v5.x–7.0 only (dropped in v8.0). Column
    /// `expenditure_purpose_code`.
    pub expenditure_purpose_code: Option<String>,
    /// "Purpose of Disbursement". Column `expenditure_purpose_description`
    /// (field 24).
    pub expenditure_purpose_description: Option<String>,
    /// Disbursement category, `001`…, kept raw: the workbook allows 001–010
    /// (field 25), the H4 instructions list 001–012
    /// ([fecfrm3xi.pdf p29](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=29)),
    /// and the sources disagree (fec-docs `wiki/Open-Questions.md`). Column
    /// `category_code`.
    pub category_code: Option<String>,
    /// Activity is administrative only. Exactly one activity box should be
    /// checked per row (fields 26–31, "One Act/Event required, but only one
    /// may be selected"). Column `administrative_voter_drive_activity`
    /// (field 26, "YES/NO (Activity is Administrative - Only)"; the
    /// "voter_drive" in the column name looks like a misnomer, fec-docs
    /// `wiki/Open-Questions.md`); v5.0–5.1 also `admin_voter_drive_activity`.
    pub administrative_activity: bool,
    /// Activity is direct fundraising. Column `fundraising_activity`
    /// (field 27).
    pub fundraising_activity: bool,
    /// Activity is an exempt activity. Column `exempt_activity` (field 28).
    pub exempt_activity: bool,
    /// Activity is a generic voter drive only. Column
    /// `generic_voter_drive_activity` (field 29).
    pub generic_voter_drive_activity: bool,
    /// Activity is direct candidate support. Column
    /// `direct_candidate_support_activity` (field 30).
    pub direct_candidate_support_activity: bool,
    /// Activity is a public communication referring only to a party, made by
    /// a PAC. Column `public_communications_party_activity` (field 31).
    pub public_communications_party_activity: bool,
    /// True for a memo entry, not counted in totals: a payment covering
    /// several activity types is a memo entry followed by its breakdown
    /// ([fecfrm3xi.pdf p30](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=30)).
    /// In the corpus most memo rows are instead the vendors behind a
    /// reimbursement, back-referenced to it. Column `memo_code`, `X` when
    /// true (field 32).
    pub memo: bool,
    /// Column `memo_text` (field 33).
    pub memo_text: Option<String>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl ScheduleH4 {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id_number"),
            back_reference_transaction_id: text(data, "back_reference_tran_id_number"),
            back_reference_schedule_name: text(data, "back_reference_sched_name"),
            payee: Entity::from_prefixed(data, "payee_", "payee_name"),
            activity_event_identifier: text(data, "account_identifier"),
            expenditure_date: date(data, "expenditure_date"),
            total_amount: amount(data, "total_amount"),
            federal_share: amount(data, "federal_share"),
            nonfederal_share: amount(data, "nonfederal_share"),
            event_year_to_date: amount_opt(data, "event_year_to_date"),
            expenditure_purpose_code: text(data, "expenditure_purpose_code"),
            expenditure_purpose_description: text(data, "expenditure_purpose_description"),
            category_code: text(data, "category_code"),
            administrative_activity: flag(data, "administrative_voter_drive_activity")
                || flag(data, "admin_voter_drive_activity"),
            fundraising_activity: flag(data, "fundraising_activity"),
            exempt_activity: flag(data, "exempt_activity"),
            generic_voter_drive_activity: flag(data, "generic_voter_drive_activity"),
            direct_candidate_support_activity: flag(data, "direct_candidate_support_activity"),
            public_communications_party_activity: flag(
                data,
                "public_communications_party_activity",
            ),
            memo: flag(data, "memo_code"),
            memo_text: text(data, "memo_text"),
            image_number: text(data, "image_number"),
        })
    }

    /// The checked activity box's name: "Administrative", "Direct
    /// Fundraising", "Exempt Activity", "Generic Voter Drive", "Direct
    /// Candidate Support" or "Public Communications Referring Only to Party"
    /// (FEC format workbook v8.4, sheet `Sch H4`, fields 26–31). `None` unless
    /// exactly one box is checked.
    pub fn activity_label(&self) -> Option<&'static str> {
        only_one(&[
            (self.administrative_activity, "Administrative"),
            (self.fundraising_activity, "Direct Fundraising"),
            (self.exempt_activity, "Exempt Activity"),
            (self.generic_voter_drive_activity, "Generic Voter Drive"),
            (
                self.direct_candidate_support_activity,
                "Direct Candidate Support",
            ),
            (
                self.public_communications_party_activity,
                "Public Communications Referring Only to Party",
            ),
        ])
    }
}

/// The label of the one checked box, `None` for none or several.
pub(crate) fn only_one(boxes: &[(bool, &'static str)]) -> Option<&'static str> {
    let mut checked = boxes.iter().filter(|(on, _)| *on);
    match (checked.next(), checked.next()) {
        (Some((_, label)), None) => Some(label),
        _ => None,
    }
}
