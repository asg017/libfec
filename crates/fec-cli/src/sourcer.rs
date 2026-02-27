use anyhow::{Context, Error, Result};
use fec_api::{CandidateId, CommitteeId, Office};
use fec_parser::{Filing, FilingRow};
use indicatif::{HumanBytes, MultiProgress, ProgressBar, ProgressStyle};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
    sync::LazyLock,
};
use url::Url;

use crate::{
    api_flags::Trace,
    cache::{
        bulk::candidates::{ResolveCandidateParams, ResolveCandidateParamsBuilder},
        Cache,
    },
    cli::FilingsApiFlags,
};

pub struct FilingSourcer {
    pub cache: Cache,
}

struct ResolvedFiling {
    reader: Box<dyn Read>,
    filing_id: String,
    source_length: usize,
}

#[derive(Debug)]
pub struct FecGov403Error {
    pub filing_id: FecFilingId,
    pub url: String,
}

impl std::fmt::Display for FecGov403Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Access to {} was forbidden (HTTP 403)", self.url)
    }
}

impl std::error::Error for FecGov403Error {}

fn resolve_from_url(url: &Url) -> Result<ResolvedFiling> {
    let request = ureq::get(url.as_str());
    let response = match request.call() {
        Ok(resp) => resp,
        Err(error) => {
            // wrap 403 errors from docquery.fec.gov as a FecGov403Error, for clients to detect and warn

            if matches!(error, ureq::Error::StatusCode(403))
                && url
                    .host()
                    .is_some_and(|h| h.to_string() == "docquery.fec.gov")
            {
                let filing_id = Path::new(url.path())
                    .file_stem()
                    .map(|os_str| os_str.to_string_lossy().to_string())
                    .unwrap_or_default();
                return Err(Error::new(FecGov403Error {
                    filing_id: FecFilingId::from_str(&filing_id).unwrap_or(FecFilingId(0)),
                    url: url.as_str().to_string(),
                }));
            }
            return Err(Error::msg(format!(
                "Failed to fetch URL {}: HTTP status code {}",
                url.as_str(),
                error
            )));
        }
    };
    let filing_id = Path::new(url.path())
        .file_stem()
        .map(|os_str| os_str.to_string_lossy().to_string())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract filing ID from URL: {}", url.as_str()))?;
    let source_length: usize = response
        .headers()
        .get("Content-Length")
        .ok_or_else(|| anyhow::anyhow!("No Content-Length header found in response",))?
        .to_str()
        .map_err(|_| anyhow::anyhow!("Content-Length header is not valid UTF-8"))?
        .parse()
        .map_err(|_| anyhow::anyhow!("Content-Length header is not a valid number"))?;
    let r = response.into_parts().1.into_reader();
    Ok(ResolvedFiling {
        reader: Box::new(r),
        filing_id,
        source_length,
    })
}

fn resolve_filing_from_url(url: &Url) -> Result<Filing<Box<dyn Read>>> {
    let request = ureq::get(url.as_str());
    let response = match request.call() {
        Ok(resp) => resp,
        Err(error) => {
            // wrap 403 errors from docquery.fec.gov as a FecGov403Error, for clients to detect and warn

            if matches!(error, ureq::Error::StatusCode(403))
                && url
                    .host()
                    .is_some_and(|h| h.to_string() == "docquery.fec.gov")
            {
                let filing_id = Path::new(url.path())
                    .file_stem()
                    .map(|os_str| os_str.to_string_lossy().to_string())
                    .unwrap_or_default();
                return Err(Error::new(FecGov403Error {
                    filing_id: FecFilingId::from_str(&filing_id).unwrap_or(FecFilingId(0)),
                    url: url.as_str().to_string(),
                }));
            }
            return Err(Error::msg(format!(
                "Failed to fetch URL {}: HTTP status code {:?}",
                url.as_str(),
                error
            )));
        }
    };
    let filing_id = Path::new(url.path())
        .file_stem()
        .map(|os_str| os_str.to_string_lossy().to_string())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract filing ID from URL: {}", url.as_str()))?;
    let source_length: usize = response
        .headers()
        .get("Content-Length")
        .ok_or_else(|| anyhow::anyhow!("No Content-Length header found in response",))?
        .to_str()
        .map_err(|_| anyhow::anyhow!("Content-Length header is not valid UTF-8"))?
        .parse()
        .map_err(|_| anyhow::anyhow!("Content-Length header is not a valid number"))?;
    let r = response.into_parts().1.into_reader();
    Filing::from_reader(Box::new(r), filing_id, source_length)
}

