pub use crate::api_flags::FilingsApiFlags;
use clap::{Args, Parser, Subcommand, ValueEnum};
use core::str;
use fec_parser::schedules::ScheduleType;
use std::{
    env,
    path::PathBuf,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ExportFormat {
    Sqlite,
    Excel,
    Csv,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ExportTarget {
    #[value(alias = "receipts")]
    ScheduleA,
    #[value(alias = "disbursements")]
    ScheduleB,
}

impl Into<ScheduleType> for ExportTarget {
    fn into(self) -> ScheduleType {
        match self {
            ExportTarget::ScheduleA => ScheduleType::ScheduleA,
            ExportTarget::ScheduleB => ScheduleType::ScheduleB,
        }
    }
}

#[derive(Args, Debug)]
pub struct ExportArgs {
    #[arg(required = false)]
    pub filings: Vec<String>,

    #[arg(long, short = 'f', help = "Output file")]
    pub format: Option<ExportFormat>,

    #[arg(long, help = "Output file")]
    pub target: Option<ExportTarget>,

    #[arg(long, short = 'o', help = "Output file")]
    pub output: Option<PathBuf>,

    #[arg(long, alias = "outdir", help = "Output directory")]
    pub output_directory: Option<PathBuf>,

    #[arg(long, action, help = "Overwrite existing files")]
    pub clobber: bool,

    #[arg(long, action, help = "Only export cover records, not itemizations")]
    pub cover_only: bool,

    //#[arg(long, short = 'f', help = "Format to export to")]
    //pub format: Option<ExportFormat>,
    #[command(flatten)]
    pub api: FilingsApiFlags,
}

#[derive(Args, Debug)]
pub struct FastFecArgs {
    /// FEC filing id OR path to a .fec file
    pub filing_id: String,

    /// Output directory

    #[arg(default_value = "output")]
    pub output_directory: PathBuf,
}

#[derive(Args, Debug)]
pub struct CacheAddArgs {
    /// FEC filing IDs or paths to .fec files to cache
    #[arg(required = false)]
    pub filings: Vec<String>,

    #[command(flatten)]
    pub api: FilingsApiFlags,
}

#[derive(Subcommand, Debug)]
pub enum CacheSubcommand {
    /// Print the cache directory path
    Print,
    /// Show summary information about the cache
    Info,
    /// Download and cache filings from fec.gov
    Add(CacheAddArgs),
}

#[derive(Args, Debug)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: CacheSubcommand,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    #[arg(default_value = "", help = "Initial search query (optional)")]
    pub query: String,
    #[arg(long, default_value_t = 2026, help = "Election cycle year to search")]
    pub cycle: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CycleArg {
    Single(u16),
    Range(u16, u16),
}

impl std::str::FromStr for CycleArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((start, end)) = s.split_once('-') {
            let start_year = start.trim().parse::<u16>()
                .map_err(|_| format!("Invalid start year: {}", start))?;
            let end_year = end.trim().parse::<u16>()
                .map_err(|_| format!("Invalid end year: {}", end))?;
            
            if start_year > end_year {
                return Err(format!("Start year {} cannot be greater than end year {}", start_year, end_year));
            }
            
            Ok(CycleArg::Range(start_year, end_year))
        } else {
            let year = s.trim().parse::<u16>()
                .map_err(|_| format!("Invalid year: {}", s))?;
            Ok(CycleArg::Single(year))
        }
    }
}

impl std::fmt::Display for CycleArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CycleArg::Single(year) => write!(f, "{}", year),
            CycleArg::Range(start, end) => write!(f, "{}-{}", start, end),
        }
    }
}

#[derive(Args, Debug)]
pub struct BulkArgs {
  #[arg(long, short = 'o',  help = "Output file path")]
  pub output: PathBuf,
  #[arg(long, help = "Election cycle year (e.g., 2024) or range (e.g., 2024-2026)")]
  pub cycle: CycleArg,

  #[arg(long, value_delimiter = ',', value_enum, help = "Bulk data source(s) to export (comma-separated, e.g., candidates,committees)")]
  pub source: Vec<BulkSource>,

}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum BulkSource {
  Opex,
  Committees,
  Candidates,
}


#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Export FEC filings into SQLite, Excel, CSV, or JSON
    Export(Box<ExportArgs>),

    /// Cache .fec files from fec.gov to your filesystem
    Cache(CacheArgs),

    /// Print debug information about a FEC filing, committee, or candidate
    Info(InfoArgs),

    //Feed(FeedArgs),
    /// FastFEC compatible export
    Fastfec(FastFecArgs),

    Search(SearchArgs),

    // Export bulk datasets from fec.gov
    Bulk(BulkArgs),
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