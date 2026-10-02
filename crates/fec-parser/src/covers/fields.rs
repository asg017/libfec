//! Small, shared helpers for reading typed values out of a cover record's
//! `column name -> raw string` map.
//!
//! Every typed cover struct is built from [`crate::FilingCover::cover_record_kv`],
//! whose keys are the column names `fec-parser` assigns from `mappings2.json`
//! for the filing's form type and FEC version. Values are the raw strings from
//! the `.fec` file. These helpers centralise the three decisions every cover
//! makes over and over:
//!
//! - **Blank text is absent.** A field that is missing from the record (older
//!   format versions have fewer columns) or is empty/whitespace-only is `None`.
//! - **Blank amounts are zero.** On the paper forms an empty money box means no
//!   activity, and filers leave summary lines blank rather than writing `0.00`.
//!   [`amount`] therefore returns `0.0` for blank or unparsable values;
//!   [`amount_opt`] keeps the distinction for the few places it matters.
//! - **Dates are `YYYYMMDD`** in every v6+ filing. Older versions sometimes use
//!   `MM/DD/YYYY`; [`date`] accepts both.

use indexmap::IndexMap;
use jiff::civil::Date;

pub(crate) type Data = IndexMap<String, String>;

/// Trimmed text value, or `None` if the column is missing or blank.
pub(crate) fn text(data: &Data, key: &str) -> Option<String> {
    data.get(key)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
}

/// Trimmed text value, or `""` if missing or blank.
pub(crate) fn text_or_empty(data: &Data, key: &str) -> String {
    text(data, key).unwrap_or_default()
}

/// Money amount. Blank, missing, or unparsable values are `0.0`.
pub(crate) fn amount(data: &Data, key: &str) -> f64 {
    amount_opt(data, key).unwrap_or(0.0)
}

/// Money amount, or `None` if the column is missing, blank, or unparsable.
pub(crate) fn amount_opt(data: &Data, key: &str) -> Option<f64> {
    data.get(key)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<f64>().ok())
}

/// A date in `YYYYMMDD` (v6+) or `MM/DD/YYYY` (some legacy versions).
pub(crate) fn date(data: &Data, key: &str) -> Option<Date> {
    let s = data.get(key)?.trim();
    if s.is_empty() {
        return None;
    }
    Date::strptime("%Y%m%d", s)
        .or_else(|_| Date::strptime("%m/%d/%Y", s))
        .ok()
}

/// A checkbox. The FEC format marks checked boxes with `X`; anything else
/// (including blank) is unchecked.
#[allow(dead_code)] // used by the per-form cover modules as they land
pub(crate) fn flag(data: &Data, key: &str) -> bool {
    data.get(key)
        .map(|s| s.trim().eq_ignore_ascii_case("x"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(pairs: &[(&str, &str)]) -> Data {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn text_blank_is_none() {
        let d = data(&[("a", "  "), ("b", " x ")]);
        assert_eq!(text(&d, "a"), None);
        assert_eq!(text(&d, "b").as_deref(), Some("x"));
        assert_eq!(text(&d, "missing"), None);
    }

    #[test]
    fn amounts() {
        let d = data(&[("a", ""), ("b", "12.50"), ("c", "junk"), ("d", "-3")]);
        assert_eq!(amount(&d, "a"), 0.0);
        assert_eq!(amount_opt(&d, "a"), None);
        assert_eq!(amount(&d, "b"), 12.5);
        assert_eq!(amount(&d, "c"), 0.0);
        assert_eq!(amount(&d, "d"), -3.0);
    }

    #[test]
    fn dates() {
        let d = data(&[("a", "20240131"), ("b", "01/31/2024"), ("c", "")]);
        assert_eq!(date(&d, "a"), Some(jiff::civil::date(2024, 1, 31)));
        assert_eq!(date(&d, "b"), Some(jiff::civil::date(2024, 1, 31)));
        assert_eq!(date(&d, "c"), None);
    }

    #[test]
    fn flags() {
        let d = data(&[("a", "X"), ("b", "x"), ("c", ""), ("d", "N")]);
        assert!(flag(&d, "a"));
        assert!(flag(&d, "b"));
        assert!(!flag(&d, "c"));
        assert!(!flag(&d, "d"));
    }
}
