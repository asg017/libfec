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
//! # Form types
//!
//! A cover record's form type is the base form plus an optional suffix: `N`
//! (new), `A` (amendment) or `T` (termination), e.g. `F3XN`, `F3XA`, `F3XT`.
//! [`base_form_type`] strips the suffix; dispatch in `cover_from_form_type`
//! matches on the base.

pub(crate) mod fields;
mod form1;
mod form2;
mod form3;
mod form3p;
mod form99;

pub use crate::covers::form1::{Form1, Form1Candidate};
pub use crate::covers::form2::{Form2, Form2Committee, Form2PersonalFundsDeclaration};
pub use crate::covers::form3::{
    Form3, Form3DetailedSummary, Form3DetailedSummaryDisbursements, Form3DetailedSummaryReceipts,
    Form3Summary,
};
pub use crate::covers::form3p::{
    Form3P, Form3PDetailedSummary, Form3PDetailedSummaryReceipts, Form3PSummary,
};
pub use crate::covers::form99::Form99;
use fields::{text, text_or_empty, Data};
use indexmap::IndexMap;
use serde::Serialize;

/// A typed cover record. See each variant's struct for field documentation.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "form", content = "data")]
pub enum Cover {
    Form1(Form1),
    Form3(Form3),
    Form3P(Form3P),
    Form2(Box<Form2>),
    Form99(Form99),
}

impl Cover {
    /// The person who signed the filing (usually the treasurer), if the form has one.
    pub fn signer(&self) -> Option<&PersonName> {
        match self {
            Cover::Form1(f) => Some(&f.treasurer),
            Cover::Form3(f) => Some(&f.treasurer),
            Cover::Form3P(f) => Some(&f.treasurer),
            Cover::Form2(f) => Some(&f.signer),
            Cover::Form99(f) => Some(&f.treasurer),
        }
    }

    /// The date the filing was signed, if the form records one.
    pub fn date_signed(&self) -> Option<jiff::civil::Date> {
        match self {
            Cover::Form1(f) => f.date_signed,
            Cover::Form3(f) => Some(f.signed),
            Cover::Form3P(f) => Some(f.signed),
            Cover::Form2(f) => f.date_signed,
            Cover::Form99(f) => f.date_signed,
        }
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

pub(crate) fn cover_from_form_type(cover_record_form_type: &str, data: &Data) -> Option<Cover> {
    match base_form_type(cover_record_form_type).as_str() {
        "F1" => Form1::from_data(data).map(Cover::Form1),
        "F3" => Form3::from_data(data).map(Cover::Form3),
        "F3P" => Form3P::from_data(data).map(Cover::Form3P),
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
    pub(crate) fn from_prefixed(data: &Data, prefix: &str) -> Self {
        Self {
            first_name: text_or_empty(data, &format!("{prefix}first_name")),
            last_name: text_or_empty(data, &format!("{prefix}last_name")),
            middle_name: text(data, &format!("{prefix}middle_name")),
            prefix: text(data, &format!("{prefix}prefix")),
            suffix: text(data, &format!("{prefix}suffix")),
        }
    }

    /// The committee treasurer (`treasurer_*` columns).
    pub(crate) fn from_data(data: &Data) -> Self {
        Self::from_prefixed(data, "treasurer_")
    }

    /// True when every part of the name is blank.
    pub fn is_empty(&self) -> bool {
        self.to_string().is_empty()
    }
}

/// A mailing address as FEC forms collect it.
///
/// Read with a column prefix: `Address::from_prefixed(data, "")` reads
/// `street_1`, `street_2`, `city`, `state`, `zip_code`; with `"candidate_"` it
/// reads `candidate_street_1`, … .
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
    pub(crate) fn from_prefixed(data: &Data, prefix: &str) -> Self {
        Self {
            street_1: text(data, &format!("{prefix}street_1")),
            street_2: text(data, &format!("{prefix}street_2")),
            city: text(data, &format!("{prefix}city")),
            state: text(data, &format!("{prefix}state")),
            zip_code: text(data, &format!("{prefix}zip_code")),
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
