mod cmd_download;
mod cmd_export;
mod cmd_fastfec;
mod cmd_feed;
mod cmd_info;
mod sourcer;

use std::{error::Error, fs, process};

use clap::{parser::ValuesRef, Arg, Command};
use cmd_export::CmdExportTarget;
use cmd_info::CmdInfoFormat;

fn resolve_filing_ids(
    filing_matches: Option<ValuesRef<String>>,
    input_file_match: Option<&String>,
) -> Vec<String> {
    match (filing_matches, input_file_match) {
        (None, Some(input_file)) => fs::read_to_string(input_file)
            .unwrap()
            .lines()
            .map(|v| v.trim().to_owned())
            .filter(|line| !line.is_empty() && !line.starts_with("#"))
            .collect(),
        (Some(filing_ids), None) => filing_ids.map(|v| v.to_owned()).collect(),
        (None, None) => todo!(),
        (Some(_), Some(_)) => todo!(),
    }
}

fn cmd() -> Command {
    let arg_filings = Arg::new("filing")
        .help("Filing ID to download")
        .num_args(1..);
    let arg_input_file = Arg::new("input-file")
        .short('i')
        .help(".txt files of FEC filing IDs to fetch, 1 line per filing ID");

    let info = Command::new("info")
        .about("Retrieve information about a specified FEC filing")
        .arg(arg_filings.clone())
        .arg(arg_input_file.clone())
        .arg(
            Arg::new("format")
                .short('f')
                .help("Format to output information to")
                .required(false),
        )
        .arg(
            Arg::new("full")
                .long("full")
                .help("Calculate stats on all itemizations in the provided filings")
                .num_args(0)
                .required(false),
        );
    let download = Command::new("download")
        .about("Downloads a FEC filing files from the fec.gov website.")
        .arg(arg_filings.clone())
        .arg(arg_input_file.clone())
        .arg(Arg::new("output-directory").help("Directory to store downloaded files into"));

    let export = Command::new("export")
        .about("Export FEC filings itemizations to a SQLite database.")
        .arg(arg_filings.clone())
        .arg(
            Arg::new("db")
                .short('o')
                .help("SQLite db to export to")
                .required(true),
        )
        .arg(arg_input_file.clone())
        .arg(
            Arg::new("target")
                .long("target")
                .help("Which itemizations to export and in what format")
                .default_value("form-type"),
        );

    let feed = Command::new("feed").hide(true);

    let fastfec_compat = Command::new("fastfec-compat")
        .arg(Arg::new("filing-path").help(".fec file to read from"))
        .arg(Arg::new("output-directory").help("directory to write CSV files to"))
        .hide(true);

    Command::new(clap::crate_name!())
  .version(clap::crate_version!())
  .about("A CLI for downloading, inspecting, and exporting data found in United States Federal Election Commission filings (aka FEC filings). ")
  .subcommand(info)
  .subcommand(download)
  .subcommand(feed)
  .subcommand(export)
  .subcommand(fastfec_compat)
}

fn main() {
    let mut cmd = cmd();
    let matches = cmd.clone().get_matches();

    let result: Result<_, Box<dyn Error>> = match matches.subcommand() {
        Some(("fastfec-compat", m)) => cmd_fastfec::cmd_fastfec_compat(
            m.get_one::<String>("filing-path")
                .expect("filing-path is required"),
            m.get_one::<String>("output-directory")
                .expect("output-directory is required."),
        ),
        Some(("info", m)) => {
            let filings = resolve_filing_ids(
                m.get_many::<String>("filing"),
                m.get_one::<String>("input-file"),
            );
            let format = match m.get_one::<String>("format").map(String::as_str) {
                None => CmdInfoFormat::Human,
                Some("json") => CmdInfoFormat::Json,
                Some(f) => todo!("Unknown format {f}"),
            };
            let full = *m.get_one::<bool>("full").unwrap();
            cmd_info::cmd_info(filings, format, full)
        }
        Some(("export", m)) => {
            let filings = resolve_filing_ids(
                m.get_many::<String>("filing"),
                m.get_one::<String>("input-file"),
            );
            let db = m.get_one::<String>("db").unwrap();
            let target = match m.get_one::<String>("target").map(|v| v.as_str()) {
                Some("form-type") => CmdExportTarget::ByFormType,
                Some("schedule-a") | Some("a") => CmdExportTarget::ScheduleA,
                Some(_) | None => todo!(),
            };
            cmd_export::cmd_export(filings, db, target)
        }
        Some(("download", m)) => {
            let filings = resolve_filing_ids(
                m.get_many::<String>("filing"),
                m.get_one::<String>("input-file"),
            );
            let output_directory = m
                .get_one::<String>("output-directory")
                .map(|v| v.to_owned());

            cmd_download::cmd_download(filings, output_directory)
        }
        Some(("feed", _)) => {
            cmd_feed::cmd_feed().unwrap();
            todo!()
        }
        Some(_) => todo!(),
        None => cmd.print_help().map_err(|_e| todo!()),
    };

    match result {
        Ok(_) => process::exit(0),
        Err(_) => process::exit(1),
    }
}
