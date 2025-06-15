use colored::Colorize;
use fec_parser::{report_code_label, Filing};
use indicatif::{HumanBytes, ProgressBar};
use serde_json::Value;
use std::{collections::HashMap, error::Error, io::Read, time::Duration};

use tabled::{
    builder::Builder as TableBuilder,
    settings::{object::Columns as TableColumns, Alignment as TableAlignment, Style as TableStyle},
};

use crate::{cli::{CmdInfoFormat, InfoArgs}, sourcer::FilingSourcer};
struct FilingFormMetadata {
    count: usize,
    bytes: usize,
}


fn form_name(form_type: &str) -> &str {
    let base_form_type = if form_type.ends_with('A') || form_type.ends_with('N') {
        &form_type[..form_type.len() - 1]
    } else {
        form_type
    };

    match base_form_type {
    "F1"  => "Statement of Organization",
    "F1M"  => "Notification of Multicandidate Status",
    "F2"  => "Statement of Candidacy",
    "F24"  => "24/48 Hour Report of Independent Expenditures",
    "F3"  => "Report of Receipts and Disbursements for an Authorized Committee",
    "F3P"  => "Report of Receipts and Disbursements by an Authorized Committee of a Candidate for The Office of President or Vice President",
    "F3L"  => "Report of Contributions Bundled by Lobbyists/Registrants and Lobbyist/Registrant PACs",
    "F3X"  => "Report of Receipts and Disbursements for other than an Authorized Committee",
    "F4"  => "Report of Receipts and Disbursements for a Committee or Organization Supporting a Nomination Convention",
    "F5"  => "Report of Independent Expenditures Made and Contributions Received",
    "F6"  => "48 Hour Notice of Contributions/Loans Received",
    "F7"  => "Report of Communication Costs by Corporations and Membership Organizations",
    "F8"  => "Debt Settlement Plan",
    "F9"  => "24 Hour Notice of Disbursements for Electioneering Communications",
    "F13"  => "Report of Donations Accepted for Inaugural Committee",
    "F99"  => "Miscellaneous Text",
    "FRQ"  => "Request for Additional Information",
    _ => "",
  }
}

fn process_filing<R: Read>(
    filing: &mut Filing<R>,
    format: &CmdInfoFormat,
    spinner: &Option<ProgressBar>,
    full: bool,
) {
    if matches!(format, CmdInfoFormat::Human) {
        

        print!(
            "{} {} {} by {} ({})",
            format!("{}", filing.filing_id).bold(),
            
            filing.cover.form_type,
            filing
                .cover
                .report_code
                .as_ref()
                .map(|report_code| report_code_label(report_code.as_str()))
                .unwrap_or(""),
            filing.cover.filer_name.bold(),
            filing.cover.filer_id,
        );
        if let (Some(from), Some(through)) = (filing.cover.coverage_from_date, filing.cover.coverage_through_date) {
            print!(
                " covering {} to {}",
                from.to_string(),
                through.to_string(),
            );
        }
        println!();

        println!("{}", form_name(&filing.cover.form_type).dimmed());

        println!(
            "v{} {} filed with {} {}",
            filing.header.fec_version,
            filing
                .source_length
                .map_or("".to_owned(), |v| format!("({})", HumanBytes(v as u64))),
                filing.header.soft_name, filing.header.soft_ver
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
        
    }
    if !full {
        if let Some(ref spinner) = spinner {
            spinner.finish_and_clear();
        }
        return;
    }

    if let Some(spinner) = spinner {
      spinner.set_message("Summarizing rows...");
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

enum InfoInput {
  Filing(String),
  Commitee(String),
  //Canddate(String),
}
pub fn cmd_info(
    args: InfoArgs
) -> Result<(), Box<dyn Error>> {
    let filing_sourcer = FilingSourcer::new();

    let spinner = match args.format {
        CmdInfoFormat::Human => {
            let s = ProgressBar::new_spinner();
            s.enable_steady_tick(Duration::from_millis(100));
            Some(s)
        }
        _ => None,
    };
    let inputs = args.filings.iter().map(|v| {
      if v.starts_with("C") {
        InfoInput::Commitee(v.clone())
      } else {
        InfoInput::Filing(v.clone())
      }
    });
    for input in inputs {
      match input {
        InfoInput::Filing(filing) => {
            let mut filing = filing_sourcer.resolve(&filing);
            process_filing(&mut filing, &args.format, &spinner, args.full);
        }
        InfoInput::Commitee(commitee_id) => {
          // TODO print info about the committee
            println!("{commitee_id}");
        }
      }
    }

    

    Ok(())
}
