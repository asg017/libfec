//! `cover_end`: on every committed fixture, every prefix shorter than the
//! settled length is `None`, every longer one gives the same length, and
//! the settled prefix parses to the same header and cover as the whole file.

use std::path::{Path, PathBuf};

use fec_parser::{cover_end, Filing};

/// Every `.fec` fixture under the crates' test fixture directories.
fn fixtures() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|e| e == "fec") {
                out.push(p);
            }
        }
    }
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut out = Vec::new();
    for dir in ["fec-parser", "fec-py", "fec-cli", "fec-node"] {
        walk(&crates.join(dir).join("tests").join("fixtures"), &mut out);
    }
    out.sort();
    assert!(out.len() > 100, "found only {} fixtures", out.len());
    out
}

/// The header and cover of `bytes`, comparably, or the error.
fn opened(bytes: &[u8]) -> String {
    match Filing::from_reader(bytes, String::new(), 0) {
        Ok(f) => {
            let c = &f.cover;
            format!(
                "{:?}\n{:?} {:?} {} {} {} {:?} {:?} {:?}\n{:?}",
                f.header,
                c.record,
                c.record.position().map(|p| p.line()),
                c.form_type,
                c.filer_id,
                c.filer_name,
                c.report_code,
                c.coverage_from_date,
                c.coverage_through_date,
                c.cover_data,
            )
        }
        Err(e) => format!("error: {e:#}"),
    }
}

#[test]
fn every_fixture_prefix() {
    for path in fixtures() {
        let full = std::fs::read(&path).expect("read fixture");
        let name = path.display();
        let Some(n) = cover_end(&full) else {
            // Nothing after the cover, and no final line ending: only the
            // end of the file settles it.
            for k in 0..full.len() {
                assert_eq!(cover_end(&full[..k]), None, "{name}: prefix {k}");
            }
            continue;
        };
        for k in 0..n {
            assert_eq!(cover_end(&full[..k]), None, "{name}: prefix {k} of {n}");
        }
        for k in n..full.len().min(n + 64) {
            assert_eq!(cover_end(&full[..k]), Some(n), "{name}: prefix {k}");
        }
        assert_eq!(opened(&full[..n]), opened(&full), "{name}: cut at {n}");
    }
}

/// `(input, expected)` for hand-made inputs.
fn check(cases: &[(&[u8], Option<usize>)]) {
    for (input, want) in cases {
        assert_eq!(
            cover_end(input),
            *want,
            "{:?}",
            String::from_utf8_lossy(input)
        );
    }
}

const HDR: &[u8] = b"HDR\x1cFEC\x1c8.4\x1cFECfile\x1c8.4.0.0\n";

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

#[test]
fn edges() {
    let cover: &[u8] = b"F3XN\x1cC00000001\x1cA Committee\n";
    let row: &[u8] = b"SA11AI\x1cC00000001\x1cIND\n";
    let n = HDR.len() + cover.len();
    let whole = cat(&[HDR, cover, row]);
    let crlf = cat(&[
        b"HDR\x1cFEC\x1c8.4\x1cFECfile\x1c8.4.0.0\r\n",
        b"F3XN\x1cC00000001\x1cA Committee\r\n",
    ]);
    check(&[
        (b"", None),
        (HDR, None),
        (&whole[..n - 1], None),
        (&whole[..n], Some(n)),
        (&whole, Some(n)),
        // The csv reader ends a record at `\r`: the `\n` isn't needed.
        (&crlf[..crlf.len() - 2], None),
        (&crlf[..crlf.len() - 1], Some(crlf.len() - 1)),
        (&crlf, Some(crlf.len() - 1)),
    ]);
}

#[test]
fn f99_text_belongs_to_the_cover() {
    let cover: &[u8] = b"F99\x1cC00000001\x1cA Committee\n";
    let text: &[u8] = b"[BEGINTEXT]\nDear FEC,\n\nhello\n[ENDTEXT]\n";
    let row: &[u8] = b"SA11AI\x1cC00000001\x1cIND\n";
    let with_text = cat(&[HDR, cover, text, row]);
    let settled = HDR.len() + cover.len() + text.len();
    let before_end = settled - b"[ENDTEXT]\n".len();
    let no_text = cat(&[HDR, cover, row]);
    let peeked = HDR.len() + cover.len() + row.len();
    check(&[
        // The next line decides whether a text block follows.
        (&with_text[..HDR.len() + cover.len()], None),
        (&with_text[..before_end], None),
        (&with_text[..settled - 1], None),
        (&with_text[..settled], Some(settled)),
        (&with_text, Some(settled)),
        (&no_text[..peeked - 1], None),
        (&no_text, Some(peeked)),
    ]);
    let f = Filing::from_reader(&with_text[..settled], String::new(), 0).expect("parse");
    assert!(format!("{:?}", f.cover.cover_data).contains("hello"));
}

#[test]
fn errors_are_settled_too() {
    // Not a filing: the first line already decides that.
    let junk: &[u8] = b"hello world\nmore\n";
    let n = cover_end(junk).expect("settled");
    assert!(Filing::from_reader(&junk[..n], String::new(), 0).is_err());
}
