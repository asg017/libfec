//! Helpers for exporting itemization rows of any format family.
//!
//! A row's layout depends on its row type *and* the filing's version, and
//! legacy filings (1.x–5.x, paper) contain row types that mappings2.json
//! does not (yet) describe for that version. Exports skip such rows with one
//! warning per row type per filing instead of aborting or panicking.

use std::{borrow::Cow, collections::HashSet};

use csv::StringRecord;
use indicatif::MultiProgress;

/// Prints a warning the first time an unmapped row type is seen in a filing.
/// Shared across filings; keyed by (filing id, row type).
pub struct UnmappedRows<'a> {
    seen: HashSet<(String, String)>,
    mb: Option<&'a MultiProgress>,
}

impl<'a> UnmappedRows<'a> {
    pub fn new(mb: Option<&'a MultiProgress>) -> Self {
        Self {
            seen: HashSet::new(),
            mb,
        }
    }

    /// Warn about an unmapped row, once per row type per filing; `what`
    /// says what happens to such rows (e.g. "skipping them").
    pub fn warn(&mut self, filing_id: &str, row_type: &str, fec_version: &str, what: &str) {
        if self
            .seen
            .insert((filing_id.to_owned(), row_type.to_owned()))
        {
            let msg = format!(
                "warning: FEC-{filing_id}: no column mapping for row type '{row_type}' in version {fec_version}, {what}"
            );
            warn(self.mb, msg);
        }
    }
}

/// Print a warning above the progress bars, or straight to stderr when
/// there are none: a hidden `MultiProgress` (stderr not a terminal, as in
/// scripts and CI) silently drops `println`.
pub fn warn(mb: Option<&MultiProgress>, msg: impl AsRef<str>) {
    match mb.filter(|mb| !mb.is_hidden()) {
        Some(mb) => {
            let _ = mb.println(msg.as_ref());
        }
        None => eprintln!("{}", msg.as_ref()),
    }
}

/// The error for an export in which every input filing failed to open
/// (each was already reported by [`warn`]), so it exits non-zero instead of
/// writing an empty result.
pub fn all_filings_failed(failed: usize) -> anyhow::Error {
    anyhow::anyhow!(
        "none of the {failed} input filing{} could be read; nothing exported",
        if failed == 1 { "" } else { "s" }
    )
}

/// `YYYY-MM-DD` from a `YYYYMMDD` or `MM/DD/YYYY` date (the latter as written
/// in some legacy and paper filings), or `None` for anything else.
pub fn normalize_fec_date(value: &str) -> Option<String> {
    let v = value.trim();
    let b = v.as_bytes();
    let digits = |s: &[u8]| s.iter().all(u8::is_ascii_digit);
    if b.len() == 8 && digits(b) {
        return Some(format!("{}-{}-{}", &v[0..4], &v[4..6], &v[6..8]));
    }
    let mut parts = v.split('/');
    if let (Some(m), Some(d), Some(y), None) = (parts.next(), parts.next(), parts.next(), parts.next())
    {
        if (1..=2).contains(&m.len())
            && (1..=2).contains(&d.len())
            && y.len() == 4
            && digits(m.as_bytes())
            && digits(d.as_bytes())
            && digits(y.as_bytes())
        {
            return Some(format!("{y}-{m:0>2}-{d:0>2}"));
        }
    }
    None
}

/// A row type usable as a file name: `SC/10` → `SC-10` (as FastFEC does).
pub fn file_stem(row_type: &str) -> String {
    row_type
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// The columns a row type is exported with, whatever the filing's version:
/// its 8.5 layout, else its 8.4 one (forms removed in 8.5, like F3Z1), else
/// `fec_version`'s own layout (legacy-only row types, e.g. paper `F3Z`: the
/// first filing with one then fixes the columns and rows of other legacy
/// versions are rearranged by name).
pub fn export_columns(row_type: &str, fec_version: &str) -> Option<&'static [String]> {
    use fec_parser::mappings::column_names_for_field;
    column_names_for_field(row_type, "8.5")
        .or_else(|_| column_names_for_field(row_type, "8.4"))
        .or_else(|_| column_names_for_field(row_type, fec_version))
        .ok()
        .map(Vec::as_slice)
}

/// The fields of `record` (laid out as `source` columns) rearranged into the
/// `target` columns by name; missing columns are empty. When the layouts are
/// identical, the record's fields as they are (possibly fewer or more than
/// `target`).
pub fn remap_by_name<'r>(
    target: &[String],
    source: &[String],
    record: &'r StringRecord,
) -> Vec<&'r str> {
    if target == source {
        return record.iter().collect();
    }
    target
        .iter()
        .map(|name| {
            source_index(source, name)
                .and_then(|i| record.get(i))
                .unwrap_or("")
        })
        .collect()
}

/// Index in a `source` layout of the column that fills target column
/// `name`: the column of that name, else its counterpart in an 8.x SA3L
/// (lobbyist bundling) layout. SA3L rows are filed under Schedule A, whose
/// 45 columns their 8.5 rows fill positionally (sqlite), so other versions
/// map by name the same way: `lobbyist_registrant_*` → `contributor_*`,
/// `bundled_amount_period` → `contribution_amount`,
/// `bundled_amount_semi_annual` → `contribution_aggregate`, `memo_text` →
/// `memo_text_description`.
pub fn source_index(source: &[String], name: &str) -> Option<usize> {
    source.iter().position(|c| c == name).or_else(|| {
        let alias: Cow<str> = match name {
            "contribution_amount" => "bundled_amount_period".into(),
            "contribution_aggregate" => "bundled_amount_semi_annual".into(),
            "memo_text_description" => "memo_text".into(),
            _ => format!(
                "lobbyist_registrant_{}",
                name.strip_prefix("contributor_")?
            )
            .into(),
        };
        source.iter().position(|c| *c == alias)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(normalize_fec_date("20240131").as_deref(), Some("2024-01-31"));
        assert_eq!(normalize_fec_date("01/31/2024").as_deref(), Some("2024-01-31"));
        assert_eq!(normalize_fec_date("1/3/1998").as_deref(), Some("1998-01-03"));
        assert_eq!(normalize_fec_date(""), None);
        assert_eq!(normalize_fec_date("1/1/98"), None);
        assert_eq!(normalize_fec_date("2024013é"), None);
        assert_eq!(normalize_fec_date("abcdefgh"), None);
    }

    #[test]
    fn file_stems() {
        assert_eq!(file_stem("SC/10"), "SC-10");
        assert_eq!(file_stem("SA11AI"), "SA11AI");
        assert_eq!(file_stem("../x"), "---x");
    }

    #[test]
    fn remap() {
        let rec = StringRecord::from(vec!["a", "b", "c"]);
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(remap_by_name(&s(&["x", "y"]), &s(&["x", "y"]), &rec), ["a", "b", "c"]);
        assert_eq!(remap_by_name(&s(&["x", "y"]), &s(&["x", "y", "z"]), &rec), ["a", "b"]);
        assert_eq!(remap_by_name(&s(&["z", "q", "x"]), &s(&["x", "y", "z"]), &rec), ["c", "", "a"]);
    }
}
