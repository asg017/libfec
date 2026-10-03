//! Census of how far fec-parser gets on each filing of a (legacy) corpus.
//!
//! ```sh
//! # every .fec under a directory (recursive)
//! cargo run -p fec-parser --release --example legacy_census -- wiki/legacy/samples > census.tsv
//! # a TSV whose first column is a path (relative paths resolve against the
//! # TSV's directory; a first row starting with `path` is skipped)
//! cargo run -p fec-parser --release --example legacy_census -- wiki/legacy/samples/index.tsv
//! # or explicit files
//! cargo run -p fec-parser --release --example legacy_census -- a.fec b.fec
//! ```
//!
//! Prints one TSV row per file to stdout (columns: see [`COLUMNS`] and
//! [`header_columns`]) and a per-version-family summary to stderr: files,
//! header-ok, cover-ok, typed-cover, rows-all-mapped.
//!
//! The version family comes from a tiny independent sniff of the file's first
//! line (see [`sniff_version`]), so files the parser rejects are still
//! grouped by family.
//!
//! To add a column: header-derived ones go in [`header_columns`] (it must
//! return the same names for `None`), everything else in [`Census::cells`].

use fec_parser::{mappings::column_names_for_field, Filing, FilingHeader, FilingHeaderError};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Columns before the header-derived ones.
const LEAD_COLUMNS: &[&str] = &[
    "path",
    "sniff_version",
    "family",
    "status",
    "stage",
    "error",
];
/// Columns after the header-derived ones.
const COLUMNS: &[&str] = &[
    "cover_form_type",
    "typed_cover",
    "n_rows",
    "n_row_errors",
    "unmapped_row_types",
];

/// Header-derived columns. Called with `None` for files whose header did not
/// parse (and to print the TSV header line), so names must not depend on
/// `header`.
fn header_columns(header: Option<&FilingHeader>) -> Vec<(&'static str, String)> {
    let get = |f: fn(&FilingHeader) -> String| header.map(f).unwrap_or_default();
    vec![
        ("fec_version", get(|h| h.fec_version.clone())),
        ("ef_type", get(|h| h.ef_type.clone())),
        ("soft_name", get(|h| h.software_name.clone())),
        // ticket 02: ("style", get(|h| format!("{:?}", h.style))),
        // ticket 02: ("delimiter", get(|h| format!("{:?}", h.delimiter))),
    ]
}

/// How far parsing got before failing.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Stage {
    #[default]
    /// Header and cover parsed; rows were read.
    Ok,
    /// Header parsed, cover did not.
    Cover,
    /// Header (or the first records) did not parse.
    Header,
}

impl Stage {
    fn as_str(self) -> &'static str {
        match self {
            Stage::Ok => "ok",
            Stage::Cover => "cover",
            Stage::Header => "header",
        }
    }
}

#[derive(Default)]
struct Census {
    display_path: String,
    sniff_version: String,
    error: String,
    stage: Stage,
    header: Vec<(&'static str, String)>,
    cover_form_type: String,
    typed_cover: Option<bool>,
    n_rows: usize,
    n_row_errors: usize,
    unmapped: BTreeSet<String>,
}

impl Census {
    fn cells(&self) -> Vec<String> {
        let stage = self.stage;
        let mut cells = vec![
            self.display_path.clone(),
            self.sniff_version.clone(),
            family(&self.sniff_version).to_owned(),
            if stage == Stage::Ok { "ok" } else { "err" }.to_owned(),
            stage.as_str().to_owned(),
            self.error.clone(),
        ];
        cells.extend(self.header.iter().map(|(_, v)| v.clone()));
        cells.extend([
            self.cover_form_type.clone(),
            match self.typed_cover {
                Some(true) => "some",
                Some(false) => "none",
                None => "",
            }
            .to_owned(),
            self.n_rows.to_string(),
            self.n_row_errors.to_string(),
            self.unmapped.iter().cloned().collect::<Vec<_>>().join(";"),
        ]);
        cells
    }
}

fn tsv_cell(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

/// First line of an error (anyhow chains can be multi-line).
fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").to_owned()
}

fn census(path: &Path, display_path: String) -> Census {
    let mut c = Census {
        display_path,
        sniff_version: sniff_version(path).unwrap_or_default(),
        header: header_columns(None),
        ..Default::default()
    };
    let mut filing = match Filing::<std::fs::File>::from_path(path) {
        Ok(f) => f,
        Err(e) => {
            c.error = first_line(&e.to_string());
            // `Filing::from_path` has one error type for everything; the
            // cover errors are the ones that say so.
            c.stage = if e.downcast_ref::<FilingHeaderError>().is_none()
                && c.error.to_ascii_lowercase().contains("cover")
            {
                Stage::Cover
            } else {
                Stage::Header
            };
            return c;
        }
    };
    c.header = header_columns(Some(&filing.header));
    c.cover_form_type = filing.cover.form_type.clone();
    c.typed_cover = Some(filing.cover.cover_data.is_some());
    let version = filing.header.fec_version.clone();
    let mut seen = BTreeSet::new();
    while let Some(row) = filing.next_row() {
        match row {
            Ok(row) => {
                c.n_rows += 1;
                let row_type = row.row_type.trim().to_owned();
                if seen.insert(row_type.clone()) {
                    let mapped = column_names_for_field(&row_type, &version)
                        .map(|cols| !cols.is_empty())
                        .unwrap_or(false);
                    if !mapped {
                        c.unmapped.insert(row_type);
                    }
                }
            }
            Err(_) => c.n_row_errors += 1,
        }
    }
    c
}

/// The version string from a file's first line, without using fec-parser:
/// `/*` header → the `FEC_Ver_#` value (any case); `HDR␜FEC␜8.4` /
/// `HDR,FEC,5.30` → field 2; paper `HDR␜P3.2` → field 1.
fn sniff_version(path: &Path) -> Option<String> {
    let mut buf = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(64 * 1024)
        .read_to_end(&mut buf)
        .ok()?;
    let text = String::from_utf8_lossy(&buf);
    let text = text.trim_start_matches('\u{feff}').trim_start();
    if text.starts_with("/*") {
        return text.lines().find_map(|line| {
            let (k, v) = line.split_once('=')?;
            k.trim()
                .eq_ignore_ascii_case("FEC_Ver_#")
                .then(|| v.trim().to_owned())
        });
    }
    let line = text.lines().next()?;
    let delim = if line.contains('\x1c') { '\x1c' } else { ',' };
    let fields: Vec<String> = line
        .split(delim)
        .map(|f| f.trim().trim_matches('"').trim().to_owned())
        .collect();
    match fields.get(1).map(String::as_str) {
        Some(f) if f.eq_ignore_ascii_case("FEC") => fields.get(2).cloned(),
        Some(f) if f.starts_with(['P', 'p']) => Some(f.to_owned()),
        _ => None,
    }
}

/// Version family per PLAN.md R3.
fn family(version: &str) -> &'static str {
    let v = version.trim();
    let p = |prefix: &str| v.starts_with(prefix);
    match () {
        _ if p("P1.") => "P1.x",
        _ if p("P2.") => "P2.x",
        _ if p("P3.") => "P3.x",
        _ if p("1.") => "1.x",
        _ if p("2.") => "2.x",
        _ if p("3.") => "3.x",
        _ if p("5.") => "5.x",
        _ if p("6.") => "6.x",
        _ if p("7.") => "7.x",
        _ if p("8.") => "8.x",
        _ if v.is_empty() => "unknown",
        _ => "other",
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        eprintln!("cannot read dir {}", dir.display());
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("fec") {
            out.push(path);
        }
    }
}

