//! `make gen-types`, step 1 of 2: export fec-parser's typed covers and
//! itemizations with ts-rs, and tabulate the public label functions, into a
//! scratch directory. `scripts/gen-types.mjs` (step 2) post-processes that
//! into the committed `js/generated/`.
//!
//! Usage: `cargo run -p fec-node --features gen-types --bin gen-types -- <out-dir>`

use std::collections::BTreeMap;
use std::path::PathBuf;

use fec_parser::covers::{self, Cover};
use fec_parser::itemizations::{self, Itemization};
use ts_rs::{Config, TS};

/// How a label function reads its code, so JS can reproduce it from a table.
#[derive(Clone, Copy)]
enum Shape {
    /// The trimmed, upper-cased code is looked up as is.
    Exact,
    /// Only the first character of the trimmed, upper-cased code counts.
    FirstChar,
    /// The trimmed code must be all digits; its numeric value is looked up.
    Number,
}

impl Shape {
    fn as_str(self) -> &'static str {
        match self {
            Shape::Exact => "exact",
            Shape::FirstChar => "first_char",
            Shape::Number => "number",
        }
    }
}

type LabelFn = fn(&str) -> Option<&'static str>;

/// Every public free-standing label function: `(rust path, function, shape)`.
/// The per-struct `*_label` methods are Rust-only (JS records are plain data).
const LABELS: &[(&str, LabelFn, Shape)] = &[
    (
        "covers::election_code_label",
        covers::election_code_label,
        Shape::FirstChar,
    ),
    ("covers::office_label", covers::office_label, Shape::Exact),
    ("covers::party_label", covers::party_label, Shape::Exact),
    (
        "itemizations::entity_type_label",
        itemizations::entity_type_label,
        Shape::Exact,
    ),
    (
        "itemizations::support_oppose_label",
        itemizations::support_oppose_label,
        Shape::Exact,
    ),
    (
        "itemizations::category_code_label",
        itemizations::category_code_label,
        Shape::Number,
    ),
];

/// Every code of 1–3 characters from `A–Z0–9`: the domain the label tables
/// are probed over (FEC codes are short and alphanumeric).
fn candidate_codes() -> Vec<String> {
    let alphabet: Vec<char> = ('A'..='Z').chain('0'..='9').collect();
    let mut out: Vec<String> = alphabet.iter().map(|c| c.to_string()).collect();
    for len in 2..=3 {
        let prev: Vec<String> = out.iter().filter(|s| s.len() == len - 1).cloned().collect();
        for p in &prev {
            for c in &alphabet {
                out.push(format!("{p}{c}"));
            }
        }
    }
    out
}

fn label_table(f: LabelFn, shape: Shape, codes: &[String]) -> BTreeMap<String, &'static str> {
    let mut table = BTreeMap::new();
    match shape {
        Shape::Exact => {
            for code in codes {
                if let Some(label) = f(code) {
                    // The JS side trims and upper-cases; check Rust agrees.
                    assert_eq!(f(&format!(" {} ", code.to_ascii_lowercase())), Some(label));
                    table.insert(code.clone(), label);
                }
            }
        }
        Shape::FirstChar => {
            for code in codes.iter().filter(|c| c.len() == 1) {
                if let Some(label) = f(code) {
                    assert_eq!(
                        f(&format!("{code}2026")),
                        Some(label),
                        "{code}: first char only"
                    );
                    table.insert(code.clone(), label);
                }
            }
        }
        Shape::Number => {
            for n in 0..1000u16 {
                if let Some(label) = f(&n.to_string()) {
                    assert_eq!(f(&format!("{n:03}")), Some(label), "{n}: leading zeros");
                    table.insert(n.to_string(), label);
                }
            }
        }
    }
    assert!(!table.is_empty(), "label table came out empty");
    table
}

fn main() {
    let out: PathBuf = std::env::args_os()
        .nth(1)
        .expect("usage: gen-types <out-dir>")
        .into();
    std::fs::create_dir_all(&out).expect("create out dir");

    let cfg = Config::new()
        .with_out_dir(&out)
        .with_import_extension(Some("js"));
    Itemization::export_all(&cfg).expect("export Itemization");
    Cover::export_all(&cfg).expect("export Cover");

    let codes = candidate_codes();
    let labels: Vec<serde_json::Value> = LABELS
        .iter()
        .map(|&(path, f, shape)| {
            serde_json::json!({
                "rust": path,
                "shape": shape.as_str(),
                "table": label_table(f, shape, &codes),
            })
        })
        .collect();
    std::fs::write(
        out.join("labels.json"),
        serde_json::to_string_pretty(&labels).unwrap(),
    )
    .expect("write labels.json");
}
