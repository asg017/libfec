use core::str;
use std::{env, path::PathBuf};
use clap::{Args, Parser, Subcommand};
pub use crate::api_flags::FilingsApiFlags;

#[derive(Args, Debug)]
pub struct RunArgs {
    pub database: Option<PathBuf>,
    pub script: Option<PathBuf>,

    #[arg(long, short = 'p', num_args = 2)]
    pub parameters: Vec<String>,

    #[arg(long)]
    pub trace: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct XArgs {}

#[derive(Args, Debug)]
pub struct DownloadArgs {
    pub filings: Option<Vec<String>>,

    #[arg(
        long,
        short = 'i',
        help = ".txt files of FEC filing IDs to fetch, 1 line per filing ID"
    )]
    pub input_file: Option<PathBuf>,

    #[arg(long, short = 'f', help = "Format to output information to")]
    pub format: Option<String>,

    // output-directory
    #[arg(
        long,
        short = 'o',
        help = "Directory to output downloaded files to",
        default_value = ".",
        last = true
    )]
    pub output_directory: PathBuf,
}



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

#[derive(Args, Debug)]
pub struct FilingsArgs {
    #[command(flatten)]
    pub api: FilingsApiFlags,

    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub ids_only: bool,
    #[arg(long)]
    pub print_url: bool,
    #[arg(long)]
    pub url_only: bool,
    #[arg(long)]
    pub filing_urls_only: bool,
}

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    clap::ValueEnum,
)]
enum ExportFormat {
  #[default]
    Sqlite,
    Excel,
    Csv,
    Json,
}


#[derive(Args, Debug)]
pub struct ExportArgs {
    #[arg(required=false)]
    pub filings: Vec<String>,

    #[arg(long, short = 'o', help = "Output file")]
    pub output: PathBuf,

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

    #[command(flatten)]
    pub api: FilingsApiFlags,
}

#[derive(Subcommand, Debug)]
pub enum Commands {

  /// Export FEC Filings into SQLite, Excel, CSV, or JSON
    Export(ExportArgs),

    /// Retrive info about FEC filings from the fec.gov API
    Filings(FilingsArgs),

    /// Cache .fec files from fec.giv to LIBFEC_CACHE_DIRECTORY
    Cache(CacheArgs),

    /// Print debug information about a FEC filing, committee, or candidate 
    Info(InfoArgs),

    /// Download .fec files from fec.gov
    Download(DownloadArgs),

    
    //Feed(FeedArgs),
    //Export(ExportArgs),

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
}
