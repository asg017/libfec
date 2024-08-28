use fec_parser::Filing;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
pub struct FilingSourcer {
    pub cache_directory: Option<PathBuf>,
}

impl FilingSourcer {
    pub fn new() -> Self {
        let cache_directory = std::env::var("LIBFEC_CACHE_DIRECTORY")
            .ok()
            .map(|s| Path::new(&s).to_path_buf());
        Self { cache_directory }
    }

    pub fn resolve(&self, input: &str) -> Filing<Box<dyn Read>> {
        let (r, filing_id, source_length): (Box<dyn Read>, String, Option<usize>) =
            match File::open(input) {
                Ok(f) => {
                    let filing_id = Path::new(input)
                        .file_stem()
                        .map(|os_str| os_str.to_string_lossy().to_string());
                    let source_length = f.metadata().map(|v| (v.len() as usize)).ok();
                    (Box::new(f), filing_id.unwrap(), source_length)
                }
                Err(_) => {
                    if let Ok(url) = url::Url::parse(input) {
                        let request = ureq::get(input);
                        let response = request.call().unwrap();
                        let filing_id = Path::new(url.path())
                            .file_stem()
                            .map(|os_str| os_str.to_string_lossy().to_string());
                        let source_length = response
                            .header("Content-Length")
                            .map(|v| v.parse().unwrap());
                        (
                            Box::new(response.into_reader()),
                            filing_id.unwrap(),
                            source_length,
                        )
                    } else {
                        match self.cache_directory.as_ref() {
                            Some(cache_directory) => todo!(),
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
                                    .header("Content-Length")
                                    .map(|v| v.parse().unwrap());
                                (Box::new(response.into_reader()), filing_id, source_length)
                            }
                        }
                    }
                }
            };
        Filing::from_reader(r, filing_id, source_length).unwrap()
    }
}
