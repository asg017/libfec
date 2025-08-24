pub mod bulk_candidates;
mod bulk_utils;

use crate::{
    cache::bulk_candidates::ResolveCandidateParams,
    cli::FilingsApiFlags,
    commands::download::BAR_FILE_STYLE,
    sourcer::{FecFilingId, FilingSourcer},
};
use anyhow::{Context, Result};
use etcetera::BaseStrategy;
use indicatif::{MultiProgress, ProgressBar};
use std::{fs::File, io::BufWriter, path::PathBuf, thread::spawn};

pub(crate) struct CacheAllStats {
    pub number_downloaded: usize,
    pub number_preexisting: usize,
    pub number_unfinished: usize,
    pub downloaded_bytes: usize,
}

pub(crate) struct Cache {
    pub cache_directory: PathBuf,
    number_concurrent: usize,
}

pub struct CacheFilingResult {
    pub length: usize,
    pub output_path: PathBuf,
}

impl Cache {
    pub fn new(cli_cache_directory: Option<PathBuf>) -> Self {
        let cache_directory = cli_cache_directory.unwrap_or_else(|| {
            let strat =
                etcetera::choose_base_strategy().expect("Could not determine cache directory");
            strat.cache_dir()
        });
        Cache {
            cache_directory,
            number_concurrent: 4,
        }
    }

    pub fn resolve_candidate_committees(
        &self,
        params: ResolveCandidateParams,
    ) -> Result<Vec<String>> {
        bulk_candidates::resolve_candidate_committees(
            &self.cache_directory.join(".bulk-data.db"),
            params,
        )
    }

    pub fn resolve_filing(&self, filing_id: &FecFilingId) -> Option<PathBuf> {
        let filing_path = self
            .cache_directory
            .join(filing_id.to_bare())
            .with_extension("fec");
        let part_path = self
            .cache_directory
            .join(filing_id.to_bare())
            .with_extension("fec.part");

        if filing_path.exists() && !part_path.exists() {
            Some(filing_path)
        } else {
            None
        }
    }

    pub fn cache_all(
        &self,
        sourcer: &FilingSourcer,
        ids: Vec<FecFilingId>,
        api_flags: &FilingsApiFlags,
    ) -> Result<CacheAllStats> {
        todo!("Fix cache_all");
        let mut possible: Vec<FecFilingId> = ids.clone();
        let mut stats = CacheAllStats {
            number_downloaded: 0,
            number_preexisting: 0,
            number_unfinished: 0,
            downloaded_bytes: 0,
        };
        if api_flags.any_provided() {
            let resolved_ids = api_flags.resolve_ids(sourcer)?;
            possible.extend(resolved_ids);
        }

        let mut queue = vec![];
        for item in possible {
            let filing_path = self
                .cache_directory
                .join(item.to_bare())
                .with_extension("fec");
            let part_path = self
                .cache_directory
                .join(item.to_bare())
                .with_extension("fec.part");
            if filing_path.exists() {
                if part_path.exists() {
                    stats.number_unfinished += 1;
                    std::fs::remove_file(&filing_path).unwrap();
                } else {
                    stats.number_preexisting += 1;
                    continue;
                }
            }
            queue.push(item);
        }

        let mb = MultiProgress::new();
        let spinner = mb.add(ProgressBar::new_spinner());

        let (tx, rx) = std::sync::mpsc::channel();
        let mut handles = Vec::with_capacity(self.number_concurrent);
        let mut active = 0;

        spinner.set_message(format!("{}/{}", 0, queue.len()));

        for _ in 0..self.number_concurrent.min(queue.len()) {
            let item = match queue.pop() {
                Some(item) => item,
                None => break,
            };
            let pb: ProgressBar = mb.add(ProgressBar::new(0));
            pb.set_style(BAR_FILE_STYLE.clone());

            let tx = tx.clone();
            let cache_directory = self.cache_directory.clone();
            handles.push(spawn(move || {
                match Cache::cache_filing_static(cache_directory, item, &pb) {
                    Ok(result) => {
                        tx.send(result.length).unwrap();
                    }
                    Err(e) => {
                        pb.println(format!("Error processing item: {}", e));
                        tx.send(0).unwrap(); // Send 0 bytes on error
                    }
                }
            }));
            active += 1;
        }

        while let Some(x) = queue.pop() {
            stats.downloaded_bytes += rx.recv().unwrap();
            active -= 1;
            stats.number_downloaded += 1;
            spinner.set_message(format!("{} filings left...", active));
            let pb: ProgressBar = mb.add(ProgressBar::new(0));
            pb.set_style(BAR_FILE_STYLE.clone());
            let tx = tx.clone();
            let cache_directory = self.cache_directory.clone();
            handles.push(std::thread::spawn(move || {
                match Cache::cache_filing_static(cache_directory, x, &pb) {
                    Ok(result) => {
                        tx.send(result.length).unwrap();
                    }
                    Err(e) => {
                        pb.println(format!("Error processing item: {}", e));
                        tx.send(0).unwrap(); // Send 0 bytes on error
                    }
                }
            }));
            active += 1;
        }

        while active > 0 {
            stats.downloaded_bytes += rx.recv().unwrap();
            active -= 1;
            stats.number_downloaded += 1;
            spinner.set_message(format!("{} filings left..", active));
        }

        for handle in handles {
            handle.join().unwrap();
        }
        spinner.finish_and_clear();
        mb.clear().unwrap();

        Ok(stats)
    }
    pub fn cache_filing(
        &self,
        filing_id: FecFilingId,
        pb: &ProgressBar,
    ) -> Result<CacheFilingResult> {
        Cache::cache_filing_static(self.cache_directory.clone(), filing_id, pb)
    }

    pub fn cache_filing_static(
        cache_directory: PathBuf,
        filing_id: FecFilingId,
        pb: &ProgressBar,
    ) -> Result<CacheFilingResult> {
        let url = format!(
            "https://docquery.fec.gov/dcdev/posted/{}.fec",
            filing_id.to_bare()
        );
        let part_path = cache_directory
            .join(filing_id.to_bare())
            .with_extension("fec.part");
        let output_path = cache_directory
            .join(filing_id.to_bare())
            .with_extension("fec");

        File::create(&part_path).context("Failed to create part file")?;
        let mut f = File::create(&output_path).context("Failed to create output file")?;

        let request = ureq::get(url);
        let mut response = request.call().context("Failed to make request")?;
        let length: usize = response
            .headers()
            .get("Content-Length")
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();

        pb.set_length(length as u64);
        pb.set_message(filing_id.to_human_readable());
        std::io::copy(
            &mut pb.wrap_read(response.body_mut().as_reader()),
            &mut BufWriter::new(&mut f),
        )?;
        // safe to ignore if this fails for some reason
        let _ = std::fs::remove_file(part_path);
        Ok(CacheFilingResult {
            length,
            output_path,
        })
    }
}
