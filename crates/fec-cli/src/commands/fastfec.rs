use anyhow::Context;
/**
 * Export FEC filing data in a format compatible with FastFEC.
 * FastFEC reads in single FEC file and writes multiple CSV files
 * in the following directory structure:
 *
 *   <filing_id>/
 *      header.csv
 *      <cover_record>.csv
 *      <...form_type.>.csv
 *
 * where <...form_type> are all the itemization form types for the filing (" SA15A", "F3X", etc.)
 *
 * Reference: https://github.com/washingtonpost/FastFEC
 */
use fec_parser::Filing;
use indicatif::{ProgressBar, ProgressStyle};
use std::{collections::HashMap, fs::File, io::Read, path::Path, sync::LazyLock};

use crate::{cli::FastFecArgs, sourcer::FilingSourcer};

static STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::with_template(
    "{msg}.fec:\t[{elapsed_precise}] {bar:40.cyan/blue} {eta} {decimal_bytes_per_sec} {decimal_total_bytes} total",
  )
  .expect("valid progress style")
});

fn write_header_csv<R: Read>(filing: &Filing<R>, header_csv_path: &Path) -> anyhow::Result<()> {
    let f = File::create_new(header_csv_path)?;
    let mut w = csv::WriterBuilder::new()
        .flexible(true)
        .has_headers(false)
        .from_writer(f);

    w.write_record(&[
        "record_type",
        "ef_type",
        "fec_version",
        "soft_name",
        "soft_ver",
        "report_id",
        "report_number",
        "comment",
    ])?;
    w.write_record(vec![
        filing.header.record_type.clone(),
        filing.header.ef_type.clone(),
        filing.header.fec_version.clone(),
        filing.header.software_name.clone(),
        filing.header.software_version.clone(),
        filing.header.report_id.clone().unwrap_or_default(),
        filing.header.report_number.clone().unwrap_or("".to_owned()),
        filing.header.comment.clone().unwrap_or_default(),
    ])?;
    Ok(())
}

fn write_cover_csv<R: Read>(filing: &Filing<R>, cover_csv_path: &Path) -> anyhow::Result<()> {
    let f = File::create_new(cover_csv_path)?;
    let mut w = csv::WriterBuilder::new()
        .flexible(true)
        .has_headers(false)
        .from_writer(f);
    w.write_record(&filing.cover.record_column_names)?;
    w.write_record(&filing.cover.record.clone())?;
    Ok(())
}

fn write_fastfec_compat<R: Read>(mut filing: Filing<R>, directory: &Path) -> anyhow::Result<()> {
    let mut csv_writers: HashMap<String, csv::Writer<File>> = HashMap::new();
    let pb = ProgressBar::new(filing.source_length as u64).with_style(STYLE.clone());
    pb.set_message(filing.filing_id.to_owned());

    let filing_directory = directory.join(filing.filing_id.to_string());
    std::fs::create_dir_all(&filing_directory)?;

    write_header_csv(&filing, &filing_directory.join("header.csv"))?;
    write_cover_csv(
        &filing,
        &filing_directory.join(format!("{}.csv", filing.cover.form_type)),
    )?;

    while let Some(r) = filing.next_row() {
        let r = r.context("Error reading next row")?;
        pb.set_position(
            r.record
                .position()
                .expect("CSV position to be available")
                .byte(),
        );

        if let Some(w) = csv_writers.get_mut(&r.row_type) {
            w.write_record(&r.record.clone())?;
        } else {
            let f = File::create_new(filing_directory.join(format!("{}.csv", r.row_type)))?;
            let mut w = csv::WriterBuilder::new()
                .flexible(true)
                .has_headers(false)
                .from_writer(f);

            let column_names = fec_parser::mappings::column_names_for_field(
                &r.row_type,
                &filing.header.fec_version,
            )?;
            w.write_record(column_names)?;
            w.write_record(&r.record.clone())?;
            csv_writers.insert(r.row_type, w);
        }
    }
    Ok(())
}

pub fn fastfec(sourcer: FilingSourcer, args: FastFecArgs) -> anyhow::Result<()> {
    let filing = sourcer.resolve_from_user_argument(&args.filing_id)?;
    std::fs::create_dir_all(&args.output_directory)?;
    write_fastfec_compat(filing, &args.output_directory)?;
    Ok(())
}
