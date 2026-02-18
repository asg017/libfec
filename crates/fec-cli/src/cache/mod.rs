pub mod api_cache;
pub mod bulk_candidate_committee_linkage;
pub mod bulk_candidates;
pub mod bulk_committee;
pub mod bulk_opexp;
pub mod bulk_pas2;
mod bulk_utils;

use crate::{cache::bulk_candidates::ResolveCandidateParams, sourcer::FecFilingId};
use anyhow::{Context, Result};
use api_cache::SqliteApiCache;
use etcetera::BaseStrategy;
use indicatif::ProgressStyle;
use indicatif::{MultiProgress, ProgressBar};
use jiff::civil::Date;
use rusqlite::Connection;
use std::io::Read;
use std::io::Write;
use std::{
    fs::File,
    io::{BufWriter, Cursor},
    path::PathBuf,
    sync::{
        mpsc::{Receiver, Sender},
        LazyLock,
    },
    thread::spawn,
};
use ureq::http::header::{CONTENT_LENGTH, LAST_MODIFIED};

pub static BAR_FILE_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::with_template(
        "{msg}:\t[{elapsed_precise}] {bar:40.cyan/blue} {eta} {decimal_total_bytes} {decimal_bytes_per_sec:.dim}",
    )
    .expect("valid progress style template")
});

pub(crate) struct CacheAllResult {
    pub(crate) stats: CacheAllStats,
    pub(crate) paths: Vec<PathBuf>,
}
pub(crate) struct CacheAllStats {
    pub number_downloaded: usize,
    pub number_preexisting: usize,
    pub number_unfinished: usize,
    pub downloaded_bytes: usize,
}

pub(crate) struct CacheBulkDailyZipResultItem {
    pub(crate) filing_id: FecFilingId,
    pub(crate) output_path: PathBuf,
}

pub(crate) struct Cache {
    cache_directory: PathBuf,
    number_concurrent: usize,
    api_cache: Option<SqliteApiCache>,
}

pub struct CachedFiling {
    pub length: usize,
    pub output_path: PathBuf,
}
static BASE_URL: &str =
    "https://cg-519a459a-0ea3-42c2-b7bc-fa1143481f74.s3-us-gov-west-1.amazonaws.com";

impl Cache {
    pub fn new(cli_cache_directory: Option<PathBuf>) -> Self {
        let cache_directory = cli_cache_directory
            .unwrap_or_else(|| {
                let strat =
                    etcetera::choose_base_strategy().expect("Could not determine cache directory");
                strat.cache_dir()
            })
            .join("libfec")
            .join("cache");

        std::fs::create_dir_all(&cache_directory).expect("Could not create cache directory");

        let api_cache = SqliteApiCache::new(&cache_directory).ok();

        Cache {
            cache_directory,
            number_concurrent: 8,
            api_cache,
        }
    }

    /// Get a mutable reference to the API cache, if available.
    pub fn api_cache_mut(&mut self) -> Option<&mut SqliteApiCache> {
        self.api_cache.as_mut()
    }

    /// Get the cache directory path.
    pub fn cache_directory(&self) -> &PathBuf {
        &self.cache_directory
    }

    pub(crate) fn open_bulk_data_database(&mut self) -> Result<Connection> {
        let db_path = self.bulk_data_database_path();
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Could not open or create database at {:?}", db_path))?;
        Ok(conn)
    }

    pub(crate) fn bulk_data_database_path(&self) -> PathBuf {
        self.cache_directory.join(".bulk-data.db")
    }

