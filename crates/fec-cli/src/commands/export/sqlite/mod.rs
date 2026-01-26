use crate::{
    cache::bulk_candidates,
    cli::ExportArgs,
    sourcer::{FilingSourcer, ItemizationProgressBar},
};
use anyhow::Context;
use colored::Colorize;
use csv::StringRecord;
use fec_parser::{
    mappings::{DATE_COLUMNS, FLOAT_COLUMNS},
    schedules::{form_type_schedule_type, ScheduleType},
    try_format_fec_date, Filing, FilingRow,
};
use indicatif::{HumanDuration, MultiProgress};
use rusqlite::{
    params_from_iter,
    types::{ToSqlOutput, Value},
    Connection, Statement, ToSql, Transaction,
};
use std::{collections::HashMap, hash::Hash, io::Read, path::PathBuf, time::Instant};

const CREATE_FILINGS_SQL: &str = r#"
  CREATE TABLE IF NOT EXISTS libfec_filings(
    /**
     * 
     * 
     */

    --- Unique numeric identifier for this filing, assigned by the FEC, ex 1884420
    filing_id TEXT PRIMARY KEY NOT NULL,
    
    --- Version of the FEC filing format, ex '8.4'
    fec_version TEXT NOT NULL,
    
    --- Name of the software that produced this filing, ex 'NetFile'
    software_name TEXT NOT NULL,

    --- Version of the software that produced this filing, ex '2022451'
    software_version TEXT NOT NULL,

    --- If this filing is an amendment, the report_id of the original filing, otherwise null. ex 1884419
    report_id TEXT,

    --- Sequential number of amendments
    report_number TEXT,

    --- Any header comments provided by the filer
    comment TEXT,

    --- Form type of the cover record, ex 'F3'
    cover_record_form TEXT NOT NULL,
    cover_record_form_amendment_indicator TEXT,

    filer_id TEXT NOT NULL,
    filer_name TEXT NOT NULL,
    report_code TEXT,
    coverage_from_date TEXT,
    coverage_through_date TEXT
  )
"#;

const INSERT_FILING_SQL: &str = r#"
  INSERT INTO libfec_filings VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?)
"#;

#[derive(Clone, Copy)]
enum FieldFormat {
    Text,
    Float,
    Date,
}

#[derive(Clone)]
enum FieldValue {
    Text(String),
    Float(f64),
    Date(String),
}

impl ToSql for FieldValue {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        match self {
            FieldValue::Text(v) => Ok(ToSqlOutput::Owned(Value::Text(v.to_owned()))),
            FieldValue::Date(v) => Ok(ToSqlOutput::Owned(Value::Text(v.to_owned()))),
            FieldValue::Float(v) => Ok(ToSqlOutput::Owned(Value::Real(*v))),
        }
    }
}

struct Warning {
    _message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ItemizationKey {
    Schedule(ScheduleType),
    FormType(String),
}

struct ItemizationValue<'a> {
    record_table: RecordTable,
    insert_statement: Statement<'a>,
}

struct RecordTable {
    column_names: Vec<String>,
    column_types: Vec<FieldFormat>,
    suffix: String,
    current_mapping: Vec<String>,
    legacy_field_mapping: HashMap<String, Vec<Option<usize>>>,
}

