use std::{fs::File, io::Write, path::PathBuf};

use fec_parser::{
    mappings::column_names_for_field,
    schedules::{form_type_schedule_type, ScheduleType},
};

use crate::{
    cli::{ExportArgs, ExportTarget},
    sourcer::{FilingSourcer, ItemizationProgressBar},
    utils::rows::{legacy_name_delimiter, remap_row, UnmappedRows},
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
    },
    Json {
        file: File,
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
    // Every row is written in the 8.5 layout; rows of other versions are
    // rearranged by column name, legacy combined names split (see `remap_row`).
    let columns: Vec<String> = Into::<ScheduleType>::into(target).column_names("8.5")?;
    let mut output = match output_type {
        SingleOutput::Csv => {
            let mut writer = csv::WriterBuilder::new()
                .flexible(true)
                .has_headers(true)
                .from_writer(std::fs::File::create_new(&output_path)?);
            writer.write_field("filing_id")?;
            writer.write_record(&columns)?;
            Writer::Csv {
                writer: Box::new(writer),
            }
        }
        SingleOutput::Json => {
            let mut f = File::create_new(&output_path)?;
            f.write_all(b"[")?;
            Writer::Json { file: f }
        }
    };

    let mb = indicatif::MultiProgress::new();
    let (_trace, _input_mappings, iter) =
        sourcer.resolve_iterator_from_flags(args.filings, args.api, Some(&mb))?;
    let mut nrows = 0;
    let mut unmapped = UnmappedRows::new(Some(&mb));
    let mut first = true;

    for filing in iter {
        let mut filing = match filing {
            Ok(f) => f,
            Err(e) => {
                let _ = mb.println(format!("Error fetching filing, skipping: {e:?}"));
                continue;
            }
        };
        let pb = ItemizationProgressBar::new(&mb, &filing);
        while let Some(r) = filing.next_row() {
            let row = match r {
                Ok(row) => row,
                Err(e) => {
                    let _ = mb.println(format!(
                        "warning: FEC-{}: skipping unreadable row: {e}",
                        filing.filing_id
                    ));
                    continue;
                }
            };
            pb.update(&row);

            if target_matches_form_type(&target, row.row_type.as_str()) {
                let fec_version = &filing.header.fec_version;
                let Ok(row_columns) = column_names_for_field(&row.row_type, fec_version) else {
                    unmapped.warn(&filing.filing_id, &row.row_type, fec_version, "skipping them");
                    continue;
                };
                let fields = remap_row(
                    &columns,
                    row_columns,
                    &row.record,
                    legacy_name_delimiter(&filing.header),
                );
                nrows += 1;
                match &mut output {
                    Writer::Csv { writer } => {
                        writer.write_field(&filing.filing_id)?;
                        writer.write_record(fields.iter().take(columns.len()).map(|f| f.as_bytes()))?;
                    }
                    Writer::Json { file } => {
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
                        for (name, field) in columns.iter().zip(fields) {
                            record.insert(
                                name.clone(),
                                serde_json::Value::String(field.into_owned()),
                            );
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
    let duration = jiff::Timestamp::now() - t0;
    eprintln!(
        "Exported {} rows to {} in {:.2?}",
        nrows,
        output_path.display(),
        duration
    );
    Ok(())
}