    pub fn cache_bulk_daily_zip(
        &self,
        date: Date,
        mb: Option<&MultiProgress>,
    ) -> Result<Vec<CacheBulkDailyZipResultItem>> {
        // .daily-zip.YYYY-MM-DD.meta format: Each line is a filing ID that was in the zip for that day.

        let mut items = vec![];

        let meta_path = self
            .cache_directory
            .join(format!(".daily-zip.{}.meta", date.strftime("%Y-%m-%d")));
        let meta_part_path = meta_path.with_extension("meta.part");
        if meta_path.exists() {
            if meta_part_path.exists() {
                let _ = std::fs::remove_file(&meta_path);
                let _ = std::fs::remove_file(&meta_part_path);
            } else {
                // TODO logic for last-modified check
                let mut f = File::open(&meta_path)?;
                let mut contents = String::new();
                f.read_to_string(&mut contents)?;
                for line in contents.lines() {
                    let filing_id = FecFilingId::from_str(line)
                        .with_context(|| format!("Invalid filing ID in meta file: {}", line))?;
                    let output_path = self
                        .resolve_filing(&filing_id)
                        .ok_or_else(|| anyhow::anyhow!("Filing {} not found in cache", line))?;
                    items.push(CacheBulkDailyZipResultItem {
                        filing_id,
                        output_path,
                    });
                }
                return Ok(items);
            }
        }

        File::create(&meta_part_path).context("Failed to create meta part file")?;

        let zip_url = format!(
            "{BASE_URL}/bulk-downloads/electronic/{}.zip",
            date.strftime("%Y%m%d")
        );

        let response = ureq::get(&zip_url)
            .call()
            .with_context(|| format!("Failed to download daily zip at {}", zip_url))?;

        let length: usize = response
            .headers()
            .get(CONTENT_LENGTH)
            .ok_or_else(|| anyhow::anyhow!("No Content-Length header in response"))?
            .to_str()
            .with_context(|| "Content-Length header is not valid UTF-8")?
            .parse()
            .with_context(|| "Content-Length header is not a valid number")?;

        let last_modified = response
            .headers()
            .get(LAST_MODIFIED)
            .ok_or_else(|| anyhow::anyhow!("No Last-Modified header in response"))?
            .to_str()
            .with_context(|| "Last-Modified header is not valid UTF-8")?
            .to_owned();
        let last_modified = jiff::fmt::rfc2822::parse(&last_modified)
            .with_context(|| {
                format!(
                    "Last-Modified header is not a valid RFC 2822 date: {}",
                    last_modified
                )
            })?
            .timestamp();

        let pb = mb.map(|mb| {
            let pb = mb.add(ProgressBar::new(length as u64));
            pb.set_style(BAR_FILE_STYLE.clone());
            pb.set_message(format!("{}", date));
            pb
        });

        let mut r = response.into_parts().1.into_reader();

        let mut buffer = Vec::new();
        if let Some(ref pb) = pb {
            std::io::copy(&mut pb.wrap_read(r), &mut buffer)
                .with_context(|| format!("Failed to read response body for {}", zip_url))?;
        } else {
            std::io::copy(&mut r, &mut buffer)
                .with_context(|| format!("Failed to read response body for {}", zip_url))?;
        }
        if let Some(pb) = pb {
            pb.finish_and_clear()
        }

        let reader = Cursor::new(buffer);
        let mut archive = zip::ZipArchive::new(reader)?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if !file.name().ends_with(".fec") {
                continue;
            }
            let file_name = file.name().trim_end_matches(".fec");
            let filing_id = FecFilingId::from_str(file_name)
                .with_context(|| format!("Invalid filing ID in zip: {}", file_name))?;
            let output_path = self.cache_directory.join(file.name());
            if !output_path.exists() {
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut out_file = std::fs::File::create(&output_path)?;
                std::io::copy(&mut file, &mut out_file)
                    .with_context(|| format!("Failed to write to {:?}", output_path))?;
            }
            items.push(CacheBulkDailyZipResultItem {
                filing_id,
                output_path,
            });
        }

        let mut meta_file = File::create(&meta_path).context("Failed to create meta file")?;
        for item in &items {
            writeln!(meta_file, "{}", item.filing_id.to_bare())?;
        }
        let _ = std::fs::remove_file(meta_part_path);
        meta_file.set_modified(last_modified.into())?;

