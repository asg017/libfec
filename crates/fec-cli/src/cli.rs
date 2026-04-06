pub use crate::api_flags::FilingsApiFlags;
use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Args, Parser, Subcommand, ValueEnum};
use core::str;
use fec_parser::schedules::ScheduleType;
use std::path::PathBuf;

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
#[command(after_long_help = "\x1b[1;32mExamples:\x1b[0m

  Export itemizations for a specific filing:

    libfec export FEC-1949543 -o filing.db
    libfec export FEC-1949543 --output-directory filings/
    libfec export FEC-1949543 --target schedule-a -o contributions.csv
    libfec export FEC-1949543 --target schedule-b -o disbursements.json

  Export filings from specific committees:

    libfec export C00835959 --cycle 2026 -o fairshake.db

  Export all filings for all candidate committes in an election:

    libfec export TX-S --cycle 2026 -o texas-senate.db
    libfec export CA41 --cycle 2024 -o california-house-41.db
")]
pub struct ExportArgs {
    #[arg(
        required = false,
        help = "Each argument can be one of:

- Filing ID, ex. FEC-1949543
- Path or URL to a .fec file, ex ./1949543.fec
- Committee ID, ex. C00835959
- Contest shorthand, ex. TX-S, CA41, H-IL03
- .txt file containing filing IDs, committee IDs, or contest shorthands (one per line)
"
    )]
    pub filings: Vec<String>,

    #[arg(
        long,
        short = 'o',
        help = "Output data to a specific file. File type is inferred from extension (.db, .xlsx, .csv, .json)"
    )]
    pub output: Option<PathBuf>,

    #[arg(
        long,
        alias = "outdir",
        help = "Output data into a directory, with one file per form type"
    )]
    pub output_directory: Option<PathBuf>,

    #[arg(
        long,
        short = 'f',
        help = "Which file format to output. Inferred from file extension if not provided. Required if using --output-directory."
    )]
    pub format: Option<ExportFormat>,

    #[arg(
        long,
        help = "Choose which itemizations to export when using a single output file (e.g., a single CSV with all contributions or disbursements). Required when exporting to a single CSV or JSON file. "
    )]
    pub target: Option<ExportTarget>,

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
    #[arg(
        long,
        requires = "cycle",
        help = "Include all bulk data (candidates, committees, etc.) for the specified cycle"
    )]
    pub include_all_bulk: bool,

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

    /// Override today's date (YYYY-MM-DD). Useful for testing and debugging.
    #[arg(long)]
    pub as_of: Option<String>,
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

    /// Filter by committee ID(s), comma-separated (e.g., C00505412,C00513531) or a .txt file of IDs
    #[arg(long, short = 'c')]
    pub committee: Option<String>,

    /// Display label for committee filter (set automatically when a .txt file is used)
    #[arg(skip)]
    pub committee_label: Option<String>,

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

    /// Include all bulk candidate/committee data for the latest cycle before the first export (requires --export)
    #[arg(long, requires = "export")]
    pub include_all_bulk: bool,
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
    pub output: Option<PathBuf>,
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
    #[value(
        help = "Committee master file (\x1b]8;;https://www.fec.gov/campaign-finance-data/committee-master-file-description/\x1b\\\x1b[34mdocs\x1b[0m\x1b]8;;\x1b\\)"
    )]
    Committees,
    #[value(
        help = "Candidate master file (\x1b]8;;https://www.fec.gov/campaign-finance-data/candidate-master-file-description/\x1b\\\x1b[34mdocs\x1b[0m\x1b]8;;\x1b\\)"
    )]
    Candidates,

    #[value(
        help = "Operating expenditures (\x1b]8;;https://www.fec.gov/campaign-finance-data/operating-expenditures-file-description/\x1b\\\x1b[34mdocs\x1b[0m\x1b]8;;\x1b\\)"
    )]
    Opex,
    #[value(
        help = "Independent expenditures (\x1b]8;;https://www.fec.gov/campaign-finance-data/independent-expenditures-file-description/\x1b\\\x1b[34mdocs\x1b[0m\x1b]8;;\x1b\\)"
    )]
    IndependentExpenditures,

    #[value(
        help = "Contributions from committees to candidates (\x1b]8;;https://www.fec.gov/campaign-finance-data/contributions-committees-candidates-file-description/\x1b\\\x1b[34mdocs\x1b[0m\x1b]8;;\x1b\\)"
    )]
    ContributionsToCandidates,
    #[value(
        help = "PAC and party summary (\x1b]8;;https://www.fec.gov/campaign-finance-data/pac-and-party-summary-file-description/\x1b\\\x1b[34mdocs\x1b[0m\x1b]8;;\x1b\\)"
    )]
    PacSummary,
    #[value(
        help = "Candidate summary - weball (\x1b]8;;https://www.fec.gov/campaign-finance-data/all-candidates-file-description/\x1b\\\x1b[34mdocs\x1b[0m\x1b]8;;\x1b\\)"
    )]
    CandidateSummary,
    #[value(
        help = "Form 1 statement of organization filers (\x1b]8;;https://www.fec.gov/campaign-finance-data/new-committee-registrations-file-description/\x1b\\\x1b[34mdocs\x1b[0m\x1b]8;;\x1b\\)"
    )]
    Form1Filers,
    #[value(
        help = "Form 2 statement of candidacy filers (\x1b]8;;https://www.fec.gov/campaign-finance-data/new-statements-candidacy-file-description/\x1b\\\x1b[34mdocs\x1b[0m\x1b]8;;\x1b\\)"
    )]
    Form2Filers,
    #[value(
        help = "Candidate summary - CSV (\x1b]8;;https://www.fec.gov/campaign-finance-data/candidate-summary-file-description/\x1b\\\x1b[34mdocs\x1b[0m\x1b]8;;\x1b\\)"
    )]
    CandidateSummaryCsv,
    #[value(
        help = "Committee summary - CSV (\x1b]8;;https://www.fec.gov/campaign-finance-data/committee-summary-file-description/\x1b\\\x1b[34mdocs\x1b[0m\x1b]8;;\x1b\\)"
    )]
    CommitteeSummaryCsv,
    #[value(
        help = "Contributions by individuals (\x1b]8;;https://www.fec.gov/campaign-finance-data/contributions-individuals-file-description/\x1b\\\x1b[34mdocs\x1b[0m\x1b]8;;\x1b\\)"
    )]
    IndividualContributions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum ApiFormat {
    /// JSON array (default, loads all results into memory)
    #[default]
    Json,
    /// Newline-delimited JSON (one object per line, streamed)
    #[value(alias = "ndjson")]
    Jsonl,
}

