//! End-to-end checks of `libfec export` / `libfec fastfec` on small
//! hand-made filings mixing legacy and 8.x versions. Each test runs the
//! binary offline, with its own cache directory, over files in a temp dir.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A 5.00 (comma, combined names) F3X with one SA11AI from "Legacy^Alice".
const LEGACY_5_00: &str = "HDR,FEC,5.00,Test,1,,,,\nF3XN,C00000001,,Test\nSA11AI,C00000001,IND,Legacy^Alice,,,,,,,,,,,,123,,,,,,,,,,,,,,,,,,TX1,,,,\n";

/// An 8.5 (FS, split names) F3X with one SA11AI from "Modern, Bob".
const MODERN_8_5: &str = "HDR\x1cFEC\x1c8.5\x1cTest\x1c1\x1c\x1c\x1c\x1c\nF3XN\x1cC00000001\x1c\x1cTest\nSA11AI\x1cC00000001\x1cTX1\x1c\x1c\x1cIND\x1c\x1cModern\x1cBob\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c123\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\x1c\n";

struct Scratch {
    dir: tempfile::TempDir,
}

impl Scratch {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn write(&self, name: &str, contents: &str) -> PathBuf {
        let p = self.path(name);
        std::fs::write(&p, contents).unwrap();
        p
    }

    fn libfec(&self, args: &[&dyn AsRef<std::ffi::OsStr>]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_libfec"))
            .arg("--offline")
            .arg("--cache-directory")
            .arg(self.path("cache"))
            .args(args.iter().map(|a| a.as_ref()))
            .output()
            .unwrap()
    }
}

fn read_csv(path: &Path) -> Vec<Vec<String>> {
    csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .records()
        .map(|r| r.unwrap().iter().map(str::to_owned).collect())
        .collect()
}

/// `(filing_id, contributor_last_name, contributor_first_name)` of every row.
fn donors(rows: &[Vec<String>]) -> Vec<(String, String, String)> {
    let header = &rows[0];
    let col = |name: &str| {
        header
            .iter()
            .position(|c| c == name)
            .unwrap_or_else(|| panic!("no column {name} in {header:?}"))
    };
    let (id, last, first) = (
        col("filing_id"),
        col("contributor_last_name"),
        col("contributor_first_name"),
    );
    let mut got: Vec<_> = rows[1..]
        .iter()
        .map(|r| (r[id].clone(), r[last].clone(), r[first].clone()))
        .collect();
    got.sort();
    got
}

/// A directory CSV export writes every schedule in the 8.5 layout, so the
/// result doesn't depend on which filing comes first: legacy combined
/// names are split and 8.x split names kept, in either order.
#[test]
fn dir_csv_mixed_versions_any_order() {
    let s = Scratch::new();
    let legacy = s.write("500.fec", LEGACY_5_00);
    let modern = s.write("850.fec", MODERN_8_5);
    let expected = vec![
        ("500".to_owned(), "Legacy".to_owned(), "Alice".to_owned()),
        ("850".to_owned(), "Modern".to_owned(), "Bob".to_owned()),
    ];
    let mut headers = Vec::new();
    for (name, order) in [("a", [&legacy, &modern]), ("b", [&modern, &legacy])] {
        let out = s.path(name);
        let o = s.libfec(&[
            &"export",
            order[0],
            order[1],
            &"--output-directory",
            &out,
            &"-f",
            &"csv",
        ]);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let rows = read_csv(&out.join("schedule_a.csv"));
        assert_eq!(donors(&rows), expected, "order {name}");
        let covers = read_csv(&out.join("cover_F3X.csv"));
        headers.push((rows[0].clone(), covers[0].clone()));
    }
    assert_eq!(headers[0], headers[1]);
}

/// FS-delimited filing text from `|`-delimited lines.
fn fs(lines: &str) -> String {
    lines.replace('|', "\x1c")
}

/// A paper P3.4 F3L with an SA3L (contributor names), trimmed from
/// FEC-1314260.
const PAPER_SA3L: &str = "HDR|P3.4|\"Aurotech/Captricity\"|1|20190204
F3LN|C00674408|||PO BOX 230069||HOLLIS|NY|114230069|||Q2|20180120|NC||20180420|20180815|||||ANGRAND|ELMINA|TRACEY|||20190110|201901300300258854|201901300300258858|20190129
SA3L|C00674408||ANGRAND|ELMINA|TRACEY|||PO BOX 230069||HOLLIS|NM|11423||C00674408|STUDENT AT IONA COLLEGE||||||||201901300300258855
";

/// An 8.4 F3L with an SA3L (lobbyist bundling layout), trimmed from
/// FEC-1807079.
const V8_4_SA3L: &str = "HDR|FEC|8.4|Test|1||||
F3LN|C00828541|Committee|X|P.O. BOX 509||ARLINGTON|VA|22216|||Q2S||||20240401|20240630|||0.00|84000.00|CRATE|BRADLEY|T.|||20240731
SA3L|C00828541|SA3L.4369|||IND||MILLER|JEFF||||4723 CAT MOUNTAIN DR||AUSTIN|TX|78731||||0.00|84000.00||MILLER STRATEGIES LLC|CEO
";

