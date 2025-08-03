mod api_flags;
mod bulk_util;
mod cli;
mod commands;
mod sourcer;
use crate::cli::Commands;
use clap::Parser;
use std::env;

fn main() {
    //bulk_util::resolve_candidates();
    let args: Vec<String> = env::args().collect();
    let cli = cli::Cli::parse_from(args.clone());
    match *cli.command {
        Commands::Info(args) => {
            commands::info(args).unwrap();
        }
        Commands::Export(args) => {
            commands::export(args).unwrap();
        }
        Commands::FastFec(args) => {
            commands::fastfec(&args.filing_id, args.output_directory.as_path()).unwrap();
        }
        Commands::Cache(ref args) => {
            commands::cache(&cli, args).unwrap();
        }
    }
}