const LATEST_FEC_VERSION: &str = "8.5";
impl RecordTable {
    fn new(row_type: &str, suffix: &str) -> anyhow::Result<Self> {
        let column_names =
            fec_parser::mappings::column_names_for_field(row_type, LATEST_FEC_VERSION)
                // some forms were *removed* in  8.5, like F3Z1, ex FEC-1890921. TODO Need a better fallback strategy
                .or_else(|_| fec_parser::mappings::column_names_for_field(row_type, "8.4"))
                .with_context(|| {
                    format!(
                        "Error getting mapping column names for field '{}' and fec version '{}'",
                        row_type, LATEST_FEC_VERSION
                    )
                })?
                .to_owned();
        let current_mapping = column_names.clone();
        let column_types: Vec<FieldFormat> = column_names
            .iter()
            .map(|c| {
                if DATE_COLUMNS.contains(c) {
                    FieldFormat::Date
                } else if FLOAT_COLUMNS.contains(c) {
                    FieldFormat::Float
                } else {
                    FieldFormat::Text
                }
            })
            .collect();

        Ok(RecordTable {
            column_names,
            column_types,
            suffix: suffix.to_owned(),
            current_mapping,
            legacy_field_mapping: HashMap::new(),
        })
    }
    fn create_sql(&self) -> String {
        let columns_defs: Vec<String> = self
            .column_names
            .iter()
            .zip(self.column_types.clone())
            .map(|(name, format)| {
                format!(
                    "{} {}",
                    name,
                    match format {
                        FieldFormat::Date => "date",
                        FieldFormat::Text => "text",
                        FieldFormat::Float => "float",
                    }
                )
            })
            .collect();

        let mut sql = String::from("CREATE TABLE IF NOT EXISTS [libfec_");
        sql += &self.suffix;
        sql += "](\n  ";
        sql += "filing_id text references libfec_filings(filing_id),\n  ";
        sql += columns_defs.join(",\n  ").as_str();
        sql += "\n)";
        sql
    }
    fn insert_statement<'a>(&self, tx: &'a Transaction) -> anyhow::Result<Statement<'a>> {
        let sql = format!(
            "INSERT INTO libfec_{} VALUES ({})",
            self.suffix,
            vec!["?"; self.column_names.len() + 1].join(",")
        );
        tx.prepare(&sql).with_context(|| {
            format!(
                "Error preparing insert statement for record table libfec_{}",
                self.suffix
            )
        })
    }

    fn insert(
        &mut self,
        insert_stmt: &mut Statement,
        filing_id: &str,
        record: &StringRecord,
        fec_version: &str,
    ) -> anyhow::Result<()> {
        let params = self.prep_row(
            filing_id,
            record,
            insert_stmt.parameter_count(),
            fec_version,
        );
        insert_stmt.execute(params_from_iter(params))?;
        Ok(())
    }
    fn prep_row(
        &mut self,
        filing_id: &str,
        record: &StringRecord,
        n_params: usize,
        fec_version: &str,
    ) -> Vec<FieldValue> {
        let mut warnings = Vec::new();
        let mut values: Vec<FieldValue> = if fec_version == LATEST_FEC_VERSION {
            record
                .iter()
                .enumerate()
                .map(|(idx, field)| match self.column_types.get(idx) {
                    Some(FieldFormat::Text) => FieldValue::Text(field.to_owned()),
                    Some(FieldFormat::Date) => match field.len() {
                        8 => FieldValue::Date(try_format_fec_date(field)),
                        _ => FieldValue::Text(field.to_owned()),
                    },
                    Some(FieldFormat::Float) => match field.parse::<f64>() {
                        Ok(value) => FieldValue::Float(value),
                        Err(_) => FieldValue::Text(field.to_owned()),
                    },
                    None => FieldValue::Text(field.to_owned()),
                })
                .collect()
        } else {
            let legacy_mapping = self
                .legacy_field_mapping
                .entry(fec_version.to_owned())
                .or_insert_with(|| {
                    let legacy_columns =
                        fec_parser::mappings::column_names_for_field(&record[0], fec_version)
                            .unwrap()
                            .to_owned();
                    self.current_mapping
                        .iter()
                        .map(|col_name| legacy_columns.iter().position(|c| c == col_name))
                        .collect::<Vec<Option<usize>>>()
                });
            legacy_mapping
                .iter()
                .enumerate()
                .map(|(idx, &record_idx)| {
                    let field = record_idx
                        .map(|i| record.get(i).unwrap_or(""))
                        .unwrap_or("");
                    match self.column_types.get(idx) {
                        Some(FieldFormat::Text) => FieldValue::Text(field.to_owned()),
                        Some(FieldFormat::Date) => match field.len() {
                            8 => FieldValue::Date(try_format_fec_date(field)),
                            _ => FieldValue::Text(field.to_owned()),
                        },
                        Some(FieldFormat::Float) => match field.parse::<f64>() {
                            Ok(value) => FieldValue::Float(value),
                            Err(_) => FieldValue::Text(field.to_owned()),
                        },
                        None => FieldValue::Text(field.to_owned()),
                    }
                })
                .collect()
        };

        // make sure filing_id is the first column
        values.insert(0, FieldValue::Text(filing_id.to_owned()));

        if values.len() == n_params + 1 {
            values.pop();
        }

        while values.len() < n_params {
            values.push(FieldValue::Text("".to_owned()));
        }
        if values.len() > n_params {
            warnings.push(Warning {
                _message: format!(
                    "Warning too long at {}:{}, {} vs {}!",
                    filing_id,
                    record.position().map(|p| p.line()).unwrap_or(0),
                    values.len(),
                    n_params
                ),
            });
            values.truncate(n_params);
        }
        values
    }
}

