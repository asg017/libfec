//! Census of typed-itemization coverage over a directory of `.fec` files.
//!
//! ```sh
//! cargo run -p fec-parser --release --example itemization_census -- ~/.cache/libfec/cache
//! ```
//!
//! For every row after the cover, groups by row family (`SA`, `SB`, `SC1`,
//! `H4`, `TEXT`, `F3PS`, …) and prints how many rows were seen, how many
//! [`Itemization::from_record`] typed, which FEC versions left rows untyped,
//! and lossy reads: typed rows whose non-blank date or amount column does not
//! parse, so the typed field is `None` / `0.0`.
//! Also reports the time spent typing versus reading.

use fec_parser::covers::fields::Fields;
use fec_parser::itemizations::{Itemization, Layout, RecordFields};
use fec_parser::Filing;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Default)]
struct Tally {
    rows: usize,
    typed: usize,
    untyped_versions: BTreeMap<String, usize>,
    untyped_example: Option<String>,
    bad_date: usize,
    bad_amount: usize,
    bad_example: Option<String>,
}

#[derive(Default)]
struct Totals {
    families: BTreeMap<String, Tally>,
    filings: usize,
    unparsable: usize,
    read_time: Duration,
    type_time: Duration,
}

/// `SA11AI` → `SA`, `SC1/10` → `SC1`, `H4` → `H4`, `F3PS` → `F3PS`.
fn family(row_type: &str) -> String {
    let t = row_type.trim().to_ascii_uppercase();
    for p in ["SC1", "SC2", "SA3L", "SA", "SB", "SC", "SD", "SE", "SF", "SI", "SL"] {
        if t.starts_with(p) {
            return p.to_owned();
        }
    }
    t
}

/// The date and amount columns a typed row must not lose, per family.
fn checked_columns(family: &str) -> (&'static [&'static str], &'static [&'static str]) {
    match family {
        "SA" => (&["contribution_date"], &["contribution_amount", "contribution_aggregate"]),
        "SB" => (&["expenditure_date"], &["expenditure_amount"]),
        // The due dates (SC `loan_due_date_terms`, SC1 `loan_due_date`) are
        // free text, kept as written, so they are not checked.
        "SC" => (&["loan_incurred_date_terms"], &["loan_amount_original", "loan_payment_to_date", "loan_balance"]),
        "SC1" => (
            &["loan_incurred_date", "loan_incurred_date_original", "established_date", "deposit_acct_auth_date_presidential", "date_signed", "authorized_date"],
            &["loan_amount", "credit_amount_this_draw", "total_balance", "collateral_value_amount", "estimated_value"],
        ),
        "SC2" => (&[], &["guaranteed_amount"]),
        "SD" => (
            &[],
            &["beginning_balance_this_period", "incurred_amount_this_period", "payment_amount_this_period", "balance_at_close_this_period"],
        ),
        "SE" => (&["dissemination_date", "disbursement_date"], &["expenditure_amount", "calendar_y_t_d_per_election_office"]),
        "SF" => (&["expenditure_date"], &["expenditure_amount", "aggregate_general_elec_expended"]),
        "H1" => (&[], &["federal_percent", "nonfederal_percent"]),
        "H2" => (&[], &["federal_percentage", "nonfederal_percentage"]),
        "H3" => (&["receipt_date"], &["total_amount_transferred", "transferred_amount"]),
        "H4" => (&["expenditure_date"], &["total_amount", "federal_share", "nonfederal_share", "event_year_to_date"]),
        "H5" => (
            &["receipt_date"],
            &["total_amount_transferred", "voter_registration_amount", "voter_id_amount", "gotv_amount", "generic_campaign_amount"],
        ),
        "H6" => (&["expenditure_date"], &["total_amount", "federal_share", "levin_share", "event_year_to_date"]),
        "F56" | "F65" => (&["contribution_date"], &["contribution_amount"]),
        "F57" => (&["dissemination_date"], &["expenditure_amount", "calendar_y_t_d_per_election_office"]),
        "F76" => (&["communication_date"], &["communication_cost"]),
        "F92" => (&["contribution_date"], &["contribution_amount"]),
        "F93" => (&["expenditure_date", "communication_date"], &["expenditure_amount"]),
        "F132" => (&["donation_date"], &["donation_amount", "donation_aggregate_amount"]),
        "F133" => (&["refund_date"], &["refund_amount"]),
        "SL" => (
            &["coverage_from_date", "coverage_through_date"],
            &["col_a_total_receipts", "col_b_total_receipts", "col_a_total_disbursements", "col_b_total_disbursements", "col_b_cash_on_hand_close_of_period", "col_b_cash_on_hand_close_of_period_TODO_DUP"],
        ),
        "SA3L" => (
            &["contribution_date"],
            &["bundled_amount_period", "bundled_amount_semi_annual", "contribution_amount", "contribution_aggregate"],
        ),
        _ => (&[], &[]),
    }
}

