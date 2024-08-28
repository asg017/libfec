use colored::Colorize;
use fec_parser::{mappings::column_names_for_field, Filing, FilingError};
use indicatif::{HumanBytes, ProgressBar};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Read,
    path::Path,
    time::Duration,
};
use zip::read::ZipFile;

use tabled::{
    builder::Builder as TableBuilder,
    settings::{object::Columns as TableColumns, Alignment as TableAlignment, Style as TableStyle},
};
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
) {
    if matches!(format, CmdInfoFormat::Human) {
        println!(
            "Info {} {}",
            filing.filing_id,
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
        println!(
            "{} {} ({})",
            filing.cover.form_type, filing.cover.filer_name, filing.cover.filer_id
        );
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

pub fn cmd_info(filing_paths: Vec<String>, format: CmdInfoFormat) -> Result<(), FilingError> {
    let spinner = match format {
        CmdInfoFormat::Human => {
            let s = ProgressBar::new_spinner().with_message("Summarizing rows...");
            s.enable_steady_tick(Duration::from_millis(100));
            Some(s)
        }
        _ => None,
    };

    for filing_path in filing_paths {
        if filing_path.ends_with(".zip") {
            let f = File::open(filing_path).unwrap();
            let mut z = zip::ZipArchive::new(f).unwrap();
            for i in 0..z.len() {
                let mut file = z.by_index(i).unwrap();
                let filing_id = Path::new(file.name())
                    .file_stem()
                    .map(|v| v.to_string_lossy().into_owned())
                    .unwrap();
                let source_length = Some(file.size() as usize);
                let mut filing =
                    Filing::<ZipFile>::from_reader(file, filing_id, source_length).unwrap();
                process_filing(&mut filing, &format, &spinner);
            }
        } else {
            let mut filing = Filing::<fs::File>::from_path(Path::new(&filing_path))?;
            process_filing(&mut filing, &format, &spinner);
        }
    }

    Ok(())
}
