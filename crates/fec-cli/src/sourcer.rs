use anyhow::Error;
use fec_parser::Filing;
use url::Url;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
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
  Ok(ResolvedFiling{
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
    // 3. If it's a FEC filing ID, construct the URL to docquery.fec.gov and fetch it.
    // 4. Check if it's in the LIBFEC_CACHE_DIRECTORY
    pub fn resolve(&self, input: &str) -> Filing<Box<dyn Read>> {
        let resolved_filing: ResolvedFiling =
            match File::open(input) {
                Ok(f) => {
                  resolve_from_file(f, PathBuf::from(input)).unwrap()
                }
                Err(_) => {
                    if let Ok(url) = url::Url::parse(input) {
                        resolve_from_url(&url).unwrap()
                    } else {
                        match self.cache_directory.as_ref() {
                            Some(cache_directory) =>  {
                              cache_directory.join(input.strip_prefix("FEC-").unwrap_or(input))
                                  .with_extension("fec")
                                  .to_str()
                                  .and_then(|s| File::open(s).ok())
                                  .and_then(|f| resolve_from_file(f, PathBuf::from(input)).ok())
                                  .unwrap_or_else(|| {
                                      panic!("Filing not found in cache: {}", input)
                                  })
                            },
                            None => {
                                let filing_id = input
                                    .strip_prefix("FEC-")
                                    .or_else(|| input.strip_prefix("FEC"))
                                    .unwrap_or(input)
                                    .to_owned();
                                let url = format!(
                                    "https://docquery.fec.gov/dcdev/posted/{filing_id}.fec"
                                );
                                let request = ureq::get(&url);
                                let response = request.call().unwrap();
                                let source_length = response
                                    .headers()
                                    .get("Content-Length")
                                    .unwrap()
                                    .to_str()
                                    .ok()
                                    .map(|v| v.parse().unwrap());
                                let r = response.into_parts().1.into_reader();
                                ResolvedFiling {
                                    reader: Box::new(r),
                                    filing_id,
                                    source_length,
                                }
                            }
                        }
                    }
                }
            };
        Filing::from_reader(resolved_filing.reader, resolved_filing.filing_id, resolved_filing.source_length).unwrap()
    }
}