fn resolve_from_file(f: File, path: PathBuf) -> Result<ResolvedFiling> {
    let filing_id = path
        .file_stem()
        .map(|os_str| os_str.to_string_lossy().to_string())
        .unwrap_or_default();
    let source_length = f.metadata().map(|v| v.len() as usize)?;
    Ok(ResolvedFiling {
        reader: Box::new(f),
        filing_id,
        source_length,
    })
}
pub(crate) fn resolve_from_path(path: PathBuf) -> Result<Filing<Box<dyn Read>>> {
    let f: File = File::open(&path).context(format!(
        "Could not open filing at path `{}`",
        path.display()
    ))?;
    let filing_id = path
        .file_stem()
        .map(|os_str| os_str.to_string_lossy().to_string())
        .unwrap_or_default();
    let source_length = f.metadata().map(|v| v.len() as usize).context(format!(
        "Could not determine file size for filing at path: {}",
        path.display()
    ))?;
    Filing::from_reader(Box::new(f), filing_id, source_length)
    /*Ok(ResolvedFiling {
        reader: Box::new(f),
        filing_id,
        source_length,
    })*/
}

fn resolve_from_filing_id(filing_id: &str) -> Result<ResolvedFiling> {
    let filing_id = filing_id
        .strip_prefix("FEC-")
        .or_else(|| filing_id.strip_prefix("FEC"))
        .unwrap_or(filing_id)
        .to_owned();

    let url = format!("https://docquery.fec.gov/dcdev/posted/{filing_id}.fec");
    resolve_from_url(&Url::from_str(&url)?)
}

#[derive(Debug, Clone)]
pub(crate) struct FecFilingId(u32);

impl FecFilingId {
    pub fn to_human_readable(&self) -> String {
        format!("FEC-{}", self.0)
    }
    pub fn to_bare(&self) -> String {
        format!("{}", self.0)
    }
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        let id = s.strip_prefix("FEC-").unwrap_or(s);
        let id = id.parse::<u32>()?;
        Ok(FecFilingId(id))
    }

    pub fn filing_url(&self) -> Url {
        Url::from_str(&format!(
            "https://docquery.fec.gov/dcdev/posted/{}.fec",
            self.to_bare()
        ))
        .expect("valid URL")
    }
}

static FILINGS_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    indicatif::ProgressStyle::with_template(
        "{pos}/{len} filings {bar:30.cyan/blue} [{elapsed_precise}]",
    )
    .expect("valid progress style")
});
static ITEMIZATION_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::with_template(
    "└─ {msg} {bar:40.cyan/blue} {elapsed} [{eta} ETA], {decimal_bytes_per_sec} [{decimal_total_bytes} total]",
  ).expect("valid progress style")
});

pub struct ItemizationProgressBar {
    pb: ProgressBar,
}
impl ItemizationProgressBar {
    pub fn new(mb: &MultiProgress, filing: &Filing<Box<dyn Read>>) -> Self {
        let pb = mb.add(ProgressBar::new(filing.source_length as u64));
        pb.set_style(ITEMIZATION_STYLE.clone());
        pb.set_message(format!("FEC-{}", filing.filing_id));
        Self { pb }
    }
    pub fn update(&self, row: &FilingRow) {
        if let Some(position) = row.record.position() {
            self.pb.set_position(position.byte());
        }
    }
}

impl Drop for ItemizationProgressBar {
    fn drop(&mut self) {
        self.pb.finish_and_clear();
    }
}

#[derive(Debug, Clone)]
pub enum Item {
    CachedFile(PathBuf),
    File(PathBuf),
    CustomUrl(Url),
    FilingId(FecFilingId),
}

/// Classification of an input for metadata tracking (mirrors UserArgument but without heavy data)
#[derive(Debug, Clone)]
pub enum InputType {
    /// A direct filing ID or .fec file
    Filing,
    /// A committee ID (e.g., C00401224)
    Committee,
    /// A candidate ID (e.g., P00009423)
    Candidate,
    /// A contest specification (e.g., P, S-CA, H-CA12)
    Contest {
        cycle: u16,
        office: Option<String>,
        state: Option<String>,
        district: Option<String>,
    },
    /// A URL to a .fec file
    Url,
    /// An input file containing other inputs
    InputFile,
}

