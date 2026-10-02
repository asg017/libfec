//! Helpers for exporting itemization rows of any format family.
//!
//! A row's layout depends on its row type *and* the filing's version, and
//! legacy filings (1.x–5.x, paper) contain row types that mappings2.json
//! does not (yet) describe for that version. Exports skip such rows with one
//! warning per row type per filing instead of aborting or panicking.

use std::{borrow::Cow, collections::HashSet};

use csv::StringRecord;
use fec_parser::{covers::split_legacy_name, FilingHeader};
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
            // A hidden MultiProgress (stderr not a terminal) drops println.
            match self.mb.filter(|mb| !mb.is_hidden()) {
                Some(mb) => {
                    let _ = mb.println(msg);
                }
                None => eprintln!("{msg}"),
            }
        }
    }
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

/// [`remap_by_name`], then [`LegacyNames::fill`] when `name_delimiter` is
/// `Some` (see [`legacy_name_delimiter`]).
pub fn remap_row<'r>(
    target: &[String],
    source: &[String],
    record: &'r StringRecord,
    name_delimiter: Option<&str>,
) -> Vec<Cow<'r, str>> {
    let mut fields: Vec<Cow<'r, str>> = remap_by_name(target, source, record)
        .into_iter()
        .map(Cow::Borrowed)
        .collect();
    if let Some(delimiter) = name_delimiter {
        LegacyNames::new(target, source).fill(record, &mut fields, delimiter);
    }
    fields
}

/// The sub-delimiter of combined `Last^First^Prefix^Suffix` names for a
/// v1–5.x electronic filing (the header's `name_delim`, `^` when unset), or
/// `None` for 6.x+ and paper filings, whose names are split already: exports
/// only split combined names when this is `Some`, so 6.x–8.x rows are never
/// touched.
pub fn legacy_name_delimiter(header: &FilingHeader) -> Option<&str> {
    let legacy = !header.is_paper()
        && matches!(header.fec_version.trim().as_bytes().first(), Some(b'1'..=b'5'));
    legacy.then(|| header.name_delimiter.as_deref().unwrap_or("^"))
}

/// Fills a target layout's split-name columns (`{p}last_name`, `{p}first_name`,
/// `{p}middle_name`, `{p}prefix`, `{p}suffix`, `{p}organization_name`) from a
/// legacy row's combined `{p}name` column, which the target layout lacks
/// (v1–5.0 rows have only the combined name; 5.1–5.3 SA/SB/F2/F92/F93 have
/// both). Per name, when every split target column is empty and `{p}name`
/// is not:
///
/// - a value containing the delimiter is a person: `Last^First^Prefix^Suffix`
///   split by [`split_legacy_name`] (a middle name stays in first name, as
///   written);
/// - a value without it is an organisation when the target has
///   `{p}organization_name` and the row's `entity_type` is not `IND`/`CAN`
///   (or the row has no entity type: v1 layouts, blank values);
/// - anything else goes whole into `{p}last_name`.
///
/// Split columns that are already filled win (5.1+ rows filed both ways).
/// Separately, a delimited value in `{p}last_name` with an empty first name
/// (Form 9 custodian in 5.x) is split in place, as the typed covers do.
/// The combined value itself is not kept: the 8.5 layouts have no column
/// for it.
pub struct LegacyNames {
    groups: Vec<NameGroup>,
    /// Source index of `entity_type`.
    entity_type: Option<usize>,
}

/// Target indices of one name's split columns, and the source index of its
/// combined column.
struct NameGroup {
    combined: Option<usize>,
    last: usize,
    first: Option<usize>,
    middle: Option<usize>,
    prefix: Option<usize>,
    suffix: Option<usize>,
    organization: Option<usize>,
}

impl LegacyNames {
    pub fn new(target: &[String], source: &[String]) -> Self {
        let find = |cols: &[String], name: &str| cols.iter().position(|c| c == name);
        let groups = target
            .iter()
            .enumerate()
            .filter_map(|(last, col)| {
                let p = col.strip_suffix("last_name")?;
                let combined_name = format!("{p}name");
                let combined = find(source, &combined_name)
                    .filter(|_| find(target, &combined_name).is_none());
                Some(NameGroup {
                    combined,
                    last,
                    first: find(target, &format!("{p}first_name")),
                    middle: find(target, &format!("{p}middle_name")),
                    prefix: find(target, &format!("{p}prefix")),
                    suffix: find(target, &format!("{p}suffix")),
                    organization: find(target, &format!("{p}organization_name")),
                })
            })
            .collect();
        LegacyNames {
            groups,
            entity_type: find(source, "entity_type"),
        }
    }

