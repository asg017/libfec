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
        items.push(Itemization::from_record(
            &row.record,
            &version,
            delimiter.as_deref(),
        ));
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
// Schedules H1–H6
itemization_snapshot!(h1_1944956_v8_5, "H1_1944956.fec");
itemization_snapshot!(h1_1948502_v8_5, "H1_1948502.fec");
itemization_snapshot!(h1_1891621_v8_4, "H1_1891621.fec");
itemization_snapshot!(h1_181668_v5_2, "H1_181668.fec");
itemization_snapshot!(h2_1955758_v8_5, "H2_1955758.fec");
itemization_snapshot!(h2_1893234_v8_4, "H2_1893234.fec");
itemization_snapshot!(h2_1904395_v8_4, "H2_1904395.fec");
itemization_snapshot!(h3_1925226_v8_5, "H3_1925226.fec");
itemization_snapshot!(h3_1891862_v8_4, "H3_1891862.fec");
itemization_snapshot!(h3_1912571_v8_4, "H3_1912571.fec");
itemization_snapshot!(h3_305041_v5_3, "H3_305041.fec");
itemization_snapshot!(h3_181668_v5_2, "H3_181668.fec");
itemization_snapshot!(h4_1924899_v8_5, "H4_1924899.fec");
itemization_snapshot!(h4_1907825_v8_4, "H4_1907825.fec");
itemization_snapshot!(h4_305041_v5_3, "H4_305041.fec");
itemization_snapshot!(h4_181668_v5_2, "H4_181668.fec");
itemization_snapshot!(h5_1948807_v8_5, "H5_1948807.fec");
itemization_snapshot!(h5_1893062_v8_4, "H5_1893062.fec");
// Form 5/6/7/9/13 line items
itemization_snapshot!(f56_1912883_v8_4, "F56_1912883.fec");
itemization_snapshot!(f56_1920821_v8_5, "F56_1920821.fec");
itemization_snapshot!(f57_1888833_v8_4, "F57_1888833.fec");
itemization_snapshot!(f57_1917549_v8_5, "F57_1917549.fec");
itemization_snapshot!(f65_1912946_v8_4, "F65_1912946.fec");
itemization_snapshot!(f65_1946674_v8_5, "F65_1946674.fec");
itemization_snapshot!(f76_1884734_v8_4, "F76_1884734.fec");
itemization_snapshot!(f132_1904840_v8_4, "F132_1904840.fec");
itemization_snapshot!(f133_1904839_v8_4, "F133_1904839.fec");
// Schedule L
itemization_snapshot!(sl_1922499_v8_5, "SL_1922499.fec");
itemization_snapshot!(sl_1893062_v8_4, "SL_1893062.fec");
// TEXT
itemization_snapshot!(text_1949756_v8_5, "TEXT_1949756.fec");
itemization_snapshot!(text_1893403_v8_4, "TEXT_1893403.fec");
itemization_snapshot!(text_727410_v7_0, "TEXT_727410.fec");
itemization_snapshot!(text_472783_v6_4, "TEXT_472783.fec");
itemization_snapshot!(text_245398_v5_3, "TEXT_245398.fec");
itemization_snapshot!(text_98650_v5_00, "TEXT_98650.fec");
itemization_snapshot!(text_48283_v3, "TEXT_48283.fec");
// Schedule E
itemization_snapshot!(se_1951289_v8_5_f24, "SE_1951289.fec");
itemization_snapshot!(se_1923919_v8_5_f24_memo, "SE_1923919.fec");
itemization_snapshot!(se_1945953_v8_5_f3x, "SE_1945953.fec");
itemization_snapshot!(se_1636509_v8_4_f24, "SE_1636509.fec");
itemization_snapshot!(se_1883470_v8_4_f3x, "SE_1883470.fec");
itemization_snapshot!(se_1909829_v8_4_f3x_backref, "SE_1909829.fec");
itemization_snapshot!(se_1466607_v8_3_f24, "SE_1466607.fec");
itemization_snapshot!(se_1455422_v8_3_f3x, "SE_1455422.fec");
itemization_snapshot!(se_1215766_p3_4, "SE_1215766.fec");

// Schedule A (Form 3L bundling)
itemization_snapshot!(sa3l_1775683_v8_4_ind, "SA3L_1775683.fec");
itemization_snapshot!(sa3l_1887911_v8_4_pac, "SA3L_1887911.fec");
itemization_snapshot!(sa3l_1922835_v8_5_ind, "SA3L_1922835.fec");
itemization_snapshot!(sa3l_1921461_v8_5_pac, "SA3L_1921461.fec");
// Schedule C
itemization_snapshot!(sc_1930372_v8_5, "SC_1930372.fec");
itemization_snapshot!(sc_1945153_v8_5, "SC_1945153.fec");
itemization_snapshot!(sc_1918637_v8_5, "SC_1918637.fec");
itemization_snapshot!(sc_1884585_v8_4, "SC_1884585.fec");
itemization_snapshot!(sc_1884032_v8_4, "SC_1884032.fec");
itemization_snapshot!(sc_785646_v8_0, "SC_785646.fec");
itemization_snapshot!(sc_509001_v6_4, "SC_509001.fec");
itemization_snapshot!(sc_306688_v5_3, "SC_306688.fec");
itemization_snapshot!(sc_130085_v5_1, "SC_130085.fec");
itemization_snapshot!(sc_60410_v3, "SC_60410.fec");

// Schedule C-1
itemization_snapshot!(sc1_1935097_v8_5, "SC1_1935097.fec");
itemization_snapshot!(sc1_1906567_v8_4, "SC1_1906567.fec");
itemization_snapshot!(sc1_130085_v5_1, "SC1_130085.fec");

// Schedule C-2
itemization_snapshot!(sc2_1944255_v8_5, "SC2_1944255.fec");
itemization_snapshot!(sc2_1883359_v8_4, "SC2_1883359.fec");
itemization_snapshot!(sc2_756408_v8_0, "SC2_756408.fec");
itemization_snapshot!(sc2_706264_v6_4, "SC2_706264.fec");
