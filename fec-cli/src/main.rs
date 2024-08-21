mod cmd_download;
mod cmd_export;
mod cmd_fastfec;
mod cmd_info;

use clap::{Arg, Command};

fn main() {
    let info = Command::new("info").arg(
        Arg::new("filing-path")
            .help(".fec file to read from")
            .num_args(1..),
    );
    let download = Command::new("download").arg(
        Arg::new("filing-id")
            .help("Filing ID to download")
            .num_args(1..),
    );
    let export = Command::new("export")
        .arg(
            Arg::new("filing-path")
                .help(".fec file to read from")
                .num_args(1..),
        )
        .arg(Arg::new("db").short('o').help("SQLite db to export to"));

    let fastfec_compat = Command::new("fastfec-compat")
        .arg(Arg::new("filing-path").help(".fec file to read from"))
        .arg(Arg::new("output-directory").help("directory to write CSV files to"));

    let matches = Command::new("libfec-cli")
        .subcommand(info)
        .subcommand(download)
        .subcommand(export)
        .subcommand(fastfec_compat)
        .get_matches();

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
            cmd_info::cmd_info(
                m.get_many::<String>("filing-path")
                    .unwrap()
                    .map(|v| v.to_owned())
                    .collect(),
            )
            .unwrap();
        }
        Some(("export", m)) => {
            cmd_export::cmd_export(
                m.get_many::<String>("filing-path")
                    .unwrap()
                    .map(|v| v.to_owned())
                    .collect(),
                m.get_one::<String>("db").unwrap(),
            )
            .unwrap();
        }
        Some(("download", m)) => {
            cmd_download::cmd_download(
                m.get_many::<String>("filing-id")
                    .unwrap()
                    .map(|v| v.to_owned())
                    .collect(),
            )
            .unwrap();
        }
        Some(_) => todo!(),
        None => todo!(),
    }
}
