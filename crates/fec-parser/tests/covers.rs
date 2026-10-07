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

// Legacy (pre-8.0 and paper) covers. `cover_snapshot!` fixtures above are
// trimmed to HDR + cover; `legacy_cover_snapshot!` reads the multi-record
// fixtures in `tests/fixtures/legacy/` (see `tests/legacy.rs`), which include
// v1/v2 `/*` header blocks. See wiki/legacy/COVERS.md for what was checked.
cover_snapshot!(f1n_337691_v6_1, "F1N_337691.fec"); // v6.1 89-field F1 layout
cover_snapshot!(f1a_357167_v6_2, "F1A_357167.fec"); // v6.2 90-field F1 layout
cover_snapshot!(f24n_99717_v5_00, "F24N_99717.fec"); // caret treasurer_name
cover_snapshot!(f3ln_1096237_p3_2, "F3LN_1096237.fec"); // paper 2nd election_state

fn legacy_cover(fixture: &str) -> fec_parser::covers::Cover {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/legacy")
        .join(fixture);
    open_fixture(&path)
        .cover
        .cover_data
        .unwrap_or_else(|| panic!("{fixture}: no typed cover_data"))
}

macro_rules! legacy_cover_snapshot {
    ($name:ident, $fixture:literal) => {
        #[test]
        fn $name() {
            insta::assert_json_snapshot!(legacy_cover($fixture));
        }
    };
}

legacy_cover_snapshot!(legacy_f3x_v1_02_497, "1.02_497.fec");
legacy_cover_snapshot!(legacy_f3p_v2_02_10665, "2.02_10665.fec");
legacy_cover_snapshot!(legacy_f3x_v3_00_13801, "3.00_13801.fec");
legacy_cover_snapshot!(legacy_f3x_v5_00_102196, "5.00_102196.fec");
legacy_cover_snapshot!(legacy_f3x_p1_0_236480, "P1.0_236480.fec"); // no-decimal amounts kept raw
legacy_cover_snapshot!(legacy_f3_p2_4_391955, "P2.4_391955.fec");
legacy_cover_snapshot!(legacy_f7_p3_2_1081726, "P3.2_1081726.fec");
legacy_cover_snapshot!(legacy_f3x_p3_4_1215766, "P3.4_1215766.fec"); // Line 6(a) year/amount order

/// An individual F5 filer leaves `organization_name` blank; `filer_name`
/// falls back to the individual's name. No corpus filing has one, so this
/// edits a real F5N cover: entity type `IND`, blank organization, a person.
#[test]
fn f5_individual_filer_name() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/covers/F5N_1888248.fec");
    let raw = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&raw);
    let mut lines = text.lines();
    let hdr = lines.next().unwrap();
    let mut cover: Vec<String> = lines
        .next()
        .unwrap()
        .split('\x1c')
        .map(str::to_owned)
        .collect();
    cover[2] = "IND".into();
    cover[3] = String::new();
    cover[4] = "Doe".into();
    cover[5] = "Jane".into();
    let bytes = format!("{hdr}\n{}\n", cover.join("\x1c")).into_bytes();
    let len = bytes.len();
    let filing = Filing::from_reader(bytes.as_slice(), "1888248".into(), len).unwrap();
    assert_eq!(filing.cover.filer_name, "Jane Doe");
}
cover_snapshot!(f2n_1923633, "F2N_1923633.fec");
cover_snapshot!(f2a_1902439, "F2A_1902439.fec");
// F99 fixtures are whole filings: the message body is the `[BEGINTEXT]` block
// after the cover record, which the typed cover reads too.
cover_snapshot!(f99_1945322, "F99_1945322.fec");
cover_snapshot!(f99_1909934, "F99_1909934.fec");

/// Paper Form 99 layouts (P3.2–P3.4) have no committee-name column. No
/// sample filing has one, so this is a synthetic record in that layout:
/// before, the cover failed to parse; now `filer_name` is empty and the
/// typed cover still carries the committee ID.
#[test]
fn paper_f99_without_name_column() {
    let fec = "HDR\x1cP3.4\x1cSOFT\x1c1\x1c20190101\n\
               F99\x1cC00123456\x1c201901010300000001\x1c201901010300000002\x1c20190101\n";
    let filing = Filing::from_reader(fec.as_bytes(), "1".into(), fec.len()).unwrap();
    assert_eq!(filing.cover.filer_id, "C00123456");
    assert_eq!(filing.cover.filer_name, "");
    match filing.cover.cover_data {
        Some(fec_parser::covers::Cover::Form99(f)) => {
            assert_eq!(f.filer_committee_id, "C00123456");
            assert_eq!(f.committee_name, "");
        }
        other => panic!("expected Form99, got {other:?}"),
    }
}