/// Tracks an input and what filings it directly resolved to
#[derive(Debug, Clone)]
pub struct InputMapping {
    /// The raw input string as provided by the user
    pub raw_input: String,
    /// The classified type of this input
    pub input_type: InputType,
    /// Filing IDs that this input directly resolved to.
    /// For Filing/Url types: contains the single resolved filing.
    /// For Committee/Candidate/Contest types: empty (resolved via API in bulk).
    /// For InputFile: empty (contents are tracked separately).
    pub direct_filings: Vec<String>,
}

pub(crate) struct ProcessedInputs {
    pub trace: Trace,
    pub queue: Vec<Item>,
    /// Mappings from inputs to their classifications and resolved filings
    pub input_mappings: Vec<InputMapping>,
}
pub(crate) fn process_inputs(
    input: &Vec<String>,
    mut api_flags: FilingsApiFlags,
    sourcer: &mut FilingSourcer,
    mb: Option<&MultiProgress>,
) -> anyhow::Result<ProcessedInputs> {
    let mut queue = vec![];
    let mut trace = Trace {
        resolve_candidate_params: vec![],
    };
    let mut input_mappings = vec![];

    // process positional user arguments, which should resolve to a UserArgument
    for item in input {
        match sourcer.resolve_user_argument(item) {
            Err(error) => todo!("{}", error),
            Ok(UserArgument::Filing(resolved_item)) => {
                let filing_id = match &resolved_item {
                    Item::FilingId(id) => id.to_bare(),
                    Item::CachedFile(p) | Item::File(p) => p
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    Item::CustomUrl(url) => url
                        .path_segments()
                        .and_then(|mut s| s.next_back())
                        .map(|s| s.trim_end_matches(".fec").to_string())
                        .unwrap_or_default(),
                };
                input_mappings.push(InputMapping {
                    raw_input: item.clone(),
                    input_type: if item.starts_with("http://") || item.starts_with("https://") {
                        InputType::Url
                    } else {
                        InputType::Filing
                    },
                    direct_filings: vec![filing_id],
                });
                queue.push(resolved_item);
            }
            Ok(UserArgument::Committee(committee)) => {
                input_mappings.push(InputMapping {
                    raw_input: item.clone(),
                    input_type: InputType::Committee,
                    direct_filings: vec![], // resolved via API
                });
                // chuck committee ids into api_flags under --committee
                api_flags
                    .committee
                    .get_or_insert_with(Vec::new)
                    .push(committee);
            }

            Ok(UserArgument::Candidate(candidate)) => {
                input_mappings.push(InputMapping {
                    raw_input: item.clone(),
                    input_type: InputType::Candidate,
                    direct_filings: vec![], // resolved via API
                });
                // chuck candidate ids into api_flags under --candidate
                api_flags
                    .candidate
                    .get_or_insert_with(Vec::new)
                    .push(candidate);
            }
            Ok(UserArgument::Contest(contest)) => {
                let cycle = api_flags
                    .election
                    .or_else(|| api_flags.cycle.as_ref().and_then(|c| c.first().copied()))
                    .ok_or_else(|| anyhow::anyhow!("--election is required for contest inputs (e.g. --election 2026)"))?;
                let params = contest.resolve_candidate_params(cycle);
                input_mappings.push(InputMapping {
                    raw_input: item.clone(),
                    input_type: InputType::Contest {
                        cycle,
                        office: params.office.as_ref().map(|o| format!("{:?}", o)),
                        state: params.state.clone(),
                        district: params.district.clone(),
                    },
                    direct_filings: vec![], // resolved via API
                });
                trace.resolve_candidate_params.push(params.clone());
                let committee_strings = sourcer
                    .cache
                    .resolve_candidate_principal_campaign_committees(params)
                    .unwrap();
                api_flags
                    .committee
                    .get_or_insert_with(Vec::new)
                    .extend(committee_strings.into_iter().map(|s| CommitteeId::new(&s).unwrap()));
            }
            Ok(UserArgument::InputFile(path)) => {
                input_mappings.push(InputMapping {
                    raw_input: item.clone(),
                    input_type: InputType::InputFile,
                    direct_filings: vec![], // contents tracked separately
                });
                let contents = std::fs::read_to_string(&path)
                    .context(format!("Could not read input file `{}`", path.display()))?;
                let spinner = mb.as_ref().map(|mb| mb.add(ProgressBar::new_spinner()));

                for (idx, line) in contents.lines().enumerate() {
                    if line.trim().is_empty() || line.trim_start().starts_with('#') {
                        continue;
                    }
                    let line_item = line.trim();
                    if let Ok(id) = FecFilingId::from_str(line_item) {
                        input_mappings.push(InputMapping {
                            raw_input: line_item.to_string(),
                            input_type: InputType::Filing,
                            direct_filings: vec![id.to_bare()],
                        });
                        queue.push(Item::FilingId(id));
                    } else if let Ok(url) = Url::parse(line_item) {
                        let filing_id = url
                            .path_segments()
                            .and_then(|mut s| s.next_back())
                            .map(|s| s.trim_end_matches(".fec").to_string())
                            .unwrap_or_default();
                        input_mappings.push(InputMapping {
                            raw_input: line_item.to_string(),
                            input_type: InputType::Url,
                            direct_filings: vec![filing_id],
                        });
                        queue.push(Item::CustomUrl(url));
                    } else if let Ok(committee_id) = CommitteeId::from_str(line_item) {
                        input_mappings.push(InputMapping {
                            raw_input: line_item.to_string(),
                            input_type: InputType::Committee,
                            direct_filings: vec![],
                        });
                        api_flags
                            .committee
                            .get_or_insert_with(Vec::new)
                            .push(committee_id);
                    } else if let Ok(candidate_id) = CandidateId::from_str(line_item) {
                        input_mappings.push(InputMapping {
                            raw_input: line_item.to_string(),
                            input_type: InputType::Candidate,
                            direct_filings: vec![],
                        });
                        api_flags
                            .candidate
                            .get_or_insert_with(Vec::new)
                            .push(candidate_id);
                    } else if let Some(contest) = Contest::from_arg(line_item).unwrap() {
                        if let Some(s) = spinner.as_ref() {
                            s.set_message(format!("Resolving {}...", line_item));
                        }
                        let cycle = api_flags
                            .election
                            .or_else(|| api_flags.cycle.as_ref().and_then(|c| c.first().copied()))
                            .ok_or_else(|| anyhow::anyhow!("--election is required for contest inputs (e.g. --election 2026)"))?;
                        let params = contest.resolve_candidate_params(cycle);
                        input_mappings.push(InputMapping {
                            raw_input: line_item.to_string(),
                            input_type: InputType::Contest {
                                cycle,
                                office: params.office.as_ref().map(|o| format!("{:?}", o)),
                                state: params.state.clone(),
                                district: params.district.clone(),
                            },
                            direct_filings: vec![],
                        });
                        trace.resolve_candidate_params.push(params.clone());
                        let committee_strings = sourcer
                            .cache
                            .resolve_candidate_principal_campaign_committees(params)
                            .unwrap();
                        api_flags
                            .committee
                            .get_or_insert_with(Vec::new)
                            .extend(committee_strings.into_iter().map(|s| CommitteeId::new(&s).unwrap()));
                    } else {
                        return Err(anyhow::anyhow!(
                            "Could not resolve input on line {} of file {}: {}",
                            idx + 1,
                            path.display(),
                            line_item
                        ));
                    }
                }
            }
        }
    }

    if api_flags.any_provided() {
        let ids = api_flags.resolve_ids(sourcer, mb, &mut trace)?;
        queue.extend(
            ids.into_iter()
                .map(|id| match sourcer.filing_cache_path(&id) {
                    Some(path) => Item::CachedFile(path),
                    None => Item::FilingId(id),
                }),
        );
    }
    if true {
        //api_flags.cache {
        let mut cacheable: Vec<FecFilingId> = Vec::new();

        // pop out all the bare Item::FilingId's items, to cache them
        queue.retain(|item| {
            if let Item::FilingId(filing_id) = item {
                cacheable.push(filing_id.clone());
                false // remove from queue
            } else {
                true // keep in queue
            }
        });

        let caching_result = sourcer.cache.cache_all(cacheable, mb).unwrap();

        if let Some(mb) = mb {
            let downloaded = caching_result.stats.number_downloaded;
            let skipped = caching_result.stats.number_preexisting
                + queue
                    .iter()
                    .filter(|item| matches!(item, Item::CachedFile(_)))
                    .count();

            let _ = mb.println(format!(
                "{} Cached {} filing(s) ({}), skipped {} already cached",
                "✓",
                downloaded,
                HumanBytes(caching_result.stats.downloaded_bytes as u64),
                skipped,
            ));
        }

        // re-add cached files to the queue, now as cached files
        queue.extend(caching_result.paths.into_iter().map(Item::CachedFile));
    }

    Ok(ProcessedInputs { trace, queue, input_mappings })
}

