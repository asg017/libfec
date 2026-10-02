//! Form 2, the Statement of Candidacy (`F2N` / `F2A` cover records).

use crate::covers::fields::{
    amount_opt, date, flag, person_name_or_legacy, text, text_or_empty, Data,
};
use crate::covers::{Address, PersonName};
use jiff::civil::Date;

/// FEC Form 2, "Statement of Candidacy".
///
/// The form a federal candidate files to register a candidacy and to
/// designate a principal campaign committee and, optionally, other authorized
/// committees that may receive and spend funds on the candidate's behalf
/// ([fecfrm2.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm2.pdf#page=1)).
/// Each individual who becomes a candidate for federal office must file it
/// within 15 days of becoming a candidate, i.e. of receiving contributions or
/// making expenditures aggregating in excess of $5,000
/// ([fecfrm2i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm2i.pdf#page=1)).
///
/// The filer is the *candidate*, not a committee: the record's ID is a
/// candidate ID, and the candidate (not a treasurer) signs it
/// ([fecfrm2.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm2.pdf#page=1);
/// FEC format workbook v8.4, sheet F2, field 2 and note before field 38).
/// The form carries no financial amounts.
///
/// The paper form's lines map onto this struct as follows: Lines 1–2
/// ([`candidate`](Self::candidate), [`candidate_address`](Self::candidate_address),
/// [`candidate_id`](Self::candidate_id)), Line 3 (new/amended, the form type
/// suffix), Lines 4–6 ([`party_code`](Self::party_code), [`office`](Self::office),
/// [`office_state`](Self::office_state), [`district`](Self::district)), Line 7
/// ([`election_year`](Self::election_year),
/// [`principal_committee`](Self::principal_committee)) and Line 8
/// ([`authorized_committee`](Self::authorized_committee)).
///
/// **Additional authorized committees are not here.** The `F2` record holds
/// only the first "other authorized committee"; any further ones are coded on
/// separate `F2S` records that follow the cover (FEC format workbook v8.4,
/// sheet F2, note before field 31; sheet F2S). Those are itemization-style
/// rows read with [`crate::Filing::next_row`], not part of the cover.
///
/// Column names are those `fec-parser` assigns for FEC format versions
/// 8.2–8.5 (the only F2 layout in the corpus for supported versions). In older
/// layouts (6.3 and earlier) both "candidate state" columns are named
/// `candidate_state`, so the second (office state) overwrites the first; see
/// [`office_state`](Self::office_state).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct Form2 {
    /// Form type as filed, e.g. `F2N`: the base form plus the
    /// amendment-indicator suffix (see [`crate::covers::base_form_type`]).
    /// Column `form_type` (FEC format workbook v8.4, sheet `F2`, field 1,
    /// value reference `F2+[N|A]`). The suffix is Line 3, "Is This Statement
    /// New (N) OR Amended (A)"
    /// ([fecfrm2.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm2.pdf#page=1)).
    pub form_type: String,

    /// FEC candidate identification number — Line 2. Column
    /// `candidate_id_number` (FEC format workbook v8.4, sheet F2, field 2,
    /// "FILER CANDIDATE ID NUMBER"). On paper, first-time candidates and
    /// candidates running for a different seat leave it blank and are
    /// assigned one; candidates running again for the same seat reuse theirs
    /// ([fecfrm2i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm2i.pdf#page=1)).
    pub candidate_id: String,

    /// Candidate's name — Line 1(a), "Name of Candidate (in full)". Columns
    /// `candidate_last_name`, `candidate_first_name`, `candidate_middle_name`,
    /// `candidate_prefix`, `candidate_suffix` (FEC format workbook v8.4,
    /// sheet F2, fields 3–7). The single caret-delimited `candidate_name`
    /// column of v3/v5.x formats is split into parts.
    pub candidate: PersonName,

    /// Candidate's mailing address — Line 1(b)–(c). Columns
    /// `candidate_street_1`, `candidate_street_2`, `candidate_city`,
    /// `candidate_state`, `candidate_zip_code` (FEC format workbook v8.4,
    /// sheet F2, fields 14–18). The state here is the *mailing* state, not
    /// the state of the office sought (see [`office_state`](Self::office_state)).
    pub candidate_address: Address,

    /// "Check if address changed" box next to Line 1(b). Column
    /// `change_of_address`, `X` = yes (FEC format workbook v8.4, sheet F2,
    /// field 13;
    /// [fecfrm2.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm2.pdf#page=1)).
    pub change_of_address: bool,

    /// Party affiliation code — Line 4, "Party Affiliation". Column
    /// `candidate_party_code` (FEC format workbook v8.4, sheet F2, field 19,
    /// e.g. `DEM`, `REP`, `IND`). See [`party_code_label`](Self::party_code_label).
    pub party_code: Option<String>,

    /// Office sought — Line 5. Column `candidate_office`: `H`, `S` or `P`
    /// (FEC format workbook v8.4, sheet F2, field 20). See
    /// [`office_label`](Self::office_label).
    pub office: Option<String>,

    /// State of the office sought — Line 6, "State & District of Candidate".
    /// Column `candidate_state_TODO_DUP`: the workbook's *second* "CANDIDATE
    /// STATE" field, required for Senate and House (FEC format workbook v8.4,
    /// sheet F2, field 21, rule "Edit: ST (if Office = Sen or House)").
    pub office_state: Option<String>,

    /// Congressional district of the office sought — Line 6. Column
    /// `candidate_district`, `01`–`99`, required for House (FEC format
    /// workbook v8.4, sheet F2, field 22). `00` is common in real filings
    /// (including Senate and President); what it means is not documented in
    /// the FEC sources indexed here.
    pub district: Option<String>,

    /// Year of the election the principal campaign committee is designated
    /// for — Line 7, "(year of election)". Column `election_year`, NUM-4,
    /// 1900–2999 (FEC format workbook v8.4, sheet F2, field 23). On paper a
    /// special-election designation is noted here, and a candidate who also
    /// runs in the regular election files a separate Form 2 for that year
    /// ([fecfrm2i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm2i.pdf#page=2)).
    /// `None` if blank or not a number.
    pub election_year: Option<u16>,

    /// Vice-presidential running mate's name. Columns
    /// `vice_president_last_name`, `vice_president_first_name`,
    /// `vice_president_middle_name`, `vice_president_prefix`,
    /// `vice_president_suffix` (FEC format workbook v8.4, sheet F2, fields
    /// 8–12, rule "If Office=Presidential"). Electronic-only: the paper form
    /// has no box for it. `None` when every part is blank.
    pub vice_president: Option<PersonName>,

    /// The principal campaign committee designated on Line 7, "Designation of
    /// principal campaign Committee" (FEC format workbook v8.4, sheet F2,
    /// fields 24–30). It must file a Statement of Organization (Form 1)
    /// within 10 days of designation, and its name must include the
    /// candidate's name
    /// ([fecfrm2i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm2i.pdf#page=1)).
    pub principal_committee: Form2Committee,

    /// The first "other authorized committee" designated on Line 8, a
    /// committee that is *not* the principal campaign committee but is
    /// authorized to receive and expend funds on the candidate's behalf,
    /// including joint fundraising representatives
    /// ([fecfrm2.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm2.pdf#page=1);
    /// FEC format workbook v8.4, sheet F2, fields 31–37). Further ones are on
    /// `F2S` records. `None` when the block is blank.
    pub authorized_committee: Option<Form2Committee>,

    /// "Declaration of Intent to Spend Personal Funds" for the primary and
    /// general elections. Only in format versions up to 6.3 (columns
    /// `primary_personal_funds_declared`, `general_personal_funds_declared`);
    /// removed in 6.4 ("Fields 33 & 34 Declaration of Intent to Spend
    /// Personal Funds for the Primary and General removed", FEC format v6.4
    /// PDF p32). Always `None` for 6.4+ filings, which are the only ones the
    /// parser currently opens.
    pub personal_funds_declaration: Option<Form2PersonalFundsDeclaration>,

    /// Name of the person who signed — "Signature of Candidate". Columns
    /// `candidate_signature_last_name`, `candidate_signature_first_name`,
    /// `candidate_signature_middle_name`, `candidate_signature_prefix`,
    /// `candidate_signature_suffix` (FEC format workbook v8.4, sheet F2,
    /// fields 38–42). The workbook notes the candidate normally signs and these
    /// "ought to match" the candidate name fields. v3/v5.x formats' single
    /// caret-delimited `candidate_signature_name` column is split into parts.
    pub signer: PersonName,

    /// Date signed — "Date" beside the candidate's signature. Column
    /// `date_signed`, `YYYYMMDD` (FEC format workbook v8.4, sheet F2, field
    /// 43).
    pub date_signed: Option<Date>,
}

