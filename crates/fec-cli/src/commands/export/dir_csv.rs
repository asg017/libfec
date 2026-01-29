use std::{collections::HashMap, fs::File, path::PathBuf};

use fec_parser::schedules::{form_type_schedule_type, ScheduleType};

use crate::{
    cli::ExportArgs,
    commands::export::sqlite::form_type_parse,
    sourcer::{FilingSourcer, ItemizationProgressBar},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ItemizationKey {
    Schedule(ScheduleType),
    FormType(String),
}

struct ItemizationValue {
    writer: csv::Writer<File>,
}

pub fn export(
    mut sourcer: FilingSourcer,
    args: ExportArgs,
    output_directory: PathBuf,
) -> anyhow::Result<()> {
    let mut cover_writers: std::collections::HashMap<String, ItemizationValue> = HashMap::new();
    let mut itemization_writers: std::collections::HashMap<ItemizationKey, ItemizationValue> =
        HashMap::new();
    let mb = indicatif::MultiProgress::new();
    let (_trace, _input_mappings, iter) = sourcer
        .resolve_iterator_from_flags(args.filings, args.api, Some(&mb))?;

    for filing in iter {
        let mut filing = match filing {
            Ok(f) => f,
            Err(_) => todo!(),
        };

        {
            let (form_type, _amendment_indicator) = form_type_parse(&filing.cover.form_type);
            cover_writers
                .entry(form_type.to_owned())
                .or_insert_with(|| {
                    let path = output_directory.join(format!("cover_{}.csv", form_type));
                    let f = File::create_new(path).unwrap();
                    let mut w = csv::WriterBuilder::new()
                        .flexible(true)
                        .has_headers(true)
                        .from_writer(f);
                    w.write_record(filing.cover.record_column_names.clone())
                        .unwrap();
                    ItemizationValue { writer: w }
                })
                .writer
                .write_record(&filing.cover.record)
                .unwrap();
        }

        let pb = ItemizationProgressBar::new(&mb, &filing);
        while let Some(r) = filing.next_row() {
            let row = r?;
            pb.update(&row);

            let key = match form_type_schedule_type(row.row_type.as_str()) {
                Some(schedule_type) => ItemizationKey::Schedule(schedule_type),
                None => ItemizationKey::FormType(row.row_type.as_str().to_owned()),
            };
            let writer = &mut itemization_writers
                .entry(key.clone())
                .or_insert_with(|| {
                    let path = match key.clone() {
                        ItemizationKey::Schedule(schedule_type) => output_directory
                            .join(format!("{}.csv", schedule_type.to_sqlite_tablename())),
                        ItemizationKey::FormType(form_type) => {
                            output_directory.join(format!("form_{}.csv", form_type))
                        }
                    };
                    let f = File::create_new(path).unwrap();
                    let mut w = csv::WriterBuilder::new()
                        .flexible(true)
                        .has_headers(true)
                        .from_writer(f);
                    let mut column_names = fec_parser::mappings::column_names_for_field(
                        &row.row_type,
                        &filing.header.fec_version,
                    )
                    .unwrap()
                    .to_owned();
                    column_names.insert(0, "filing_id".to_owned());
                    w.write_record(column_names).expect("Writing CSV header");
                    ItemizationValue { writer: w }
                })
                .writer;

            writer.write_field(filing.filing_id.as_str())?;
            for field in &row.record {
                writer.write_field(field)?;
            }
            writer.write_record(None::<&[u8]>)?;
        }
    }
    Ok(())
}