/// A sqlite export's `libfec_schedule_a` has Schedule A's 8.5 layout
/// whichever row comes first: an SA3L (paper or 8.x lobbyist bundling)
/// first no longer gives the table its layout, which blanked later rows'
/// names. 8.x SA3L rows fill the contributor columns, as their 8.5 rows
/// always have positionally.
#[test]
fn sqlite_schedule_a_layout_does_not_depend_on_first_row() {
    let s = Scratch::new();
    let paper = s.write("1314260.fec", &fs(PAPER_SA3L));
    let v84 = s.write("1807079.fec", &fs(V8_4_SA3L));
    let modern = s.write("850.fec", MODERN_8_5);
    for (name, order) in [
        ("a.db", [&paper, &v84, &modern]),
        ("b.db", [&modern, &v84, &paper]),
    ] {
        let db = s.path(name);
        let o = s.libfec(&[&"export", order[0], order[1], order[2], &"-o", &db]);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let conn = rusqlite::Connection::open(&db).unwrap();
        let mut stmt = conn
            .prepare(
                "select filing_id, form_type, contributor_last_name, contribution_aggregate \
                 from libfec_schedule_a order by filing_id",
            )
            .unwrap();
        let got: Vec<(String, String, String, Option<f64>)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3).ok())))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            got,
            vec![
                ("1314260".into(), "SA3L".into(), "ANGRAND".into(), None),
                (
                    "1807079".into(),
                    "SA3L".into(),
                    "MILLER".into(),
                    Some(84000.0)
                ),
                ("850".into(), "SA11AI".into(), "Modern".into(), Some(123.0)),
            ],
            "{name}"
        );
    }
}

/// `SC/10` and a literal `SC-10` row type share `SC-10.csv`: one writer
/// writes both, in file order, under a single header line.
#[test]
fn fastfec_row_types_sharing_a_file_name() {
    let s = Scratch::new();
    let input = s.write(
        "77.fec",
        "HDR,FEC,5.00,Test,1,,,,\nF3XN,C00000001,,Test\nSC/10,C00000001,FIRST\nSC-10,C00000001,SECOND\nSC/10,C00000001,THIRD\n",
    );
    let out = s.path("out");
    let o = s.libfec(&[&"fastfec", &input, &out]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let rows = read_csv(&out.join("77").join("SC-10.csv"));
    assert_eq!(rows[0][0], "form_type");
    let data: Vec<(&str, &str)> = rows[1..]
        .iter()
        .map(|r| (r[0].as_str(), r[2].as_str()))
        .collect();
    assert_eq!(
        data,
        vec![("SC/10", "FIRST"), ("SC-10", "SECOND"), ("SC/10", "THIRD")]
    );
}

/// Without a terminal (the progress bars are hidden), a filing that can't
/// be read is still reported on stderr, and an export in which every
/// filing failed exits non-zero.
#[test]
fn unreadable_filings_are_reported_without_a_terminal() {
    let s = Scratch::new();
    let bad = s.write("999.fec", "not a filing\n");
    let good = s.write("500.fec", LEGACY_5_00);
    let outputs: [(&str, &[&str]); 4] = [
        ("o.json", &["--target", "schedule-a", "-o"]),
        ("o.csv", &["--target", "schedule-a", "-o"]),
        ("o.db", &["-o"]),
        ("dir", &["-f", "csv", "--output-directory"]),
    ];
    for (name, flags) in outputs {
        let run = |inputs: &[&PathBuf], out: &str| {
            let out = s.path(out);
            let mut args: Vec<&dyn AsRef<std::ffi::OsStr>> = vec![&"export"];
            args.extend(inputs.iter().map(|p| *p as &dyn AsRef<std::ffi::OsStr>));
            args.extend(flags.iter().map(|f| f as &dyn AsRef<std::ffi::OsStr>));
            args.push(&out);
            s.libfec(&args)
        };
        let o = run(&[&bad], &format!("bad-{name}"));
        let stderr = String::from_utf8_lossy(&o.stderr);
        assert!(!o.status.success(), "{name}: {stderr}");
        assert!(stderr.contains("999.fec"), "{name}: {stderr}");
        assert!(stderr.contains("could be read"), "{name}: {stderr}");

        let o = run(&[&bad, &good], &format!("mixed-{name}"));
        let stderr = String::from_utf8_lossy(&o.stderr);
        assert!(o.status.success(), "{name}: {stderr}");
        assert!(stderr.contains("999.fec"), "{name}: {stderr}");
    }
}

/// A lowercase row type (`sa11ai`) is a Schedule A row in a targeted export.
#[test]
fn lowercase_schedule_row_types_are_exported() {
    let s = Scratch::new();
    let input = s.write("501.fec", &LEGACY_5_00.replace("SA11AI", "sa11ai"));
    let out = s.path("a.csv");
    let o = s.libfec(&[&"export", &input, &"--target", &"schedule-a", &"-o", &out]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        donors(&read_csv(&out)),
        vec![("501".to_owned(), "Legacy".to_owned(), "Alice".to_owned())]
    );
}

/// An input that resolves to nothing is an error, not a `todo!()` panic.
#[test]
fn unresolvable_input_is_an_error() {
    let s = Scratch::new();
    for input in ["no/such/file.fec", "H-C", "S-CA-X"] {
        let o = s.libfec(&[&"export", &input, &"-o", &s.path("x.db")]);
        let stderr = String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(1), "{input}: {stderr}");
        assert!(
            stderr.contains("Could not resolve input"),
            "{input}: {stderr}"
        );
        assert!(!stderr.contains("panicked"), "{input}: {stderr}");
    }
}