/// A committee designated on Form 2: the principal campaign committee (Line 7)
/// or an other authorized committee (Line 8).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Form2Committee {
    /// FEC committee ID. Columns `committee_id_number` (Line 7, "PCC
    /// COMMITTEE ID NUMBER") and `authorized_committee_id_number` (Line 8,
    /// "AUTH COMMITTEE ID NUMBER") (FEC format workbook v8.4, sheet F2,
    /// fields 24 and 31). Electronic-only: the paper form prints name and
    /// address boxes but no ID box. Often blank for a new committee that has
    /// not yet been assigned one.
    pub id: Option<String>,
    /// Committee name — "(a) Name of Committee (in full)". Columns
    /// `committee_name` / `authorized_committee_name` (FEC format workbook
    /// v8.4, sheet F2, fields 25 and 32).
    pub name: Option<String>,
    /// Committee address — "(b) Address" and "(c) City, State, and ZIP Code".
    /// Columns `committee_street_1` … `committee_zip_code` (fields 26–30) and
    /// `authorized_committee_street_1` … `authorized_committee_zip_code`
    /// (fields 33–37) (FEC format workbook v8.4, sheet F2).
    pub address: Address,
}

impl Form2Committee {
    fn from_prefixed(data: &Data, prefix: &str) -> Self {
        Self {
            id: text(data, &format!("{prefix}id_number")),
            name: text(data, &format!("{prefix}name")),
            address: Address::from_prefixed(data, prefix),
        }
    }

