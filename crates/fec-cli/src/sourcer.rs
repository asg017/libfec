use anyhow::Error;
use fec_parser::Filing;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
};
use url::Url;

pub struct FilingSourcer {
    pub cache_directory: Option<PathBuf>,
}

struct ResolvedFiling {
    reader: Box<dyn Read>,
    filing_id: String,
    source_length: Option<usize>,
}

fn resolve_from_url(url: &Url) -> Result<ResolvedFiling, Error> {
    let request = ureq::get(url.as_str());
    let response = request.call().unwrap();
    let filing_id = Path::new(url.path())
        .file_stem()
        .map(|os_str| os_str.to_string_lossy().to_string());
    let source_length = response
        .headers()
        .get("Content-Length")
        .unwrap()
        .to_str()
        .ok()
        .map(|v| v.parse().unwrap());
    let r = response.into_parts().1.into_reader();
    Ok(ResolvedFiling {
        reader: Box::new(r),
        filing_id: filing_id.unwrap(),
        source_length,
    })
}

fn resolve_from_file(f: File, path: PathBuf) -> Result<ResolvedFiling, Error> {
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

fn resolve_from_filing_id(filing_id: &str) -> Result<ResolvedFiling, Error> {
    let filing_id = filing_id
        .strip_prefix("FEC-")
        .or_else(|| filing_id.strip_prefix("FEC"))
        .unwrap_or(filing_id)
        .to_owned();

    let url = format!("https://docquery.fec.gov/dcdev/posted/{filing_id}.fec");
    resolve_from_url(&Url::from_str(&url).unwrap())
}

fn resolve_from_cache(
    cache_directory: &Path,
    input: &str,
) -> Option<Result<ResolvedFiling, Error>> {
    let path = cache_directory
        .join(input.strip_prefix("FEC-").unwrap_or(input))
        .with_extension("fec");
    let file = File::open(&path).ok()?;
    Some(resolve_from_file(file, path))
}

impl FilingSourcer {
    pub fn new() -> Self {
        let cache_directory = std::env::var("LIBFEC_CACHE_DIRECTORY")
            .ok()
            .map(|s| Path::new(&s).to_path_buf());
        Self { cache_directory }
    }

    // Resolve a FEC filing from an "input" source such as:
    // 1. If it's a file, read it from the file system.
    // 2. If it's a URL, fetch it directly from the web.
    // 3. Check if it's in the cache directory
    // 4. If it's a FEC filing ID, construct the URL to docquery.fec.gov and fetch it.

    pub fn resolve(&self, input: &str) -> Filing<Box<dyn Read>> {
        let resolved_filing: ResolvedFiling = {
            // 1. if it's a file, read it
            if let Ok(file) = File::open(input) {
                resolve_from_file(file, PathBuf::from(input)).unwrap()
            }
            // 2. if it's a URL, fetch it
            else if let Ok(url) = Url::parse(input) {
                resolve_from_url(&url).unwrap()
            }
            // 3. check to see if it's cached
            else if let Some(cache_directory) = self.cache_directory.as_ref() {
                match resolve_from_cache(cache_directory, input) {
                    Some(Ok(filing)) => filing,
                    Some(Err(_)) => todo!(),
                    None => resolve_from_filing_id(input).unwrap(),
                }
            } else {
                resolve_from_filing_id(input).unwrap()
            }
        };
        Filing::from_reader(
            resolved_filing.reader,
            resolved_filing.filing_id,
            resolved_filing.source_length,
        )
        .unwrap()
    }
}