#[derive(Subcommand, Debug)]
#[command(after_help = r#"Examples:
  libfec api filings C00401224 --cycle 2026
  libfec api schedule-a --committee C00401224 --min-amount 1000
  libfec api schedule-b --committee C00401224 --min-date 2025-01-01
  libfec api schedule-e --candidate P80001571 --cycle 2024"#)]
pub enum ApiSubcommand {
    /// Query the /v1/filings endpoint
    Filings(Box<ApiFilingsArgs>),
    /// Query /v1/schedules/schedule_a/ — individual contributions (receipts)
    ScheduleA(ApiScheduleAArgs),
    /// Query /v1/schedules/schedule_b/ — disbursements
    ScheduleB(ApiScheduleBArgs),
    /// Query /v1/schedules/schedule_c/ — loans
    ScheduleC(ApiScheduleCArgs),
    /// Query /v1/schedules/schedule_d/ — debts & obligations
    ScheduleD(ApiScheduleDArgs),
    /// Query /v1/schedules/schedule_e/ — independent expenditures
    ScheduleE(ApiScheduleEArgs),
    /// Query /v1/schedules/schedule_f/ — coordinated expenditures
    ScheduleF(ApiScheduleFArgs),
}

#[derive(Args, Debug)]
pub struct ApiArgs {
    #[command(subcommand)]
    pub command: ApiSubcommand,
}

#[derive(Args, Debug)]
pub struct ApiFilingsArgs {
    /// Committee IDs, candidate IDs, or contest shorthand (e.g. C00257337, H8VA07024, H-VA07)
    #[arg(required = false)]
    pub inputs: Vec<String>,

    /// Only output filing IDs, one per line
    #[arg(long)]
    pub filing_ids_only: bool,

    /// Output format
    #[arg(long, short = 'f', value_enum, default_value = "json")]
    pub format: ApiFormat,

    #[command(flatten)]
    pub api: FilingsApiFlags,
}