fn prepare_schedule_statement<'a>(
    row: &FilingRow,
    schedule_type: &ScheduleType,
    tx: &'a Transaction,
) -> anyhow::Result<ItemizationValue<'a>> {
    let record_table: RecordTable =
        RecordTable::new(&row.row_type, schedule_type.to_sqlite_tablename().as_str())?;
    tx.execute(&record_table.create_sql(), [])?;
    let insert_statement = record_table.insert_statement(tx)?;
    Ok(ItemizationValue {
        record_table,
        insert_statement,
    })
}

fn prepare_form_type_statement<'a>(
    row: &FilingRow,
    form_type: &str,
    tx: &'a Transaction,
) -> anyhow::Result<ItemizationValue<'a>> {
    let record_table: RecordTable = RecordTable::new(&row.row_type, form_type)?;
    tx.execute(&record_table.create_sql(), [])?;
    let insert_statement = record_table.insert_statement(tx)?;
    Ok(ItemizationValue {
        record_table,
        insert_statement,
    })
}

fn export_itemizations<R: Read>(
    tx: &mut Transaction,
    mut filing: Filing<R>,
    pb: Option<&ItemizationProgressBar>,
) -> anyhow::Result<usize> {
    let mut itemizations_statements: HashMap<ItemizationKey, ItemizationValue> = HashMap::new();
    let mut count = 0;

    while let Some(r) = filing.next_row() {
        let r = r.context("Error reading next row from filing")?;
        if let Some(pb) = pb {
            pb.update(&r)
        }
        count += 1;

        let key = match form_type_schedule_type(&r.row_type) {
            Some(schedule_type) => ItemizationKey::Schedule(schedule_type),
            None => ItemizationKey::FormType(r.row_type.clone()),
        };

        // We need to handle the or_insert_with error case, but HashMap::Entry doesn't
        // support fallible closures directly. So we check if we need to insert first.
        if !itemizations_statements.contains_key(&key) {
            let value = match &key {
                ItemizationKey::Schedule(schedule_type) => {
                    prepare_schedule_statement(&r, schedule_type, tx).with_context(|| {
                        format!(
                            "Error preparing statement for schedule type {:?}",
                            schedule_type
                        )
                    })?
                }
                ItemizationKey::FormType(form_type) => {
                    prepare_form_type_statement(&r, form_type, tx).with_context(|| {
                        format!("Error preparing statement for form type {}", form_type)
                    })?
                }
            };
            itemizations_statements.insert(key.clone(), value);
        }

        let v = itemizations_statements.get_mut(&key).unwrap(); // Safe: we just inserted it above
        v.record_table
            .insert(
                &mut v.insert_statement,
                &filing.filing_id,
                &r.record,
                &filing.header.fec_version,
            )
            .with_context(|| {
                format!(
                    "Error inserting itemization row for filing FEC-{}, line {}",
                    &filing.filing_id,
                    r.record.position().map(|p| p.line()).unwrap_or(0),
                )
            })?;
    }
    Ok(count)
}

pub(crate) fn form_type_parse(form_type: &str) -> (&str, Option<&str>) {
    if let Some(stripped) = form_type.strip_suffix('A') {
        (stripped, Some("A"))
    } else if let Some(stripped) = form_type.strip_suffix('N') {
        (stripped, Some("N"))
    } else if let Some(stripped) = form_type.strip_suffix('T') {
        (stripped, Some("T"))
    } else {
        (form_type, None)
    }
}

// insert row into libfec_filings and the summary record table
fn insert_filing_metadata(tx: &mut Transaction, filing: &Filing<impl Read>) -> anyhow::Result<()> {
    let (form_type, amendment_indicator) = form_type_parse(&filing.cover.form_type);
    tx.execute(
        INSERT_FILING_SQL,
        rusqlite::params![
            &filing.filing_id,
            &filing.header.fec_version,
            &filing.header.software_name,
            &filing.header.software_version,
            &filing.header.report_id,
            &filing.header.report_number,
            &filing.header.comment,
            form_type,
            amendment_indicator,
            &filing.cover.filer_id,
            &filing.cover.filer_name,
            &filing.cover.report_code.clone(),
            &filing.cover.coverage_from_date.clone(),
            &filing.cover.coverage_through_date.clone(),
        ],
    )?;

    let (form_type, _amendment_indicator) = form_type_parse(&filing.cover.form_type);

    let mut rt = RecordTable::new(
        &filing.cover.form_type,
        // strip any lagging "A", "N", or "T", or return form_type
        form_type,
    )
    .with_context(|| {
        format!(
            "Error creating record table for form type '{}' and filing FEC-{}",
            &filing.cover.form_type, &filing.filing_id
        )
    })?;
    tx.execute(&rt.create_sql(), [])?;
    let mut stmt = rt.insert_statement(tx)?;
    rt.insert(
        &mut stmt,
        &filing.filing_id,
        &filing.cover.record,
        &filing.header.fec_version,
    )?;
    Ok(())
}