fn census(paths: &[PathBuf]) -> Totals {
    let mut t = Totals::default();
    for path in paths {
        let filing_id = path.file_stem().unwrap().to_string_lossy().into_owned();
        let start = Instant::now();
        let Ok(mut filing) = Filing::<std::fs::File>::from_path(path) else {
            t.unparsable += 1;
            continue;
        };
        t.filings += 1;
        let version = filing.header.fec_version.clone();
        let delimiter = filing.header.name_delimiter.clone();
        let mut typing = Duration::ZERO;
        while let Some(row) = filing.next_row() {
            let Ok(row) = row else { continue };
            let fam = family(&row.row_type);
            let tally = t.families.entry(fam.clone()).or_default();
            tally.rows += 1;
            let s = Instant::now();
            let typed = Itemization::from_record(&row.record, &version, delimiter.as_deref());
            typing += s.elapsed();
            if typed.is_none() {
                *tally.untyped_versions.entry(version.clone()).or_default() += 1;
                tally.untyped_example.get_or_insert_with(|| format!("{filing_id}:{}", row.line));
                continue;
            }
            tally.typed += 1;
            // Lossy reads: re-read the raw column and compare with the JSON.
            let (dates, amounts) = checked_columns(&fam);
            let Some(layout) = Layout::get(&row.row_type, &version) else { continue };
            let raw = RecordFields { layout: &layout, record: &row.record, name_delimiter: "^" };
            for col in dates {
                if let Some(v) = raw.raw(col).map(str::trim).filter(|v| !v.is_empty()) {
                    let ok = jiff::civil::Date::strptime("%Y%m%d", v).is_ok()
                        || jiff::civil::Date::strptime("%m/%d/%Y", v).is_ok();
                    if !ok {
                        tally.bad_date += 1;
                        tally.bad_example.get_or_insert_with(|| format!("{filing_id}:{} {col}={v:?}", row.line));
                    }
                }
            }
            for col in amounts {
                if let Some(v) = raw.raw(col).map(str::trim).filter(|v| !v.is_empty()) {
                    if v.parse::<f64>().is_err() {
                        tally.bad_amount += 1;
                        tally.bad_example.get_or_insert_with(|| format!("{filing_id}:{} {col}={v:?}", row.line));
                    }
                }
            }
        }
        t.type_time += typing;
        t.read_time += start.elapsed() - typing;
    }
    t
}

fn main() {
    let dir = std::env::args().nth(1).expect("usage: itemization_census DIR");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("read dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("fec"))
        .collect();
    paths.sort();

    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = paths.len().div_ceil(threads).max(1);
    let parts: Vec<Totals> = std::thread::scope(|s| {
        let handles: Vec<_> = paths.chunks(chunk).map(|c| s.spawn(|| census(c))).collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    let mut total = Totals::default();
    for p in parts {
        total.filings += p.filings;
        total.unparsable += p.unparsable;
        total.read_time += p.read_time;
        total.type_time += p.type_time;
        for (fam, t) in p.families {
            let into = total.families.entry(fam).or_default();
            into.rows += t.rows;
            into.typed += t.typed;
            into.bad_date += t.bad_date;
            into.bad_amount += t.bad_amount;
            for (v, n) in t.untyped_versions {
                *into.untyped_versions.entry(v).or_default() += n;
            }
            if into.untyped_example.is_none() {
                into.untyped_example = t.untyped_example;
            }
            if into.bad_example.is_none() {
                into.bad_example = t.bad_example;
            }
        }
    }

    println!(
        "{} filings ({} unparsable); CPU time reading {:.1}s, typing {:.1}s",
        total.filings,
        total.unparsable,
        total.read_time.as_secs_f64(),
        total.type_time.as_secs_f64()
    );
    println!("{:<8} {:>10} {:>10} {:>7} {:>8} {:>8}  untyped versions / examples", "family", "rows", "typed", "%", "bad_dt", "bad_amt");
    let mut fams: Vec<_> = total.families.into_iter().collect();
    fams.sort_by_key(|(_, t)| std::cmp::Reverse(t.rows));
    for (fam, t) in fams {
        let pct = 100.0 * t.typed as f64 / t.rows.max(1) as f64;
        let versions: Vec<String> = t.untyped_versions.iter().map(|(v, n)| format!("{v}:{n}")).collect();
        println!(
            "{:<8} {:>10} {:>10} {:>6.1}% {:>8} {:>8}  {} {} {}",
            fam,
            t.rows,
            t.typed,
            pct,
            t.bad_date,
            t.bad_amount,
            if t.typed < t.rows { versions.join(",") } else { String::new() },
            t.untyped_example.unwrap_or_default(),
            t.bad_example.unwrap_or_default(),
        );
    }
}
