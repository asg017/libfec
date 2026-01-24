/**
 * libfec CLI - Command-line interface for FEC data
 *
 * Main entry point for the libfec command-line tool, which provides access to
 * Federal Election Commission (FEC) filing data and bulk datasets.
 */

mod api_flags;
mod cache;
mod cli;
mod commands;
mod rss;
mod sourcer;
mod tui;
use crate::cli::Commands;
use clap::Parser;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let cli = cli::Cli::parse_from(args.clone());
    let sourcer = sourcer::FilingSourcer::new(cli.top_level.cache_directory.clone());
    let result = match *cli.command {
        Commands::Info(args) => commands::info(sourcer, args),
        Commands::Export(args) => commands::export(sourcer, *args),
        Commands::Fastfec(args) => commands::fastfec(sourcer, args),
        Commands::Cache(ref args) => commands::cache(sourcer, args),
        Commands::Search(ref args) => commands::search(sourcer, args),
        Commands::Bulk(ref args) => commands::bulk(sourcer, args),
        Commands::Rss(ref args) => commands::rss(sourcer, args),
        Commands::Dates(ref args) => commands::dates(sourcer, args),
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
