//! Snapshot tests for the typed itemization structs in `fec_parser::itemizations`.
//!
//! Each fixture in `tests/fixtures/itemizations/` is a real filing cut down to
//! its header, cover and a few rows of one record family, named
//! `{FAMILY}_{FILING_ID}.fec` (made with `examples/itemization_fixture.rs`).
//! The snapshot is the JSON of every row's `Itemization`, `null` for a row
//! that did not type, so every typed field is visible in review.
//!
//! To add one: cut a fixture, add an `itemization_snapshot!` line, run
//! `cargo insta test -p fec-parser --accept`, and read the new snapshot.

use fec_parser::itemizations::Itemization;
use fec_parser::Filing;
use std::path::Path;

fn itemizations(fixture: &str) -> Vec<Option<Itemization>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/itemizations")
        .join(fixture);
    let mut filing = Filing::<std::fs::File>::from_path(&path)
        .unwrap_or_else(|e| panic!("parsing {fixture}: {e}"));
    let version = filing.header.fec_version.clone();
    let delimiter = filing.header.name_delimiter.clone();
    let mut items = vec![];
    while let Some(row) = filing.next_row() {
        let row = row.unwrap_or_else(|e| panic!("{fixture}: {e}"));
        items.push(Itemization::from_record(&row.record, &version, delimiter.as_deref()));
    }
    assert!(!items.is_empty(), "{fixture}: no rows");
    items
}

macro_rules! itemization_snapshot {
    ($name:ident, $fixture:literal) => {
        #[test]
        fn $name() {
            insta::assert_json_snapshot!(itemizations($fixture));
        }
    };
}

// Schedule A
itemization_snapshot!(sa_1907925_v8_4, "SA_1907925.fec");
itemization_snapshot!(sa_1920342_v8_5, "SA_1920342.fec");
itemization_snapshot!(sa_462580_v6_4, "SA_462580.fec");
itemization_snapshot!(sa_265857_v5_3, "SA_265857.fec");
itemization_snapshot!(sa_42174_v3, "SA_42174.fec");
