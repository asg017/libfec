use colored::Colorize;
use indicatif::HumanBytes;
use num_format::{Locale, ToFormattedString};
use std::path::Path;

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
}
