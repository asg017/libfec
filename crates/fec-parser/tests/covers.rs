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
cover_snapshot!(f3n_1858438, "F3N_1858438.fec");
cover_snapshot!(f3t_1917347, "F3T_1917347.fec");
cover_snapshot!(f3pa_1863008, "F3PA_1863008.fec");
cover_snapshot!(f3pn_1920459, "F3PN_1920459.fec");
