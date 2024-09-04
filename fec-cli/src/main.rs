mod cmd_download;
mod cmd_export;
mod cmd_fastfec;
mod cmd_feed;
mod cmd_info;
mod sourcer;

use std::fs;

use clap::{Arg, Command};
use cmd_export::CmdExportTarget;
use cmd_info::CmdInfoFormat;

fn main() {
    let info = Command::new("info")
        .arg(
            Arg::new("filing-path")
                .help(".fec file to read from")
                .num_args(1..),
        )
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
        .arg(
            Arg::new("filing")
                .help("Filing ID to download")
                .num_args(1..),
        )
        .arg(
            Arg::new("input-file")
                .short('i')
                .help(".txt files of FEC filing IDs to fetch, 1 line per filing ID"),
        )
        .arg(Arg::new("output-directory").help("Directory to store downloaded files into"));
    let feed = Command::new("feed").hide(true);
    let export = Command::new("export")
        .arg(
            Arg::new("filing")
                .help("FEC file, id, or URL to read from")
                .num_args(1..),
        )
        .arg(
            Arg::new("db")
                .short('o')
                .help("SQLite db to export to")
                .required(true),
        )
        .arg(
            Arg::new("input-file")
                .short('i')
                .help(".txt files of FEC filing IDs to fetch, 1 line per filing ID"),
        )
        .arg(
            Arg::new("target")
                .long("target")
                .help("Which itemizations to export and in what format")
                .default_value("form-type"),
        );

    let fastfec_compat = Command::new("fastfec-compat")
        .arg(Arg::new("filing-path").help(".fec file to read from"))
        .arg(Arg::new("output-directory").help("directory to write CSV files to"))
        .hide(true);

    let mut cmd = Command::new("libfec-cli")
        .subcommand(info)
        .subcommand(download)
        .subcommand(feed)
        .subcommand(export)
        .subcommand(fastfec_compat);
    let matches = cmd.clone().get_matches();

    match matches.subcommand() {
        Some(("fastfec-compat", m)) => {
            cmd_fastfec::cmd_fastfec_compat(
                m.get_one::<String>("filing-path")
                    .expect("filing-path is required"),
                m.get_one::<String>("output-directory")
                    .expect("output-directory is required."),
            )
            .unwrap();
        }
        Some(("info", m)) => {
            let format = match m.get_one::<String>("format").map(String::as_str) {
                None => CmdInfoFormat::Human,
                Some("json") => CmdInfoFormat::Json,
                Some(f) => todo!("Unknown format {f}"),
            };
            cmd_info::cmd_info(
                m.get_many::<String>("filing-path")
                    .unwrap()
                    .map(|v| v.to_owned())
                    .collect(),
                format,
                *m.get_one::<bool>("full").unwrap(),
            )
            .unwrap();
        }
        Some(("export", m)) => {
            let target = match m.get_one::<String>("target").map(|v| v.as_str()) {
                Some("form-type") => CmdExportTarget::ByFormType,
                Some("schedule-a") | Some("a") => CmdExportTarget::ScheduleA,
                Some(_) | None => todo!(),
            };
            let filing_paths: Vec<String> = match (
                m.get_many::<String>("filing"),
                m.get_one::<String>("input-file"),
            ) {
                (None, None) => todo!(),
                (None, Some(input_file)) => fs::read_to_string(input_file)
                    .unwrap()
                    .lines()
                    .map(|v| v.trim().to_owned())
                    .filter(|line| !line.is_empty() && !line.starts_with("#"))
                    .collect(),
                (Some(filing_paths), None) => filing_paths.map(|v| v.to_owned()).collect(),
                (Some(_), Some(_)) => todo!(),
            };
            cmd_export::cmd_export(filing_paths, m.get_one::<String>("db").unwrap(), target)
                .unwrap();
        }
        Some(("download", m)) => {
            let filing_ids: Vec<String> = match (
                m.get_many::<String>("filing"),
                m.get_one::<String>("input-file"),
            ) {
                (None, Some(input_file)) => fs::read_to_string(input_file)
                    .unwrap()
                    .lines()
                    .map(|v| v.trim().to_owned())
                    .filter(|line| !line.is_empty() && !line.starts_with("#"))
                    .collect(),
                (Some(filing_ids), None) => filing_ids.map(|v| v.to_owned()).collect(),
                (None, None) => todo!(),
                (Some(_), Some(_)) => todo!(),
            };
            cmd_download::cmd_download(
                filing_ids,
                m.get_one::<String>("output-directory")
                    .map(|v| v.to_owned()),
            )
            .unwrap();
        }
        Some(("feed", m)) => {
            cmd_feed::cmd_feed().unwrap();
        }
        Some(_) => todo!(),
        None => {
            cmd.print_help();
        }
    }
}
