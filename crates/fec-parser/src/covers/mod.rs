//! Typed views of a filing's cover record.
//!
//! The second record of every `.fec` file is the *cover* record: the first page
//! (and, for financial reports, the summary pages) of the paper form the filing
//! stands in for. [`crate::FilingCover`] always carries the raw
//! `column -> value` map; this module turns the ones we understand into rich
//! structs, exposed as [`crate::FilingCover::cover_data`].
//!
//! # Conventions
//!
//! Every cover module follows the same rules, so the structs read alike:
//!
//! - **One module per form**, named after the form (`form3x.rs` for Form 3X).
//!   The top-level struct is `FormN` and has a `from_data(&IndexMap) -> Option<Self>`
//!   constructor that returns `None` only when the record is unusable.
//! - **Every public field is documented** with what it means *on the form*:
//!   the form line or box it comes from, the raw column name it is read from,
//!   and a citation to the FEC source that establishes it (a page of the FEC
//!   form or instructions PDF, or the FEC e-filing format workbook sheet and
//!   field number). Claims about meaning come from those sources, not from
//!   memory of campaign-finance law.
//! - **Summary-page amounts are `f64`**, blank read as `0.0` (see
//!   `fields::amount`). Two-column summary lines use [`DetailedSummaryRow`];
//!   what Column A and Column B mean differs by form and is documented on each
//!   form's struct.
//! - **Optional text is `Option<String>`**, blank read as `None`.
//! - **Shared shapes** — people's names ([`PersonName`]) and mailing addresses
//!   ([`Address`]) — are read with a column prefix, so `treasurer_first_name`
//!   and `candidate_first_name` both become a [`PersonName`].
//! - **Coded values** (committee types, report codes, office codes, …) are kept
//!   as the raw code in a `String`, with a `*_label()` helper method returning
//!   the FEC's own description where one is sourced.
//! - All structs derive `Debug`, `Clone` and `serde::Serialize`, so frontends
//!   (the TUI, the desktop viewer, Python/JS bindings) can consume them as-is.
//!
//! # Field names
//!
//! Field names are public API (they are the JSON keys of `libfec info -f json`
//! and of every binding), so the same thing has the same name on every form:
//!
//! - `form_type: String` — the cover's raw form type as filed (`"F3XN"`),
//!   first field of every top-level `FormN`, with an `is_amendment()` method
//!   (via [`is_amendment_form_type`]). Form 99 has no suffix, so its
//!   `is_amendment()` is always false.
//! - `filer_committee_id: String` — the filer's own FEC ID (column
//!   `filer_committee_id_number`), whatever kind of filer it is. Other IDs keep
//!   their role in the name (`candidate_id`, `committee_id` on a nested
//!   affiliated committee).
//! - `coverage_from_date` / `coverage_through_date: Option<Date>` — the
//!   covering period, named like the columns and [`crate::FilingCover`].
//! - `election_date: Option<Date>` — the date of the election a report is for
//!   (whether the column is `election_date` or `date_of_election`).
//! - `original_amendment_date: Option<Date>` — on an amendment, the date of
//!   the report it amends.
//! - `date_signed: Option<Date>` — the signature date.
//! - `line6a_year: Option<i16>` — a year printed on the form, parsed.
//! - The signer keeps the form's own role name: `treasurer` where the form says
//!   treasurer, otherwise `signer`, `person_completing`, `person_designated`
//!   or `designated_officer`. [`Cover::signer`] gives a uniform view.
//! - Summary-page amounts are `lineN_<description>`, numbered as on the form.
//! - Label helpers are `<field>_label()`; labels shared by several forms come
//!   from one function here ([`office_label`], [`party_label`],
//!   [`election_code_label`]).
//! - Names come from `fields::person_name_or_legacy` wherever the mapping has a
//!   legacy (v1–v5.x) single-column name, which splits its `^` parts the same
//!   way on every form.
//!
//! # Form types
//!
//! A cover record's form type is the base form plus an optional suffix: `N`
//! (new), `A` (amendment) or `T` (termination), e.g. `F3XN`, `F3XA`, `F3XT`.
//! [`base_form_type`] strips the suffix; dispatch in `cover_from_form_type`
//! matches on the base.

