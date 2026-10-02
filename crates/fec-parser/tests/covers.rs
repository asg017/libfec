//! Snapshot tests for the typed cover structs in `fec_parser::covers`.
//!
//! Each fixture in `tests/fixtures/covers/` is a real filing truncated to its
//! first two records (HDR + cover), named `{FORM_TYPE}_{FILING_ID}.fec`. The
//! snapshot is the JSON serialisation of `FilingCover::cover_data`, so every
//! typed field is visible in review.
//!
//! To add a cover test: drop a fixture in, add a `cover_snapshot!` line, run
//! `cargo insta test -p fec-parser --accept`, and read the new snapshot.

use fec_parser::Filing;
use std::path::Path;

/// Open a fixture named `{FORM_TYPE}_{FILING_ID}.fec`, using the real filing id.
fn open_fixture(path: &Path) -> Filing<std::fs::File> {
    let name = path.file_stem().unwrap().to_string_lossy();
    let filing_id = name.rsplit('_').next().unwrap().to_string();
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("{name}: {e}"));
    let len = file.metadata().unwrap().len() as usize;
    Filing::from_reader(file, filing_id, len).unwrap_or_else(|e| panic!("parsing {name}: {e}"))
}

fn cover(fixture: &str) -> fec_parser::covers::Cover {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/covers")
        .join(fixture);
    let filing = open_fixture(&path);
    filing
        .cover
        .cover_data
        .unwrap_or_else(|| panic!("{fixture}: no typed cover_data"))
}

macro_rules! cover_snapshot {
    ($name:ident, $fixture:literal) => {
        #[test]
        fn $name() {
            insta::assert_json_snapshot!(cover($fixture));
        }
    };
}

cover_snapshot!(f1n_1910281, "F1N_1910281.fec");
cover_snapshot!(f3n_1918805, "F3N_1918805.fec");
cover_snapshot!(f3pn_1887806, "F3PN_1887806.fec");
cover_snapshot!(f1n_1906351, "F1N_1906351.fec");
cover_snapshot!(f1a_1914988, "F1A_1914988.fec");
cover_snapshot!(f1a_1917499, "F1A_1917499.fec");
cover_snapshot!(f1mn_1917288, "F1MN_1917288.fec");
cover_snapshot!(f1mn_1924609, "F1MN_1924609.fec");
cover_snapshot!(f3xn_1926068, "F3XN_1926068.fec");
cover_snapshot!(f3xa_1909193, "F3XA_1909193.fec");
cover_snapshot!(f3ln_1902042, "F3LN_1902042.fec");
cover_snapshot!(f3ln_1941874, "F3LN_1941874.fec");
cover_snapshot!(f4n_1901605, "F4N_1901605.fec");
cover_snapshot!(f4a_1920140, "F4A_1920140.fec");
cover_snapshot!(f7n_1884734, "F7N_1884734.fec");
cover_snapshot!(f13a_1910509, "F13A_1910509.fec");
cover_snapshot!(f13n_1904840, "F13N_1904840.fec");
cover_snapshot!(f3n_1858438, "F3N_1858438.fec");
cover_snapshot!(f3t_1917347, "F3T_1917347.fec");
cover_snapshot!(f3pa_1863008, "F3PA_1863008.fec");
cover_snapshot!(f3pn_1920459, "F3PN_1920459.fec");
cover_snapshot!(f24n_1946204, "F24N_1946204.fec");
cover_snapshot!(f24a_1952541, "F24A_1952541.fec");
cover_snapshot!(f5n_1888248, "F5N_1888248.fec");
cover_snapshot!(f5n_1914346, "F5N_1914346.fec");
cover_snapshot!(f5a_1900837, "F5A_1900837.fec");
cover_snapshot!(f6n_1947008, "F6N_1947008.fec");
cover_snapshot!(f6a_1952182, "F6A_1952182.fec");
cover_snapshot!(f9a_2015422, "F9A_2015422.fec");