pub struct IterFilingsX<'a> {
    sourcer: &'a FilingSourcer,
    queue: Vec<Item>,
    filing_progress: Option<ProgressBar>,
}

impl<'a> IterFilingsX<'a> {
    pub fn new(
        filing_progress: Option<ProgressBar>,
        sourcer: &'a FilingSourcer,
        queue: Vec<Item>,
    ) -> Self {
        IterFilingsX {
            sourcer,
            queue,
            filing_progress,
        }
    }
}

impl<'a> Iterator for IterFilingsX<'a> {
    type Item = anyhow::Result<Filing<Box<dyn Read>>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pb) = &self.filing_progress {
            pb.inc(1);
        }
        match self.queue.pop()? {
            Item::File(path) => {
                if let Some(pb) = self.filing_progress.as_ref() {
                    pb.set_message(format!("{}", path.display()))
                }
                Some(resolve_from_path(path))
            }
            Item::CachedFile(path) => {
                if let Some(pb) = self.filing_progress.as_ref() {
                    pb.set_message(format!("{} [cached]", path.display()))
                }
                Some(resolve_from_path(path))
            }
            Item::CustomUrl(url) => {
                if let Some(pb) = self.filing_progress.as_ref() {
                    pb.set_message(format!("{}", url))
                }
                Some(resolve_filing_from_url(&url))
            }
            Item::FilingId(filing_id) => {
                if let Some(pb) = self.filing_progress.as_ref() {
                    pb.set_message(filing_id.to_human_readable())
                }
                Some(self.sourcer.resolve_from_fec_id(&filing_id))
            }
        }
    }
}

