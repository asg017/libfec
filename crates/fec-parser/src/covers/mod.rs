mod form1;
mod form3;
mod form3p;
pub use crate::covers::form1::Form1;
pub use crate::covers::form3::{Form3, Form3Summary};
pub use crate::covers::form3p::{Form3P, Form3PSummary};
use indexmap::IndexMap;

pub enum Cover {
    Form1(Form1),
    Form3(Form3),
    Form3P(Form3P),
}

/// A cover form type without its `N`/`A`/`T` (new, amendment, termination)
/// suffix, uppercased: `"F3XN"` → `"F3X"`, `"f3a"` → `"F3"`, `"F99"` → `"F99"`.
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

/// Routes by exact base form type: F3X, F3L, F1M, F13 etc. have their own
/// layouts and must not be read as Form 1 / Form 3.
pub(crate) fn cover_from_form_type(
    cover_record_form_type: &str,
    data: &IndexMap<String, String>,
) -> Option<Cover> {
    match base_form_type(cover_record_form_type).as_str() {
        "F1" => Form1::from_data(data).map(Cover::Form1),
        "F3" => Form3::from_data(data).map(Cover::Form3),
        "F3P" => Form3P::from_data(data).map(Cover::Form3P),
        _ => None,
    }
}

pub struct Treasurer {
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}
impl std::fmt::Display for Treasurer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut name = String::new();
        if let Some(prefix) = &self.prefix {
            name.push_str(prefix.trim());
            name.push(' ');
        }
        name.push_str(self.first_name.trim());
        if let Some(middle_name) = &self.middle_name {
            name.push(' ');
            name.push_str(middle_name.trim());
        }
        name.push(' ');
        name.push_str(self.last_name.trim());
        if let Some(suffix) = &self.suffix {
            name.push(' ');
            name.push_str(suffix.trim());
        }
        write!(f, "{}", name.trim())
    }
}
impl Treasurer {
    pub(crate) fn from_data(data: &IndexMap<String, String>) -> Self {
        let first_name = data
            .get("treasurer_first_name")
            .cloned()
            .unwrap_or_default();
        let last_name = data.get("treasurer_last_name").cloned().unwrap_or_default();
        let middle_name = data
            .get("treasurer_middle_name")
            .cloned()
            .filter(|s| !s.is_empty());
        let prefix = data
            .get("treasurer_prefix")
            .cloned()
            .filter(|s| !s.is_empty());
        let suffix = data
            .get("treasurer_suffix")
            .cloned()
            .filter(|s| !s.is_empty());

        Self {
            first_name,
            last_name,
            middle_name,
            prefix,
            suffix,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_form_type_strips_suffix() {
        assert_eq!(base_form_type("F3XN"), "F3X");
        assert_eq!(base_form_type("F3XA"), "F3X");
        assert_eq!(base_form_type("F3XT"), "F3X");
        assert_eq!(base_form_type("f3a"), "F3");
        assert_eq!(base_form_type("F1MN"), "F1M");
        assert_eq!(base_form_type("F99"), "F99");
        assert_eq!(base_form_type("F3"), "F3");
    }

    #[test]
    fn other_forms_are_not_read_as_form1_or_form3() {
        let data: IndexMap<String, String> = [
            ("committee_name", "X"),
            ("candidate_id_number", "H0XX00000"),
            ("date_signed", "20250101"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
        assert!(matches!(
            cover_from_form_type("F1N", &data),
            Some(Cover::Form1(_))
        ));
        assert!(matches!(
            cover_from_form_type("F3A", &data),
            Some(Cover::Form3(_))
        ));
        for form_type in ["F3XN", "F3LA", "F1MN", "F10", "F13N", "F99"] {
            assert!(
                cover_from_form_type(form_type, &data).is_none(),
                "{form_type}"
            );
        }
    }
}
