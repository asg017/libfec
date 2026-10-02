//! Form 6: 48-hour notice of contributions/loans received.

use crate::covers::fields::person_name_or_legacy;
use crate::covers::fields::{date, text, text_or_empty, Data};
use crate::covers::{Address, PersonName};
use jiff::civil::Date;

/// "FORM 6 - 48 HOUR NOTICE": the 48-Hour Notice of Contributions/Loans
/// Received, printed with "To be used to report all contributions (including
/// loans) of $1000 or more, received within 20 days of the election" (form
/// revised 03/2016,
/// [fecfrm6.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm6.pdf#page=1)).
/// Form types `F6N` (new) and `F6A` (amendment).
///
/// **Who files.** "Principal campaign committees must file 48-hour notices of
/// contributions of $1,000 or more received after the 20th day, but more than
/// 48 hours, before 12:01 a.m. of the day of any election in which the
/// candidate participates"; it applies to all types of elections "and even
/// when a candidate is unopposed", and to loans, guarantees, advances and the
/// candidate's own contributions and loans as well as ordinary contributions
/// (instructions revised 03/2016,
/// [fecfrm6i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm6i.pdf#page=1)).
///
/// **What the record holds.** This cover is the notice's header, Lines 1-5:
/// the committee, the candidate, the office sought, the committee's FEC ID and
/// whether this is an amendment, plus the signer. It has **no dollar total**;
/// the contributions themselves are the filing's `F65` rows (FEC format
/// workbook v8.4, sheets F6 and F65). "The committee must itemize the
/// contributions and loans a second time in the first report filed after the
/// election"
/// ([fecfrm6i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm6i.pdf#page=1)),
/// so Form 6 amounts and regular-report receipts overlap.
///
/// # Versions
///
/// v8.0–8.5 carry all 24 fields. v6.1–7.0 lack `original_amendment_date`. v3
/// and v5.x give the candidate as one caret-delimited `candidate_name` (read
/// into [`Form6Candidate::name`]) and have no signer name columns.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Form6 {
    /// Form type as filed: `F6N` or `F6A`. Column `form_type` (FEC format
    /// workbook v8.4, sheet F6, field 1).
    pub form_type: String,
    /// Line 4, the committee's FEC identification number. Column
    /// `filer_committee_id_number` (FEC format workbook v8.4, sheet F6,
    /// field 2;
    /// [fecfrm6.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm6.pdf#page=1)).
    pub filer_committee_id: String,
    /// Line 5, on an amendment "the POSTed Filed on Date of Original (1st) F6
    /// being amended" (the paper form: "YES, IT AMENDS THE NOTICE FILED ON").
    /// Column `original_amendment_date` (FEC format workbook v8.4, sheet F6,
    /// field 3).
    pub original_amendment_date: Option<Date>,
    /// Line 1, "NAME OF COMMITTEE IN FULL". Column `committee_name` (FEC
    /// format workbook v8.4, sheet F6, field 4).
    pub committee_name: String,
    /// Line 1, the committee's address. Columns `street_1`, `street_2`,
    /// `city`, `state`, `zip_code` (FEC format workbook v8.4, sheet F6,
    /// fields 5-9).
    pub address: Address,
    /// Lines 2-3, the candidate and the office sought.
    pub candidate: Form6Candidate,
    /// The person who signed the notice. The paper form marks the signature
    /// "(optional)", but the workbook flags a missing signer last/first name
    /// as an error. Columns `signer_last_name`, `signer_first_name`,
    /// `signer_middle_name`, `signer_prefix`, `signer_suffix` (FEC format
    /// workbook v8.4, sheet F6, fields 19-23;
    /// [fecfrm6.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm6.pdf#page=1)).
    /// Empty on v3/v5.x filings.
    pub signer: PersonName,
    /// Date signed. Column `date_signed` (FEC format workbook v8.4, sheet F6,
    /// field 24).
    pub date_signed: Option<Date>,
}

/// The candidate on a Form 6 (Lines 2-3).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Form6Candidate {
    /// The candidate's FEC ID. Electronic-format only: no candidate ID box is
    /// printed on Form 6 (its Line 4 ID is the committee's). Column
    /// `candidate_id_number` (FEC format workbook v8.4, sheet F6, field 10).
    pub candidate_id: Option<String>,
    /// Line 2, "NAME OF CANDIDATE". Columns `candidate_last_name`,
    /// `candidate_first_name`, `candidate_middle_name`, `candidate_prefix`,
    /// `candidate_suffix` (FEC format workbook v8.4, sheet F6, fields 11-15);
    /// the caret-delimited `candidate_name` in v3/v5.x.
    pub name: PersonName,
    /// Line 3, office sought: `H`, `S` or `P`. Column `candidate_office` (FEC
    /// format workbook v8.4, sheet F6, field 16). See
    /// [`Form6Candidate::office_label`].
    pub office: Option<String>,
    /// Line 3, state of the office sought. Column `candidate_state` (FEC
    /// format workbook v8.4, sheet F6, field 17).
    pub state: Option<String>,
    /// Line 3, district of the office sought (`01` ... `99`). Column
    /// `candidate_district` (FEC format workbook v8.4, sheet F6, field 18).
    pub district: Option<String>,
}

impl Form6 {
    pub fn from_data(data: &Data) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            original_amendment_date: date(data, "original_amendment_date"),
            committee_name: text_or_empty(data, "committee_name"),
            address: Address::from_prefixed(data, ""),
            candidate: Form6Candidate {
                candidate_id: text(data, "candidate_id_number"),
                name: person_name_or_legacy(data, "candidate_", "candidate_name"),
                office: text(data, "candidate_office"),
                state: text(data, "candidate_state"),
                district: text(data, "candidate_district"),
            },
            signer: PersonName::from_prefixed(data, "signer_"),
            date_signed: date(data, "date_signed"),
        })
    }

    /// True for an amendment (`F6A`).
    pub fn is_amendment(&self) -> bool {
        self.form_type.to_ascii_uppercase().ends_with('A')
    }
}

impl Form6Candidate {
    /// "House", "Senate" or "President" for the office codes `H`, `S`, `P`.
    ///
    /// The workbook lists the codes `H,S,P` (FEC format workbook v8.4, sheet
    /// F6, field 16) without descriptions; the names are the office-sought
    /// boxes FEC forms print, "House", "Senate", "President"
    /// ([fecfrm1.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=2)).
    pub fn office_label(&self) -> Option<&'static str> {
        office_label(self.office.as_deref()?)
    }
}

/// Office-sought label for `H` / `S` / `P` (see [`Form6Candidate::office_label`]).
pub(crate) fn office_label(code: &str) -> Option<&'static str> {
    match code.trim().to_ascii_uppercase().as_str() {
        "H" => Some("House"),
        "S" => Some("Senate"),
        "P" => Some("President"),
        _ => None,
    }
}
