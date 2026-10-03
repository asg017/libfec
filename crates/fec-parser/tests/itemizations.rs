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

// Schedule B
itemization_snapshot!(sb_1907931_v8_4, "SB_1907931.fec");
itemization_snapshot!(sb_1902156_v8_4_f3l, "SB_1902156.fec");
itemization_snapshot!(sb_1953421_v8_5, "SB_1953421.fec");
itemization_snapshot!(sb_730985_v7_0, "SB_730985.fec");
itemization_snapshot!(sb_398066_v6_2, "SB_398066.fec");
itemization_snapshot!(sb_269708_v5_3, "SB_269708.fec");
itemization_snapshot!(sb_84285_v5_0, "SB_84285.fec");
itemization_snapshot!(sb_27475_v3, "SB_27475.fec");
// Schedule D
itemization_snapshot!(sd_1916849_v8_5, "SD_1916849.fec");
itemization_snapshot!(sd_1913965_v8_5_f3p, "SD_1913965.fec");
itemization_snapshot!(sd_1887446_v8_4, "SD_1887446.fec");
itemization_snapshot!(sd_853272_v8_0, "SD_853272.fec");
itemization_snapshot!(sd_509001_v6_4, "SD_509001.fec");
itemization_snapshot!(sd_279768_v5_3, "SD_279768.fec");
itemization_snapshot!(sd_163291_v5_1, "SD_163291.fec");
itemization_snapshot!(sd_98650_v5_0, "SD_98650.fec");
itemization_snapshot!(sd_53060_v3, "SD_53060.fec");

// Schedule F
itemization_snapshot!(sf_1903343_v8_4, "SF_1903343.fec");
itemization_snapshot!(sf_1904186_v8_4, "SF_1904186.fec");
itemization_snapshot!(sf_1955691_v8_5, "SF_1955691.fec");
itemization_snapshot!(sf_1954205_v8_5, "SF_1954205.fec");
