//! Regression tests for small hand-made inputs: line-ending and leading-line
//! quirks the real-filing fixtures in `tests/fixtures/legacy/` don't cover.

use fec_parser::{Delimiter, Filing, HeaderStyle};

/// What a parse of `input` yields: header version, delimiter and style,
/// cover form type and filer, then `(row_type, line, byte)` of every row.
#[derive(Debug, PartialEq)]
struct Parsed {
    version: String,
    delimiter: Delimiter,
    style: HeaderStyle,
    form_type: String,
    filer_id: String,
    rows: Vec<(String, u64, u64)>,
}

fn parse(input: &[u8]) -> Parsed {
    let mut filing = Filing::from_reader(input, "1".into(), input.len())
        .unwrap_or_else(|e| panic!("{:?}: {e:#}", String::from_utf8_lossy(input)));
    let mut rows = Vec::new();
    while let Some(row) = filing.next_row() {
        let row = row.expect("row");
        let p = row.record.position().expect("position");
        rows.push((row.row_type, p.line(), p.byte()));
    }
    Parsed {
        version: filing.header.fec_version.clone(),
        delimiter: filing.header.delimiter,
        style: filing.header.style,
        form_type: filing.cover.form_type.clone(),
        filer_id: filing.cover.filer_id.clone(),
        rows,
    }
}

/// An 8.x filing that starts with blank lines parses like it always did:
/// the csv reader of the FS path skips them, so sniffing must too.
#[test]
fn fs_leading_blank_lines() {
    for prefix in ["\n", "\r\n", "\n\n", "\r\n\r\n"] {
        let input = format!(
            "{prefix}HDR\x1cFEC\x1c8.4\x1cTest\x1c1\nF3XN\x1cC00000001\x1cCommittee\nSA11AI\x1cC00000001\x1cX\n"
        );
        let got = parse(input.as_bytes());
        let byte = input.find("SA11AI").unwrap() as u64;
        assert_eq!(
            (got.version.as_str(), got.delimiter, got.form_type.as_str()),
            ("8.4", Delimiter::Fs, "F3XN"),
            "{prefix:?}"
        );
        // Same positions as a plain FS csv reader gives.
        let plain = csv::ReaderBuilder::new()
            .delimiter(0x1c)
            .flexible(true)
            .has_headers(false)
            .from_reader(input.as_bytes())
            .into_records()
            .last()
            .unwrap()
            .unwrap();
        let p = plain.position().unwrap();
        assert_eq!(
            got.rows,
            vec![("SA11AI".into(), p.line(), byte)],
            "{prefix:?}"
        );
    }
}

/// Comma and `/* Header` filings may start with blank lines too; positions
/// stay absolute.
#[test]
fn comma_and_legacy_block_leading_blank_lines() {
    let got = parse(b"\n\nHDR,FEC,5.3,Test,1,^\nF3XN,C00000001,Committee\nSA11AI,C00000001,X\n");
    assert_eq!(
        (got.delimiter, got.style, got.rows),
        (
            Delimiter::Comma,
            HeaderStyle::Hdr,
            vec![("SA11AI".into(), 5, 48)]
        )
    );
    let got = parse(
        b"\n \n/* Header\nFEC_Ver_# = 2.02\n/* End Header\nF3XN,C00000001,Committee\nSA11AI,C00000001,X\n",
    );
    assert_eq!(
        (
            got.version.as_str(),
            got.style,
            got.filer_id.as_str(),
            got.rows
        ),
        (
            "2.02",
            HeaderStyle::LegacyBlock,
            "C00000001",
            vec![("SA11AI".into(), 7, 69)]
        )
    );
}

/// Comma files with old-Mac CR-only line endings: a lone `\r` ends a line,
/// as it does for the csv reader of the FS path.
#[test]
fn comma_cr_only_line_endings() {
    let got = parse(b"HDR,FEC,5.3,Test,1,^\rF3XN,C00000001,Committee\rSA11AI,C00000001,X\r");
    assert_eq!(
        (got.version.as_str(), got.form_type.as_str(), got.rows),
        ("5.3", "F3XN", vec![("SA11AI".into(), 3, 46)])
    );

    let input = b"HDR,FEC,5.3,Test,1,^\rF99,C00000001,Committee\r[BEGINTEXT]\rDear FEC,\r\rThanks\r[ENDTEXT]\rSA11AI,C00000001,X\r";
    let filing = Filing::from_reader(input.as_slice(), "1".into(), input.len()).expect("parse");
    let Some(fec_parser::covers::Cover::Form99(f)) = &filing.cover.cover_data else {
        panic!("expected a typed F99 cover");
    };
    assert_eq!(f.text.as_deref(), Some("Dear FEC,\n\nThanks"));
}

/// Typed covers split legacy combined names on the header's `name_delim`,
/// not a hard-coded `^`.
#[test]
fn typed_cover_uses_header_name_delimiter() {
    let treasurer = |delim: &str, name: &str| {
        let input = format!(
            "HDR,FEC,5.00,Vocus PAC Management,3.00.1028,{delim},,0,\nF24N,C00000885,Committee,\"1750 New York Avenue, NW\",,Washington,DC,20006,{name},20031114,48\n"
        );
        let filing = Filing::from_reader(input.as_bytes(), "1".into(), input.len()).expect("parse");
        let Some(fec_parser::covers::Cover::Form24(f)) = filing.cover.cover_data else {
            panic!("expected a typed F24 cover");
        };
        (f.treasurer.last_name, f.treasurer.first_name)
    };
    let expected = ("Galis".to_owned(), "George".to_owned());
    assert_eq!(treasurer("|", "Galis|George||"), expected);
    assert_eq!(treasurer("", "Galis^George^^"), expected);
    assert_eq!(treasurer("^", "Galis^George^^"), expected);
}
