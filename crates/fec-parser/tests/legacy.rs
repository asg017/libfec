//! Tests for every format family the parser reads (see `fec_parser::format`).
//!
//! Each fixture in `tests/fixtures/legacy/` is a real filing trimmed to its
//! header, cover, any text block, and up to two rows of each row type, named
//! `{FEC_VERSION}_{FILING_ID}.fec`. The `8.x_*` ones are 8.x filings with
//! quirks the FS path must keep handling as before (CRLF, quoted HDR fields,
//! a `"Jack" C.` name).
//!
//! The snapshot shows the header (Debug), cover kv, typed-cover presence and
//! F99 text, then every row with its position, so a change in how any family
//! is read is visible in review.

use fec_parser::{Delimiter, Filing, HeaderStyle};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/legacy")
        .join(name)
}

fn open(name: &str) -> Filing<std::fs::File> {
    let path = fixture_path(name);
    let id = name.rsplit('_').next().unwrap().trim_end_matches(".fec");
    let file = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    let len = file.metadata().unwrap().len() as usize;
    Filing::from_reader(file, id.to_string(), len).unwrap_or_else(|e| panic!("{name}: {e:#}"))
}

fn render(name: &str) -> String {
    let mut filing = open(name);
    let mut out = String::new();
    writeln!(out, "{:#?}", filing.header).unwrap();
    let c = &filing.cover;
    writeln!(
        out,
        "cover: form_type={:?} filer_id={:?} filer_name={:?} report_code={:?} from={:?} through={:?} typed={}",
        c.form_type,
        c.filer_id,
        c.filer_name,
        c.report_code,
        c.coverage_from_date,
        c.coverage_through_date,
        c.cover_data.is_some(),
    )
    .unwrap();
    for (k, v) in &c.cover_record_kv {
        writeln!(out, "  {k} = {v:?}").unwrap();
    }
    if let Some(fec_parser::covers::Cover::Form99(f)) = &c.cover_data {
        writeln!(out, "f99 text: {:#?}", f.text).unwrap();
    }
    while let Some(row) = filing.next_row() {
        let row = row.unwrap_or_else(|e| panic!("{name}: {e}"));
        let p = row.record.position().expect("row position");
        let fields: Vec<&str> = row.record.iter().collect();
        writeln!(
            out,
            "row {:?} line={} byte={} record={} fields={:?}",
            row.row_type,
            p.line(),
            p.byte(),
            p.record(),
            fields
        )
        .unwrap();
    }
    out
}

macro_rules! legacy_snapshot {
    ($name:ident, $fixture:literal) => {
        #[test]
        fn $name() {
            insta::assert_snapshot!(render($fixture));
        }
    };
}

legacy_snapshot!(v1_02_497, "1.02_497.fec");
legacy_snapshot!(v2_02_10665, "2.02_10665.fec");
legacy_snapshot!(v3_00_13801, "3.00_13801.fec");
legacy_snapshot!(v5_00_102196, "5.00_102196.fec");
legacy_snapshot!(v5_3_300707, "5.3_300707.fec");
legacy_snapshot!(v6_1_342096, "6.1_342096.fec");
legacy_snapshot!(v7_0_730663, "7.0_730663.fec");
legacy_snapshot!(p1_0_236480, "P1.0_236480.fec");
legacy_snapshot!(p2_4_391955, "P2.4_391955.fec");
legacy_snapshot!(p2_6_716051, "P2.6_716051.fec");
legacy_snapshot!(p3_2_1081726, "P3.2_1081726.fec");
legacy_snapshot!(p3_4_1215766, "P3.4_1215766.fec");
legacy_snapshot!(v8_crlf_1671120, "8.x_1671120.fec");
legacy_snapshot!(v8_quoted_hdr_1892731, "8.x_1892731.fec");
legacy_snapshot!(v8_quoted_hdr_1914568, "8.x_1914568.fec");
legacy_snapshot!(v8_quoted_name_1926167, "8.x_1926167.fec");

fn all_fixtures() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(fixture_path(""))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".fec"))
        .collect();
    names.sort();
    names
}

#[test]
fn styles_and_delimiters() {
    let got: Vec<(String, &str, &str, bool)> = all_fixtures()
        .iter()
        .map(|n| {
            let h = open(n).header;
            (
                n.clone(),
                h.style.as_str(),
                h.delimiter.as_str(),
                h.is_paper(),
            )
        })
        .collect();
    insta::assert_debug_snapshot!(got);
}

