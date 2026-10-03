//! Cut a small test fixture out of a real filing: its header and cover
//! (byte for byte) plus up to N rows of one record family.
//!
//! ```sh
//! cargo run -p fec-parser --example itemization_fixture -- FILE FAMILY N OUT
//! cargo run -p fec-parser --example itemization_fixture -- ~/.cache/libfec/cache/1805248.fec SA 5 tests/fixtures/itemizations/SA_1805248.fec
//! ```
//!
//! Rows are taken from the file's bytes via `FilingRow::byte_offset`, so the
//! fixture keeps the original delimiters, quoting and line endings. Picks the
//! first N rows of the family, but prefers distinct row types.

use fec_parser::itemizations::record_family;
use fec_parser::Filing;
use std::collections::HashSet;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [file, family, n, out] = args.as_slice() else {
        panic!("usage: itemization_fixture FILE FAMILY N OUT");
    };
    let n: usize = n.parse().expect("N");
    let bytes = std::fs::read(file).expect("read FILE");
    let mut filing = Filing::<std::fs::File>::from_path(file.as_ref()).expect("parse FILE");

    // (start, end) of every row, in file order.
    let mut spans: Vec<(u64, String)> = Vec::new();
    while let Some(row) = filing.next_row() {
        let row = row.expect("row");
        spans.push((row.byte_offset, row.row_type.clone()));
    }
    let header_end = spans.first().map_or(bytes.len() as u64, |s| s.0) as usize;
    let mut ends: Vec<u64> = spans.iter().skip(1).map(|s| s.0).collect();
    ends.push(bytes.len() as u64);

    let candidates: Vec<usize> = (0..spans.len())
        .filter(|&i| record_family(&spans[i].1).is_some_and(|f| f.eq_ignore_ascii_case(family)))
        .collect();
    // Distinct row types first, then fill up in file order.
    let mut seen = HashSet::new();
    let mut picked: Vec<usize> = candidates
        .iter()
        .copied()
        .filter(|&i| seen.insert(spans[i].1.to_ascii_uppercase()))
        .take(n)
        .collect();
    for &i in &candidates {
        if picked.len() >= n {
            break;
        }
        if !picked.contains(&i) {
            picked.push(i);
        }
    }
    picked.sort();

    let mut fixture = bytes[..header_end].to_vec();
    for &i in &picked {
        fixture.extend_from_slice(&bytes[spans[i].0 as usize..ends[i] as usize]);
    }
    std::fs::write(out, fixture).expect("write OUT");
    eprintln!("{out}: {} {family} rows", picked.len());
}
