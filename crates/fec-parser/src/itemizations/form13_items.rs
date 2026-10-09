//! Form 13 line items (inaugural committees): Schedule 13-A donations
//! accepted (`F132`) and Schedule 13-B refunds of donations (`F133`).

use jiff::civil::Date;

use crate::covers::fields::{amount, amount_opt, date, flag, text, text_or_empty, Fields};
use crate::itemizations::Entity;

/// The donor or refund recipient of a Form 13 row: [`Entity::from_prefixed`]
/// on `contributor_`, whose ZIP column is `contributor_zip` rather than
/// `contributor_zip_code` on these two records.
fn contributor<F: Fields + ?Sized>(data: &F) -> Entity {
    let mut entity = Entity::from_prefixed(data, "contributor_", "contributor_name");
    if entity.address.zip_code.is_none() {
        entity.address.zip_code = text(data, "contributor_zip");
    }
    entity
}

/// "SCHEDULE 13-A ITEMIZED DONATIONS ACCEPTED": one donation to an inaugural
/// committee, itemized "for each donation of money or anything of value
/// aggregating $200 or more", with "the aggregate total of donations
/// accepted to date from that donor" (instructions dated 10/2004)
/// ([fecfrm13.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13.pdf#page=2),
/// [fecfrm13i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=2)).
/// The `F132` record (FEC format workbook v8.4, sheet `F132`, "FORM 132 - FOR
/// EACH ITEMIZED DONATION ACCEPTED"). Itemized donations are added to those
/// itemized before and the sum goes on Line 5 of Form 13, so the cover total
/// is cumulative across filings
/// ([fecfrm13i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=2)).
/// Neither the printed schedule nor the record has employer or occupation.
///
/// # Versions
///
/// v6.1–8.5 share the 22-field layout. v5.2–5.3 put the transaction ID and
/// back reference at the end and have no memo columns (FEC format workbook
/// v5.3, sheet `F132`). Paper layouts have no transaction ID, back reference
/// or entity type but carry an `image_number` (and a memo code from P3.2).
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
pub struct Form13Donation {
    /// The row type as filed, `F132`. Column `form_type` (FEC format
    /// workbook v8.4, sheet `F132`, field 1).
    pub form_type: String,
    /// The inaugural committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transaction, unique for the life of the
    /// report; electronic-only. Column `transaction_id_number` (field 3).
    pub transaction_id: Option<String>,
    /// "Reference to the Tran ID of a Related Record". Column
    /// `back_reference_tran_id_number` (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// The schedule of that record (sample `F133`). Column
    /// `back_reference_sched_name` (field 5).
    pub back_reference_schedule_name: Option<String>,
    /// "Full Name (Last, First, Middle Initial) or Full Organization Name"
    /// and "Mailing Address" of the donor
    /// ([fecfrm13.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13.pdf#page=2)).
    /// Columns `entity_type`, `contributor_organization_name`,
    /// `contributor_last_name` … `contributor_suffix`, `contributor_street_1`
    /// … `contributor_zip` (fields 6–17).
    pub donor: Entity,
    /// "Date Donation Received". Column `donation_date` (field 18).
    pub donation_date: Option<Date>,
    /// "Amount of This Donation". Column `donation_amount` (field 19).
    pub donation_amount: f64,
    /// "Donor's Aggregate Donations To Date": a running figure per donor;
    /// sum `donation_amount`, not this. Column `donation_aggregate_amount`
    /// (field 20).
    pub donation_aggregate_amount: Option<f64>,
    /// True for a memo entry; electronic-only (no box on the printed
    /// schedule). Column `memo_code`, `X` when true (field 21).
    pub memo: bool,
    /// Column `memo_text_description` (field 22).
    pub memo_text: Option<String>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl Form13Donation {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id_number"),
            back_reference_transaction_id: text(data, "back_reference_tran_id_number"),
            back_reference_schedule_name: text(data, "back_reference_sched_name"),
            donor: contributor(data),
            donation_date: date(data, "donation_date"),
            donation_amount: amount(data, "donation_amount"),
            donation_aggregate_amount: amount_opt(data, "donation_aggregate_amount"),
            memo: flag(data, "memo_code"),
            memo_text: text(data, "memo_text_description"),
            image_number: text(data, "image_number"),
        })
    }
}

/// "SCHEDULE 13-B ITEMIZED REFUNDS OF DONATIONS": "for each refund of a
/// reported donation made by the committee, enter the payee's name and
/// address, the date the refund was made and the amount" (instructions dated
/// 10/2004)
/// ([fecfrm13.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13.pdf#page=3),
/// [fecfrm13i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=2)).
/// The `F133` record (FEC format workbook v8.4, sheet `F133`, "FORM 133 - FOR
/// EACH ITEMIZED REFUND OF DONATIONS"). Itemized refunds are added to those
/// itemized before and the sum goes on Line 6 of Form 13
/// ([fecfrm13i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13i.pdf#page=2)).
/// Unlike [`Form13Donation`] there is no aggregate.
///
/// # Versions
///
/// v6.1–8.5 share the 21-field layout. v5.2–5.3 put the transaction ID and
/// back reference at the end and have no memo columns (FEC format workbook
/// v5.3, sheet `F133`). Paper layouts have no transaction ID, back reference
/// or entity type but carry an `image_number` (and a memo code from P3.2).
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
pub struct Form13Refund {
    /// The row type as filed, `F133`. Column `form_type` (FEC format
    /// workbook v8.4, sheet `F133`, field 1).
    pub form_type: String,
    /// The inaugural committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transaction, unique for the life of the
    /// report; electronic-only. Column `transaction_id_number` (field 3).
    pub transaction_id: Option<String>,
    /// "Reference to the Tran ID of a Related Record". Column
    /// `back_reference_tran_id_number` (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// The schedule of that record (sample `F132`). Column
    /// `back_reference_sched_name` (field 5).
    pub back_reference_schedule_name: Option<String>,
    /// Who was refunded: "Full Name (Last, First, Middle Initial) or Full
    /// Organization Name" and "Mailing Address", in the `contributor_*`
    /// columns ([fecfrm13.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm13.pdf#page=3)).
    /// Columns `entity_type`, `contributor_organization_name`,
    /// `contributor_last_name` … `contributor_suffix`, `contributor_street_1`
    /// … `contributor_zip` (fields 6–17).
    pub recipient: Entity,
    /// "Date Refund Made". Column `refund_date` (field 18).
    pub refund_date: Option<Date>,
    /// "Amount of This Refund". Column `refund_amount` (field 19).
    pub refund_amount: f64,
    /// True for a memo entry; electronic-only. Column `memo_code`, `X` when
    /// true (field 20).
    pub memo: bool,
    /// Column `memo_text_description` (field 21).
    pub memo_text: Option<String>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl Form13Refund {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id_number"),
            back_reference_transaction_id: text(data, "back_reference_tran_id_number"),
            back_reference_schedule_name: text(data, "back_reference_sched_name"),
            recipient: contributor(data),
            refund_date: date(data, "refund_date"),
            refund_amount: amount(data, "refund_amount"),
            memo: flag(data, "memo_code"),
            memo_text: text(data, "memo_text_description"),
            image_number: text(data, "image_number"),
        })
    }
}
