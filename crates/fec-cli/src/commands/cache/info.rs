use colored::Colorize;
use indicatif::HumanBytes;
use jiff::Timestamp;
use num_format::{Locale, ToFormattedString};
use rusqlite::Connection;
use std::path::Path;
use std::str::FromStr;

use crate::sourcer::FilingSourcer;

fn get_file_size(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.len())
}

fn get_fec_files_stats(cache_dir: &Path) -> (usize, u64) {
    let mut count = 0;
    let mut total_bytes = 0;

    if let Ok(entries) = std::fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "fec") {
                count += 1;
                if let Ok(metadata) = std::fs::metadata(&path) {
                    total_bytes += metadata.len();
                }
            }
        }
    }

    (count, total_bytes)
}

fn get_daily_zip_meta_count(cache_dir: &Path) -> usize {
    let mut count = 0;

    if let Ok(entries) = std::fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Match files like .daily-zip.YYYY-MM-DD.meta
            if name_str.starts_with(".daily-zip.") && name_str.ends_with(".meta") {
                count += 1;
            }
        }
    }

    count
}

struct BulkDataSource {
    display_name: &'static str,
    table_name: &'static str,
}

const BULK_DATA_SOURCES: &[BulkDataSource] = &[
    BulkDataSource {
        display_name: "Candidates",
        table_name: "libfec_candidates",
    },
    BulkDataSource {
        display_name: "Committees",
        table_name: "libfec_committees",
    },
    BulkDataSource {
        display_name: "Candidate-Committee Linkages",
        table_name: "libfec_candidate_committee_linkages",
    },
    BulkDataSource {
        display_name: "Operating Expenditures",
        table_name: "operating_expenses",
    },
    BulkDataSource {
        display_name: "Contributions to Candidates",
        table_name: "committee_contributions_to_candidates",
    },
    BulkDataSource {
        display_name: "Independent Expenditures",
        table_name: "independent_expenditures",
    },
    BulkDataSource {
        display_name: "PAC Summary",
        table_name: "pac_summary",
    },
    BulkDataSource {
        display_name: "Candidate Summary",
        table_name: "candidate_summary",
    },
    BulkDataSource {
        display_name: "Candidate Summary (CSV)",
        table_name: "candidate_summary_csv",
    },
    BulkDataSource {
        display_name: "Committee Summary (CSV)",
        table_name: "committee_summary_csv",
    },
    BulkDataSource {
        display_name: "Form 1 Filers",
        table_name: "form1_filers",
    },
    BulkDataSource {
        display_name: "Form 2 Filers",
        table_name: "form2_filers",
    },
];

const INDIVIDUAL_CONTRIBUTIONS_SOURCE: BulkDataSource = BulkDataSource {
    display_name: "Individual Contributions",
    table_name: "individual_contributions",
};

struct CycleInfo {
    year: u16,
    modified_at: String,
    last_checked_at: String,
}

fn human_duration_since_rfc2822(rfc2822: &str) -> String {
    let ts = match jiff::fmt::rfc2822::parse(rfc2822) {
        Ok(zdt) => zdt.timestamp(),
        Err(_) => return "unknown".to_string(),
    };
    human_duration_since(ts)
}

/// Parse SQLite datetime('now') format: "YYYY-MM-DD HH:MM:SS" (UTC)
fn human_duration_since_sqlite(datetime_str: &str) -> String {
    let dt = match jiff::civil::DateTime::from_str(datetime_str) {
        Ok(dt) => dt,
        Err(_) => return "unknown".to_string(),
    };
    let ts = match dt.in_tz("UTC") {
        Ok(zdt) => zdt.timestamp(),
        Err(_) => return "unknown".to_string(),
    };
    human_duration_since(ts)
}

fn human_duration_since(ts: Timestamp) -> String {
    let now = Timestamp::now();
    let span = match now.since(ts) {
        Ok(span) => span,
        Err(_) => return "unknown".to_string(),
    };

    let total_seconds = span.total(jiff::Unit::Second).unwrap_or(0.0) as i64;
    if total_seconds < 60 {
        return "just now".to_string();
    }

    let total_minutes = total_seconds / 60;
    if total_minutes < 60 {
        return format!(
            "{} minute{} ago",
            total_minutes,
            if total_minutes == 1 { "" } else { "s" }
        );
    }

    let total_hours = total_minutes / 60;
    if total_hours < 24 {
        return format!(
            "{} hour{} ago",
            total_hours,
            if total_hours == 1 { "" } else { "s" }
        );
    }

    let total_days = total_hours / 24;
    format!(
        "{} day{} ago",
        total_days,
        if total_days == 1 { "" } else { "s" }
    )
}

