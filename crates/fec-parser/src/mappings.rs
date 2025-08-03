use regex::{RegexSet, RegexSetBuilder};
use std::collections::HashSet;

use fec_parser_macros::{
    gen_column_names, gen_date_columns, gen_float_columns, gen_form_type_version_set,
    gen_form_types,
};

lazy_static::lazy_static! {
  pub static ref DATE_COLUMNS: HashSet<String> = HashSet::from(gen_date_columns!(""));
}
lazy_static::lazy_static! {
  pub static ref FLOAT_COLUMNS: HashSet<String> = HashSet::from(gen_float_columns!(""));
}

pub static FORM_TYPES: &[&str] = &gen_form_types!("");

lazy_static::lazy_static! {
  pub static ref FORM_TYPES_SET: RegexSet = RegexSetBuilder::new(FORM_TYPES)
    .case_insensitive(true)
    .build()
    .unwrap();
}
lazy_static::lazy_static! {
  pub static ref FORM_TYPE_VERSIONS_SET: Vec<RegexSet> = gen_form_type_version_set!("");
}

lazy_static::lazy_static! {
  pub static ref COLUMN_NAMES: Vec<Vec<Vec<String>>> = gen_column_names!();
}

pub fn field_idx(field: &str) -> Option<usize> {
    let matches = FORM_TYPES_SET.matches(field);
    matches.iter().next()
}

pub fn column_names_for_field<'a>(
    form_type: &str,
    fec_version: &str,
) -> std::result::Result<&'a Vec<String>, ()> {
    let idx = field_idx(form_type).unwrap();
    let idx2 = FORM_TYPE_VERSIONS_SET
        .get(idx)
        .unwrap()
        .matches(fec_version)
        .iter()
        .next()
        .unwrap();
    let columns = COLUMN_NAMES.get(idx).unwrap().get(idx2).unwrap();
    Ok(columns)
}
