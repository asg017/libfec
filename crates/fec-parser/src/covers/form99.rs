//! Form 99, the Miscellaneous Electronic Submission (`F99` cover records).

use crate::covers::fields::{date, flag, person_name_or_legacy, text, text_or_empty, Data};
use crate::covers::{Address, PersonName};
use jiff::civil::Date;

/// FEC Form 99, "Miscellaneous Text" / "Miscellaneous Electronic Submission".
///
/// A free-text letter from a committee to the FEC. The format workbook titles
/// the record "FORM 99 - MISCELLANEOUS TEXT" (FEC format workbook v8.4, sheet
/// F99); the Commission's guides call it a "Miscellaneous Electronic
/// Submission" that "may be used for narrative responses that do not affect
/// actual entries within a report"
/// ([candgui.pdf p150](https://www.fec.gov/resources/cms-content/documents/policy-guidance/candgui.pdf#page=150)).
/// Uses the guides name include a treasurer's resignation where no
/// replacement is named
/// ([candgui.pdf p21](https://www.fec.gov/resources/cms-content/documents/policy-guidance/candgui.pdf#page=21)),
/// notice of a change in filing frequency
/// ([partygui.pdf p159](https://www.fec.gov/resources/cms-content/documents/policy-guidance/partygui.pdf#page=159))
/// and responses to requests for additional information
/// ([RAD_FAQ-Candidate_Committees_last_visited_may_5_2021.pdf p16](https://www.fec.gov/resources/cms-content/documents/policy-guidance/RAD_FAQ-Candidate_Committees_last_visited_may_5_2021.pdf#page=16)).
/// There is no paper form, and no financial fields.
///
/// # Where the text lives
///
/// The delimited record is only the header fields (committee, address,
/// treasurer, date, text code). The workbook says the *next line* of the file
/// is `[BEGINTEXT]`, everything after it is "the body of the text message",
/// which "may include formatting, such as carriage return and line feed and
/// tabs but may not exceed 20,000 characters", and the last line is
/// `[ENDTEXT]` (FEC format workbook v8.4, sheet F99, notes after field 15).
/// Every F99 in the libfec test corpus follows that layout; the trailing
/// `text` column that `fec-parser`'s mapping adds to the record was blank in
/// all of them. [`crate::Filing::from_reader`] therefore reads the
/// `[BEGINTEXT]` block right after an F99 cover and stores it in
/// [`text`](Self::text).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Form99 {
    /// Form type as filed: `F99`. Other covers carry an amendment-indicator
    /// suffix here (see [`crate::covers::base_form_type`]); Form 99 has
    /// none, so [`is_amendment`](Self::is_amendment) is always false. Column
    /// `form_type` (FEC format workbook v8.4, sheet `F99`, field 1).
    pub form_type: String,

    /// Filer's FEC committee ID. Column `filer_committee_id_number` (FEC
    /// format workbook v8.4, sheet F99, field 2).
    pub filer_committee_id: String,

    /// Filer's committee name. Column `committee_name` (FEC format workbook
    /// v8.4, sheet F99, field 3).
    pub committee_name: String,

    /// Committee's mailing address. Columns `street_1`, `street_2`, `city`,
    /// `state`, `zip_code` (FEC format workbook v8.4, sheet F99, fields 4–8).
    pub address: Address,

    /// Treasurer's name. Columns `treasurer_last_name`,
    /// `treasurer_first_name`, `treasurer_middle_name`, `treasurer_prefix`,
    /// `treasurer_suffix` (FEC format workbook v8.4, sheet F99, fields 9–13).
    /// For format 5.x and earlier, whose single caret-delimited
    /// `treasurer_name` column holds the whole name, that value is split into
    /// parts.
    pub treasurer: PersonName,

    /// Date signed, `YYYYMMDD`. Column `date_signed` (FEC format workbook
    /// v8.4, sheet F99, field 14). Optional in the workbook.
    pub date_signed: Option<Date>,

    /// Type of miscellaneous document. Column `text_code` (FEC format
    /// workbook v8.4, sheet F99, field 15), e.g. `MST`. Optional, and absent
    /// from many 8.4 filings. See [`text_code_label`](Self::text_code_label).
    pub text_code: Option<String>,

    /// Column `filing_frequency`, format 8.5 only. Values seen in real
    /// filings are `M` and `Q`. No 8.5 format workbook is in the indexed FEC
    /// sources, so its meaning and code list are **not sourced**; the guides
    /// do say a change in filing frequency is noticed on Form 99
    /// ([partygui.pdf p159](https://www.fec.gov/resources/cms-content/documents/policy-guidance/partygui.pdf#page=159)).
    pub filing_frequency: Option<String>,

    /// Column `pdf_attachment`, format 8.5 only, `X` read as `true`. Like
    /// [`filing_frequency`](Self::filing_frequency) it is **not sourced**:
    /// the name suggests the submission came with a PDF, but no indexed FEC
    /// document says so.
    pub pdf_attachment: bool,

    /// The body of the message: the lines between `[BEGINTEXT]` and
    /// `[ENDTEXT]` that follow the record (see the struct docs), joined with
    /// `\n`, blank lines kept. Falls back to the record's own `text` column
    /// if that is filled. `None` if neither has any non-blank text.
    ///
    /// The file is read without CSV quoting, so `"` characters in the body
    /// are kept as written.
    pub text: Option<String>,
}

impl Form99 {
    /// Build from a cover record's `column -> value` map. [`text`](Self::text)
    /// is only the record's own `text` column here; the `[BEGINTEXT]` body is
    /// filled in by [`crate::Filing::from_reader`]. Returns `None` if the
    /// record has neither a committee ID nor a committee name.
    pub fn from_data(data: &Data) -> Option<Self> {
        let filer_committee_id = text_or_empty(data, "filer_committee_id_number");
        let committee_name = text_or_empty(data, "committee_name");
        if filer_committee_id.is_empty() && committee_name.is_empty() {
            return None;
        }
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id,
            committee_name,
            address: Address::from_prefixed(data, ""),
            treasurer: person_name_or_legacy(data, "treasurer_", "treasurer_name"),
            date_signed: date(data, "date_signed"),
            text_code: text(data, "text_code"),
            filing_frequency: text(data, "filing_frequency"),
            pdf_attachment: flag(data, "pdf_attachment"),
            text: text(data, "text"),
        })
    }

    /// Always false: Form 99 has no amendment indicator (`F99` takes no
    /// suffix). Present so every cover struct answers the question the same
    /// way.
    pub fn is_amendment(&self) -> bool {
        crate::covers::is_amendment_form_type(&self.form_type)
    }

    /// The FEC's description of [`text_code`](Self::text_code):
    /// `MSI` = "Disavowal Response", `MSM` = "Filing Freq. Change Notice",
    /// `MST` = "Misc. Report to the FEC" (FEC format workbook v8.4, sheet F99,
    /// field 15; the same list appears in the 7.0–8.3 workbooks). Real filings
    /// also use `MSW` and `MSR`, which no indexed FEC source defines; those
    /// return `None`. Case-insensitive.
    pub fn text_code_label(&self) -> Option<&'static str> {
        match self
            .text_code
            .as_deref()?
            .trim()
            .to_ascii_uppercase()
            .as_str()
        {
            "MSI" => Some("Disavowal Response"),
            "MSM" => Some("Filing Freq. Change Notice"),
            "MST" => Some("Misc. Report to the FEC"),
            _ => None,
        }
    }
}
