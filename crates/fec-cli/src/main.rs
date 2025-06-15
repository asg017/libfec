mod api_flags;
mod cache;
mod cli;
mod cmd_download;
mod cmd_export;
mod cmd_fastfec;
mod cmd_feed;
mod cmd_filings;
mod cmd_info;
mod cmd_interactive;
mod sourcer;

use crate::cli::Commands;
use clap::Parser;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    match *cli::Cli::parse_from(args).command {
        Commands::Filings(args) => {
            cmd_filings::cmd_filings(args);
        }
        Commands::Info(args) => {
            cmd_info::cmd_info(args);
        }
        Commands::Download(args) => todo!(),
        Commands::Export(args) => {
            cmd_export::cmd_export(args);
        }
        Commands::FastFec(args) => {
            cmd_fastfec::cmd_fastfec_compat(&args.filing_id, args.output_directory.as_path());
        }
        Commands::Cache(args) => {
            cache::cache(args);
        }
    }
}
