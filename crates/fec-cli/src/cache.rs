use indicatif::{HumanBytes, HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use std::{fs::File, io::BufWriter, path::PathBuf, time::Duration};

use crate::{
    cli::CacheArgs,
    cmd_download::{BAR_FILES_STYLE, BAR_FILE_STYLE},
};

struct Stats {
    number_downloaded: usize,
    number_preexisting: usize,
    downloaded_bytes: usize,
}

pub fn cache(args: CacheArgs) -> Result<(), ()> {
    let cache_directory = match std::env::var("LIBFEC_CACHE_DIRECTORY") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => {
            eprintln!("Errror: LIBFEC_CACHE_DIRECTORY environment variable is not set.");
            return Err(());
        }
    };

    if !cache_directory.exists() {
        std::fs::create_dir_all(&cache_directory).map_err(|_| {
            eprintln!(
                "Error: Could not create cache directory at {}",
                cache_directory.display()
            );
            ()
        })?;
    }

    let mut filings = args.filings.unwrap_or_default();

    if args.api.any_provided() {
      match args.api.resolve_ids() {
        Ok(resolved_filings) => {
            filings.extend(resolved_filings);
        }
        Err(e) => {
            eprintln!("Error resolving filings: {:?}", e);
            return Err(());
        }
      }
    }

    let mb = MultiProgress::new();
    let pb_files = mb.add(ProgressBar::new(filings.len() as u64));
    pb_files.set_style(BAR_FILES_STYLE.clone());
    pb_files.enable_steady_tick(Duration::from_millis(750));
    let mut stats = Stats {
        number_downloaded: 0,
        number_preexisting: 0,
        downloaded_bytes: 0,
    };

    for filing in filings {
        let filing_id = filing
            .strip_prefix("FEC-")
            .or_else(|| filing.strip_prefix("FEC"))
            .unwrap_or(&filing)
            .to_owned();
        if filing_id.chars().any(|c| !c.is_ascii_digit()) {
            eprintln!("Error: Invalid filing ID: {}", filing);
            return Err(());
        }
        let filing_path = cache_directory.join(&filing_id).with_extension("fec");
        if filing_path.exists() {
            stats.number_preexisting += 1;
            continue;
        }

        let request =
            ureq::get(format!("https://docquery.fec.gov/dcdev/posted/{filing_id}.fec").as_str());
        let mut response = request.call().unwrap();
        let length: usize = response
            .headers()
            .get("Content-Length")
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();

        let mut f = File::create_new(&filing_path).unwrap();
        let pb_file = mb.add(ProgressBar::new(length as u64));
        pb_file.set_style(BAR_FILE_STYLE.clone());
        std::io::copy(
            &mut pb_file.wrap_read(response.body_mut().as_reader()),
            &mut BufWriter::new(&mut f),
        )
        .unwrap();
        pb_file.set_message(filing.clone());
        pb_files.inc(1);

        stats.number_downloaded += 1;
        stats.downloaded_bytes += length;
    }

    println!(
        "Cached {} filings ({} downloaded, {} skipped, {} downloaded)",
        stats.number_downloaded + stats.number_preexisting,
        stats.number_downloaded,
        stats.number_preexisting,
        HumanBytes(stats.downloaded_bytes as u64)
    );

    Ok(())
}
