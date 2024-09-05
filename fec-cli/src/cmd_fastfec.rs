use fec_parser::Filing;
use indicatif::{ProgressBar, ProgressStyle};
use std::{collections::HashMap, error::Error, fs::File, io::Read, path::Path};

fn write_fastfec_compat<R: Read>(mut filing: Filing<R>, directory: &Path) {
    let mut csv_writers: HashMap<String, csv::Writer<File>> = HashMap::new();

    let pb_style = ProgressStyle::with_template(
      "{msg}.fec:\t[{elapsed_precise}] {bar:40.cyan/blue} {eta} {decimal_bytes_per_sec} {decimal_total_bytes} total",
  )
  .unwrap();
    let pb = ProgressBar::new(filing.source_length.unwrap() as u64).with_style(pb_style);

    while let Some(r) = filing.next_row() {
        let r = r.unwrap();
        pb.set_position(r.record.position().unwrap().byte());

        if let Some(w) = csv_writers.get_mut(&r.row_type) {
            w.write_record(&r.record.clone()).unwrap();
        } else {
            let f = File::create_new(directory.join(format!("{}.csv", r.row_type))).unwrap();
            let mut w = csv::WriterBuilder::new()
                .flexible(true)
                .has_headers(false)
                .from_writer(f);

            let column_names = fec_parser::mappings::column_names_for_field(
                &r.row_type,
                &filing.header.fec_version,
            )
            .unwrap();
            w.write_record(column_names).unwrap();
            w.write_record(&r.record.clone()).unwrap();
            csv_writers.insert(r.row_type, w);
        }
    }
}

pub fn cmd_fastfec_compat(filing_file: &str, output_directory: &str) -> Result<(), Box<dyn Error>> {
    let filing = Filing::<File>::from_path(Path::new(filing_file))?;
    let output_directory = Path::new(output_directory);
    write_fastfec_compat(filing, output_directory);
    Ok(())
}
