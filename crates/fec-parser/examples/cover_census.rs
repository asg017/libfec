//! Census of typed-cover coverage over a directory of `.fec` files.
//!
//! ```sh
//! cargo run -p fec-parser --release --example cover_census -- ~/.cache/libfec/cache [F3X]
//! ```
//!
//! For each base form type (optionally only those starting with the second
//! argument) prints how many cover records were seen, how many produced typed
//! `cover_data`, and up to three filing ids that did not, plus a breakdown by
//! FEC version. Use it to check a cover struct against every format version in
//! the corpus, not just the fixture.

use fec_parser::{covers::base_form_type, Filing};
use std::collections::BTreeMap;

#[derive(Default)]
struct Tally {
    seen: usize,
    typed: usize,
    errors: usize,
    untyped_examples: Vec<String>,
    by_version: BTreeMap<String, (usize, usize)>,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: cover_census DIR [FORM_PREFIX]");
    let only = args.next().map(|s| s.to_ascii_uppercase());

    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).expect("read dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("fec") {
            continue;
        }
        let filing_id = path.file_stem().unwrap().to_string_lossy().into_owned();
        let filing = match Filing::<std::fs::File>::from_path(&path) {
            Ok(f) => f,
            Err(_) => {
                tallies.entry("<unparsable>".into()).or_default().errors += 1;
                continue;
            }
        };
        let base = base_form_type(&filing.cover.form_type);
        if let Some(prefix) = &only {
            if !base.starts_with(prefix.as_str()) {
                continue;
            }
        }
        let tally = tallies.entry(base).or_default();
        tally.seen += 1;
        let v = tally
            .by_version
            .entry(filing.header.fec_version.clone())
            .or_default();
        v.0 += 1;
        if filing.cover.cover_data.is_some() {
            tally.typed += 1;
            v.1 += 1;
        } else if tally.untyped_examples.len() < 3 {
            tally.untyped_examples.push(filing_id);
        }
    }

    for (form, t) in &tallies {
        if form == "<unparsable>" {
            println!("{form}: {}", t.errors);
            continue;
        }
        println!(
            "{form:6} seen={:6} typed={:6} untyped-examples={:?}",
            t.seen, t.typed, t.untyped_examples
        );
        for (version, (seen, typed)) in &t.by_version {
            println!("         v{version:8} seen={seen:6} typed={typed:6}");
        }
    }
}
