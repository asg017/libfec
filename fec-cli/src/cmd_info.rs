use colored::Colorize;
use fec_parser::{Filing, FilingError};
use indicatif::{HumanBytes, ProgressBar};
use std::{collections::HashMap, fs, path::Path, time::Duration};

use tabled::{
    builder::Builder as TableBuilder,
    settings::{object::Columns as TableColumns, Alignment as TableAlignment, Style as TableStyle},
};
struct FilingFormMetadata {
    count: usize,
    bytes: usize,
}
pub fn cmd_info(filing_paths: Vec<String>) -> Result<(), FilingError> {
    for filing_path in filing_paths {
        let mut filing = Filing::<fs::File>::from_path(Path::new(&filing_path))?;
        println!(
            "Info {} {}",
            filing_path.bold(),
            filing
                .source_length
                .map_or("".to_owned(), |v| format!("({})", HumanBytes(v as u64)))
        );
        println!("{}", filing.filing_id);
        println!("{}: {}", "FEC Version".bold(), filing.header.fec_version);
        println!(
            "{}: {} ({})",
            "Software".bold(),
            filing.header.soft_name,
            filing.header.soft_ver
        );
        if let Some(ref report_id) = filing.header.report_id {
            println!("{}: '{}'", "Report ID".bold(), report_id);
        }
        if let Some(ref report_number) = filing.header.report_number {
            println!("Report #{}", report_number);
        }
        if let Some(ref comment) = filing.header.comment {
            println!("{}: '{}'", "Comment".bold(), comment);
        }

        let mut status: HashMap<String, FilingFormMetadata> = HashMap::new();

        let spinner = ProgressBar::new_spinner().with_message("Summarizing rows...");
        spinner.enable_steady_tick(Duration::from_millis(100));

        while let Some(row) = filing.next_row() {
            let row = row.unwrap();
            if let Some(x) = status.get_mut(&row.row_type) {
                x.count += 1;
                x.bytes += row.original_size;
            } else {
                status.insert(
                    row.row_type.clone(),
                    FilingFormMetadata {
                        count: 1,
                        bytes: row.original_size,
                    },
                );
            }
        }
        spinner.finish_and_clear();

        let mut x: Vec<_> = status.iter().collect();
        x.sort_by(|a, b| b.1.count.cmp(&a.1.count));

        let mut tbl = TableBuilder::new();
        tbl.push_record(["Form Type", "# Rows", "Size"]);
        for (x, y) in x {
            tbl.push_record([
                x,
                &indicatif::HumanCount(y.count as u64).to_string(),
                &indicatif::HumanBytes(y.bytes as u64).to_string(),
            ]);
        }
        let tbl = tbl
            .build()
            .with(TableStyle::modern_rounded())
            .modify(TableColumns::new(1..3), TableAlignment::right())
            .to_string();

        println!("{tbl}");
    }

    Ok(())
}