        Ok(items)
    }

    pub fn resolve_candidate_principal_campaign_committees(
        &mut self,
        params: ResolveCandidateParams,
    ) -> Result<Vec<String>> {
        bulk_candidates::resolve_candidate_principal_campaign_committees(
            self.open_bulk_data_database()?,
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
        ids: Vec<FecFilingId>,
        mb: Option<&MultiProgress>,
    ) -> Result<CacheAllResult> {
        let mut stats = CacheAllStats {
            number_downloaded: 0,
            number_preexisting: 0,
            number_unfinished: 0,
            downloaded_bytes: 0,
        };
        let mut paths = vec![];

        let mut queue = vec![];
        for item in ids {
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
                    std::fs::remove_file(&filing_path)?;
                } else {
                    stats.number_preexisting += 1;
                    paths.push(filing_path);
                    continue;
                }
            }
            queue.push(item);
        }

        let spinner = mb.map(|mb| {
            let spinner = mb.add(ProgressBar::new_spinner());
            spinner.set_message(format!("{}/{}", 0, queue.len()));
            spinner
        });

        let (tx, rx): (
            Sender<anyhow::Result<CachedFiling>>,
            Receiver<anyhow::Result<CachedFiling>>,
        ) = std::sync::mpsc::channel();
        let mut handles = Vec::with_capacity(self.number_concurrent);
        let mut active = 0;

        for _ in 0..self.number_concurrent.min(queue.len()) {
            let item = match queue.pop() {
                Some(item) => item,
                None => break,
            };
            let pb = mb.map(|mb| {
                let pb = mb.add(ProgressBar::new(0));
                pb.set_style(BAR_FILE_STYLE.clone());
                pb
            });

            let tx = tx.clone();
            let cache_directory = self.cache_directory.clone();
            handles.push(spawn(move || {
                match Cache::cache_filing_static(cache_directory, item, &pb) {
                    Ok(result) => {
                        let _ = tx.send(Ok(result));
                    }
                    Err(e) => {
                        if let Some(pb) = pb {
                            pb.println(format!("Error processing item: {}", e));
                        }
                        let _ = tx.send(Err(anyhow::anyhow!("Download failed: {}", e)));
                    }
                }
            }));
            active += 1;
        }

        while let Some(filing_id) = queue.pop() {
            let result = rx
                .recv()
                .expect("receiver should not be disconnected while workers are active");
            match result {
                Err(e) => {
                    if let Some(spinner) = spinner.as_ref() {
                        spinner.println(format!("Error processing item: {}", e))
                    }
                }
                Ok(result) => {
                    paths.push(result.output_path.clone());
                    stats.downloaded_bytes += result.length;
                    stats.number_downloaded += 1;
                }
            }
            active -= 1;
            if let Some(spinner) = spinner.as_ref() {
                spinner.set_message(format!("{} filings left…", queue.len()))
            }
            let pb = mb.map(|mb| {
                let pb = mb.add(ProgressBar::new(0));
                pb.set_style(BAR_FILE_STYLE.clone());
                pb
            });
            let tx = tx.clone();
            let cache_directory = self.cache_directory.clone();
            handles.push(std::thread::spawn(
                move || match Cache::cache_filing_static(cache_directory, filing_id.clone(), &pb) {
                    Ok(result) => {
                        let _ = tx.send(Ok(result));
                    }
                    Err(e) => {
                        if let Some(pb) = pb {
                            pb.println(format!("Error processing {:?}: {}", filing_id, e));
                        }
                        let _ = tx.send(Err(anyhow::anyhow!("Download failed: {}", e)));
                    }
                },
            ));
            active += 1;
        }

        while active > 0 {
            let result = rx
                .recv()
                .expect("receiver should not be disconnected while workers are active");
            match result {
                Err(e) => {
                    if let Some(spinner) = spinner.as_ref() {
                        spinner.println(format!("Error processing item: {}", e))
                    }
                }
                Ok(result) => {
                    paths.push(result.output_path.clone());
                    stats.downloaded_bytes += result.length;
                    stats.number_downloaded += 1;
                }
            }

            active -= 1;

            if let Some(spinner) = spinner.as_ref() {
                spinner.set_message(format!("{} filings left…", queue.len()))
            }
        }

        for handle in handles {
            let _ = handle.join();
        }

        if let Some(spinner) = spinner {
            spinner.finish_and_clear()
        }

        Ok(CacheAllResult { stats, paths })
    }

    pub fn cache_filing_static(
        cache_directory: PathBuf,
        filing_id: FecFilingId,
        pb: &Option<ProgressBar>,
    ) -> Result<CachedFiling> {
        let url = filing_id.filing_url();
        let part_path = cache_directory
            .join(filing_id.to_bare())
            .with_extension("fec.part");
        let output_path = cache_directory
            .join(filing_id.to_bare())
            .with_extension("fec");

        File::create(&part_path).context("Failed to create part file")?;
        let mut f = File::create(&output_path).context("Failed to create output file")?;

        let request = ureq::get(url.to_string());
        let mut response = request.call().context(format!(
            "Failed to download filing {:?} from {}",
            filing_id, url
        ))?;
        let length: usize = response
            .headers()
            .get("Content-Length")
            .ok_or_else(|| anyhow::anyhow!("No Content-Length header in response"))?
            .to_str()
            .with_context(|| "Content-Length header is not valid UTF-8")?
            .parse()
            .with_context(|| "Content-Length header is not a valid number")?;

        if let Some(pb) = pb {
            pb.set_message(filing_id.to_human_readable());
            pb.set_length(length as u64);
        }
        let mut reader = response.body_mut().as_reader();
        if let Some(ref pb) = pb {
            std::io::copy(&mut pb.wrap_read(&mut reader), &mut BufWriter::new(&mut f))?
        } else {
            std::io::copy(&mut reader, &mut BufWriter::new(&mut f))?
        };
        // safe to ignore if this fails for some reason
        let _ = std::fs::remove_file(part_path);
        Ok(CachedFiling {
            length,
            output_path,
        })
    }
}
