//! `TEXT`: free text attached to a report, schedule or itemization.

use crate::covers::fields::{text, text_or_empty, Fields};
use crate::itemizations::record_family;

/// "TEXT - MISC. TEXT RELATED TO A REPORT, SCHEDULE OR ITEMIZATION": a block
/// of free text, up to 4,000 characters, attached to the filing's form record
/// or to one itemized transaction (FEC format workbook v8.4, sheet `Text`,
/// fields 4–6). Not an FEC form: there is no printed counterpart.
///
/// # Linking to the record it describes
///
/// `back_reference_schedule_name` is "the REC TYPE of the form or schedule
/// to which this text record is related" (`F3XN`, `SB21B`, `SC/10`). If
/// `back_reference_transaction_id` is supplied "it must exist within file
/// along with corresponding Schedule Name"; when the text refers to the form
/// itself it "must be blank" (workbook sheet `Text`, fields 4–5). So join on
/// `back_reference_transaction_id` = the `transaction_id` of a record in the
/// same filing; a blank one means the text is about the whole report (or, in
/// a few filings, a whole schedule line).
///
/// # Versions
///
/// v6.1–8.5 have all 6 fields. v3 and v5.x have 4 — `rec_type`,
/// `back_reference_sched_form_name`, `back_reference_tran_id_number`, `text`,
/// in that order — with no filer ID (read as `""`) and no transaction ID of
/// their own. The legacy `[BEGINTEXT]` … `[ENDTEXT]` blocks are not `TEXT`
/// records: [`crate::Filing::next_row`] skips them, and the one on a Form 99
/// cover is read into [`crate::covers::Form99::text`].
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TextRecord {
    /// The row type as filed, `TEXT`. Column `rec_type` (FEC format workbook
    /// v8.4, sheet `Text`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID; `""` in v3–5.x layouts, which have no
    /// such column. Column `filer_committee_id_number` (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this text record, unique for the life of the
    /// report; v6.1+ only. Column `transaction_id_number` (field 3).
    pub transaction_id: Option<String>,
    /// The `transaction_id` of the record this text is about; blank when it
    /// is about the form. Column `back_reference_tran_id_number` (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// The row type of the form or schedule this text is about (`F3XN`,
    /// `SA11AI`, `SC/10`). Column `back_reference_sched_form_name` (field 5).
    pub back_reference_schedule_name: Option<String>,
    /// The text: "Unformatted Text {text may not contain formatting
    /// characters such as tabs and line-feeds}". Column `text` (field 6).
    pub text: Option<String>,
}

impl TextRecord {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "rec_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id_number"),
            back_reference_transaction_id: text(data, "back_reference_tran_id_number"),
            back_reference_schedule_name: text(data, "back_reference_sched_form_name"),
            text: text(data, "text"),
        })
    }

    /// The record family of the record this text is about (`SA` for
    /// `SA11AI`, `SC` for `SC/10`; see [`record_family`]), or `None` when it
    /// is about a cover (`F3XN`) or the back reference is blank.
    pub fn back_reference_family(&self) -> Option<&'static str> {
        record_family(self.back_reference_schedule_name.as_deref()?)
    }
}