pub enum UserArgument {
    Filing(Item),
    Committee(CommitteeId),
    Candidate(CandidateId),
    Contest(Contest),
    InputFile(PathBuf),
}

#[derive(Debug)]
pub enum Contest {
    // ex "P" or "president"
    President,
    // ex "S-CA", "senate-CA", or "CA-S"
    Senate { state: String },
    // ex "H-CA12" or "house-CA12"
    House { state: String, district: String },
}

impl Contest {
    pub fn from_arg(input: &str) -> anyhow::Result<Option<Self>> {
        if input == "P" || input.to_lowercase() == "president" {
            return Ok(Some(Contest::President));
        }
        if input.starts_with("S-") || input.to_lowercase().starts_with("senate-") {
            let parts: Vec<&str> = input.split('-').collect();
            if parts.len() == 2 {
                return Ok(Some(Contest::Senate {
                    state: parts[1].to_uppercase(),
                }));
            }
            todo!("Invalid senate contest format");
        }
        if input.ends_with("-S") || input.to_lowercase().ends_with("-senate") {
            let parts: Vec<&str> = input.split('-').collect();
            if parts.len() == 2 {
                return Ok(Some(Contest::Senate {
                    state: parts[0].to_uppercase(),
                }));
            }
            todo!("Invalid senate contest format");
        }
        if input.starts_with("H-") || input.to_lowercase().starts_with("house-") {
            let parts: Vec<&str> = input.split('-').collect();
            if parts.len() == 2 {
                let state = &parts[1][0..2].to_uppercase();
                let district = &parts[1][2..];
                if district.parse::<u8>().is_ok() {
                    return Ok(Some(Contest::House {
                        state: state.to_string(),
                        district: district.to_string(),
                    }));
                }
            }
            todo!("Invalid house contest format");
        }
        if input.len() == 4 {
            let b = input.as_bytes();
            if b[0].is_ascii_alphabetic()
                && b[1].is_ascii_alphabetic()
                && b[2].is_ascii_digit()
                && b[3].is_ascii_digit()
            {
                return Ok(Some(Contest::House {
                    state: input[0..2].to_uppercase(),
                    district: input[2..4].to_string(),
                }));
            }
        }
        Ok(None)
    }
    pub fn resolve_candidate_params(&self, cycle: u16) -> ResolveCandidateParams {
        let mut b = ResolveCandidateParamsBuilder::default();
        b.cycle(cycle);
        match self {
            Contest::President => {
                b.office(Some(Office::President)).state(None).district(None);
            }
            Contest::Senate { state } => {
                b.office(Some(Office::Senate))
                    .state(Some(state.to_string()))
                    .district(None);
            }
            Contest::House { state, district } => {
                b.office(Some(Office::House))
                    .state(Some(state.to_string()))
                    .district(Some(district.clone()));
            }
        }

        b.build().unwrap()
    }
}


