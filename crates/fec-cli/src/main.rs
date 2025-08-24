mod api_flags;
mod cache;
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
    let sourcer = sourcer::FilingSourcer::new(cli.top_level.cache_directory.clone());
    match *cli.command {
        Commands::Info(args) => {
            commands::info(sourcer, args).unwrap();
        }
        Commands::Export(args) => {
            commands::export(sourcer, args).unwrap();
        }
        Commands::FastFec(args) => {
            commands::fastfec(&args.filing_id, args.output_directory.as_path()).unwrap();
        }
        Commands::Cache(ref args) => {
            commands::cache(sourcer, args).unwrap();
        }
    }
}
