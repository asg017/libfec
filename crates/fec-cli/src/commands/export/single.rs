use std::{fs::File, io::Write, path::PathBuf};

use fec_parser::schedules::{form_type_schedule_type, ScheduleType};

use crate::{
    cli::{ExportArgs, ExportTarget},
    sourcer::{FilingSourcer, ItemizationProgressBar},
    utils::rows::{all_filings_failed, warn},
};

pub enum SingleOutput {
    Csv,
    Json,
}

fn target_matches_form_type(target: &ExportTarget, form_type: &str) -> bool {
    match form_type_schedule_type(form_type) {
        Some(ScheduleType::ScheduleA) => matches!(target, ExportTarget::ScheduleA),
        Some(ScheduleType::ScheduleB) => matches!(target, ExportTarget::ScheduleB),
        _ => false,
    }
}

enum Writer {
    Csv {
        writer: Box<csv::Writer<File>>,
        nrecords: usize,
    },
    Json {
        file: File,
        column_names: Vec<String>,
    },
}
pub fn cmd_export_single(
    mut sourcer: FilingSourcer,
    output_path: PathBuf,
    args: ExportArgs,
    target: ExportTarget,
    output_type: SingleOutput,
) -> anyhow::Result<()> {
    let t0 = jiff::Timestamp::now();
    let mut output = match output_type {
        SingleOutput::Csv => {
            let mut writer = csv::WriterBuilder::new()
                .flexible(true)
                .has_headers(true)
                .from_writer(std::fs::File::create_new(&output_path)?);
            let mut columns: Vec<String> =
                Into::<ScheduleType>::into(target).column_names("8.5")?;
            columns.insert(0, "filing_id".to_owned());
            let nrecords = columns.len();
            writer.write_record(columns).expect("Writing CSV header");
            Writer::Csv {
                writer: Box::new(writer),
                nrecords,
            }
        }
        SingleOutput::Json => {
            let mut f = File::create_new(&output_path)?;
            f.write_all(b"[")?;
            let mut columns: Vec<String> =
                Into::<ScheduleType>::into(target).column_names("8.5")?;
            columns.insert(0, "filing_id".to_owned());
            Writer::Json {
                file: f,
                column_names: columns,
            }
        }
    };

    let mb = indicatif::MultiProgress::new();
    let (_trace, _input_mappings, iter) =
        sourcer.resolve_iterator_from_flags(args.filings, args.api, Some(&mb))?;
    let mut nrows = 0;
    // Shared across filings: every row after the first needs a comma.
    let mut first = true;
    let (mut read, mut failed) = (0usize, 0usize);

    for filing in iter {
        let mut filing = match filing {
            Ok(f) => f,
            Err(e) => {
                warn(Some(&mb), format!("Error fetching filing, skipping: {e:?}"));
                failed += 1;
                continue;
            }
        };
        read += 1;
        let pb = ItemizationProgressBar::new(&mb, &filing);
        while let Some(r) = filing.next_row() {
            let row = match r {
                Ok(row) => row,
                Err(e) => {
                    warn(
                        Some(&mb),
                        format!(
                            "warning: FEC-{}: skipping unreadable row: {e}",
                            filing.filing_id
                        ),
                    );
                    continue;
                }
            };
            pb.update(&row);

            if target_matches_form_type(&target, row.row_type.as_str()) {
                nrows += 1;
                match &mut output {
                    Writer::Csv { writer, nrecords } => {
                        writer.write_field(&filing.filing_id)?;
                        // TODO: check if theres non-empty rows beyond nrecords - 1
                        for field in row.record.iter().take(*nrecords - 1) {
                            writer.write_field(field)?;
                        }
                        writer.write_record(None::<&[u8]>)?;
                    }
                    Writer::Json { file, column_names } => {
                        if first {
                            first = false;
                        } else {
                            file.write_all(b",")?;
                        }
                        let mut record = serde_json::map::Map::new();
                        record.insert(
                            "filing_id".to_owned(),
                            serde_json::Value::String(filing.filing_id.clone()),
                        );
                        // Like the CSV writer, drop fields beyond the 8.5 layout.
                        for (name, field) in column_names.iter().skip(1).zip(row.record.iter()) {
                            record
                                .insert(name.clone(), serde_json::Value::String(field.to_string()));
                        }
                        let value = serde_json::Value::Object(record);
                        let s = serde_json::to_string(&value)?;
                        file.write_all(s.as_bytes())?;
                    }
                }
            }
        }
    }
    if let Writer::Json { file, .. } = &mut output {
        file.write_all(b"]")?;
    }
    if read == 0 && failed > 0 {
        return Err(all_filings_failed(failed));
    }
    let duration = jiff::Timestamp::now() - t0;
    eprintln!(
        "Exported {} rows to {} in {:.2?}",
        nrows,
        output_path.display(),
        duration
    );
    Ok(())
}
