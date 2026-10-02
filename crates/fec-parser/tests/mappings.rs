//! Tests for the `mappings2.json` lookup (`fec_parser::mappings`).
//!
//! - `resolution_table` snapshots which version entry (index and column
//!   count) every form regex picks for a fixed list of version strings, so any
//!   change to the JSON or to the version-regex rewriting in
//!   `fec-parser-macros` shows up in review.
//! - `observed_row_types_resolve` checks every (version, row type) seen in the
//!   legacy sample corpus (`tests/fixtures/legacy/observed_row_types.tsv`, see
//!   `crates/fec-parser-macros/MAPPINGS_CHANGES.md`) resolves to a non-empty
//!   column list at least as wide as the widest real row (+2 slack).

use fec_parser::mappings::{
    column_names_for_field, COLUMN_NAMES, FORM_TYPES, FORM_TYPE_VERSIONS_SET,
};
use std::collections::HashSet;
use std::fmt::Write;

/// Version strings as they appear in real filings (several spellings per
/// version, e.g. "3.0" / "3.00", "5.2" / "5.20").
const VERSIONS: &[&str] = &[
    "1", "1.00", "1.02", "2.00", "2.02", "3", "3.0", "3.00", "3.01", "5.00", "5.1", "5.2", "5.20",
    "5.3", "5.30", "6.1", "6.2", "6.3", "6.4", "7.0", "8.0", "8.1", "8.2", "8.3", "8.4", "8.5",
    "P1.0", "P2.2", "P2.3", "P2.4", "P2.6", "P3.0", "P3.1", "P3.2", "P3.3", "P3.4",
];

/// Extra slack allowed between the widest observed row and the column list.
const WIDTH_SLACK: usize = 2;

/// (fec_version, row_type) pairs from the samples that are known not to
/// resolve, with the reason.
const KNOWN_UNMAPPED: &[(&str, &str, &str)] = &[(
    "P3.1",
    "SA32",
    "paper/1018710.fec: data-entry typo for SA3L (only itemization of an F3LN; amounts equal the F3L totals)",
)];

#[test]
fn resolution_table() {
    let mut out = String::new();
    writeln!(out, "# form regex: compiled version patterns").unwrap();
    writeln!(
        out,
        "#   then one line per version: version -> entry index (column count), or '-' for no match"
    )
    .unwrap();
    for (idx, form) in FORM_TYPES.iter().enumerate() {
        let set = &FORM_TYPE_VERSIONS_SET[idx];
        writeln!(out, "{form}: {}", set.patterns().join("  ")).unwrap();
        let cells: Vec<String> = VERSIONS
            .iter()
            .map(|v| match set.matches(v).iter().next() {
                Some(i) => format!("{v}={i}({})", COLUMN_NAMES[idx][i].len()),
                None => format!("{v}=-"),
            })
            .collect();
        for chunk in cells.chunks(12) {
            writeln!(out, "    {}", chunk.join(" ")).unwrap();
        }
    }
    insta::assert_snapshot!(out);
}

#[test]
fn observed_row_types_resolve() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/legacy/observed_row_types.tsv"
    );
    let tsv = std::fs::read_to_string(path).unwrap();
    let known: HashSet<(&str, &str)> = KNOWN_UNMAPPED.iter().map(|(v, r, _)| (*v, *r)).collect();
    let mut failures = vec![];
    let mut seen_known = HashSet::new();
    let mut checked = 0;
    for line in tsv.lines().filter(|l| !l.starts_with('#')).skip(1) {
        let fields: Vec<&str> = line.split('\t').collect();
        let (version, row_type, max_fields, example) =
            (fields[0], fields[1], fields[2].parse::<usize>().unwrap(), fields[4]);
        if known.contains(&(version, row_type)) {
            seen_known.insert((version, row_type));
            assert!(
                column_names_for_field(row_type, version).is_err(),
                "{version} {row_type} is listed in KNOWN_UNMAPPED but now resolves; remove it"
            );
            continue;
        }
        checked += 1;
        match column_names_for_field(row_type, version) {
            Ok(columns) if columns.is_empty() => {
                failures.push(format!("{version} {row_type}: empty column list ({example})"))
            }
            Ok(columns) if max_fields > columns.len() + WIDTH_SLACK => failures.push(format!(
                "{version} {row_type}: rows have {max_fields} fields, mapping has {} ({example})",
                columns.len()
            )),
            Ok(_) => {}
            Err(e) => failures.push(format!("{version} {row_type}: {e} ({example})")),
        }
    }
    assert!(checked > 1000, "only {checked} rows read from {path}");
    assert_eq!(
        seen_known.len(),
        KNOWN_UNMAPPED.len(),
        "a KNOWN_UNMAPPED pair is no longer in the fixture"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Legacy lists must not repeat a column name or leave one blank: covers and
/// exporters key rows by name. (8.x lists keep their historical
/// `TODO_UNKNOWN_BLANK`, which is a name, not a blank.)
#[test]
fn legacy_column_names_unique_and_named() {
    let mut problems = vec![];
    for (idx, form) in FORM_TYPES.iter().enumerate() {
        let patterns = FORM_TYPE_VERSIONS_SET[idx].patterns();
        for (entry, columns) in COLUMN_NAMES[idx].iter().enumerate() {
            let mut seen = HashSet::new();
            for name in columns {
                if name.is_empty() {
                    problems.push(format!("{form} {}: blank name", patterns[entry]));
                } else if !seen.insert(name) {
                    problems.push(format!("{form} {}: duplicate {name}", patterns[entry]));
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Legacy columns that exports remap into 8.x layouts by name must carry
/// the 8.x name of the same field (see MAPPINGS_CHANGES.md): Schedule L's
/// column A line 10/11 (8.x mislabels them `col_b_*`) and TEXT's
/// back-reference form.
#[test]
fn legacy_names_match_8x_by_meaning() {
    use fec_parser::mappings::column_names_for_field;
    let at = |row_type: &str, version: &str, idx: usize| {
        column_names_for_field(row_type, version).unwrap()[idx].clone()
    };
    // 8.x, unchanged: index 22 is column A line 10.
    assert_eq!(at("SL", "8.5", 22), "col_b_disbursements_period");
    assert_eq!(at("SL", "8.5", 39), "col_b_disbursements_period_TODO_DUP");
    assert_eq!(at("SL", "5.1", 21), "col_b_disbursements_period");
    assert_eq!(at("SL", "5.1", 37), "col_b_disbursements_period_TODO_DUP");
    assert_eq!(at("SL", "5.1", 38), "col_b_cash_on_hand_close_of_period_TODO_DUP");
    assert_eq!(at("SL", "P3.4", 18), "col_b_disbursements_period");
    assert_eq!(at("SL", "P3.4", 19), "col_b_cash_on_hand_close_of_period");
    assert_eq!(at("SL", "P3.4", 36), "col_b_cash_on_hand_close_of_period_TODO_DUP");
    for version in ["3.00", "5.00", "5.3", "8.5"] {
        let cols = column_names_for_field("TEXT", version).unwrap();
        assert!(
            cols.iter().any(|c| c == "back_reference_sched_form_name")
                && !cols.iter().any(|c| c == "form_type"),
            "{version}: {cols:?}"
        );
    }
}
