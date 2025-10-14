use std::{fs::File, io::Write, path::PathBuf};

use fec_parser::schedules::{form_type_schedule_type, ScheduleType};

use crate::{
    cli::{ExportArgs, ExportTarget},
    sourcer::{FilingSourcer, ItemizationProgressBar},
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
    Csv(csv::Writer<File>),
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
            let mut w = csv::WriterBuilder::new()
                .flexible(true)
                .has_headers(true)
                .from_writer(std::fs::File::create_new(&output_path)?);
            let mut columns: Vec<String> =
                Into::<ScheduleType>::into(target).column_names("8.5")?;
            columns.insert(0, "filing_id".to_owned());
            w.write_record(columns).expect("Writing CSV header");
            Writer::Csv(w)
        }
        SingleOutput::Json => {
            let mut f = File::create_new(&output_path)?;
            f.write(b"[")?;
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
    let iter = sourcer
        .resolve_iterator_from_flags(args.filings, args.api, Some(&mb))?
        .1;
    let mut nrows = 0;

    for filing in iter {
        let mut filing = match filing {
            Ok(f) => f,
            Err(_) => todo!(),
        };
        let pb = ItemizationProgressBar::new(&mb, &filing);
        let mut first = true;
        while let Some(r) = filing.next_row() {
            let row = r?;
            pb.update(&row);

            if target_matches_form_type(&target, row.row_type.as_str()) {
                nrows += 1;
                match &mut output {
                    Writer::Csv(w) => {
                        w.write_field(&filing.filing_id)?;
                        for field in row.record.iter() {
                            w.write_field(field)?;
                        }
                        w.write_record(None::<&[u8]>)?;
                    }
                    Writer::Json { file, column_names } => {
                        if first {
                            first = false;
                        } else {
                            file.write(b",")?;
                        }
                        let mut record = serde_json::map::Map::new();
                        record.insert(
                            "filing_id".to_owned(),
                            serde_json::Value::String(filing.filing_id.clone()),
                        );
                        for (i, field) in row.record.iter().enumerate() {
                            record.insert(
                                column_names[i + 1].clone(),
                                serde_json::Value::String(field.to_string()),
                            );
                        }
                        let value = serde_json::Value::Object(record);
                        let s = serde_json::to_string(&value)?;
                        file.write(s.as_bytes())?;
                    }
                }
            }
        }
    }
    if let Writer::Json { file, .. } = &mut output {
        file.write(b"]")?;
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