#[derive(Args, Debug)]
pub struct ApiScheduleAArgs {
    /// Committee ID(s)
    #[arg(long)]
    pub committee: Option<Vec<String>>,
    /// Contributor name(s)
    #[arg(long)]
    pub contributor_name: Option<Vec<String>>,
    /// Contributor city
    #[arg(long)]
    pub contributor_city: Option<Vec<String>>,
    /// Contributor state (2-letter code)
    #[arg(long)]
    pub contributor_state: Option<Vec<String>>,
    /// Contributor ZIP code
    #[arg(long)]
    pub contributor_zip: Option<Vec<String>>,
    /// Contributor employer
    #[arg(long)]
    pub contributor_employer: Option<Vec<String>>,
    /// Contributor occupation
    #[arg(long)]
    pub contributor_occupation: Option<Vec<String>>,
    /// Minimum contribution amount
    #[arg(long)]
    pub min_amount: Option<f64>,
    /// Maximum contribution amount
    #[arg(long)]
    pub max_amount: Option<f64>,
    /// Minimum contribution date (YYYY-MM-DD)
    #[arg(long)]
    pub min_date: Option<String>,
    /// Maximum contribution date (YYYY-MM-DD)
    #[arg(long)]
    pub max_date: Option<String>,
    /// Only individual contributions
    #[arg(long)]
    pub is_individual: bool,
    /// Schedule A line number
    #[arg(long)]
    pub line_number: Option<String>,
    /// Two-year transaction period(s)
    #[arg(long)]
    pub two_year_transaction_period: Option<Vec<u16>>,
    /// Sort field
    #[arg(long)]
    pub sort: Option<String>,

    /// Output format
    #[arg(long, short = 'f', value_enum, default_value = "json")]
    pub format: ApiFormat,
    /// API key for OpenFEC
    #[arg(long, env = "LIBFEC_API_KEY", hide_env = true)]
    pub api_key: Option<String>,
    /// Results per page (max 100)
    #[arg(long, default_value = "100")]
    pub per_page: u16,
    /// Fetch all pages automatically
    #[arg(long)]
    pub all: bool,
    /// Print the API URL that would be requested, without making the request
    #[arg(long)]
    pub url_only: bool,
}

#[derive(Args, Debug)]
pub struct ApiScheduleBArgs {
    /// Committee ID(s)
    #[arg(long)]
    pub committee: Option<Vec<String>>,
    /// Recipient name
    #[arg(long)]
    pub recipient_name: Option<Vec<String>>,
    /// Recipient city
    #[arg(long)]
    pub recipient_city: Option<Vec<String>>,
    /// Recipient state (2-letter code)
    #[arg(long)]
    pub recipient_state: Option<Vec<String>>,
    /// Disbursement purpose category
    #[arg(long)]
    pub disbursement_purpose_category: Option<Vec<String>>,
    /// Minimum disbursement amount
    #[arg(long)]
    pub min_amount: Option<f64>,
    /// Maximum disbursement amount
    #[arg(long)]
    pub max_amount: Option<f64>,
    /// Minimum disbursement date (YYYY-MM-DD)
    #[arg(long)]
    pub min_date: Option<String>,
    /// Maximum disbursement date (YYYY-MM-DD)
    #[arg(long)]
    pub max_date: Option<String>,
    /// Schedule B line number
    #[arg(long)]
    pub line_number: Option<String>,
    /// Two-year transaction period(s)
    #[arg(long)]
    pub two_year_transaction_period: Option<Vec<u16>>,
    /// Sort field
    #[arg(long)]
    pub sort: Option<String>,

    /// Output format
    #[arg(long, short = 'f', value_enum, default_value = "json")]
    pub format: ApiFormat,
    /// API key for OpenFEC
    #[arg(long, env = "LIBFEC_API_KEY", hide_env = true)]
    pub api_key: Option<String>,
    /// Results per page (max 100)
    #[arg(long, default_value = "100")]
    pub per_page: u16,
    /// Fetch all pages automatically
    #[arg(long)]
    pub all: bool,
    /// Print the API URL that would be requested, without making the request
    #[arg(long)]
    pub url_only: bool,
}

#[derive(Args, Debug)]
pub struct ApiScheduleCArgs {
    /// Committee ID(s)
    #[arg(long)]
    pub committee: Option<Vec<String>>,
    /// Candidate ID(s)
    #[arg(long)]
    pub candidate: Option<Vec<String>>,
    /// Loan source name
    #[arg(long)]
    pub loan_source_name: Option<Vec<String>>,
    /// Minimum loan amount
    #[arg(long)]
    pub min_amount: Option<f64>,
    /// Maximum loan amount
    #[arg(long)]
    pub max_amount: Option<f64>,
    /// Minimum incurred date (YYYY-MM-DD)
    #[arg(long)]
    pub min_date: Option<String>,
    /// Maximum incurred date (YYYY-MM-DD)
    #[arg(long)]
    pub max_date: Option<String>,
    /// Sort field
    #[arg(long)]
    pub sort: Option<String>,