/// Expand args into (path to open, path to print).
fn inputs(args: &[String]) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    for arg in args {
        let path = PathBuf::from(arg);
        if path.is_dir() {
            let mut files = Vec::new();
            walk(&path, &mut files);
            files.sort();
            out.extend(files.into_iter().map(|p| {
                let shown = p.display().to_string();
                (p, shown)
            }));
        } else if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("tsv" | "txt")
        ) {
            let base = path.parent().unwrap_or(Path::new(".")).to_path_buf();
            let text = std::fs::read_to_string(&path).expect("read path list");
            for (i, line) in text.lines().enumerate() {
                let first = line.split('\t').next().unwrap_or("").trim();
                if first.is_empty() || (i == 0 && first == "path") {
                    continue;
                }
                let p = Path::new(first);
                let full = if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    base.join(p)
                };
                out.push((full, first.to_owned()));
            }
        } else {
            out.push((path, arg.clone()));
        }
    }
    out
}

#[derive(Default)]
struct FamilyTally {
    files: usize,
    header_ok: usize,
    cover_ok: usize,
    typed_cover: usize,
    rows_all_mapped: usize,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: legacy_census (DIR | PATHS.tsv | FILE.fec)...");
        std::process::exit(2);
    }

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let names: Vec<&str> = LEAD_COLUMNS
        .iter()
        .copied()
        .chain(header_columns(None).iter().map(|(n, _)| *n))
        .chain(COLUMNS.iter().copied())
        .collect();
    writeln!(out, "{}", names.join("\t")).expect("write");

    let mut tallies: BTreeMap<&'static str, FamilyTally> = BTreeMap::new();
    for (path, shown) in inputs(&args) {
        let c = census(&path, shown);
        let cells: Vec<String> = c.cells().iter().map(|s| tsv_cell(s)).collect();
        writeln!(out, "{}", cells.join("\t")).expect("write");

        let t = tallies.entry(family(&c.sniff_version)).or_default();
        t.files += 1;
        let stage = c.stage;
        if stage != Stage::Header {
            t.header_ok += 1;
        }
        if stage == Stage::Ok {
            t.cover_ok += 1;
            if c.typed_cover == Some(true) {
                t.typed_cover += 1;
            }
            if c.unmapped.is_empty() && c.n_row_errors == 0 {
                t.rows_all_mapped += 1;
            }
        }
    }
    out.flush().expect("flush");

    eprintln!(
        "{:8} {:>6} {:>9} {:>8} {:>11} {:>15}",
        "family", "files", "header-ok", "cover-ok", "typed-cover", "rows-all-mapped"
    );
    let mut total = FamilyTally::default();
    for (fam, t) in &tallies {
        eprintln!(
            "{fam:8} {:>6} {:>9} {:>8} {:>11} {:>15}",
            t.files, t.header_ok, t.cover_ok, t.typed_cover, t.rows_all_mapped
        );
        total.files += t.files;
        total.header_ok += t.header_ok;
        total.cover_ok += t.cover_ok;
        total.typed_cover += t.typed_cover;
        total.rows_all_mapped += t.rows_all_mapped;
    }
    eprintln!(
        "{:8} {:>6} {:>9} {:>8} {:>11} {:>15}",
        "total",
        total.files,
        total.header_ok,
        total.cover_ok,
        total.typed_cover,
        total.rows_all_mapped
    );
}