pub mod fields;
#[cfg(feature = "python")]
pub mod python;
pub use crate::covers::fields::split_legacy_name;
pub use fields::Data as CoverData;
mod form1;
mod form13;
mod form1m;
mod form2;
mod form24;
mod form3;
mod form3l;
mod form3p;
mod form3x;

pub use crate::covers::form1::{
    Form1, Form1Affiliated, Form1Bank, Form1Candidate, Form1Contact, Form1PacFlags,
};
pub use crate::covers::form1m::{Form1M, Form1MAffiliation, Form1MCandidate, Form1MQualification};
mod form4;
mod form7;

pub use crate::covers::form13::Form13;
mod form5;
mod form6;
mod form9;

pub use crate::covers::form24::Form24;
mod form99;

pub use crate::covers::form2::{Form2, Form2Committee, Form2PersonalFundsDeclaration};
pub use crate::covers::form3::{
    Form3, Form3CashSummary, Form3DetailedSummary, Form3DetailedSummaryDisbursements,
    Form3DetailedSummaryReceipts, Form3Summary,
};
pub use crate::covers::form3l::Form3L;
pub use crate::covers::form3p::{
    Form3P, Form3PDetailedSummary, Form3PDetailedSummaryDisbursements,
    Form3PDetailedSummaryReceipts, Form3PStateAllocation, Form3PStateAllocations, Form3PSummary,
};
pub use crate::covers::form3x::{
    Form3X, Form3XDetailedSummary, Form3XDisbursements,
    Form3XNetContributionsAndOperatingExpenditures, Form3XReceipts, Form3XSummary,
};
pub use crate::covers::form4::{
    Form4, Form4DetailedSummary, Form4Disbursements, Form4ItemizedLine, Form4LoanLine,
    Form4Receipts, Form4Summary,
};
pub use crate::covers::form5::Form5;
pub use crate::covers::form6::{Form6, Form6Candidate};
pub use crate::covers::form7::Form7;
pub use crate::covers::form9::{Form9, Form9Custodian};
pub use crate::covers::form99::Form99;
use fields::{key, text, text_or_empty, Data};
use indexmap::IndexMap;
use serde::Serialize;

/// A typed cover record. See each variant's struct for field documentation.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "form", content = "data")]
// One `Cover` per filing, so variant size differences do not matter.
#[allow(clippy::large_enum_variant)]
pub enum Cover {
    Form1(Form1),
    Form3(Form3),
    Form3P(Form3P),
    Form1M(Form1M),
    Form3X(Box<Form3X>),
    Form3L(Form3L),
    Form4(Box<Form4>),
    Form7(Form7),
    Form13(Form13),
    Form24(Form24),
    Form5(Box<Form5>),
    Form6(Form6),
    Form9(Box<Form9>),
    Form2(Box<Form2>),
    Form99(Form99),
}

impl Cover {
    /// The person who signed the filing (usually the treasurer), if the form has one.
    pub fn signer(&self) -> Option<&PersonName> {
        match self {
            Cover::Form1(f) => Some(if f.signer.is_empty() {
                &f.treasurer.name
            } else {
                &f.signer
            }),
            Cover::Form3(f) => Some(&f.treasurer),
            Cover::Form3P(f) => Some(&f.treasurer),
            Cover::Form1M(f) => Some(&f.treasurer),
            Cover::Form3X(f) => Some(&f.treasurer),
            Cover::Form3L(f) => Some(&f.treasurer),
            Cover::Form4(f) => Some(&f.treasurer),
            Cover::Form7(f) => Some(&f.person_designated),
            Cover::Form13(f) => Some(&f.designated_officer),
            Cover::Form24(f) => Some(&f.treasurer),
            Cover::Form5(f) => Some(&f.person_completing),
            Cover::Form6(f) => Some(&f.signer),
            Cover::Form9(f) => Some(&f.person_completing),
            Cover::Form2(f) => Some(&f.signer),
            Cover::Form99(f) => Some(&f.treasurer),
        }
    }

    /// The filer's name when the cover's generic filer-name column can be
    /// blank: an individual (not an organization) filing Form 5 or Form 9
    /// leaves `organization_name` empty and gives a personal name instead.
    /// `None` for every other form.
    pub fn filer_name(&self) -> Option<String> {
        match self {
            Cover::Form5(f) => Some(f.filer_name()),
            Cover::Form9(f) => Some(f.filer_name()),
            _ => None,
        }
        .filter(|s| !s.trim().is_empty())
    }