impl FilingSourcer {
    pub fn new(cli_cache_directory: Option<PathBuf>) -> Self {
        Self {
            cache: Cache::new(cli_cache_directory),
        }
    }

    pub fn resolve_iterator_from_flags(
        &'_ mut self,
        input: Vec<String>,
        api_flags: FilingsApiFlags,
        mb: Option<&MultiProgress>,
    ) -> anyhow::Result<(Trace, Vec<InputMapping>, IterFilingsX<'_>)> {
        let result = process_inputs(&input, api_flags, self, mb)?;
        let filing_progress = if let Some(mb) = mb {
            let pb = mb.add(ProgressBar::new(result.queue.len() as u64));
            pb.set_style(FILINGS_STYLE.clone());
            Some(pb)
        } else {
            None
        };
        Ok((
            result.trace,
            result.input_mappings,
            IterFilingsX::new(filing_progress, self, result.queue),
        ))
    }

    // Resolve a FEC filing from an "input" source such as:
    // 1. If it's a file, read it from the file system.
    // 2. If it's a URL, fetch it directly from the web.
    // 3. Check if it's in the cache directory
    // 4. If it's a FEC filing ID, construct the URL to docquery.fec.gov and fetch it.

    pub fn resolve_from_user_argument(&self, input: &str) -> anyhow::Result<Filing<Box<dyn Read>>> {
        let resolved_filing: ResolvedFiling = {
            // 1. if it's a file, read it
            if let Ok(file) = File::open(input) {
                resolve_from_file(file, PathBuf::from(input))?
            }
            // 2. if it's a URL, fetch it
            else if let Ok(url) = Url::parse(input) {
                resolve_from_url(&url)?
            }
            // 3. check to see if it's cached
            else if let Some(filing_path) =
                self.cache.resolve_filing(&FecFilingId::from_str(input)?)
            {
                match resolve_from_file(File::open(&filing_path).unwrap(), filing_path) {
                    Ok(filing) => filing,
                    Err(_) => todo!(),
                }
            } else {
                resolve_from_filing_id(input)?
            }
        };
        Filing::from_reader(
            resolved_filing.reader,
            resolved_filing.filing_id,
            resolved_filing.source_length,
        )
    }
    pub fn resolve_user_argument(&self, input: &str) -> anyhow::Result<UserArgument> {
        if input.ends_with(".fec") && File::open(input).is_ok() {
            return Ok(UserArgument::Filing(Item::File(PathBuf::from(input))));
        }
        if let Ok(url) = Url::parse(input) {
            return Ok(UserArgument::Filing(Item::CustomUrl(url)));
        }
        if let Ok(filing_id) = &FecFilingId::from_str(input) {
            match self.cache.resolve_filing(filing_id) {
                Some(filing_path) => {
                    return Ok(UserArgument::Filing(Item::CachedFile(filing_path)))
                }
                None => {
                    return Ok(UserArgument::Filing(Item::FilingId(filing_id.clone())));
                }
            }
        }

        if let Ok(id) = CommitteeId::from_str(input) {
            return Ok(UserArgument::Committee(id));
        }
        if let Ok(id) = CandidateId::from_str(input) {
            return Ok(UserArgument::Candidate(id));
        }
        if let Ok(Some(contest)) = Contest::from_arg(input) {
            return Ok(UserArgument::Contest(contest));
        }
        if File::open(input).is_ok() {
            return Ok(UserArgument::InputFile(PathBuf::from(input)));
        }

        Err(anyhow::anyhow!("Could not resolve input: {}", input))
    }