    /// Output format
    #[arg(long, short = 'f', value_enum, default_value = "json")]
    pub format: ApiFormat,
    /// API key for OpenFEC
    #[arg(long, env = "LIBFEC_API_KEY", hide_env = true)]
    pub api_key: Option<String>,
    /// Results per page (max 100)
    #[arg(long, default_value = "100")]
    pub per_page: u16,
    /// Fetch all pages automatically
    #[arg(long)]
    pub all: bool,
    /// Print the API URL that would be requested, without making the request
    #[arg(long)]
    pub url_only: bool,
}

#[derive(Args, Debug)]
pub struct ApiScheduleDArgs {
    /// Committee ID(s)
    #[arg(long)]
    pub committee: Option<Vec<String>>,
    /// Candidate ID(s)
    #[arg(long)]
    pub candidate: Option<Vec<String>>,
    /// Creditor/debtor name
    #[arg(long)]
    pub creditor_debtor_name: Option<Vec<String>>,
    /// Minimum amount
    #[arg(long)]
    pub min_amount: Option<f64>,
    /// Maximum amount
    #[arg(long)]
    pub max_amount: Option<f64>,
    /// Minimum coverage end date (YYYY-MM-DD)
    #[arg(long)]
    pub min_date: Option<String>,
    /// Maximum coverage end date (YYYY-MM-DD)
    #[arg(long)]
    pub max_date: Option<String>,
    /// Sort field
    #[arg(long)]
    pub sort: Option<String>,

    /// Output format
    #[arg(long, short = 'f', value_enum, default_value = "json")]
    pub format: ApiFormat,
    /// API key for OpenFEC
    #[arg(long, env = "LIBFEC_API_KEY", hide_env = true)]
    pub api_key: Option<String>,
    /// Results per page (max 100)
    #[arg(long, default_value = "100")]
    pub per_page: u16,
    /// Fetch all pages automatically
    #[arg(long)]
    pub all: bool,
    /// Print the API URL that would be requested, without making the request
    #[arg(long)]
    pub url_only: bool,
}

#[derive(Args, Debug)]
pub struct ApiScheduleEArgs {
    /// Committee ID(s)
    #[arg(long)]
    pub committee: Option<Vec<String>>,
    /// Candidate ID(s)
    #[arg(long)]
    pub candidate: Option<Vec<String>>,
    /// Support or oppose (S or O)
    #[arg(long)]
    pub support_oppose_indicator: Option<String>,
    /// Is notice (filed within 24/48 hours of expenditure)
    #[arg(long)]
    pub is_notice: Option<bool>,
    /// Filing form (F24 or F3X)
    #[arg(long)]
    pub filing_form: Option<Vec<String>>,
    /// Minimum expenditure amount
    #[arg(long)]
    pub min_amount: Option<f64>,
    /// Maximum expenditure amount
    #[arg(long)]
    pub max_amount: Option<f64>,
    /// Minimum expenditure date (YYYY-MM-DD)
    #[arg(long)]
    pub min_date: Option<String>,
    /// Maximum expenditure date (YYYY-MM-DD)
    #[arg(long)]
    pub max_date: Option<String>,
    /// Election cycle(s)
    #[arg(long)]
    pub cycle: Option<Vec<u16>>,
    /// Most recent filing only
    #[arg(long)]
    pub most_recent: Option<bool>,
    /// Sort field
    #[arg(long)]
    pub sort: Option<String>,

    /// Output format
    #[arg(long, short = 'f', value_enum, default_value = "json")]
    pub format: ApiFormat,
    /// API key for OpenFEC
    #[arg(long, env = "LIBFEC_API_KEY", hide_env = true)]
    pub api_key: Option<String>,
    /// Results per page (max 100)
    #[arg(long, default_value = "100")]
    pub per_page: u16,
    /// Fetch all pages automatically
    #[arg(long)]
    pub all: bool,
    /// Print the API URL that would be requested, without making the request
    #[arg(long)]
    pub url_only: bool,
}

#[derive(Args, Debug)]
pub struct ApiScheduleFArgs {
    /// Committee ID(s)
    #[arg(long)]
    pub committee: Option<Vec<String>>,
    /// Candidate ID(s)
    #[arg(long)]
    pub candidate: Option<Vec<String>>,
    /// Payee name
    #[arg(long)]
    pub payee_name: Option<Vec<String>>,
    /// Minimum expenditure amount
    #[arg(long)]
    pub min_amount: Option<f64>,
    /// Maximum expenditure amount
    #[arg(long)]
    pub max_amount: Option<f64>,
    /// Minimum expenditure date (YYYY-MM-DD)
    #[arg(long)]
    pub min_date: Option<String>,
    /// Maximum expenditure date (YYYY-MM-DD)
    #[arg(long)]
    pub max_date: Option<String>,
    /// Election cycle(s)
    #[arg(long)]
    pub cycle: Option<Vec<u16>>,
    /// Sort field
    #[arg(long)]
    pub sort: Option<String>,