    /// True when the ID, name and address are all blank.
    pub fn is_empty(&self) -> bool {
        self.id.is_none() && self.name.is_none() && self.address.is_empty()
    }
}

/// Legacy (format ≤ 6.3) declaration of intent to spend personal funds; see
/// [`Form2::personal_funds_declaration`]. The meaning of the amounts beyond
/// the column names and the v6.4 change note is not documented in the FEC
/// sources indexed for this crate.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize)]
pub struct Form2PersonalFundsDeclaration {
    /// Column `primary_personal_funds_declared`.
    pub primary: Option<f64>,
    /// Column `general_personal_funds_declared`.
    pub general: Option<f64>,
}

impl Form2 {
    /// Build from a cover record's `column -> value` map. Returns `None` only
    /// if the record has no candidate ID and no candidate last name, i.e. it
    /// is not a usable Form 2.
    pub fn from_data(data: &Data) -> Option<Self> {
        let candidate_id = text_or_empty(data, "candidate_id_number");
        let candidate = person_name_or_legacy(data, "candidate_", "candidate_name");
        if candidate_id.is_empty() && candidate.is_empty() {
            return None;
        }

        let vice_president =
            Some(PersonName::from_prefixed(data, "vice_president_")).filter(|p| !p.is_empty());
        let authorized_committee =
            Some(Form2Committee::from_prefixed(data, "authorized_committee_"))
                .filter(|c| !c.is_empty());
        let personal_funds_declaration = Some(Form2PersonalFundsDeclaration {
            primary: amount_opt(data, "primary_personal_funds_declared"),
            general: amount_opt(data, "general_personal_funds_declared"),
        })
        .filter(|d| d.primary.is_some() || d.general.is_some());

        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            candidate_id,
            candidate,
            candidate_address: Address::from_prefixed(data, "candidate_"),
            change_of_address: flag(data, "change_of_address"),
            party_code: text(data, "candidate_party_code"),
            office: text(data, "candidate_office"),
            office_state: text(data, "candidate_state_TODO_DUP"),
            district: text(data, "candidate_district"),
            election_year: text(data, "election_year").and_then(|y| y.parse().ok()),
            vice_president,
            principal_committee: Form2Committee::from_prefixed(data, "committee_"),
            authorized_committee,
            personal_funds_declaration,
            signer: person_name_or_legacy(data, "candidate_signature_", "candidate_signature_name"),
            date_signed: date(data, "date_signed"),
        })
    }

    /// True for an amended statement (`F2A`), false for a new one (`F2N`) —
    /// Line 3
    /// ([fecfrm2i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm2i.pdf#page=1):
    /// check "New" if filing Form 2 for the first time, otherwise "Amended").
    pub fn is_amendment(&self) -> bool {
        crate::covers::is_amendment_form_type(&self.form_type)
    }

    /// Office sought: `H` = House, `S` = Senate, `P` = President. The codes
    /// are the workbook's value list (FEC format workbook v8.4, sheet F2,
    /// field 20, `H,S,P`); the names are the matching "Office Sought"
    /// checkboxes on Form 1
    /// ([fecfrm1.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=2)).
    /// See [`crate::covers::office_label`].
    pub fn office_label(&self) -> Option<&'static str> {
        crate::covers::office_label(self.office.as_deref()?)
    }

    /// Party name for the abbreviations the FEC spells out in its
    /// instructions: DEM, REP, REF, GRE and IND
    /// ([fecfrm1i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=2),
    /// Line 5). Other codes (the workbook's list is only "AIC,AIP,...") return
    /// `None`. Case-insensitive. See [`crate::covers::party_label`].
    pub fn party_code_label(&self) -> Option<&'static str> {
        crate::covers::party_label(self.party_code.as_deref()?)
    }
}
