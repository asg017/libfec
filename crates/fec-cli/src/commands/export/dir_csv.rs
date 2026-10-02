use std::{
    collections::{hash_map::Entry, HashMap},
    fs::File,
    path::PathBuf,
};

use anyhow::Context;

use fec_parser::schedules::{form_type_schedule_type, ScheduleType};

use crate::{
    cli::ExportArgs,
    commands::export::sqlite::form_type_parse,
    sourcer::{FilingSourcer, ItemizationProgressBar},
    utils::rows::{export_columns, file_stem, legacy_name_delimiter, remap_row, UnmappedRows},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ItemizationKey {
    Schedule(ScheduleType),
    FormType(String),
}

struct ItemizationValue {
    writer: csv::Writer<File>,
    /// The columns of the file's header (after `filing_id`).
    columns: Vec<String>,
}

fn new_writer(path: PathBuf, header: &[String]) -> anyhow::Result<csv::Writer<File>> {
    let f = File::create_new(&path)
        .with_context(|| format!("Could not create {}", path.display()))?;
    let mut w = csv::WriterBuilder::new()
        .flexible(true)
        .has_headers(true)
        .from_writer(f);
    w.write_record(header)?;
    Ok(w)
}

/// One CSV per cover form and per schedule (or other row type), in the 8.5
/// layout whatever the filings' versions and order: a schedule's file has
/// the columns of its ordinary itemizations
/// ([`ScheduleType::column_names`]), a cover's or other row type's file
/// those of [`export_columns`]. Rows of other versions are rearranged by
/// column name (legacy combined names split, see [`remap_row`]); rows with no
/// known layout are skipped with a warning.
pub fn export(
    mut sourcer: FilingSourcer,
    args: ExportArgs,
    output_directory: PathBuf,
) -> anyhow::Result<()> {
    let mut cover_writers: HashMap<String, ItemizationValue> = HashMap::new();
    let mut itemization_writers: HashMap<ItemizationKey, ItemizationValue> = HashMap::new();
    let mb = indicatif::MultiProgress::new();
    let mut unmapped = UnmappedRows::new(Some(&mb));
    let (_trace, _input_mappings, iter) =
        sourcer.resolve_iterator_from_flags(args.filings, args.api, Some(&mb))?;

    for filing in iter {
        let mut filing = match filing {
            Ok(f) => f,
            Err(e) => {
                let _ = mb.println(format!("Error fetching filing, skipping: {e:?}"));
                continue;
            }
        };
        let fec_version = filing.header.fec_version.clone();

        {
            let (form_type, _amendment_indicator) = form_type_parse(&filing.cover.form_type);
            let form_type = form_type.to_ascii_uppercase();
            let entry = match cover_writers.entry(form_type.clone()) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => {
                    let columns = export_columns(&filing.cover.form_type, &fec_version)
                        .map(<[String]>::to_vec)
                        .unwrap_or_else(|| filing.cover.record_column_names.clone());
                    let path =
                        output_directory.join(format!("cover_{}.csv", file_stem(&form_type)));
                    let writer = new_writer(path, &columns)?;
                    e.insert(ItemizationValue { writer, columns })
                }
            };
            let fields = remap_row(
                &entry.columns,
                &filing.cover.record_column_names,
                &filing.cover.record,
                legacy_name_delimiter(&filing.header),
            );
            entry.writer.write_record(fields.iter().map(|f| f.as_bytes()))?;
        }

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

            let Ok(row_columns) =
                fec_parser::mappings::column_names_for_field(&row.row_type, &fec_version)
            else {
                unmapped.warn(&filing.filing_id, &row.row_type, &fec_version, "skipping them");
                continue;
            };

            let key = match form_type_schedule_type(row.row_type.as_str()) {
                Some(schedule_type) => ItemizationKey::Schedule(schedule_type),
                None => ItemizationKey::FormType(row.row_type.as_str().to_owned()),
            };
            let entry = match itemization_writers.entry(key) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => {
                    let (path, columns) = match e.key() {
                        ItemizationKey::Schedule(schedule_type) => (
                            output_directory
                                .join(format!("{}.csv", schedule_type.to_sqlite_tablename())),
                            schedule_type.column_names("8.5")?,
                        ),
                        ItemizationKey::FormType(form_type) => (
                            output_directory.join(format!("form_{}.csv", file_stem(form_type))),
                            export_columns(form_type, &fec_version)
                                .unwrap_or(row_columns.as_slice())
                                .to_vec(),
                        ),
                    };
                    let mut header = columns.clone();
                    header.insert(0, "filing_id".to_owned());
                    let writer = new_writer(path, &header)?;
                    e.insert(ItemizationValue { writer, columns })
                }
            };

            entry.writer.write_field(filing.filing_id.as_str())?;
            let fields = remap_row(
                &entry.columns,
                row_columns,
                &row.record,
                legacy_name_delimiter(&filing.header),
            );
            entry.writer.write_record(fields.iter().map(|f| f.as_bytes()))?;
        }
    }
    Ok(())
}
