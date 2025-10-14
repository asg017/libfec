use anyhow::{Context, Error, Result};
use fec_api::Office;
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
        bulk_candidates::{ResolveCandidateParams, ResolveCandidateParamsBuilder},
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
                error.to_string()
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
        filing_id: filing_id,
        source_length: source_length,
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
                "Failed to fetch URL {}: HTTP status code {}",
                url.as_str(),
                error.to_string()
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
fn resolve_from_path(path: PathBuf) -> Result<Filing<Box<dyn Read>>> {
    let f = File::open(&path).context(format!(
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

pub struct IterFilingsX<'a> {
    sourcer: &'a FilingSourcer,
    queue: Vec<Item>,
    filing_progress: Option<ProgressBar>,
}

impl<'a> IterFilingsX<'a> {
    pub fn new(
        sourcer: &'a mut FilingSourcer,
        input: &Vec<String>,
        mut api_flags: FilingsApiFlags,
        mb: Option<&MultiProgress>,
    ) -> anyhow::Result<(Trace, Self)> {
        let mut queue = vec![];
        let mut trace = Trace {
            resolve_candidate_params: vec![],
        };

        for item in input {
            match sourcer.resolve_user_argument(item) {
                Err(error) => todo!("{}", error),
                Ok(UserArgument::Filing(item)) => {
                    queue.push(item);
                }
                Ok(UserArgument::Committee(committee)) => {
                    api_flags
                        .committee
                        .get_or_insert_with(Vec::new)
                        .push(committee);
                }

                Ok(UserArgument::Candidate(candidate)) => {
                    api_flags
                        .candidate
                        .get_or_insert_with(Vec::new)
                        .push(candidate);
                }
                Ok(UserArgument::Contest(contest)) => {
                    let cycle = api_flags.election.clone().unwrap();
                    let params = contest.resolve_candidate_params(cycle);
                    trace.resolve_candidate_params.push(params.clone());
                    let committees = sourcer
                        .cache
                        .resolve_candidate_principal_campaign_committees(params)
                        .unwrap();
                    api_flags
                        .committee
                        .get_or_insert_with(Vec::new)
                        .extend(committees);
                }
                Ok(UserArgument::InputFile(path)) => {
                    let contents = std::fs::read_to_string(&path)
                        .context(format!("Could not read input file `{}`", path.display()))?;
                    let spinner = mb.as_ref().map(|mb| {
                        mb.add(ProgressBar::new_spinner())
                    });
                    
                    for (idx, line) in contents.lines().enumerate() {
                        if line.trim().is_empty() || line.trim_start().starts_with('#') {
                            continue;
                        }
                        let item = line.trim();
                        if let Ok(id) = FecFilingId::from_str(item) {
                            queue.push(Item::FilingId(id));
                        } else if let Ok(url) = Url::parse(item) {
                            queue.push(Item::CustomUrl(url));
                        } else if is_committee_input(item) {
                            api_flags
                                .committee
                                .get_or_insert_with(Vec::new)
                                .push(item.to_owned());
                        } else if is_candidate_input(item) {
                            api_flags
                                .candidate
                                .get_or_insert_with(Vec::new)
                                .push(item.to_owned());
                        } else if let Some(contest) = Contest::from_arg(item).unwrap() {
                            spinner.as_ref().map(|s| {
                                s.set_message(format!("Resolving {}...", item));
                            });
                            let cycle = api_flags.election.clone().unwrap();
                            let params = contest.resolve_candidate_params(cycle);
                            trace.resolve_candidate_params.push(params.clone());
                            let committees = sourcer
                                .cache
                                .resolve_candidate_principal_campaign_committees(params)
                                .unwrap();
                            api_flags
                                .committee
                                .get_or_insert_with(Vec::new)
                                .extend(committees);
                        } else {
                            return Err(anyhow::anyhow!(
                                "Could not resolve input on line {} of file {}: {}",
                                idx + 1,
                                path.display(),
                                item
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
                if caching_result.stats.number_downloaded > 0 {
                    let _ = mb.println(format!(
                        "{} Cached {} filings, {}",
                        "✓",
                        caching_result.stats.number_downloaded,
                        HumanBytes(caching_result.stats.downloaded_bytes as u64),
                    ));
                }
            }
            queue.extend(
                caching_result
                    .paths
                    .into_iter()
                    .map(|path| Item::CachedFile(path)),
            );
        }
        let filing_progress = if let Some(mb) = mb {
            let pb = mb.add(ProgressBar::new(queue.len() as u64));
            pb.set_style(FILINGS_STYLE.clone());
            Some(pb)
        } else {
            None
        };

        Ok((
            trace,
            IterFilingsX {
                sourcer,
                queue,
                filing_progress,
            },
        ))
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
                self.filing_progress
                    .as_ref()
                    .map(|pb| pb.set_message(format!("{}", path.display())));
                Some(resolve_from_path(path))
            }
            Item::CachedFile(path) => {
                self.filing_progress
                    .as_ref()
                    .map(|pb| pb.set_message(format!("{} [cached]", path.display())));
                Some(resolve_from_path(path))
            }
            Item::CustomUrl(url) => {
                self.filing_progress
                    .as_ref()
                    .map(|pb| pb.set_message(format!("{}", url)));
                Some(resolve_filing_from_url(&url))
            }
            Item::FilingId(filing_id) => {
                self.filing_progress
                    .as_ref()
                    .map(|pb| pb.set_message(filing_id.to_human_readable()));
                Some(self.sourcer.resolve_from_fec_id(&filing_id))
            }
        }
    }
}

pub enum UserArgument {
    Filing(Item),
    Committee(String),
    Candidate(String),
    Contest(Contest),
    InputFile(PathBuf),
}

pub enum Contest {
    // ex "P" or "president"
    President,
    // ex "S-CA" or "senate-CA"
    Senate { state: String },
    // ex "H-CA12" or "house-CA12"
    House { state: String, district: u8 },
}

impl Contest {
    fn from_arg(input: &str) -> anyhow::Result<Option<Self>> {
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
        if input.starts_with("H-") || input.to_lowercase().starts_with("house-") {
            let parts: Vec<&str> = input.split('-').collect();
            if parts.len() == 2 {
                let state = &parts[1][0..2].to_uppercase();
                let district = &parts[1][2..];
                if let Ok(district_num) = district.parse::<u8>() {
                    return Ok(Some(Contest::House {
                        state: state.to_string(),
                        district: district_num,
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
                    district: input[2..4].parse::<u8>()?,
                }));
            }
        }
        Ok(None)
    }
    fn resolve_candidate_params(&self, cycle: u16) -> ResolveCandidateParams {
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
                    .district(Some(district.to_string()));
            }
        }

        b.build().unwrap()
    }
}

// "Committee FEC ID codes consist of the letter C=Committee in the first position,
// followed by 7 digits, followed by a ‘checkdigit’ in the 9th position."
fn is_committee_input(input: &str) -> bool {
    input.len() == 9 && matches!(input.chars().nth(0), Some('C'))
}

// "House & Senate Candidate FEC ID codes have the following formats: H9ST99999, S9ST99999, and P99999999...
// (where the 1st Character is H=House, S=Senate, P=Presidential, and the 3rd & 4th characters
//  of House & Senate codes are the 2letter State Code, and the remaining parts of all codes are numeric).""
fn is_candidate_input(input: &str) -> bool {
    input.len() == 9 && matches!(input.chars().nth(0), Some('H') | Some('S') | Some('P'))
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
    ) -> anyhow::Result<(Trace, IterFilingsX<'_>)> {
        IterFilingsX::new(self, &input, api_flags, mb)
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

        if is_committee_input(input) {
            return Ok(UserArgument::Committee(input.to_owned()));
        }
        if is_candidate_input(input) {
            return Ok(UserArgument::Candidate(input.to_owned()));
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
        Ok(Filing::from_reader(
            resolved.reader,
            resolved.filing_id,
            resolved.source_length,
        )?)
    }
}
