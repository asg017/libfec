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
use fec_parser::{Filing, FilingHeader, HeaderStyle};
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    fs::File,
    io::Read,
    path::Path,
    sync::LazyLock,
};

use crate::{
    cli::FastFecArgs,
    sourcer::FilingSourcer,
    utils::rows::{file_stem, UnmappedRows},
};

static STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::with_template(
    "{msg}.fec:\t[{elapsed_precise}] {bar:40.cyan/blue} {eta} {decimal_bytes_per_sec} {decimal_total_bytes} total",
  )
  .expect("valid progress style")
});

/// `header.csv`: column names from the version's `hdr` mapping (paper and
/// 3.x–5.x layouts differ from 6.x+), or for a `/* Header` block its keys
/// (lowercased, as FastFEC writes them) followed by one
/// `SCHEDULE_COUNTS_<row type>` column per schedule count.
fn header_rows(header: &FilingHeader) -> (Vec<String>, Vec<String>) {
    if header.style == HeaderStyle::LegacyBlock {
        let mut names: Vec<String> = header
            .legacy_fields
            .keys()
            .map(|k| k.to_ascii_lowercase())
            .collect();
        let mut values: Vec<String> = header.legacy_fields.values().cloned().collect();
        for (k, v) in &header.schedule_counts {
            names.push(format!("SCHEDULE_COUNTS_{}", k.to_ascii_lowercase()));
            values.push(v.clone());
        }
        return (names, values);
    }
    let names = match fec_parser::mappings::column_names_for_field("hdr", &header.fec_version) {
        Ok(names) => names.clone(),
        // Unreachable for a header the parser accepted; keep the 8.x layout.
        Err(_) => [
            "record_type",
            "ef_type",
            "fec_version",
            "soft_name",
            "soft_ver",
            "report_id",
            "report_number",
            "comment",
        ]
        .map(String::from)
        .to_vec(),
    };
    let opt = |v: &Option<String>| v.clone().unwrap_or_default();
    let values = names
        .iter()
        .map(|name| match name.as_str() {
            "record_type" => header.record_type.clone(),
            "ef_type" => header.ef_type.clone(),
            "fec_version" => header.fec_version.clone(),
            "soft_name" => header.software_name.clone(),
            "soft_ver" => header.software_version.clone(),
            "name_delim" => opt(&header.name_delimiter),
            "report_id" => opt(&header.report_id),
            "report_number" => opt(&header.report_number),
            "comment" => opt(&header.comment),
            "batch_number" => opt(&header.batch_number),
            "received_date" => opt(&header.received_date),
            _ => String::new(),
        })
        .collect();
    (names, values)
}

fn write_header_csv<R: Read>(filing: &Filing<R>, header_csv_path: &Path) -> anyhow::Result<()> {
    let f = File::create_new(header_csv_path)?;
    let mut w = csv::WriterBuilder::new()
        .flexible(true)
        .has_headers(false)
        .from_writer(f);

    let (names, values) = header_rows(&filing.header);
    w.write_record(names)?;
    w.write_record(values)?;
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
    let mut seen_row_types: HashSet<String> = HashSet::new();
    let mut unmapped = UnmappedRows::new(None);
    let pb = ProgressBar::new(filing.source_length as u64).with_style(STYLE.clone());
    pb.set_message(filing.filing_id.to_owned());

    let filing_directory = directory.join(filing.filing_id.clone());
    std::fs::create_dir_all(&filing_directory)?;

    write_header_csv(&filing, &filing_directory.join("header.csv"))?;
    write_cover_csv(
        &filing,
        &filing_directory.join(format!("{}.csv", file_stem(&filing.cover.form_type))),
    )?;

    while let Some(r) = filing.next_row() {
        let r = r.context("Error reading next row")?;
        pb.set_position(r.byte_offset);

        // Writers are keyed by file name: several row types can share a
        // file stem (`SC/10` and a literal `SC-10` are both `SC-10.csv`),
        // and two buffered writers on one file interleave their output.
        // Their rows go to one file, under the header of the first.
        let w = match csv_writers.entry(file_stem(&r.row_type)) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let path = filing_directory.join(format!("{}.csv", e.key()));
                // The cover's file, when a row type shares its stem.
                let existed = path.exists();
                let f = File::options().create(true).append(true).open(&path)?;
                let mut w = csv::WriterBuilder::new()
                    .flexible(true)
                    .has_headers(false)
                    .from_writer(f);
                // Like FastFEC, rows of an unmapped type are written without
                // a header line.
                if let (false, Ok(column_names)) = (
                    existed,
                    fec_parser::mappings::column_names_for_field(
                        &r.row_type,
                        &filing.header.fec_version,
                    ),
                ) {
                    w.write_record(column_names)?;
                }
                e.insert(w)
            }
        };
        if seen_row_types.insert(r.row_type.clone())
            && fec_parser::mappings::column_names_for_field(&r.row_type, &filing.header.fec_version)
                .is_err()
        {
            unmapped.warn(
                &filing.filing_id,
                &r.row_type,
                &filing.header.fec_version,
                "writing its rows without a header line",
            );
        }
        w.write_record(&r.record)?;
    }
    Ok(())
}

pub fn fastfec(sourcer: FilingSourcer, args: FastFecArgs) -> anyhow::Result<()> {
    let filing = sourcer.resolve_from_user_argument(&args.filing_id)?;
    std::fs::create_dir_all(&args.output_directory)?;
    write_fastfec_compat(filing, &args.output_directory)?;
    Ok(())
}
