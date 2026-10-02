//! Dump the raw records fec-parser reads from one filing as JSONL.
//!
//! ```sh
//! cargo run -p fec-parser --release --example dump_records -- path/to/1234.fec
//! ```
//!
//! Lines, in order:
//! - `{"kind":"header","fields":[..],"parsed":{..}}` — `header_record` plus the
//!   decoded [`FilingHeader`] fields (see [`header_parsed`]).
//! - `{"kind":"cover","form_type":..,"columns":[..],"fields":[..],"parsed":{..},"typed":..}`
//!   — `typed` is the serialized typed cover or `null`.
//! - one `{"kind":"row","row_type":..,"line":..,"byte":..,"record":..,"size":..,"fields":[..]}`
//!   per `next_row()` (`line`/`byte`/`record` from `record.position()`,
//!   `null` when unset; `size` is `original_size`), or
//!   `{"kind":"error","msg":..}` for a row error.
//!
//! If the filing cannot be opened at all, the only line is
//! `{"kind":"error","stage":"open","msg":..}`. Exit status is 0 either way,
//! so the output (including errors) can be hashed and compared across
//! revisions — see `wiki/legacy/tools/regress_8x.sh` and `diff_fastfec.py`.
//!
//! The header line also carries `legacy_fields` and `schedule_counts` (the
//! `/*` block, both empty for HDR-style filings), which the FastFEC
//! differential compares to FastFEC's `header.csv`. Those fields do not exist
//! at the pre-legacy base (b1abe2e), so `regress_8x.sh` overlays a
//! base-compatible copy (`wiki/legacy/tools/dump_base.rs`) on both revisions
//! instead of this file.

use csv::StringRecord;
use fec_parser::{Filing, FilingHeader};
use serde_json::{json, Map, Value};
use std::io::Write;
use std::path::PathBuf;

fn fields(record: &StringRecord) -> Vec<&str> {
    record.iter().collect()
}

/// Decoded header fields. Add new `FilingHeader` fields here.
fn header_parsed(h: &FilingHeader) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("record_type".into(), json!(h.record_type));
    m.insert("ef_type".into(), json!(h.ef_type));
    m.insert("fec_version".into(), json!(h.fec_version));
    m.insert("software_name".into(), json!(h.software_name));
    m.insert("software_version".into(), json!(h.software_version));
    m.insert("report_id".into(), json!(h.report_id));
    m.insert("report_number".into(), json!(h.report_number));
    m.insert("comment".into(), json!(h.comment));
    m
}

fn main() {
    let path = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| panic!("usage: dump_records FILE.fec")),
    );
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let mut emit = |v: Value| writeln!(out, "{v}").expect("write stdout");

    let mut filing = match Filing::<std::fs::File>::from_path(&path) {
        Ok(f) => f,
        Err(e) => {
            emit(json!({"kind": "error", "stage": "open", "msg": format!("{e:#}")}));
            return;
        }
    };

    emit(json!({
        "kind": "header",
        "fields": fields(&filing.header.header_record),
        "parsed": header_parsed(&filing.header),
        "legacy_fields": filing.header.legacy_fields,
        "schedule_counts": filing.header.schedule_counts,
    }));

    let cover = &filing.cover;
    emit(json!({
        "kind": "cover",
        "form_type": cover.form_type,
        "columns": cover.record_column_names,
        "fields": fields(&cover.record),
        "parsed": {
            "filer_id": cover.filer_id,
            "filer_name": cover.filer_name,
            "report_code": cover.report_code,
            "coverage_from_date": cover.coverage_from_date.map(|d| d.to_string()),
            "coverage_through_date": cover.coverage_through_date.map(|d| d.to_string()),
        },
        "typed": cover.cover_data.as_ref().map(|c| serde_json::to_value(c).expect("serialize cover")),
    }));

    while let Some(row) = filing.next_row() {
        match row {
            Ok(row) => {
                let pos = row.record.position();
                emit(json!({
                    "kind": "row",
                    "row_type": row.row_type,
                    "line": pos.map(|p| p.line()),
                    "byte": pos.map(|p| p.byte()),
                    "record": pos.map(|p| p.record()),
                    "size": row.original_size,
                    "fields": fields(&row.record),
                }));
            }
            Err(e) => emit(json!({"kind": "error", "msg": e.to_string()})),
        }
    }
}