/// In comma files, every record's position is the absolute start of its
/// line: the byte offset includes any `/* Header` block, and the line number
/// counts every line before it.
#[test]
fn comma_positions_are_absolute_line_starts() {
    for name in all_fixtures() {
        let mut filing = open(&name);
        if filing.header.delimiter != Delimiter::Comma {
            continue;
        }
        let data = std::fs::read(fixture_path(&name)).unwrap();
        let check = |record: &csv::StringRecord| {
            let p = record.position().expect("position");
            let byte = p.byte() as usize;
            assert!(byte == 0 || data[byte - 1] == b'\n', "{name}: byte {byte}");
            let line = 1 + data[..byte].iter().filter(|b| **b == b'\n').count() as u64;
            assert_eq!(line, p.line(), "{name}: line at byte {byte}");
        };
        check(&filing.cover.record);
        if filing.header.style != HeaderStyle::LegacyBlock {
            check(&filing.header.header_record);
        }
        while let Some(row) = filing.next_row() {
            check(&row.unwrap().record);
        }
    }
}

/// FS files go through the same csv reader as always: header, cover and
/// rows are exactly the records a plain FS csv reader yields, positions
/// included.
#[test]
fn fs_records_match_plain_csv_reader() {
    for name in all_fixtures() {
        let mut filing = open(&name);
        if filing.header.delimiter != Delimiter::Fs {
            continue;
        }
        let mut plain = csv::ReaderBuilder::new()
            .delimiter(0x1c)
            .flexible(true)
            .has_headers(false)
            .from_path(fixture_path(&name))
            .unwrap()
            .into_records()
            .map(|r| r.unwrap())
            .filter(|r| r.iter().any(|f| !f.trim().is_empty()));
        let same = |a: &csv::StringRecord, b: &csv::StringRecord| {
            assert_eq!(a, b, "{name}");
            assert_eq!(a.position(), b.position(), "{name}");
        };
        same(&filing.header.header_record, &plain.next().unwrap());
        same(&filing.cover.record, &plain.next().unwrap());
        let in_text = filing.cover.form_type.starts_with("F99");
        if in_text {
            continue;
        }
        while let Some(row) = filing.next_row() {
            same(&row.unwrap().record, &plain.next().unwrap());
        }
        assert!(plain.next().is_none(), "{name}: rows left over");
    }
}

/// The F99 letter in a comma file keeps its commas, quotes and blank lines.
#[test]
fn comma_f99_text_is_verbatim() {
    let filing = open("5.3_300707.fec");
    let Some(fec_parser::covers::Cover::Form99(f)) = &filing.cover.cover_data else {
        panic!("expected a typed F99 cover");
    };
    let data = String::from_utf8(std::fs::read(fixture_path("5.3_300707.fec")).unwrap()).unwrap();
    let start = data.find("[BEGINTEXT]\n").unwrap() + "[BEGINTEXT]\n".len();
    let end = data.find("[ENDTEXT]").unwrap();
    // Leading and trailing blank lines are dropped (read_text_block).
    let expected = data[start..end].trim_end().trim_start_matches('\n');
    assert_eq!(f.text.as_deref(), Some(expected));
    assert!(expected.contains("\n\n") && expected.contains(','));
}

#[test]
fn unsupported_versions_are_rejected() {
    for (input, version) in [
        ("HDR\x1cFEC\x1c8.6\x1cX\x1c1\nF3XN\x1cC1\n", "8.6"),
        ("HDR,FEC,4.0,X,1,,,\nF3XN,C1\n", "4.0"),
        ("HDR\x1cFEC\x1c60\x1cX\x1c1\nF3XN\x1cC1\n", "60"),
        (
            "/* Header\nFEC_Ver_# = 9.1\n/* End Header\nF3XN,C1\n",
            "9.1",
        ),
    ] {
        let err = Filing::from_reader(input.as_bytes(), "1".into(), input.len())
            .err()
            .unwrap_or_else(|| panic!("{input:?} should be rejected"));
        let msg = format!("{err:#}");
        assert!(
            msg.contains(&format!("Unsupported version '{version}'")) && msg.contains("P3.0-P3.4"),
            "{msg}"
        );
    }
    let input = "/* Header\nSoft_Name = X\n/* End Header\nF3XN,C1\n";
    let err = Filing::from_reader(input.as_bytes(), "1".into(), input.len())
        .err()
        .expect("missing FEC_Ver_# is an error");
    assert!(format!("{err:#}").contains("FEC_Ver_#"));
}
