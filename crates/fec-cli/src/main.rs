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
            }
        }
        Err(parse_err) => {
            // If parsing failed, check if the first argument could be an InfoInput
            if args.len() > 1 {
                if commands::InfoInput::from_arg(&args[1]).is_ok() {
                    // Treat as info command with the argument as a filing/committee/candidate
                    let sourcer = sourcer::FilingSourcer::new(None);
                    let info_args = cli::InfoArgs {
                        filings: args[1..].to_vec(),
                        input_file: None,
                        format: cli::CmdInfoFormat::Human,
                        display: cli::InfoDisplayMode::Text,
                        full: false,
                    };
                    commands::info(sourcer, info_args)
                } else {
                    eprintln!("error: unrecognized command '{}'\n", args[1]);
                    parse_err.exit();
                }
            } else if std::io::stdin().is_terminal() {
                // No arguments and TTY available - start search TUI
                let sourcer = sourcer::FilingSourcer::new(None);
                let search_args = cli::SearchArgs {
                    query: String::new(),
                    cycle: 2026,
                    rpc: false,
                };
                commands::search(sourcer, &search_args)
            } else {
                parse_err.exit();
            }
        }
    };

    match result {
        Ok(()) => {
            std::process::exit(0);
        }
        Err(_) => {
            std::process::exit(1);
        }
    }
}
