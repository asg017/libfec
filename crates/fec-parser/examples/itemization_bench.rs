//! Time reading a filing's rows raw versus typing them.
//!
//! ```sh
//! cargo run -p fec-parser --release --example itemization_bench -- benchmarks/1805248.fec
//! ```

use fec_parser::itemizations::Itemization;
use fec_parser::Filing;
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("usage: itemization_bench FILE");
    for typed in [false, true, false, true] {
        let start = Instant::now();
        let mut filing = Filing::<std::fs::File>::from_path(path.as_ref()).unwrap();
        let version = filing.header.fec_version.clone();
        let delimiter = filing.header.name_delimiter.clone();
        let (mut rows, mut items) = (0usize, 0usize);
        while let Some(row) = filing.next_row() {
            let row = row.unwrap();
            rows += 1;
            if typed && Itemization::from_record(&row.record, &version, delimiter.as_deref()).is_some() {
                items += 1;
            }
        }
        let secs = start.elapsed().as_secs_f64();
        println!(
            "{}: {rows} rows, {items} typed, {secs:.2}s ({:.0} ns/row)",
            if typed { "typed" } else { "raw  " },
            secs * 1e9 / rows as f64
        );
    }
}