    /// The date the filing was signed, if the form records one.
    pub fn date_signed(&self) -> Option<jiff::civil::Date> {
        match self {
            Cover::Form1(f) => f.date_signed,
            Cover::Form3(f) => f.date_signed,
            Cover::Form3P(f) => f.date_signed,
            Cover::Form1M(f) => f.date_signed,
            Cover::Form3X(f) => f.date_signed,
            Cover::Form3L(f) => f.date_signed,
            Cover::Form4(f) => f.date_signed,
            Cover::Form7(f) => f.date_signed,
            Cover::Form13(f) => f.date_signed,
            Cover::Form24(f) => f.date_signed,
            Cover::Form5(f) => f.date_signed,
            Cover::Form6(f) => f.date_signed,
            Cover::Form9(f) => f.date_signed,
            Cover::Form2(f) => f.date_signed,
            Cover::Form99(f) => f.date_signed,
        }
    }
}

/// The election type for a cover record's election code (`P2026` →
/// `"Primary"`), from the code's first letter.
///
/// Letters per the FEC e-filing format specification, "Election Code (A.K.A.
/// Primary/General Indicator or PGI)": `P` Primary, `G` General, `O` Other,
/// `C` Convention, `R` Runoff, `S` Special, `E` Recount, each followed by the
/// election year (FEC_Format_v8.4.pdf p11). Unknown letters return `None`.
pub fn election_code_label(code: &str) -> Option<&'static str> {
    match code.trim().chars().next()?.to_ascii_uppercase() {
        'P' => Some("Primary"),
        'G' => Some("General"),
        'O' => Some("Other"),
        'C' => Some("Convention"),
        'R' => Some("Runoff"),
        'S' => Some("Special"),
        'E' => Some("Recount"),
        _ => None,
    }
}

/// Strip the `N`/`A`/`T` amendment-indicator suffix from a cover form type,
/// returning the base form (`"F3XN"` → `"F3X"`, `"F1MA"` → `"F1M"`,
/// `"F99"` → `"F99"`). Case-insensitive; the result is uppercase.
///
/// Only cover form types are handled; a form whose name itself ends in one of
/// those letters (none do among cover records) would be mis-stripped.
pub fn base_form_type(form_type: &str) -> String {
    let upper = form_type.trim().to_ascii_uppercase();
    match upper.as_bytes().last() {
        Some(b'N' | b'A' | b'T') if upper.len() > 2 => upper[..upper.len() - 1].to_owned(),
        _ => upper,
    }
}

/// True when a cover form type carries the `A` (amendment) suffix: `"F3XA"`
/// → `true`; `"F3XN"`, `"F3XT"` and the suffix-less `"F99"` → `false`.
/// Case-insensitive. Backs every `FormN::is_amendment`.
pub fn is_amendment_form_type(form_type: &str) -> bool {
    let upper = form_type.trim().to_ascii_uppercase();
    upper.ends_with('A') && base_form_type(&upper) != upper
}

/// "House", "Senate" or "President" for the office-sought codes `H`, `S`,
/// `P` (case-insensitive; other codes return `None`).
///
/// The format workbook lists the codes `H,S,P` without descriptions (FEC
/// format workbook v8.4, sheet `F1` field 29, sheet `F2` field 20, sheet `F6`
/// field 16); the names are Form 1's "Office Sought" boxes, which read
/// "House", "Senate", "President"
/// ([fecfrm1.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=2)).
pub fn office_label(code: &str) -> Option<&'static str> {
    match code.trim().to_ascii_uppercase().as_str() {
        "H" => Some("House"),
        "S" => Some("Senate"),
        "P" => Some("President"),
        _ => None,
    }
}

