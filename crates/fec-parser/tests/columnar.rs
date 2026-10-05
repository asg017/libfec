//! The `columnar` feature: the flattened column schema of every itemization
//! family and cover form (the contract R users see as table columns), and
//! typed fixture rows pushed through the column builders.
//!
//! Accept new or changed snapshots with
//! `cargo insta test -p fec-parser --features columnar --accept`.
#![cfg(feature = "columnar")]

use fec_parser::columnar::{new_builders, ColumnBuilder, ColumnDef, ColumnarEnum};
use fec_parser::covers::Cover;
use fec_parser::itemizations::{record_family, Itemization};
use fec_parser::Filing;
use std::fmt::Write;
use std::path::Path;

fn schema(defs: &[ColumnDef], skipped: &[String]) -> String {
    let mut s = String::new();
    for d in defs {
        let _ = writeln!(s, "  {:<6} {}", format!("{:?}", d.kind), d.name);
    }
    for name in skipped {
        let _ = writeln!(s, "  (skipped Vec) {name}");
    }
    s
}

#[test]
fn itemization_family_columns() {
    let mut out = String::new();
    for family in Itemization::families() {
        let defs = Itemization::family_columns(family).expect("known family");
        let skipped = <Itemization as ColumnarEnum>::skipped_for(family).unwrap_or_default();
        let _ = writeln!(out, "{family} ({} columns)", defs.len());
        out.push_str(&schema(&defs, &skipped));
    }
    insta::assert_snapshot!(out);
}

#[test]
fn cover_form_columns() {
    let mut out = String::new();
    for form in Cover::forms() {
        let defs = Cover::form_columns(form).expect("known form");
        let skipped = <Cover as ColumnarEnum>::skipped_for(form).unwrap_or_default();
        let _ = writeln!(out, "{form} ({} columns)", defs.len());
        out.push_str(&schema(&defs, &skipped));
    }
    insta::assert_snapshot!(out);
}

fn open(fixture: &str) -> Filing<std::fs::File> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/itemizations")
        .join(fixture);
    Filing::<std::fs::File>::from_path(&path).unwrap_or_else(|e| panic!("parsing {fixture}: {e}"))
}

/// Every pushed value that isn't missing, as `row.column = value`.
fn render(defs: &[ColumnDef], builders: &[ColumnBuilder], rows: usize) -> String {
    let mut s = String::new();
    for row in 0..rows {
        for (d, b) in defs.iter().zip(builders) {
            let v = match b {
                ColumnBuilder::Text(v) => v[row].as_ref().map(|x| format!("{x:?}")),
                ColumnBuilder::Float(v) => v[row].map(|x| format!("{x}")),
                ColumnBuilder::Date(v) => v[row].map(|x| format!("{x}d")),
                ColumnBuilder::Bool(v) => v[row].map(|x| format!("{x}")),
                ColumnBuilder::Int(v) => v[row].map(|x| format!("{x}i")),
            };
            if let Some(v) = v {
                let _ = writeln!(s, "{row}.{} = {v}", d.name);
            }
        }
    }
    s
}

/// Push a fixture's SA rows; returns the column names and the rendered table.
fn schedule_a_table(fixture: &str) -> (Vec<String>, String) {
    let mut filing = open(fixture);
    let version = filing.header.fec_version.clone();
    let delimiter = filing.header.name_delimiter.clone();
    let defs = Itemization::family_columns("SA").expect("SA");
    let mut builders = new_builders(&defs, 4);
    let mut sa_rows = 0;
    while let Some(row) = filing.next_row() {
        let row = row.unwrap_or_else(|e| panic!("{fixture}: {e}"));
        if record_family(row.record.get(0).unwrap_or_default()) != Some("SA") {
            continue;
        }
        sa_rows += 1;
        let item = Itemization::from_record(&row.record, &version, delimiter.as_deref())
            .unwrap_or_else(|| panic!("{fixture}: SA row did not type"));
        assert_eq!(item.family(), "SA");
        item.push_columns(&mut builders);
    }
    assert_eq!(sa_rows, 4, "{fixture}: SA rows");
    assert!(
        builders.iter().all(|b| b.len() == sa_rows),
        "{fixture}: every column has one value per SA row"
    );
    let names = defs.iter().map(|d| d.name.clone()).collect();
    (names, render(&defs, &builders, sa_rows))
}

#[test]
fn schedule_a_fixtures_push_the_same_columns() {
    let fixtures = [
        ("v8_5", "SA_1920342.fec"),
        ("v6_4", "SA_462580.fec"),
        ("v5_3", "SA_265857.fec"),
        ("v3", "SA_42174.fec"),
    ];
    let mut first: Option<Vec<String>> = None;
    for (version, fixture) in fixtures {
        let (names, table) = schedule_a_table(fixture);
        match &first {
            None => first = Some(names),
            Some(want) => assert_eq!(&names, want, "{fixture}: columns differ"),
        }
        insta::assert_snapshot!(format!("schedule_a_{version}"), table);
    }
}

#[test]
fn cover_pushes_one_row() {
    let filing = open("SA_1920342.fec");
    let cover = filing.cover.cover_data.expect("typed cover");
    assert_eq!(cover.form(), "Form3");
    let defs = cover.columns();
    let mut builders = new_builders(&defs, 1);
    cover.push_columns(&mut builders);
    assert!(builders.iter().all(|b| b.len() == 1));
    insta::assert_snapshot!(render(&defs, &builders, 1));
}
