use anyhow::Context;
use colored::Colorize;
use indicatif::{HumanBytes, MultiProgress, ProgressBar};
use jiff::Timestamp;
use std::{fs::File, io::BufWriter, path::PathBuf, thread::spawn};

use crate::{
    cli::{CacheArgs, Cli},
    commands::download::BAR_FILE_STYLE,
};

struct Stats {
    number_downloaded: usize,
    number_preexisting: usize,
    number_unfinished: usize,
    downloaded_bytes: usize,
}

struct QueueItem {
    filing_id: String,
    url: String,
    output_path: PathBuf,
    part_path: PathBuf,
}

fn process_item(item: QueueItem, pb: &ProgressBar) -> anyhow::Result<usize> {
    let request = ureq::get(&item.url);
    File::create(&item.part_path).context("Failed to create part file")?;
    let mut f = File::create(&item.output_path).context("Failed to create output file")?;
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
    pb.set_message(format!("FEC-{}", item.filing_id));
    std::io::copy(
        &mut pb.wrap_read(response.body_mut().as_reader()),
        &mut BufWriter::new(&mut f),
    )?;
    // safe to ignore if this fails for some reason
    let _ = std::fs::remove_file(item.part_path);
    Ok(length)
}

pub fn cache(cli: &Cli, args: &CacheArgs) -> Result<(), ()> {
    let cache_directory = match &cli.top_level.cache_directory {
        Some(dir) => dir,
        None => {
            eprintln!("Error: LIBFEC_CACHE_DIRECTORY environment variable is not set.");
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

    let mut filings = args.filings.clone().unwrap_or_default();

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

    let t0 = jiff::Timestamp::now();
    let mut stats = Stats {
        number_downloaded: 0,
        number_preexisting: 0,
        number_unfinished: 0,
        downloaded_bytes: 0,
    };

    let mut queue: Vec<QueueItem> = vec![];
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
        let part_path = cache_directory.join(&filing_id).with_extension("fec.part");

        if filing_path.exists() {
            if part_path.exists() {
                stats.number_unfinished += 1;
                std::fs::remove_file(&filing_path).unwrap();
            } else {
                stats.number_preexisting += 1;
                continue;
            }
        }
        queue.push(QueueItem {
            filing_id: filing_id.clone(),
            url: format!("https://docquery.fec.gov/dcdev/posted/{filing_id}.fec"),
            output_path: filing_path,
            part_path,
        })
    }

    let mb = MultiProgress::new();
    let top = mb.add(ProgressBar::new_spinner());

    let (tx, rx) = std::sync::mpsc::channel();
    let mut handles = Vec::with_capacity(args.number_concurrent);
    let mut active = 0;
    
    top.set_message(format!("{}/{}", 0, queue.len()));

    for _ in 0..args.number_concurrent.min(queue.len()) {
        let item = match queue.pop() {
            Some(item) => item,
            None => break,
        };
        let pb: ProgressBar = mb.add(ProgressBar::new(0));
        pb.set_style(BAR_FILE_STYLE.clone());
        let tx = tx.clone();
        handles.push(spawn(move || {
            match process_item(item, &pb) {
                Ok(length) => {
                    tx.send(length).unwrap();
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
        top.set_message(format!("{}", active));

        let pb: ProgressBar = mb.add(ProgressBar::new(0));
        pb.set_style(BAR_FILE_STYLE.clone());
        let tx = tx.clone();
        handles.push(std::thread::spawn(move || {
            match process_item(x, &pb) {
                Ok(length) => {
                    tx.send(length).unwrap();
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
        top.set_message(format!("{}", active));
    }

    for handle in handles {
        handle.join().unwrap();
    }
    mb.clear().unwrap();

    let duration = Timestamp::now() - t0;
    println!(
        "{} Cached {} filings in {:#}",
        "✓".green(),
        stats.number_downloaded + stats.number_preexisting,
        duration,
    );
    println!(
        "  {} filings downloaded ({})",
        stats.number_downloaded,
        HumanBytes(stats.downloaded_bytes as u64)
    );
    if stats.number_preexisting > 0 {
        println!("  {} pre-existing", stats.number_preexisting);
    }

    Ok(())
}