/// Party name for the five party abbreviations the FEC spells out: the Form
/// 1 Line 5 instructions say "for Democratic party, list “DEM,” for
/// Republican party, list “REP,” for Reform party, list “REF,” for Green
/// party, list “GRE” or for Independent, list “IND.”"
/// ([fecfrm1i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=2)).
/// Labels use that wording, capitalised as names ("Democratic Party").
/// Other codes return `None`: the workbook's list is only `AIC,AIP,...` /
/// `Edit: PTY` (FEC format workbook v8.4, sheet `F1`, field 32) with no
/// descriptions. Case-insensitive.
pub fn party_label(code: &str) -> Option<&'static str> {
    match code.trim().to_ascii_uppercase().as_str() {
        "DEM" => Some("Democratic Party"),
        "REP" => Some("Republican Party"),
        "REF" => Some("Reform Party"),
        "GRE" => Some("Green Party"),
        "IND" => Some("Independent"),
        _ => None,
    }
}

pub(crate) fn cover_from_form_type(cover_record_form_type: &str, data: &Data) -> Option<Cover> {
    match base_form_type(cover_record_form_type).as_str() {
        "F1" => Form1::from_data(data).map(Cover::Form1),
        "F3" => Form3::from_data(data).map(Cover::Form3),
        "F3P" => Form3P::from_data(data).map(Cover::Form3P),
        "F1M" => Form1M::from_data(data).map(Cover::Form1M),
        "F3X" => Form3X::from_data(data).map(|f| Cover::Form3X(Box::new(f))),
        "F3L" => Form3L::from_data(data).map(Cover::Form3L),
        "F4" => Form4::from_data(data).map(|f| Cover::Form4(Box::new(f))),
        "F7" => Form7::from_data(data).map(Cover::Form7),
        "F13" => Form13::from_data(data).map(Cover::Form13),
        "F24" => Form24::from_data(data).map(Cover::Form24),
        "F5" => Form5::from_data(data).map(|f| Cover::Form5(Box::new(f))),
        "F6" => Form6::from_data(data).map(Cover::Form6),
        "F9" => Form9::from_data(data).map(|f| Cover::Form9(Box::new(f))),
        "F2" => Form2::from_data(data).map(|f| Cover::Form2(Box::new(f))),
        "F99" => Form99::from_data(data).map(Cover::Form99),
        _ => None,
    }
}

/// A person's name, split the way FEC forms collect it.
///
/// Read with a column prefix: `PersonName::from_prefixed(data, "treasurer_")`
/// reads `treasurer_last_name`, `treasurer_first_name`, `treasurer_middle_name`,
/// `treasurer_prefix` and `treasurer_suffix`.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PersonName {
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}

/// Historical name; the treasurer is just a [`PersonName`].
pub type Treasurer = PersonName;

impl std::fmt::Display for PersonName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parts = [
            self.prefix.as_deref(),
            Some(self.first_name.as_str()),
            self.middle_name.as_deref(),
            Some(self.last_name.as_str()),
            self.suffix.as_deref(),
        ];
        let name = parts
            .into_iter()
            .flatten()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        write!(f, "{name}")
    }
}

impl PersonName {
    /// Read `{prefix}last_name`, `{prefix}first_name`, `{prefix}middle_name`,
    /// `{prefix}prefix` and `{prefix}suffix`.
    pub(crate) fn from_prefixed<F: fields::Fields + ?Sized>(data: &F, prefix: &str) -> Self {
        Self {
            first_name: text_or_empty(data, &key(prefix, "first_name")),
            last_name: text_or_empty(data, &key(prefix, "last_name")),
            middle_name: text(data, &key(prefix, "middle_name")),
            prefix: text(data, &key(prefix, "prefix")),
            suffix: text(data, &key(prefix, "suffix")),
        }
    }

    /// The committee treasurer (`treasurer_*` columns).
    pub(crate) fn from_data(data: &Data) -> Self {
        Self::from_prefixed(data, "treasurer_")
    }

    /// True when every part of the name is blank.
    pub fn is_empty(&self) -> bool {
        let blank = |s: &str| s.trim().is_empty();
        blank(&self.first_name)
            && blank(&self.last_name)
            && [&self.middle_name, &self.prefix, &self.suffix]
                .into_iter()
                .all(|part| part.as_deref().is_none_or(blank))
    }
}

/// A mailing address as FEC forms collect it.
///
/// Read with a column prefix: `Address::from_prefixed(data, "")` reads
/// `street_1`, `street_2`, `city`, `state`, `zip_code`; with `"candidate_"` it
/// reads `candidate_street_1`, … .
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Address {
    pub street_1: Option<String>,
    pub street_2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
}

