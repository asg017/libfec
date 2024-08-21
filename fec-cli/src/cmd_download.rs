use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use std::{
    fs::File,
    io::BufWriter,
    time::{Duration, Instant},
};

pub fn cmd_download(filing_ids: Vec<String>) -> Result<(), ()> {
    let t0 = Instant::now();
    let mb = MultiProgress::new();
    let pb_files = mb.add(ProgressBar::new(filing_ids.len() as u64));
    pb_files.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap(),
    );
    pb_files.enable_steady_tick(Duration::from_millis(750));

    for filing_id in filing_ids {
        let request =
            ureq::get(format!("https://docquery.fec.gov/dcdev/posted/{filing_id}.fec").as_str());
        let response = request.call().unwrap();
        let length: usize = response.header("Content-Length").unwrap().parse().unwrap();
        let mut f = File::create_new(format!("{filing_id}.fec")).unwrap();
        //pb_files.set_message(filing_path.clone());
        let pb_file = mb.add(ProgressBar::new(length as u64));
        pb_file.set_style(
          indicatif::ProgressStyle::with_template(
            "{msg}.fec:\t[{elapsed_precise}] {bar:40.cyan/blue} {eta} {decimal_bytes_per_sec} {decimal_total_bytes} total",
          )
          .unwrap(),
        );
        std::io::copy(
            &mut pb_file.wrap_read(response.into_reader()),
            &mut BufWriter::new(&mut f),
        )
        .unwrap();
        pb_file.set_message(filing_id.clone());
        pb_files.inc(1);
    }
    pb_files.finish_and_clear();

    println!("Finished in {}", HumanDuration(Instant::now() - t0));
    Ok(())
}
