//! Schedule H5: transfers of Levin funds.

use jiff::civil::Date;

use crate::covers::fields::{amount, date, text, text_or_empty, Fields};

/// "SCHEDULE H5 - TRANSFERS OF LEVIN FUNDS RECEIVED": one transfer of Levin
/// funds from a party committee's Levin or nonfederal account into its
/// federal (or allocation) account, to pay the Levin share of allocable
/// federal election activity
/// ([fecfrm3xi.pdf p31](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=31)).
/// State, district and local party committees only
/// ([fecfrm3x.pdf p17](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=17));
/// supports Form 3X Line 18(b), and the disbursements it pays for are on
/// Schedule H6 ([fecfrm3xi.pdf p31](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=31)).
///
/// Unlike Schedule H3, one row is the whole transfer, with its four-way
/// breakdown in columns (FEC format workbook v8.4, sheet `Sch H5`, fields
/// 6–10). There is no memo code or back reference.
///
/// # Versions
///
/// v6.1–8.5 have all 10 fields; v5.x have them in another order (transaction
/// ID last). Paper layouts have no transaction ID but an `image_number`.
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
pub struct ScheduleH5 {
    /// The row type as filed, `H5`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `Sch H5`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transfer, unique for the life of the report;
    /// electronic-only. Column `transaction_id` (field 3).
    pub transaction_id: Option<String>,
    /// "Name of Account" the Levin funds came from. Column `account_name`
    /// (field 4).
    pub account_name: Option<String>,
    /// Date of receipt. Column `receipt_date` (field 5).
    pub receipt_date: Option<Date>,
    /// The whole transfer. Column `total_amount_transferred` (field 6).
    pub total_amount_transferred: f64,
    /// Part transferred for voter registration. Column
    /// `voter_registration_amount` (field 7).
    pub voter_registration_amount: f64,
    /// Part transferred for voter ID. Column `voter_id_amount` (field 8).
    pub voter_id_amount: f64,
    /// Part transferred for get-out-the-vote. Column `gotv_amount` (field 9).
    pub gotv_amount: f64,
    /// Part transferred for generic campaign activity. Column
    /// `generic_campaign_amount` (field 10).
    pub generic_campaign_amount: f64,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl ScheduleH5 {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id"),
            account_name: text(data, "account_name"),
            receipt_date: date(data, "receipt_date"),
            total_amount_transferred: amount(data, "total_amount_transferred"),
            voter_registration_amount: amount(data, "voter_registration_amount"),
            voter_id_amount: amount(data, "voter_id_amount"),
            gotv_amount: amount(data, "gotv_amount"),
            generic_campaign_amount: amount(data, "generic_campaign_amount"),
            image_number: text(data, "image_number"),
        })
    }
}
