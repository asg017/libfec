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

/// How a column's raw value is typed by the bindings and exporters.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColumnKind {
    Text,
    Amount,
    Date,
}

/// The kind of a column, by name. Dates win over amounts (same precedence as the
/// CLI's Excel/SQLite export and the Python bindings).
pub fn column_kind(name: &str) -> ColumnKind {
    if DATE_COLUMNS.contains(name) {
        ColumnKind::Date
    } else if FLOAT_COLUMNS.contains(name) {
        ColumnKind::Amount
    } else {
        ColumnKind::Text
    }
}

pub static FORM_TYPES: &[&str] = &gen_form_types!("");

lazy_static::lazy_static! {
  pub static ref FORM_TYPES_SET: RegexSet = RegexSetBuilder::new(FORM_TYPES)
    .case_insensitive(true)
    .build()
    .expect("Static regex set to compile for FORM_TYPES");
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
) -> anyhow::Result<&'a Vec<String>> {
    let idx = field_idx(form_type).ok_or_else(|| {
        anyhow::anyhow!(format!(
            "Unknown form type '{}'; cannot determine filed idx",
            form_type
        ))
    })?;
    let idx2 = FORM_TYPE_VERSIONS_SET
        .get(idx)
        .ok_or_else(|| {
            anyhow::anyhow!(format!(
                "No form type versions regex set for form type '{}'",
                form_type
            ))
        })?
        .matches(fec_version)
        .iter()
        .next()
        .ok_or_else(|| {
            anyhow::anyhow!(format!(
                "Unknown FEC version '{}' for form type '{}'; cannot determine filed idx2",
                fec_version, form_type
            ))
        })?;
    let columns = COLUMN_NAMES
        .get(idx)
        .ok_or_else(|| anyhow::anyhow!(format!("No column names for form type '{}'", form_type)))?
        .get(idx2)
        .ok_or_else(|| {
            anyhow::anyhow!(format!(
                "No column names for form type '{}' and FEC version '{}'",
                form_type, fec_version
            ))
        })?;
    Ok(columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_kind_by_name() {
        assert_eq!(column_kind("contribution_date"), ColumnKind::Date);
        assert_eq!(column_kind("contribution_amount"), ColumnKind::Amount);
        assert_eq!(column_kind("contributor_last_name"), ColumnKind::Text);
        assert_eq!(column_kind("expenditure_date"), ColumnKind::Date);
        assert_eq!(column_kind("col_a_total_receipts"), ColumnKind::Amount);
    }

    #[test]
    fn column_kind_dates_win_over_amounts() {
        let mut both: Vec<_> = DATE_COLUMNS.intersection(&FLOAT_COLUMNS).collect();
        both.sort();
        // Both are cycle-to-date *amounts* that the mappings also list as dates;
        // the precedence (shared with the CLI exporters) types them as dates.
        assert_eq!(
            both,
            ["expenditure_total_cycle_to_date", "loan_payment_to_date"]
        );
        for name in both {
            assert_eq!(column_kind(name), ColumnKind::Date, "{name}");
        }
    }
}
