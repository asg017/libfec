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

use crate::covers::PersonName;
use indexmap::IndexMap;
use jiff::civil::Date;

/// A cover record's `column name -> raw string` map. The value helpers
/// take it; a [`Data`] derefs to it.
pub(crate) type Kv = IndexMap<String, String>;

/// A cover record's `column name -> raw string` map, plus the filing's
/// combined-name delimiter: the input of the typed covers' `from_data`.
/// Derefs to the map; `From` a map gives the default `^` delimiter.
#[derive(Debug, Clone, Default)]
pub struct Data {
    kv: IndexMap<String, String>,
    /// Sub-delimiter of legacy combined names (`Last^First^...`): the
    /// header's `name_delim` / `NameDelim`, `^` when unset.
    name_delimiter: String,
}

impl Data {
    /// `name_delimiter`: [`crate::FilingHeader::name_delimiter`]; `None` or
    /// empty means `^`.
    pub fn new(kv: IndexMap<String, String>, name_delimiter: Option<&str>) -> Self {
        Self {
            kv,
            name_delimiter: name_delimiter
                .filter(|d| !d.is_empty())
                .unwrap_or("^")
                .to_owned(),
        }
    }
}

impl std::ops::Deref for Data {
    type Target = IndexMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.kv
    }
}

impl std::ops::DerefMut for Data {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.kv
    }
}

impl From<IndexMap<String, String>> for Data {
    fn from(kv: IndexMap<String, String>) -> Self {
        Self::new(kv, None)
    }
}

impl FromIterator<(String, String)> for Data {
    /// A map with the default `^` name delimiter.
    fn from_iter<I: IntoIterator<Item = (String, String)>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect(), None)
    }
}

/// Where the value helpers read raw column values from: a cover's
/// [`Data`] map, or an itemization's borrowed record
/// ([`crate::itemizations::RecordFields`]), which has no per-row map.
pub trait Fields {
    /// The raw value of column `key`, or `None` if the layout has no such
    /// column (or the record is cut short before it).
    fn raw(&self, key: &str) -> Option<&str>;

    /// Sub-delimiter of legacy combined names (`Last^First^...`).
    fn name_delimiter(&self) -> &str {
        "^"
    }
}

impl Fields for Kv {
    fn raw(&self, key: &str) -> Option<&str> {
        self.get(key).map(String::as_str)
    }
}

impl Fields for Data {
    fn raw(&self, key: &str) -> Option<&str> {
        self.kv.get(key).map(String::as_str)
    }

    fn name_delimiter(&self) -> &str {
        &self.name_delimiter
    }
}

/// `{prefix}{suffix}` as a column name, built on the stack: the shared shapes
/// (names, addresses) read a dozen prefixed columns per record, and on
/// itemizations allocating each key was a measurable share of the cost.
pub(crate) fn key(prefix: &str, suffix: &str) -> Key {
    let len = prefix.len() + suffix.len();
    if len > KEY_CAPACITY {
        return Key::Heap(format!("{prefix}{suffix}"));
    }
    let mut buf = [0u8; KEY_CAPACITY];
    buf[..prefix.len()].copy_from_slice(prefix.as_bytes());
    buf[prefix.len()..len].copy_from_slice(suffix.as_bytes());
    Key::Stack(buf, len as u8)
}

const KEY_CAPACITY: usize = 64;

/// A column name from [`key`]; derefs to `&str`.
pub(crate) enum Key {
    Stack([u8; KEY_CAPACITY], u8),
    Heap(String),
}

impl std::ops::Deref for Key {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            // Both halves were `&str`, so their concatenation is UTF-8.
            Key::Stack(buf, len) => std::str::from_utf8(&buf[..*len as usize]).unwrap_or_default(),
            Key::Heap(s) => s,
        }
    }
}

/// Trimmed text value, or `None` if the column is missing or blank.
pub(crate) fn text<F: Fields + ?Sized>(data: &F, key: &str) -> Option<String> {
    data.raw(key)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
}

/// Trimmed text value, or `""` if missing or blank.
pub(crate) fn text_or_empty<F: Fields + ?Sized>(data: &F, key: &str) -> String {
    text(data, key).unwrap_or_default()
}

/// Money amount. Blank, missing, or unparsable values are `0.0`.
pub(crate) fn amount<F: Fields + ?Sized>(data: &F, key: &str) -> f64 {
    amount_opt(data, key).unwrap_or(0.0)
}

/// Money amount, or `None` if the column is missing, blank, or unparsable.
pub(crate) fn amount_opt<F: Fields + ?Sized>(data: &F, key: &str) -> Option<f64> {
    data.raw(key)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<f64>().ok())
}

/// A date in `YYYYMMDD` (v6+) or `MM/DD/YYYY` (some legacy versions).
pub(crate) fn date<F: Fields + ?Sized>(data: &F, key: &str) -> Option<Date> {
    let s = data.raw(key)?.trim();
    if s.is_empty() {
        return None;
    }
    // Fast path for `YYYYMMDD`, the format of every v6+ date: `strptime`
    // re-reads its format string on each call, which shows on itemizations.
    if let [y1, y2, y3, y4, m1, m2, d1, d2] = s.as_bytes() {
        let digits = [y1, y2, y3, y4, m1, m2, d1, d2];
        if digits.iter().all(|b| b.is_ascii_digit()) {
            let n = |ds: &[&u8]| ds.iter().fold(0i16, |acc, b| acc * 10 + i16::from(**b - b'0'));
            return Date::new(n(&digits[..4]), n(&digits[4..6]) as i8, n(&digits[6..]) as i8).ok();
        }
    }
    Date::strptime("%Y%m%d", s)
        .or_else(|_| Date::strptime("%m/%d/%Y", s))
        .ok()
}