impl Address {
    #[allow(dead_code)] // used by the per-form cover modules as they land
    pub(crate) fn from_prefixed<F: fields::Fields + ?Sized>(data: &F, prefix: &str) -> Self {
        Self {
            street_1: text(data, &key(prefix, "street_1")),
            street_2: text(data, &key(prefix, "street_2")),
            city: text(data, &key(prefix, "city")),
            state: text(data, &key(prefix, "state")),
            zip_code: text(data, &key(prefix, "zip_code")),
        }
    }

    /// True when every part of the address is blank.
    pub fn is_empty(&self) -> bool {
        self.street_1.is_none()
            && self.street_2.is_none()
            && self.city.is_none()
            && self.state.is_none()
            && self.zip_code.is_none()
    }

    /// Single-line rendering: `"123 Main St, Suite 4, Springfield, IL 62701"`.
    pub fn one_line(&self) -> String {
        let city_state = [self.city.as_deref(), self.state.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ");
        let city_state_zip = match self.zip_code.as_deref() {
            Some(zip) if !city_state.is_empty() => format!("{city_state} {zip}"),
            Some(zip) => zip.to_owned(),
            None => city_state,
        };
        [
            self.street_1.as_deref(),
            self.street_2.as_deref(),
            Some(city_state_zip.as_str()),
        ]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
    }
}

/// One line of a two-column summary page: Column A and Column B.
///
/// What the columns mean depends on the form. On Forms 3 and 3P Column A is
/// "Total This Period" and Column B "Election Cycle-to-Date"; on Form 3X
/// Column B is "Calendar Year-to-Date". Each form's struct documents its own.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct DetailedSummaryRow {
    pub column_a: f64,
    pub column_b: f64,
}

impl DetailedSummaryRow {
    pub fn from_data(
        data: &IndexMap<String, String>,
        column_a_key: &str,
        column_b_key: &str,
    ) -> Self {
        Self {
            column_a: fields::amount(data, column_a_key),
            column_b: fields::amount(data, column_b_key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_form_types() {
        assert_eq!(base_form_type("F3XN"), "F3X");
        assert_eq!(base_form_type("F3XA"), "F3X");
        assert_eq!(base_form_type("F3XT"), "F3X");
        assert_eq!(base_form_type("f3n"), "F3");
        assert_eq!(base_form_type("F1MN"), "F1M");
        assert_eq!(base_form_type("F3LA"), "F3L");
        assert_eq!(base_form_type("F13N"), "F13");
        assert_eq!(base_form_type("F99"), "F99");
        assert_eq!(base_form_type("F24N"), "F24");
        assert_eq!(base_form_type("F3P"), "F3P");
        assert_eq!(base_form_type("F3PN"), "F3P");
    }

    #[test]
    fn amendment_form_types() {
        assert!(is_amendment_form_type("F3XA"));
        assert!(is_amendment_form_type("f1ma"));
        assert!(is_amendment_form_type("F3PA"));
        assert!(!is_amendment_form_type("F3XN"));
        assert!(!is_amendment_form_type("F3XT"));
        assert!(!is_amendment_form_type("F99"));
        assert!(!is_amendment_form_type(""));
    }

    #[test]
    fn shared_labels() {
        assert_eq!(office_label(" s "), Some("Senate"));
        assert_eq!(office_label("X"), None);
        assert_eq!(party_label("dem"), Some("Democratic Party"));
        assert_eq!(party_label("IND"), Some("Independent"));
        assert_eq!(party_label("LIB"), None);
    }

    #[test]
    fn person_name_display() {
        let name = PersonName {
            first_name: "Jane".into(),
            last_name: "Doe".into(),
            middle_name: Some("Q".into()),
            prefix: Some("Dr.".into()),
            suffix: None,
        };
        assert_eq!(name.to_string(), "Dr. Jane Q Doe");
        assert!(PersonName::default().is_empty());
    }

    #[test]
    fn address_one_line() {
        let addr = Address {
            street_1: Some("1 Main St".into()),
            street_2: None,
            city: Some("Springfield".into()),
            state: Some("IL".into()),
            zip_code: Some("62701".into()),
        };
        assert_eq!(addr.one_line(), "1 Main St, Springfield, IL 62701");
    }
}