fn query_bulk_cycles(conn: &Connection, table_name: &str) -> Vec<CycleInfo> {
    let cycles_table = format!("{}_cycles", table_name);
    let sql = format!(
        "SELECT year, modified_at, last_checked_at FROM {} ORDER BY year",
        cycles_table
    );
    let mut stmt = match conn.prepare(&sql) {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(), // table doesn't exist
    };
    let rows = stmt
        .query_map([], |row| {
            Ok(CycleInfo {
                year: row.get::<_, u16>(0)?,
                modified_at: row.get::<_, String>(1)?,
                last_checked_at: row.get::<_, String>(2)?,
            })
        })
        .ok();
    match rows {
        Some(rows) => rows.flatten().collect(),
        None => Vec::new(),
    }
}

fn print_bulk_data_info(conn: &Connection) {
    let mut any_found = false;

    for source in BULK_DATA_SOURCES {
        let cycles = query_bulk_cycles(conn, source.table_name);
        if cycles.is_empty() {
            continue;
        }
        any_found = true;

        let years: Vec<String> = cycles.iter().map(|c| c.year.to_string()).collect();
        println!("    {} ({})", source.display_name.bold(), years.join(", "));

        for cycle in &cycles {
            let modified = human_duration_since_rfc2822(&cycle.modified_at);
            let checked = human_duration_since_sqlite(&cycle.last_checked_at);
            println!(
                "      {} — {}",
                cycle.year.to_string().bold(),
                format!("modified {}, checked {}", modified, checked).dimmed()
            );
        }
    }

    if !any_found {
        println!("    {}", "(no bulk data synced)".dimmed());
    }
}

pub fn cache_info(sourcer: &FilingSourcer) {
    let cache_dir = sourcer.cache.cache_directory();
    println!("{}", "Cache Information".bold());
    println!();
    println!("  {}", cache_dir.display().to_string().dimmed());
    println!();

    // .fec files
    let (fec_count, fec_bytes) = get_fec_files_stats(cache_dir);
    println!(
        "  FEC filings cached:    {} .fec files ({})",
        fec_count.to_formatted_string(&Locale::en),
        HumanBytes(fec_bytes)
    );

    // Daily zip meta files
    let daily_zip_count = get_daily_zip_meta_count(cache_dir);
    println!(
        "  Daily zip meta files:  {}",
        daily_zip_count.to_formatted_string(&Locale::en)
    );

    // Bulk data SQLite DB
    let bulk_db_path = cache_dir.join(".bulk-data.db");
    if let Some(size) = get_file_size(&bulk_db_path) {
        println!(
            "  Bulk data database:    {}  {}",
            HumanBytes(size),
            bulk_db_path.display().to_string().dimmed()
        );
    } else {
        println!("  Bulk data database:    {}", "(not found)".dimmed());
    }

    // Individual contributions DB
    let ic_db_path = cache_dir.join(".individual-contributions.db");
    if let Some(size) = get_file_size(&ic_db_path) {
        println!(
            "  Individual contribs:   {}  {}",
            HumanBytes(size),
            ic_db_path.display().to_string().dimmed()
        );
    }

    // API cache DB
    let api_cache_path = cache_dir.join(".api-cache.db");
    if let Some(size) = get_file_size(&api_cache_path) {
        println!(
            "  API cache database:    {}  {}",
            HumanBytes(size),
            api_cache_path.display().to_string().dimmed()
        );
    } else {
        println!("  API cache database:    {}", "(not found)".dimmed());
    }

    // Bulk data details
    if let Ok(conn) = Connection::open(&bulk_db_path) {
        println!();
        println!("  {}", "Bulk Data".bold());
        print_bulk_data_info(&conn);
    }

    // Individual contributions cycle details
    if let Ok(conn) = Connection::open(&ic_db_path) {
        let cycles = query_bulk_cycles(&conn, INDIVIDUAL_CONTRIBUTIONS_SOURCE.table_name);
        if !cycles.is_empty() {
            println!();
            println!("  {}", "Individual Contributions".bold());
            let years: Vec<String> = cycles.iter().map(|c| c.year.to_string()).collect();
            println!(
                "    {} ({})",
                INDIVIDUAL_CONTRIBUTIONS_SOURCE.display_name.bold(),
                years.join(", ")
            );
            for cycle in &cycles {
                let modified = human_duration_since_rfc2822(&cycle.modified_at);
                let checked = human_duration_since_sqlite(&cycle.last_checked_at);
                println!(
                    "      {} — {}",
                    cycle.year.to_string().bold(),
                    format!("modified {}, checked {}", modified, checked).dimmed()
                );
            }
        }
    }
}
