use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use std::{
    error::Error,
    fs::File,
    io::BufWriter,
    time::{Duration, Instant},
};

lazy_static::lazy_static! {
  pub static ref BAR_FILES_STYLE: ProgressStyle =ProgressStyle::with_template(
    "{spinner} {pos}/{len} [{elapsed_precise}]",
  ).expect("valid progress style");
}
lazy_static::lazy_static! {
  pub static ref BAR_FILE_STYLE: ProgressStyle =indicatif::ProgressStyle::with_template(
    "{msg}.fec:\t[{elapsed_precise}] {bar:40.cyan/blue} {eta} {decimal_bytes_per_sec} {decimal_total_bytes} total",
  )
  .unwrap();
}

pub fn cmd_download(
    filings: Vec<String>,
    output_directory: Option<String>,
) -> Result<(), Box<dyn Error>> {
    let t0 = Instant::now();
    let mb = MultiProgress::new();
    let pb_files = mb.add(ProgressBar::new(filings.len() as u64));
    pb_files.set_style(BAR_FILES_STYLE.clone());
    pb_files.enable_steady_tick(Duration::from_millis(750));

    for filing in filings {
        let filing_id = filing
            .strip_prefix("FEC-")
            .or_else(|| filing.strip_prefix("FEC"))
            .unwrap_or(&filing)
            .to_owned();
        let request =
            ureq::get(format!("https://docquery.fec.gov/dcdev/posted/{filing_id}.fec").as_str());
        let response = request.call().unwrap();
        let length: usize = response.header("Content-Length").unwrap().parse().unwrap();
        let path = format!(
            "{}{filing_id}.fec",
            output_directory.clone().unwrap_or("".to_owned())
        );
        let mut f = File::create_new(&path).unwrap();
        //pb_files.set_message(filing_path.clone());
        let pb_file = mb.add(ProgressBar::new(length as u64));
        pb_file.set_style(BAR_FILE_STYLE.clone());
        std::io::copy(
            &mut pb_file.wrap_read(response.into_reader()),
            &mut BufWriter::new(&mut f),
        )
        .unwrap();
        pb_file.set_message(filing_id.clone());
        pb_files.inc(1);
        pb_files.println(format!("{filing_id} downloaded to {path}"));
    }
    pb_files.finish_and_clear();

    println!("Finished in {}", HumanDuration(Instant::now() - t0));
    Ok(())
}
