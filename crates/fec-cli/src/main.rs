//! libfec CLI - Command-line interface for FEC data
//!
//! Main entry point for the libfec command-line tool, which provides access to
//! Federal Election Commission (FEC) filing data and bulk datasets.

mod api_flags;
mod cache;
mod cli;
mod commands;
mod rss;
mod sourcer;
mod tui;
mod utils;
use crate::cli::{Cli, Commands};
use clap::Parser;
use std::env;
use std::io::IsTerminal;

fn main() {
    let args: Vec<String> = env::args().collect();
    let result = match Cli::try_parse_from(args.clone()) {
        Ok(cli) => {
            let sourcer = sourcer::FilingSourcer::new(cli.top_level.cache_directory.clone());
            match *cli.command {
                Commands::Info(args) => commands::info(sourcer, args),
                Commands::Export(args) => commands::export(sourcer, *args),
                Commands::Fastfec(args) => commands::fastfec(sourcer, args),
                Commands::Cache(ref args) => commands::cache(sourcer, args),
                Commands::Search(ref args) => commands::search(sourcer, args),
                Commands::Bulk(ref args) => commands::bulk(sourcer, args),
                Commands::Rss(ref args) => commands::rss(sourcer, args),
                Commands::Dates(ref args) => commands::dates(sourcer, args),
                Commands::Api(ref args) => commands::api(sourcer, args),
                Commands::Datasette(args) => commands::datasette(sourcer, *args),
            }
        }
        Err(parse_err) => {
            // Let clap handle --help and --version directly
            if parse_err.kind() == clap::error::ErrorKind::DisplayHelp
                || parse_err.kind() == clap::error::ErrorKind::DisplayVersion
            {
                parse_err.exit();
            }
            // If parsing failed, check if the first argument could be an InfoInput
            if args.len() > 1 {
                if let Ok(Some(contest)) = sourcer::Contest::from_arg(&args[1]) {
                    // Contest shorthand (CA41, H-CA41, S-CA, P) takes priority
                    let sourcer = sourcer::FilingSourcer::new(None);
                    commands::contest(sourcer, contest, 2026)
                } else if commands::InfoInput::from_arg(&args[1]).is_ok() {
                    // Treat as info command with the argument as a filing/committee/candidate
                    let sourcer = sourcer::FilingSourcer::new(None);
                    let info_args = cli::InfoArgs {
                        filings: args[1..].to_vec(),
                        input_file: None,
                        format: cli::CmdInfoFormat::Human,
                        full: false,
                    };
                    commands::info(sourcer, info_args)
                } else {
                    eprintln!("error: unrecognized command '{}'\n", args[1]);
                    parse_err.exit();
                }
            } else if std::io::stdin().is_terminal() {
                // No arguments and TTY available - start landing page
                commands::landing(None)
            } else {
                parse_err.exit();
            }
        }
    };

    match result {
        Ok(()) => {
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
