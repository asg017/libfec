//! Schedule H6: disbursements of federal and Levin funds.

use jiff::civil::Date;

use crate::covers::fields::{amount, amount_opt, date, flag, text, text_or_empty, Fields};
use crate::itemizations::schedule_h4::only_one;
use crate::itemizations::Entity;

/// "SCHEDULE H6 - DISBURSEMENTS OF FEDERAL AND LEVIN FUNDS": one payment for
/// allocable federal election activity that a party committee splits between
/// federal funds and Levin funds, with the total, the federal share and the
/// Levin share
/// ([fecfrm3xi.pdf p32](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=32)).
/// The activity is voter registration, voter identification,
/// get-out-the-vote or generic campaign activity
/// ([fecfrm3xi.pdf p25](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=25)).
/// Supports Form 3X Line 30(a): the federal share goes to Line 30(a)(i), the
/// Levin share to 30(a)(ii), every such payment itemized regardless of amount
/// ([fecfrm3xi.pdf p32](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=32));
/// the shares follow the Schedule H1 percentage (same page).
///
/// A payment covering several activities is a memo entry followed by a
/// breakdown (same page), so leave out `memo` rows when summing.
///
/// # Versions
///
/// v8.0–8.5 have all 31 fields; v6.1–7.0 add `expenditure_purpose_code`.
/// v5.x have one combined `payee_name`, split per [`Entity::from_prefixed`],
/// and also carry candidate, committee and conduit columns that are not
/// read. Paper layouts have no transaction IDs, entity type or activity
/// identifier but an `image_number`.
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
pub struct ScheduleH6 {
    /// The row type as filed, `H6`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `Sch H6`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transaction, unique for the life of the
    /// report; electronic-only. Column `transaction_id_number` (field 3).
    pub transaction_id: Option<String>,
    /// The `transaction_id` of the record this one is a memo or child of.
    /// Column `back_reference_tran_id_number` (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// The schedule of that parent record. Column `back_reference_sched_name`
    /// (field 5).
    pub back_reference_schedule_name: Option<String>,
    /// Who was paid: entity type, organization or person, mailing address.
    /// Columns `entity_type`, `payee_organization_name`, `payee_last_name`
    /// … `payee_suffix`, `payee_street_1` … `payee_zip_code` (fields 6–17);
    /// v5.x `payee_name`.
    pub payee: Entity,
    /// "Account/Event Identifier" (no box for it on the paper form). Column
    /// `account_identifier` (field 18).
    pub activity_event_identifier: Option<String>,
    /// Date of the disbursement. Column `expenditure_date` (field 19).
    pub expenditure_date: Option<Date>,
    /// The whole payment, federal plus Levin. Column `total_amount`
    /// (field 20, "TOTAL FED-LEVIN AMOUNT").
    pub total_amount: f64,
    /// The federal share. Column `federal_share` (field 21).
    pub federal_share: f64,
    /// The Levin funds share. Column `levin_share` (field 22).
    pub levin_share: f64,
    /// Year-to-date total for the category of activity, not for the payee
    /// ([fecfrm3xi.pdf p32](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=32)).
    /// Column `event_year_to_date` (field 23).
    pub event_year_to_date: Option<f64>,
    /// A coded purpose, v5.x–7.0 only (dropped in v8.0). Column
    /// `expenditure_purpose_code`.
    pub expenditure_purpose_code: Option<String>,
    /// "Purpose of Disbursement". Column `expenditure_purpose_description`
    /// (field 24).
    pub expenditure_purpose_description: Option<String>,
    /// Disbursement category, `001`…, kept raw: the workbook allows 001–010
    /// (field 25) and the H6 instructions list none
    /// ([fecfrm3xi.pdf p32](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=32)).
    /// Column `category_code`.
    pub category_code: Option<String>,
    /// Activity is voter registration. Exactly one activity box should be
    /// checked per row (fields 26–29, "One Act/Event required, but only one
    /// may be selected"). Column `voter_registration_activity` (field 26).
    pub voter_registration_activity: bool,
    /// Activity is get-out-the-vote. Column `gotv_activity` (field 27).
    pub gotv_activity: bool,
    /// Activity is voter ID. Column `voter_id_activity` (field 28).
    pub voter_id_activity: bool,
    /// Activity is generic campaign activity. Column
    /// `generic_campaign_activity` (field 29).
    pub generic_campaign_activity: bool,
    /// True for a memo entry, not counted in totals. Column `memo_code`, `X`
    /// when true (field 30).
    pub memo: bool,
    /// Column `memo_text` (field 31).
    pub memo_text: Option<String>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl ScheduleH6 {
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
            levin_share: amount(data, "levin_share"),
            event_year_to_date: amount_opt(data, "event_year_to_date"),
            expenditure_purpose_code: text(data, "expenditure_purpose_code"),
            expenditure_purpose_description: text(data, "expenditure_purpose_description"),
            category_code: text(data, "category_code"),
            voter_registration_activity: flag(data, "voter_registration_activity"),
            gotv_activity: flag(data, "gotv_activity"),
            voter_id_activity: flag(data, "voter_id_activity"),
            generic_campaign_activity: flag(data, "generic_campaign_activity"),
            memo: flag(data, "memo_code"),
            memo_text: text(data, "memo_text"),
            image_number: text(data, "image_number"),
        })
    }

    /// The checked activity box's name: "Voter Registration", "GOTV", "Voter
    /// ID" or "Generic Campaign" (FEC format workbook v8.4, sheet `Sch H6`,
    /// fields 26–29). `None` unless exactly one box is checked.
    pub fn activity_label(&self) -> Option<&'static str> {
        only_one(&[
            (self.voter_registration_activity, "Voter Registration"),
            (self.gotv_activity, "GOTV"),
            (self.voter_id_activity, "Voter ID"),
            (self.generic_campaign_activity, "Generic Campaign"),
        ])
    }
}