    pub fn filing_cache_path(&self, filing_id: &FecFilingId) -> Option<PathBuf> {
        self.cache.resolve_filing(filing_id)
    }

    fn resolve_from_fec_id(
        &self,
        filing_id: &FecFilingId,
    ) -> anyhow::Result<Filing<Box<dyn Read>>> {
        let resolved = if let Some(filing_path) = self.cache.resolve_filing(filing_id) {
            match resolve_from_file(File::open(&filing_path).unwrap(), filing_path) {
                Ok(filing) => filing,
                Err(_) => todo!(),
            }
        } else {
            resolve_from_filing_id(&filing_id.to_bare())?
        };
        Filing::from_reader(resolved.reader, resolved.filing_id, resolved.source_length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;

    #[test]
    fn test_contest_president_short() {
        assert_debug_snapshot!(Contest::from_arg("P").unwrap(), @r#"
        Some(
            President,
        )
        "#);
    }

    #[test]
    fn test_contest_president_long() {
        assert_debug_snapshot!(Contest::from_arg("president").unwrap(), @r#"
        Some(
            President,
        )
        "#);
    }

    #[test]
    fn test_contest_president_mixed_case() {
        assert_debug_snapshot!(Contest::from_arg("President").unwrap(), @r#"
        Some(
            President,
        )
        "#);
    }

    #[test]
    fn test_contest_senate_prefix() {
        assert_debug_snapshot!(Contest::from_arg("S-CA").unwrap(), @r#"
        Some(
            Senate {
                state: "CA",
            },
        )
        "#);
    }

    #[test]
    fn test_contest_senate_long_prefix() {
        assert_debug_snapshot!(Contest::from_arg("senate-CA").unwrap(), @r#"
        Some(
            Senate {
                state: "CA",
            },
        )
        "#);
    }

    #[test]
    fn test_contest_senate_suffix() {
        assert_debug_snapshot!(Contest::from_arg("CA-S").unwrap(), @r#"
        Some(
            Senate {
                state: "CA",
            },
        )
        "#);
    }

    #[test]
    fn test_contest_senate_long_suffix() {
        assert_debug_snapshot!(Contest::from_arg("CA-senate").unwrap(), @r#"
        Some(
            Senate {
                state: "CA",
            },
        )
        "#);
    }

    #[test]
    fn test_contest_senate_lowercase_state() {
        assert_debug_snapshot!(Contest::from_arg("S-ca").unwrap(), @r#"
        Some(
            Senate {
                state: "CA",
            },
        )
        "#);
    }

    #[test]
    fn test_contest_senate_suffix_lowercase() {
        assert_debug_snapshot!(Contest::from_arg("ca-S").unwrap(), @r#"
        Some(
            Senate {
                state: "CA",
            },
        )
        "#);
    }

    #[test]
    fn test_contest_house_prefix() {
        assert_debug_snapshot!(Contest::from_arg("H-CA12").unwrap(), @r#"
        Some(
            House {
                state: "CA",
                district: "12",
            },
        )
        "#);
    }

    #[test]
    fn test_contest_house_long_prefix() {
        assert_debug_snapshot!(Contest::from_arg("house-CA12").unwrap(), @r#"
        Some(
            House {
                state: "CA",
                district: "12",
            },
        )
        "#);
    }

    #[test]
    fn test_contest_house_short_form() {
        assert_debug_snapshot!(Contest::from_arg("CA12").unwrap(), @r#"
        Some(
            House {
                state: "CA",
                district: "12",
            },
        )
        "#);
    }

    #[test]
    fn test_contest_house_lowercase() {
        assert_debug_snapshot!(Contest::from_arg("H-ca12").unwrap(), @r#"
        Some(
            House {
                state: "CA",
                district: "12",
            },
        )
        "#);
    }

    #[test]
    fn test_contest_no_match() {
        assert_debug_snapshot!(Contest::from_arg("foobar").unwrap(), @"None");
    }
}
