pub use crate::api_flags::FilingsApiFlags;
use clap::{Args, Parser, Subcommand, ValueEnum};
use core::str;
use fec_parser::schedules::ScheduleType;
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum InfoDisplayMode {
    #[default]
    Text,
    Tui,
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

    #[arg(
      long,
      short = 'd',
      value_enum,
      help = "Display mode: text (default) or tui",
      default_value_t = InfoDisplayMode::Text)]
    pub display: InfoDisplayMode,
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

impl From<ExportTarget> for ScheduleType {
    fn from(val: ExportTarget) -> Self {
        match val {
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

    /// Enable RPC mode for programmatic control via JSONL stdio protocol
    #[arg(long)]
    pub rpc: bool,

    /// Write export metadata to the database (export ID, input mappings, filing list)
    #[arg(long)]
    pub write_metadata: bool,
    
    /// Export all bulk candidates/committees for the given cycle
    #[arg(long, requires = "cycle", help = "Include all bulk data (candidates, committees, etc.) for the specified cycle")]
    pub include_all_bulk: bool,

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
    Add(Box<CacheAddArgs>),
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
    /// Enable RPC mode for programmatic control via JSONL stdio protocol
    #[arg(long)]
    pub rpc: bool,
}

/// Parse a category name to its FEC API category ID(s)
/// Some categories like "deadlines" map to multiple IDs
pub fn category_name_to_ids(name: &str) -> Vec<u32> {
    match name.trim().to_lowercase().as_str() {
        "elections" => vec![36],
        "deadlines" => vec![21, 25, 26], // Reporting Deadlines + Quarterly + Monthly
        "quarterly" => vec![25],
        "monthly" => vec![26],
        "pre-post" => vec![27],
        "meetings" => vec![20],
        "open-meetings" => vec![32],
        "executive" => vec![39],
        "hearings" => vec![40],
        "conferences" => vec![33],
        "roundtables" => vec![34],
        "outreach" => vec![22],
        "aos-rules" => vec![23],
        "holidays" => vec![37],
        "fea" => vec![38],
        "ec" => vec![28],
        "ie" => vec![29],
        "other" => vec![24],
        _ => vec![],
    }
}

/// Get display name for a category ID
#[allow(dead_code)]
pub fn category_id_to_name(id: u32) -> &'static str {
    match id {
        36 => "Elections",
        21 => "Deadlines",
        25 => "Quarterly",
        26 => "Monthly",
        27 => "Pre/Post-Election",
        20 => "Meetings",
        32 => "Open Meetings",
        39 => "Executive Sessions",
        40 => "Public Hearings",
        33 => "Conferences",
        34 => "Roundtables",
        22 => "Outreach",
        23 => "AOs & Rules",
        37 => "Holidays",
        38 => "FEA Periods",
        28 => "EC Periods",
        29 => "IE Periods",
        24 => "Other",
        _ => "Unknown",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum DatesFormat {
    /// Interactive TUI
    #[default]
    Tui,
    /// JSON output (raw API response)
    Json,
}

#[derive(Args, Debug, Clone)]
pub struct DatesArgs {
    /// Filter by category (comma-separated). Options: elections, deadlines, quarterly,
    /// monthly, pre-post, meetings, holidays, ec, ie, fea, other, conferences, roundtables,
    /// outreach, aos-rules, open-meetings, executive, hearings.
    /// Note: When --state is provided, ec and ie are automatically added to the default categories.
    #[arg(long, short = 'c', default_value = "elections,deadlines")]
    pub category: String,

    /// Number of days to look ahead
    #[arg(long, short = 'd', default_value = "365")]
    pub days: u32,

    /// Maximum number of results
    #[arg(long, short = 'n', default_value = "500")]
    pub limit: u32,

    /// Filter by state (2-letter code, e.g., CA, TX, NY). Shows elections for the specified
    /// state plus all reporting deadlines and reporting periods.
    #[arg(long, short = 's')]
    pub state: Option<String>,

    /// Output format
    #[arg(long, short = 'f', value_enum, default_value = "tui")]
    pub format: DatesFormat,
}

impl DatesArgs {
    /// Parse the category string into a list of category IDs
    /// When a state filter is provided and categories are default, automatically include EC and IE periods
    pub fn category_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self
            .category
            .split(',')
            .flat_map(category_name_to_ids)
            .collect();

        // If state filter is provided and user is using default categories, add EC and IE
        if self.state.is_some() && self.category == "elections,deadlines" {
            ids.extend_from_slice(&[28, 29]); // EC and IE periods
        }

        ids
    }

    /// Get display string for the selected categories (shows user-friendly names)
    pub fn category_display(&self) -> String {
        // Show the category names the user provided, not the expanded IDs
        let mut names: Vec<String> = self
            .category
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|n| {
                // Capitalize first letter
                let mut c = n.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect();

        // If state filter is provided and using default categories, indicate EC/IE are included
        if self.state.is_some() && self.category == "elections,deadlines" {
            names.push("Ec".to_string());
            names.push("Ie".to_string());
        }

        if names.is_empty() {
            "All Events".to_string()
        } else {
            names.join(", ")
        }
    }

    /// Get normalized state code (uppercase 2-letter code)
    pub fn state_code(&self) -> Option<String> {
        self.state.as_ref().map(|s| s.to_uppercase())
    }
}

#[derive(Args, Debug, Clone)]
pub struct RssArgs {
    /// Watch mode: continuously fetch and display updates in a TUI
    #[arg(long, short)]
    pub watch: bool,

    /// Refresh interval in seconds (default: 300 = 5 minutes)
    #[arg(long, short = 'i', default_value = "300")]
    pub interval: u64,

    /// Number of records to display (default: 20)
    #[arg(long, short = 'n', default_value = "20")]
    pub limit: usize,

    /// Pre-defined filing type filter (all, monthly, quarterly, presidential, congressional, pac)
    #[arg(long, short = 'p', value_enum, default_value = "all")]
    pub preset: RssPreset,

    /// Filter by form type (e.g., F1, F3, F3P, F3X, F99)
    #[arg(long, short = 'f')]
    pub form_type: Option<String>,

    /// Filter by committee ID(s), comma-separated (e.g., C00505412,C00513531)
    #[arg(long, short = 'c')]
    pub committee: Option<String>,

    /// Filter by state code (e.g., CA, TX, NY)
    #[arg(long, short = 's')]
    pub state: Option<String>,

    /// Filter by party affiliation (DEM, REP, LIB, GRE, CON, REF, OTH)
    #[arg(long)]
    pub party: Option<String>,

    /// Export filings to a SQLite database
    #[arg(long, short = 'x')]
    pub export: Option<PathBuf>,

    /// Only export cover data, not itemizations (requires --export)
    #[arg(long)]
    pub cover_only: bool,

    /// Only show/export filings since this time (e.g., "2026-01-20T00:00:00Z", "1 day ago", "2 hours ago")
    #[arg(long)]
    pub since: Option<String>,

    /// Enable RPC mode for programmatic control via JSONL stdio protocol
    #[arg(long)]
    pub rpc: bool,

    /// Write metadata about RSS sync operations to the database (requires --export)
    #[arg(long)]
    pub write_metadata: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum RssPreset {
    /// All filings
    #[default]
    All,
    /// Monthly report filings
    Monthly,
    /// Quarterly report filings
    Quarterly,
    /// Presidential campaign filings (F3P)
    Presidential,
    /// Congressional campaign filings (F3)
    Congressional,
    /// PAC and Party committee filings (F3X)
    Pac,
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
            let start_year = start
                .trim()
                .parse::<u16>()
                .map_err(|_| format!("Invalid start year: {}", start))?;
            let end_year = end
                .trim()
                .parse::<u16>()
                .map_err(|_| format!("Invalid end year: {}", end))?;

            if start_year > end_year {
                return Err(format!(
                    "Start year {} cannot be greater than end year {}",
                    start_year, end_year
                ));
            }

            Ok(CycleArg::Range(start_year, end_year))
        } else {
            let year = s
                .trim()
                .parse::<u16>()
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
    #[arg(long, short = 'o', help = "Output file path")]
    pub output: PathBuf,
    #[arg(
        long,
        help = "Election cycle year (e.g., 2024) or range (e.g., 2024-2026)"
    )]
    pub cycle: CycleArg,

    #[arg(
        long,
        value_delimiter = ',',
        value_enum,
        help = "Bulk data source(s) to export (comma-separated, e.g., candidates,committees)"
    )]
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

    /// FastFEC compatible export
    Fastfec(FastFecArgs),

    /// Search candidates and committees
    Search(SearchArgs),

    /// Export bulk datasets from fec.gov
    Bulk(BulkArgs),

    /// Watch FEC RSS feed for new filings
    Rss(RssArgs),

    /// View upcoming FEC calendar dates (elections, deadlines, meetings)
    Dates(DatesArgs),
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
  subcommand_required = true,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Box<Commands>,

    #[command(flatten)]
    pub top_level: TopLevelArgs,
}
