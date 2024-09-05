use colored::Colorize;
use fec_parser::{report_code_label, Filing};
use indicatif::{HumanBytes, ProgressBar};
use serde_json::Value;
use std::{collections::HashMap, error::Error, io::Read, time::Duration};

use tabled::{
    builder::Builder as TableBuilder,
    settings::{object::Columns as TableColumns, Alignment as TableAlignment, Style as TableStyle},
};

use crate::sourcer::FilingSourcer;
struct FilingFormMetadata {
    count: usize,
    bytes: usize,
}

pub(crate) enum CmdInfoFormat {
    Human,
    Json,
}

fn process_filing<R: Read>(
    filing: &mut Filing<R>,
    format: &CmdInfoFormat,
    spinner: &Option<ProgressBar>,
    full: bool,
) {
    if matches!(format, CmdInfoFormat::Human) {
        println!(
            "{} v{} ({} {}) {}",
            format!("FEC-{}", filing.filing_id).bold(),
            filing.header.fec_version,
            filing.header.soft_name,
            filing.header.soft_ver,
            filing
                .source_length
                .map_or("".to_owned(), |v| format!("({})", HumanBytes(v as u64)))
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
        println!(
            "{} \"{}\" filed by {} ({})",
            filing.cover.form_type.bold(),
            filing
                .cover
                .report_code
                .as_ref()
                .map(|report_code| report_code_label(report_code.as_str()))
                .unwrap_or(""),
            filing.cover.filer_name,
            filing.cover.filer_id,
        );
    }
    if !full {
        if let Some(ref spinner) = spinner {
            spinner.finish_and_clear();
        }
        return;
    }

    let mut status: HashMap<String, FilingFormMetadata> = HashMap::new();
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

    if let Some(ref spinner) = spinner {
        spinner.finish_and_clear();
    }

    let mut x: Vec<_> = status.iter().collect();
    x.sort_by(|a, b| b.1.count.cmp(&a.1.count));
    match format {
        CmdInfoFormat::Human => {
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
        CmdInfoFormat::Json => {
            let v = Value::Null;
            println!("{}", v);
        }
    }
}

pub fn cmd_info(
    filings: Vec<String>,
    format: CmdInfoFormat,
    full: bool,
) -> Result<(), Box<dyn Error>> {
    let filing_sourcer = FilingSourcer::new();

    let spinner = match format {
        CmdInfoFormat::Human => {
            let s = ProgressBar::new_spinner().with_message("Summarizing rows...");
            s.enable_steady_tick(Duration::from_millis(100));
            Some(s)
        }
        _ => None,
    };

    for filing in &filings {
        let mut filing = filing_sourcer.resolve(filing);
        process_filing(&mut filing, &format, &spinner, full);
    }

    Ok(())
}