/// A checkbox. The FEC format marks checked boxes with `X`; anything else
/// (including blank) is unchecked.
#[allow(dead_code)] // used by the per-form cover modules as they land
pub(crate) fn flag<F: Fields + ?Sized>(data: &F, key: &str) -> bool {
    data.raw(key)
        .map(|s| s.trim().eq_ignore_ascii_case("x"))
        .unwrap_or(false)
}

/// Read a structured `{prefix}last_name`/… name, falling back to a legacy
/// single-column name (`legacy_key`) when the structured columns are absent.
///
/// v3/v5.x formats give names as one caret-delimited field, e.g.
/// `Smith^Pat T.^Mr.^Jr.` = last ^ first (and middle) ^ prefix ^ suffix (FEC
/// format workbook v5.2, sample data for "IND/NAME" fields). A value without
/// carets is kept whole as the last name. Some legacy layouts put that
/// caret-delimited name in the `{prefix}last_name` column itself (Form 9's
/// custodian in v5.x); that is split the same way.
///
/// Every cover reads a name this way when its mapping has a legacy
/// single-column name; the legacy key is almost always `{prefix}name`.
pub(crate) fn person_name_or_legacy<F: Fields + ?Sized>(
    data: &F,
    prefix: &str,
    legacy_key: &str,
) -> PersonName {
    let name = person_name(data, prefix);
    if !name.is_empty() {
        return name;
    }
    text(data, legacy_key)
        .map(|raw| split_legacy_name(&raw, data.name_delimiter()))
        .unwrap_or(name)
}

/// Read a structured `{prefix}last_name`/… name, splitting a combined value
/// (the filing's name delimiter, `^` by default) found in `{prefix}last_name`
/// (with `{prefix}first_name` blank) the way [`person_name_or_legacy`]
/// splits a legacy column. For names whose mappings have no legacy
/// single-name column.
pub(crate) fn person_name<F: Fields + ?Sized>(data: &F, prefix: &str) -> PersonName {
    let name = PersonName::from_prefixed(data, prefix);
    if name.first_name.is_empty() && name.last_name.contains(data.name_delimiter()) {
        return split_legacy_name(&name.last_name, data.name_delimiter());
    }
    name
}

/// Split a v1–5.x combined name, `Last^First^Prefix^Suffix` with `^` the
/// filing's name delimiter (HDR `name_delim` / `/*` `NameDelim`; an empty
/// `delimiter` means `^`), e.g. `Smith^John W.^Mr.^Jr.`. Parts are trimmed.
/// A middle name or initial stays inside `first_name` as written (the legacy
/// formats have no middle-name part), so `middle_name` is always `None`;
/// blank prefix/suffix are `None`. A value without the delimiter becomes
/// `last_name` whole.
pub fn split_legacy_name(raw: &str, delimiter: &str) -> PersonName {
    let delimiter = if delimiter.is_empty() { "^" } else { delimiter };
    let mut parts = raw.split(delimiter).map(str::trim);
    let part = |p: Option<&str>| p.filter(|s| !s.is_empty()).map(str::to_owned);
    PersonName {
        last_name: parts.next().unwrap_or_default().to_owned(),
        first_name: parts.next().unwrap_or_default().to_owned(),
        middle_name: None,
        prefix: part(parts.next()),
        suffix: part(parts.next()),
    }
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

    #[test]
    fn legacy_names() {
        let n = split_legacy_name(" Smith ^John W.^Mr.^Jr.", "^");
        assert_eq!(n.last_name, "Smith");
        assert_eq!(n.first_name, "John W.");
        assert_eq!(n.middle_name, None);
        assert_eq!(n.prefix.as_deref(), Some("Mr."));
        assert_eq!(n.suffix.as_deref(), Some("Jr."));

        let n = split_legacy_name("Abbenhaus^James I.^^M.D.", "");
        assert_eq!((n.prefix, n.suffix.as_deref()), (None, Some("M.D.")));

        let n = split_legacy_name("Doe|Jane", "|");
        assert_eq!(
            (n.last_name.as_str(), n.first_name.as_str()),
            ("Doe", "Jane")
        );

        let n = split_legacy_name("Fulton Bank", "^");
        assert_eq!(
            (n.last_name.as_str(), n.first_name.as_str()),
            ("Fulton Bank", "")
        );
    }

    #[test]
    fn names_use_the_filing_delimiter() {
        let kv = data(&[
            ("treasurer_name", "Galis|George||"),
            ("a_last_name", "Doe|Jane"),
        ]);
        let d = Data::new(kv.kv.clone(), Some("|"));
        let n = person_name_or_legacy(&d, "treasurer_", "treasurer_name");
        assert_eq!(
            (n.last_name.as_str(), n.first_name.as_str()),
            ("Galis", "George")
        );
        let n = person_name(&d, "a_");
        assert_eq!(
            (n.last_name.as_str(), n.first_name.as_str()),
            ("Doe", "Jane")
        );
        // Unset or empty: `^`.
        let d = Data::new(data(&[("treasurer_name", "Doe^Jane")]).kv, Some(""));
        let n = person_name_or_legacy(&d, "treasurer_", "treasurer_name");
        assert_eq!(
            (n.last_name.as_str(), n.first_name.as_str()),
            ("Doe", "Jane")
        );
    }
}
