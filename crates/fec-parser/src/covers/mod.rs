mod form3p;
pub use crate::covers::form3p::{Form3P, Form3PSummary};
use indexmap::IndexMap;

pub enum Cover {
    Form3P(Form3P),
}

pub(crate) fn cover_from_form_type(
    cover_record_form_type: &str,
    data: &IndexMap<String, String>,
) -> Option<Cover> {
    // TODO collides with F3PS?
    if cover_record_form_type.starts_with("F3P") {
        return Some(Cover::Form3P(Form3P::from_data(data)));
    }
    None
}

pub struct Treasurer {
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}
impl ToString for Treasurer {
    fn to_string(&self) -> String {
        let mut name = String::new();
        if let Some(prefix) = &self.prefix {
            name.push_str(prefix.trim());
            name.push(' ');
        }
        name.push_str(&self.first_name.trim());
        if let Some(middle_name) = &self.middle_name {
            name.push(' ');
            name.push_str(middle_name.trim());
        }
        name.push(' ');
        name.push_str(&self.last_name.trim());
        if let Some(suffix) = &self.suffix {
            name.push(' ');
            name.push_str(suffix.trim());
        }
        name.trim().to_string()
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