    /// Output format
    #[arg(long, short = 'f', value_enum, default_value = "json")]
    pub format: ApiFormat,
    /// API key for OpenFEC
    #[arg(long, env = "LIBFEC_API_KEY", hide_env = true)]
    pub api_key: Option<String>,
    /// Results per page (max 100)
    #[arg(long, default_value = "100")]
    pub per_page: u16,
    /// Fetch all pages automatically
    #[arg(long)]
    pub all: bool,
    /// Print the API URL that would be requested, without making the request
    #[arg(long)]
    pub url_only: bool,
}

#[derive(Args, Debug)]
pub struct DatasetteArgs {
    /// FEC filing IDs, committee IDs, contest shorthand, or path to .db file
    #[arg(required = true)]
    pub inputs: Vec<String>,

    /// Starting port (increments if in use)
    #[arg(long, short = 'p', default_value_t = 8888)]
    pub port: u16,

    /// Only export cover records
    #[arg(long)]
    pub cover_only: bool,

    #[command(flatten)]
    pub api: FilingsApiFlags,
}

#[derive(Parser, Debug)]
pub struct SchemaizeArgs {
    /// Path to the SQLite database to create tables in
    pub path: PathBuf,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Export data from FEC filings into SQLite, CSV, or JSON
    Export(Box<ExportArgs>),

    /// Search candidates and committees
    Search(SearchArgs),

    /// Export bulk datasets from fec.gov
    Bulk(BulkArgs),

    /// Print debug information about a FEC filing, committee, or candidate
    Info(InfoArgs),

    /// Continuously watch the official FEC RSS feeds for new filings
    Rss(RssArgs),

    /// View upcoming FEC calendar dates (elections, deadlines, meetings)
    Dates(DatesArgs),

    /// Explicitly cache .fec files from fec.gov to your filesystem
    Cache(CacheArgs),

    /// FastFEC compatible export
    Fastfec(FastFecArgs),

    /// Query the FEC API and print raw JSON responses
    Api(ApiArgs),

    /// Use Datasette for instant SQLite database browsing and visualizations
    Datasette(Box<DatasetteArgs>),

    /// Create all possible libfec tables in a SQLite database (schema only, no data)
    #[command(hide = true)]
    Schemaize(SchemaizeArgs),
}

#[derive(Parser)]
pub struct TopLevelArgs {
    // TODO: api-key, api-url
    #[arg(
        global = true,
        long,
        env = "LIBFEC_CACHE_DIRECTORY",
        help_heading = "Global options",
        help = "Directory to use for caching .fec files downloaded from the FEC API. Can also be set via the LIBFEC_CACHE_DIRECTORY environment variable."
    )]
    pub cache_directory: Option<PathBuf>,

    #[arg(
        global = true,
        long,
        env = "LIBFEC_OFFLINE",
        help_heading = "Global options",
        help = "Run without making any network requests. Uses cached data only."
    )]
    pub offline: bool,
}

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(
  name = "libfec",
  author,
  long_version = env!("CARGO_PKG_VERSION"),
  about = "A federal campaign finance data toolkit",
  version,
  subcommand_required = true,
  styles = STYLES,
  after_long_help = "\x1b[1;32mExamples:\x1b[0m

  Set the LIBFEC_API_KEY environment variable for higher API rate limits:
  export LIBFEC_API_KEY=your_api_key_here

  Export filings to SQLite:

    libfec export FEC-1949543 -o filing.db
    libfec export C00835959 --cycle 2026 -o fairshake.db
    libfec export TX-S --cycle 2026 -o texas-senate.db
    libfec export CA40 --cycle 2026 -o california-house-40.db

  Query the FEC API:

    libfec api filings C00401224 --cycle 2026
    libfec api schedule-a --committee C00401224 --min-amount 1000
    libfec api schedule-b --committee C00401224 --min-date 2025-01-01
    libfec api schedule-e --candidate P80001571 --cycle 2024

  Watch the RSS feed:

    libfec rss --watch --preset monthly --state CA --export monthly_filings.db

  Other:
  
    libfec dates --category elections,deadlines --state TX --format json
    libfec cache add FEC-1949543 C00401224 --cycle 2026
    libfec datasette FEC-1949543 -p 9000
    libfec bulk --source candidates,committees --cycle 2024-2026 -o bulk_data.db
"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Box<Commands>,

    #[command(flatten)]
    pub top_level: TopLevelArgs,
}