    /// Fill `fields` (`record` remapped into the target layout) in place.
    pub fn fill<'r>(&self, record: &'r StringRecord, fields: &mut [Cow<'r, str>], delimiter: &str) {
        let delimiter = if delimiter.is_empty() { "^" } else { delimiter };
        let is_person = || {
            self.entity_type
                .and_then(|i| record.get(i))
                .map(|e| e.trim().to_ascii_uppercase())
                .is_some_and(|e| e == "IND" || e == "CAN")
        };
        for g in &self.groups {
            let split = [Some(g.last), g.first, g.middle, g.prefix, g.suffix, g.organization];
            let raw = g.combined.and_then(|i| record.get(i)).map_or("", str::trim);
            let person = if split.iter().all(|&i| get(fields, i).is_empty()) && !raw.is_empty() {
                if raw.contains(delimiter) {
                    split_legacy_name(raw, delimiter)
                } else if g.organization.is_some() && !is_person() {
                    set(fields, g.organization, raw.to_owned());
                    continue;
                } else {
                    set(fields, Some(g.last), raw.to_owned());
                    continue;
                }
            } else {
                let last = get(fields, Some(g.last));
                if !last.contains(delimiter) || !get(fields, g.first).is_empty() {
                    continue;
                }
                split_legacy_name(last, delimiter)
            };
            set(fields, Some(g.last), person.last_name);
            set(fields, g.first, person.first_name);
            set(fields, g.prefix, person.prefix.unwrap_or_default());
            set(fields, g.suffix, person.suffix.unwrap_or_default());
        }
    }
}

fn get<'a>(fields: &'a [Cow<str>], i: Option<usize>) -> &'a str {
    i.and_then(|i| fields.get(i)).map_or("", |v| v.trim())
}

fn set(fields: &mut [Cow<str>], i: Option<usize>, value: String) {
    if let Some(f) = i.and_then(|i| fields.get_mut(i)) {
        *f = Cow::Owned(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fec_parser::Filing;

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

    fn cols(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    const TARGET: &[&str] = &[
        "entity_type",
        "contributor_organization_name",
        "contributor_last_name",
        "contributor_first_name",
        "contributor_middle_name",
        "contributor_prefix",
        "contributor_suffix",
        "donor_candidate_last_name",
        "donor_candidate_first_name",
    ];

    fn split(source: &[&str], values: &[&str], delimiter: Option<&str>) -> Vec<String> {
        let rec = StringRecord::from(values.to_vec());
        remap_row(&cols(TARGET), &cols(source), &rec, delimiter)
            .into_iter()
            .map(Cow::into_owned)
            .collect()
    }

    #[test]
    fn legacy_names() {
        let v3 = ["entity_type", "contributor_name", "donor_candidate_name"];
        // person: Last^First^Prefix^Suffix, middle stays in first
        assert_eq!(
            split(&v3, &["IND", "Smith^John W.^Mr.^Jr.", "Doe^Jane^^"], Some("^")),
            ["IND", "", "Smith", "John W.", "", "Mr.", "Jr.", "Doe", "Jane"]
        );
        // no delimiter: organisation unless IND/CAN
        assert_eq!(
            split(&v3, &["ORG", "Fulton Bank", ""], Some("^")),
            ["ORG", "Fulton Bank", "", "", "", "", "", "", ""]
        );
        assert_eq!(
            split(&v3, &["", "Fulton Bank", ""], Some("^")),
            ["", "Fulton Bank", "", "", "", "", "", "", ""]
        );
        assert_eq!(
            split(&v3, &["ind", "Pat Doe", "Hollings For Senate"], Some("^")),
            ["ind", "", "Pat Doe", "", "", "", "", "Hollings For Senate", ""]
        );
        // the header's delimiter
        assert_eq!(
            split(&v3, &["CAN", "Doe|Jane", ""], Some("|"))[2..4],
            ["Doe", "Jane"]
        );
        // no entity type column (v1): organisation
        assert_eq!(
            split(&["contributor_name"], &["Fulton Bank"], Some("^"))[1],
            "Fulton Bank"
        );
        // 6.x+ / paper: untouched
        assert_eq!(
            split(&v3, &["IND", "Smith^John", ""], None),
            ["IND", "", "", "", "", "", "", "", ""]
        );
    }

    #[test]
    fn legacy_names_prefer_split() {
        // 5.1–5.3: split columns win over the combined one when filled
        let v51 = [
            "entity_type",
            "contributor_name",
            "contributor_last_name",
            "contributor_first_name",
        ];
        assert_eq!(
            split(&v51, &["IND", "Smith^John", "Smyth", "Jon"], Some("^"))[2..4],
            ["Smyth", "Jon"]
        );
        assert_eq!(
            split(&v51, &["IND", "Smith^John", "", ""], Some("^"))[2..4],
            ["Smith", "John"]
        );
        // a combined name in the last-name column (5.x F9 custodian)
        assert_eq!(
            split(&v51, &["IND", "", "Doe^Jane^Ms.", ""], Some("^"))[2..6],
            ["Doe", "Jane", "", "Ms."]
        );
    }

    #[test]
    fn legacy_name_delimiters() {
        let header = |hdr: &str| {
            let sep = if hdr.contains('\x1c') { '\x1c' } else { ',' };
            let bytes = format!("{hdr}\nF3XN{sep}C00000001\n").into_bytes();
            Filing::from_reader(std::io::Cursor::new(bytes), "1".into(), 0)
                .map(|f| f.header)
        };
        let h = |hdr: &str| header(hdr).map(|h| legacy_name_delimiter(&h).map(str::to_owned));
        assert_eq!(h("HDR,FEC,5.00,X,1,,,0,").ok(), Some(Some("^".into())));
        assert_eq!(h("HDR,FEC,3.00,X,1,|,,0,").ok(), Some(Some("|".into())));
        assert_eq!(h("HDR\x1cFEC\x1c8.4\x1cX\x1c1\x1c\x1c\x1c").ok(), Some(None));
    }
}
