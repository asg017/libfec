use anyhow::{Context, Error, Result};
use fec_parser::Filing;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
};
use url::Url;

use crate::{cache::Cache, cli::FilingsApiFlags};

pub struct FilingSourcer {
    pub cache: Cache,
}

struct ResolvedFiling {
    reader: Box<dyn Read>,
    filing_id: String,
    source_length: Option<usize>,
}

fn resolve_from_url(url: &Url) -> Result<ResolvedFiling> {
    let request = ureq::get(url.as_str());
    let response = request.call().context("Error requesting FEC filing from URL")?;
    let filing_id = Path::new(url.path())
        .file_stem()
        .map(|os_str| os_str.to_string_lossy().to_string())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to extract filing ID from URL: {}",
                url.as_str()
            )
        })?;
    let source_length: usize = response
        .headers()
        .get("Content-Length")
        .ok_or_else(|| anyhow::anyhow!(
            "No Content-Length header found in response",
        ))?
        .to_str()
        .map_err(|_| anyhow::anyhow!("Content-Length header is not valid UTF-8"))?
        .parse()
        .map_err(|_| anyhow::anyhow!("Content-Length header is not a valid number"))?;
    let r = response.into_parts().1.into_reader();
    Ok(ResolvedFiling {
        reader: Box::new(r),
        filing_id: filing_id,
        source_length: Some(source_length),
    })
}

fn resolve_from_file(f: File, path: PathBuf) -> Result<ResolvedFiling> {
    let filing_id = path
        .file_stem()
        .map(|os_str| os_str.to_string_lossy().to_string())
        .unwrap_or_default();
    let source_length = f.metadata().map(|v| (v.len() as usize)).ok();
    Ok(ResolvedFiling {
        reader: Box::new(f),
        filing_id,
        source_length,
    })
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
pub(crate) struct FecFilingId(usize);

impl FecFilingId {
    pub fn to_human_readable(&self) -> String {
        format!("FEC-{}", self.0)
    }
    pub fn to_bare(&self) -> String {
        format!("{}", self.0)
    }
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        let id = s.strip_prefix("FEC-").unwrap_or(s);
        let id = id.parse::<usize>()?;
        Ok(FecFilingId(id))
    }
}

enum IterFilingQueueItem {
    /// could be file path, URL, filing ID, etc.
    UserArg(String),

    /// any FEC id resolved from an API call, ex FEC-1234567
    ApiFilingId(FecFilingId),
}
pub struct IterFilings<'a> {
    sourcer: &'a FilingSourcer,
    queue: Vec<IterFilingQueueItem>,
}

impl<'a> IterFilings<'a> {
    pub fn new(
        sourcer: &'a FilingSourcer,
        input: &Vec<String>,
        api_flags: Option<FilingsApiFlags>,
    ) -> anyhow::Result<Self> {
        let mut queue = vec![];
        for item in input {
            queue.push(IterFilingQueueItem::UserArg(item.clone()));
        }
        if let Some(api_flags) = api_flags {
            if api_flags.any_provided() {
                let ids = api_flags.resolve_ids(sourcer).unwrap();
                queue.extend(
                    ids.into_iter()
                        .map(|id| IterFilingQueueItem::ApiFilingId(id)),
                );

                if api_flags.cache {
                    sourcer.cache.cache_all(sourcer, vec![], &api_flags).unwrap();
                }
            }
        }

        Ok(IterFilings { sourcer, queue })
    }
}

impl<'a> Iterator for IterFilings<'a> {
    type Item = anyhow::Result<Filing<Box<dyn Read>>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.queue.pop()? {
            IterFilingQueueItem::UserArg(arg) => {
                Some(self.sourcer.resolve_from_user_argument(&arg))
            }
            IterFilingQueueItem::ApiFilingId(filing_id) => {
                Some(Ok(self.sourcer.resolve_from_fec_id(&filing_id)))
            }
        }
    }
}

impl FilingSourcer {
    pub fn new(cli_cache_directory: Option<PathBuf>) -> Self {
        Self {
            cache: Cache::new(cli_cache_directory),
        }
    }

    pub fn resolve_iterator(
        &self,
        input: Vec<String>,
        api_flags: Option<FilingsApiFlags>,
    ) -> anyhow::Result<IterFilings> {
        IterFilings::new(&self, &input, api_flags)
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
                resolve_from_url(&url).unwrap()
            }
            // 3. check to see if it's cached
            else if let Some(filing_path) = self
                .cache
                .resolve_filing(&FecFilingId::from_str(input)?)
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

    fn resolve_from_fec_id(&self, filing_id: &FecFilingId) -> Filing<Box<dyn Read>> {
        let resolved = if let Some(filing_path) = self.cache.resolve_filing(filing_id) {
            match resolve_from_file(File::open(&filing_path).unwrap(), filing_path) {
                Ok(filing) => filing,
                Err(_) => todo!(),
            }
        } else {
            resolve_from_filing_id(&filing_id.to_bare()).unwrap()
        };
        Filing::from_reader(resolved.reader, resolved.filing_id, resolved.source_length).unwrap()
    }
}
