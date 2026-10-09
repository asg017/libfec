//! Schedule H3: transfers from nonfederal accounts.

use jiff::civil::Date;

use crate::covers::fields::{amount, amount_opt, date, text, text_or_empty, Fields};

/// "SCHEDULE H3 - TRANSFERS FROM NONFEDERAL ACCOUNTS": one line of a
/// transfer from a committee's nonfederal account into its federal (or
/// allocation) account to pay the nonfederal share of allocated expenses
/// ([fecfrm3xi.pdf p28](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=28)).
/// Supports only Form 3X Line 18(a); Levin fund transfers are on Schedule H5
/// instead (same page).
///
/// On paper one transfer is one block broken down on lines i)–vi);
/// electronically **each breakdown line is its own row**, typed by
/// `event_type`. One `AD` row is required per transfer and every row's
/// `back_reference_transaction_id` points to it; `account_name`,
/// `receipt_date` and `total_amount_transferred` repeat on every row of the
/// transfer, while `transferred_amount` is the row's own breakdown amount
/// (FEC format workbook v8.4, sheet `Sch H3`, fields 4–10). Sum
/// `transferred_amount`, not `total_amount_transferred`.
///
/// # Versions
///
/// v6.1–8.5 have all 10 fields. v3–5.x have the same fields in another order
/// (transaction ID last). v1–2 and paper layouts have no breakdown rows (v1
/// lists events in columns of one row) and are read only for the transfer
/// itself; paper layouts have no transaction IDs but an `image_number`.
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
pub struct ScheduleH3 {
    /// The row type as filed, `H3`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `Sch H3`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this row, unique for the life of the report;
    /// electronic-only. Column `transaction_id` (field 3).
    pub transaction_id: Option<String>,
    /// The `transaction_id` of the transfer's `AD` row (on the `AD` row
    /// itself, usually its own ID). Column `back_reference_tran_id_number`
    /// (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// "Name of Account": the nonfederal account the money came from.
    /// Repeated on every row of the transfer. Column `account_name`
    /// (field 5).
    pub account_name: Option<String>,
    /// What this row's amount pays for: `AD`, `GV`, `EA`, `DF`, `DC` or `PC`
    /// (see [`ScheduleH3::event_type_label`]). Column `event_type` (field 6).
    pub event_type: Option<String>,
    /// For `DF` and `DC` rows, the Schedule H2 activity or event identifier.
    /// Column `event_activity_name` (field 7).
    pub event_activity_name: Option<String>,
    /// Date of receipt of the transfer, repeated on every row. Column
    /// `receipt_date` (field 8).
    pub receipt_date: Option<Date>,
    /// The whole transfer, repeated on every row of it, so summing it over
    /// rows overstates the transfer. Column `total_amount_transferred`
    /// (field 9).
    pub total_amount_transferred: Option<f64>,
    /// This row's breakdown amount (FEC format workbook v8.4, sheet
    /// `Sch H3`, field 10, "Breakdown Amt for Items: i, ii, iii, iv)a), …").
    /// Column `transferred_amount`; absent in v1–2.
    pub transferred_amount: f64,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl ScheduleH3 {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id"),
            back_reference_transaction_id: text(data, "back_reference_tran_id_number"),
            account_name: text(data, "account_name"),
            event_type: text(data, "event_type"),
            event_activity_name: text(data, "event_activity_name"),
            receipt_date: date(data, "receipt_date"),
            total_amount_transferred: amount_opt(data, "total_amount_transferred"),
            transferred_amount: amount(data, "transferred_amount"),
            image_number: text(data, "image_number"),
        })
    }

    /// The FEC's name for `event_type`: `AD` Administrative, `GV` Generic
    /// Voter Drive, `DF` Direct Fundraising, `DC` Direct Candidate Support,
    /// `EA` Exempt Activities, `PC` Public Communications Referring Only to
    /// Party (made by PAC) (FEC format workbook v8.4, sheet `Sch H3`,
    /// field 6). `None` for any other value.
    pub fn event_type_label(&self) -> Option<&'static str> {
        Some(
            match self
                .event_type
                .as_deref()?
                .trim()
                .to_ascii_uppercase()
                .as_str()
            {
                "AD" => "Administrative",
                "GV" => "Generic Voter Drive",
                "DF" => "Direct Fundraising",
                "DC" => "Direct Candidate Support",
                "EA" => "Exempt Activities",
                "PC" => "Public Communications Referring Only to Party (made by PAC)",
                _ => return None,
            },
        )
    }
}
