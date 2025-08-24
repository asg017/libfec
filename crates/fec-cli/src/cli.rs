pub use crate::api_flags::FilingsApiFlags;
use clap::{Args, Parser, Subcommand};
use core::str;
use std::{env, path::PathBuf};

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    //serde::Serialize,
    //serde::Deserialize,
    clap::ValueEnum,
)]
pub enum CmdInfoFormat {
    #[default]
    Human,
    Json,
}

#[derive(Args, Debug)]
pub struct InfoArgs {
    pub filings: Vec<String>,

    #[arg(
        long,
        short = 'i',
        help = ".txt files of FEC filing IDs to fetch, 1 line per filing ID"
    )]
    pub input_file: Option<PathBuf>,

    #[arg(
      long, 
      short = 'f', 
      value_enum, 
      help = "Format to output information to",  
      default_value_t = CmdInfoFormat::Human)]
    pub format: CmdInfoFormat,
    //#[arg(long, short = 'f', value_enum)]
    //pub format: Option<QueryFormat>,
    #[arg(
        long,
        help = "Calculate stats on all itemizations in the provided filings"
    )]
    pub full: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ExportFormat {
    #[default]
    Sqlite,
    Excel,
    Csv,
    Json,
}

#[derive(Args, Debug)]
pub struct ExportArgs {
    #[arg(required = false)]
    pub filings: Vec<String>,

    #[arg(long, short = 'o', help = "Output file")]
    pub output: PathBuf,

    #[arg(long, action, help = "Only export cover records, not itemizations")]
    pub cover_only: bool,

    //#[arg(long, short = 'f', help = "Format to export to")]
    //pub format: Option<ExportFormat>,
    #[command(flatten)]
    pub api: FilingsApiFlags,
}

#[derive(Args, Debug)]
pub struct FastFecArgs {
    /// Output directory
    pub output_directory: PathBuf,

    /// FEC filing id OR path to a .fec file
    pub filing_id: String,
}

#[derive(Args, Debug)]
pub struct CacheArgs {
    /// FEC filing id, ex `FEC-C00606962`
    pub filings: Option<Vec<String>>,

    #[arg(
        long,
        alias = "concurrent",
        help = "Number of concurrent downloads",
        default_value_t = 8
    )]
    pub number_concurrent: usize,

    #[command(flatten)]
    pub api: FilingsApiFlags,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Export FEC filings into SQLite, Excel, CSV, or JSON
    Export(ExportArgs),

    /// Cache .fec files from fec.gov to your filesystem
    Cache(CacheArgs),

    /// Print debug information about a FEC filing, committee, or candidate
    Info(InfoArgs),

    //Feed(FeedArgs),
    /// FastFEC compatible export
    FastFec(FastFecArgs),
}

#[derive(Parser)]
#[command(
  name = "libfec", 
  author,
  long_version = env!("CARGO_PKG_VERSION"), 
  about = "libfec CLI", 
  version,
  subcommand_required = false,
  arg_required_else_help = false,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Box<Commands>,

    #[command(flatten)]
    pub top_level: TopLevelArgs,
}

#[derive(Parser)]
pub struct TopLevelArgs {
    // TODO: api-key, api-url
    #[arg(
        global = true,
        long,
        env = "LIBFEC_CACHE_DIRECTORY",
        help_heading = "Global options"
    )]
    pub cache_directory: Option<PathBuf>,
}
