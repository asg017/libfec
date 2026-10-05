//! Form 7 line items: the communications on a report of communication costs
//! (`F76`).

use jiff::civil::Date;

use crate::covers::fields::{amount, date, text, text_or_empty, Fields};
use crate::itemizations::support_oppose_label;
use crate::itemizations::CandidateRef;

/// One line of Form 7's "Summary of Communication Costs": a communication a
/// corporation or membership organization made to its restricted class, with
/// its type, the class it went to, its date, the candidate it supported or
/// opposed and its cost
/// ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1),
/// [fecfrm7i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7i.pdf#page=1)).
/// The `F76` record (FEC format workbook v8.4, sheet `F76`, "FORM 7.6 - FOR
/// EACH COMMUNICATION").
///
/// The cost is **per candidate**: a communication advocating several
/// candidates is allocated among them "in equal proportions" unless some are
/// emphasized (instructions revised 2/2001)
/// ([fecfrm7i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7i.pdf#page=1)),
/// so one mailing can be several rows. The cover's `total_costs` is the sum
/// of these rows (FEC format workbook v8.4, sheet `F7`, field 15).
///
/// # Versions
///
/// v6.1–8.5 share the 20-field layout. v1–5.x (FEC format workbook v5.3,
/// sheet `F76`) have one combined `candidate_name` (split per
/// [`CandidateRef::from_prefixed`]), no election-other description, and the
/// transaction ID at the end (none in v1); their `amended_cd` is ignored, as
/// the v5.3 specification says (`FEC_v530.rtf`, "Amend Code … Unused
/// Field"). Paper layouts have no transaction ID or candidate ID but carry
/// an `image_number`.
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
pub struct Form7Communication {
    /// The row type as filed, `F76`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `F76`, field 1).
    pub form_type: String,
    /// The filing organization's FEC ID ("2. Identification Number").
    /// Column `filer_committee_id_number` (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transaction, unique for the life of the
    /// report; electronic-only. Column `transaction_id` (field 3).
    pub transaction_id: Option<String>,
    /// "Type of Communication": the workbook allows `DM`, `TP`, `TM`, `O`
    /// (field 4) and the form prints Direct Mail, Telephone, Telegram and
    /// Other ([fecfrm7.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm7.pdf#page=1)),
    /// but no source pairs code with box, so the code is kept raw. Column
    /// `communication_type`.
    pub communication_type: Option<String>,
    /// The type when it is "Other:". Column `communication_type_description`
    /// (field 5).
    pub communication_type_description: Option<String>,
    /// "Class or Category Communicated With": the workbook allows `E`, `S`,
    /// `M` (field 6) and the form prints Executive/Administrative
    /// Personnel, Stockholders and Members; no source pairs code with box,
    /// so the code is kept raw. Column `communication_class`.
    pub communication_class: Option<String>,
    /// "Date(s) of Communication". Column `communication_date` (field 7).
    pub communication_date: Option<Date>,
    /// "Cost of Communication (Per Candidate)". Column `communication_cost`
    /// (field 8).
    pub communication_cost: f64,
    /// The candidate's "Primary or General Election", a letter and a year
    /// (`G,P,O[CCYY]`). See [`Form7Communication::election_code_label`].
    /// Column `election_code` (field 9).
    pub election_code: Option<String>,
    /// Required when `election_code` is `O…` (Other); v6.1+. Column
    /// `election_other_description` (field 10).
    pub election_other_description: Option<String>,
    /// `S` or `O`, the printed "Check One: Support Oppose". See
    /// [`Form7Communication::support_oppose_label`]. Column
    /// `support_oppose_code` (field 11).
    pub support_oppose_code: Option<String>,
    /// "Identify Candidate, Office Sought, District and State", plus the
    /// candidate's FEC ID (electronic-only). Columns `candidate_id_number`,
    /// `candidate_last_name` … `candidate_district` (fields 12–20); v1–5.x
    /// `candidate_name`.
    pub candidate: CandidateRef,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl Form7Communication {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id"),
            communication_type: text(data, "communication_type"),
            communication_type_description: text(data, "communication_type_description"),
            communication_class: text(data, "communication_class"),
            communication_date: date(data, "communication_date"),
            communication_cost: amount(data, "communication_cost"),
            election_code: text(data, "election_code"),
            election_other_description: text(data, "election_other_description"),
            support_oppose_code: text(data, "support_oppose_code"),
            candidate: CandidateRef::from_prefixed(data, "candidate_id_number", "candidate_"),
            image_number: text(data, "image_number"),
        })
    }

    /// "Primary", "General", … for `election_code`; see
    /// [`crate::covers::election_code_label`].
    pub fn election_code_label(&self) -> Option<&'static str> {
        crate::covers::election_code_label(self.election_code.as_deref()?)
    }

    /// "Support" or "Oppose" for `support_oppose_code`, as
    /// [`crate::itemizations::Form5Expenditure::support_oppose_label`].
    pub fn support_oppose_label(&self) -> Option<&'static str> {
        support_oppose_label(self.support_oppose_code.as_deref()?)
    }
}