fn init(tx: &mut Transaction) -> anyhow::Result<()> {
    tx.execute(CREATE_FILINGS_SQL, [])
        .context("Error initializing filing schema")?;
    Ok(())
}

pub fn cmd_export_sqlite(
    mut sourcer: FilingSourcer,
    path: PathBuf,
    args: ExportArgs,
) -> anyhow::Result<()> {
    let t0 = Instant::now();

    let mut db = Connection::open(&path)
        .context(format!("Could not open or create database at {:?}", path))?;
    let mb = MultiProgress::new();

    let mut tx = db
        .transaction()
        .context("Error starting SQLite transaction")?;

    init(&mut tx)?;

    let p = sourcer.cache.bulk_data_database_path();
    let (trace, iter) =
        sourcer.resolve_iterator_from_flags(args.filings, args.api.clone(), Some(&mb))?;

    for params in trace.resolve_candidate_params {
        bulk_candidates::include(&mut tx, p.clone(), &params).unwrap();
    }

    let mut nfilings = 0;

    mb.println(format!(
        "Exporting filings to SQLite database at {:?}",
        path
    ))?;

    for filing in iter {
        let filing = match filing {
            Ok(f) => f,
            Err(e) => {
                if let Some(fec_403) = e.downcast_ref::<crate::sourcer::FecGov403Error>() {
                    eprintln!(
                        "Error fetching filing {} from {}: HTTP 403 Forbidden. This filing may no longer be publicly accessible.",
                        fec_403.filing_id.to_human_readable(), fec_403.url
                    );
                    // TODO save warning somewhere
                    continue;
                } else {
                    let _ = mb.println(format!("Error fetching filing: {:?}", e));
                    todo!();
                }
            }
        };
        nfilings += 1;
        match insert_filing_metadata(&mut tx, &filing) {
            Ok(_) => {}
            Err(e) => {
                let _ = mb.println(format!(
                    "Error inserting filing metadata for FEC-{}: {:?}",
                    filing.filing_id, e
                ));
                continue;
            }
        }

        if !args.cover_only {
            let pb = ItemizationProgressBar::new(&mb, &filing);
            let filing_id = filing.filing_id.clone();
            export_itemizations(&mut tx, filing, Some(&pb))
                .with_context(|| format!("Error exporting itemizations for FEC-{}", filing_id))?;
        }
    }

    tx.commit().context("Error committing SQLite transaction")?;

    let elapsed = Instant::now() - t0;
    println!(
        "Finished exporting {} filings into {}, in {}",
        nfilings,
        path.to_string_lossy().bold(),
        HumanDuration(elapsed)
    );
    Ok(())
}

/// Initialize the SQLite database schema for filings
pub fn init_schema(db: &mut Connection) -> anyhow::Result<()> {
    let mut tx = db
        .transaction()
        .context("Error starting SQLite transaction")?;
    init(&mut tx)?;
    tx.commit()?;
    Ok(())
}

/// Get a set of filing IDs that already exist in the database
pub fn get_existing_filing_ids(
    db: &Connection,
) -> anyhow::Result<std::collections::HashSet<String>> {
    let mut stmt = db.prepare("SELECT filing_id FROM libfec_filings")?;
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(ids)
}

/// Export a single filing to the SQLite database
pub fn export_single_filing<R: Read>(
    db: &mut Connection,
    filing: Filing<R>,
    cover_only: bool,
) -> anyhow::Result<()> {
    let mut tx = db
        .transaction()
        .context("Error starting SQLite transaction")?;
    init(&mut tx)?;

    insert_filing_metadata(&mut tx, &filing)?;

    if !cover_only {
        export_itemizations(&mut tx, filing, None)?;
    }

    tx.commit().context("Error committing SQLite transaction")?;
    Ok(())
}

#[cfg(test)]
mod test;
