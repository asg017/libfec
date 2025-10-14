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
use std::{
    collections::HashMap,
    io::Read,
    path::PathBuf,
    time::Instant,
};

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
    statement: Statement<'a>,
    column_types: Vec<FieldFormat>,
}

fn insert_filing_row(
    filing_id: &str,
    column_types: &Vec<FieldFormat>,
    row: FilingRow,
    statement: &mut Statement,
) -> anyhow::Result<Vec<Warning>> {
    let mut warnings = Vec::new();
    let mut values: Vec<FieldValue> = row
        .record
        .iter()
        .enumerate()
        .map(|(idx, field)| match column_types.get(idx) {
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
        .collect();

    // make sure filing_id is the first column
    values.insert(0, FieldValue::Text(filing_id.to_owned()));

    if values.len() == statement.parameter_count() + 1 {
        values.pop();
    }

    while values.len() < statement.parameter_count() {
        values.push(FieldValue::Text("".to_owned()));
    }
    if values.len() > statement.parameter_count() {
        warnings.push(Warning {
            _message: format!(
                "Warning too long at {}:{}, {} vs {}!",
                filing_id,
                row.record.position().map(|p| p.line()).unwrap_or(0),
                values.len(),
                statement.parameter_count()
            ),
        });
        values.truncate(statement.parameter_count());
    }
    statement.execute(params_from_iter(values))?;
    statement.clear_bindings();

    Ok(warnings)
}

struct RecordTable {
    column_names: Vec<String>,
    column_types: Vec<FieldFormat>,
    suffix: String,
}

impl RecordTable {
    fn new(fec_version: &str, row_type: &str, suffix: &str) -> Self {
        let column_names = fec_parser::mappings::column_names_for_field(row_type, fec_version)
            .unwrap()
            .to_owned();
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

        RecordTable {
            column_names,
            column_types,
            suffix: suffix.to_owned(),
        }
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
    fn insert_statement<'a>(&self, tx: &'a Transaction) -> Statement<'a> {
        let sql = format!(
            "INSERT INTO libfec_{} VALUES ({})",
            self.suffix,
            vec!["?"; self.column_names.len() + 1].join(",")
        );
        tx.prepare(&sql).unwrap()
    }
    fn prep_row(&self, filing_id: &str, record: &StringRecord, n_params: usize) -> Vec<FieldValue> {
        let mut warnings = Vec::new();
        let mut values: Vec<FieldValue> = record
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
            .collect();
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
    fec_version: &str,
    row: &FilingRow,
    schedule_type: &ScheduleType,
    tx: &'a Transaction,
) -> anyhow::Result<ItemizationValue<'a>> {
    let rt = RecordTable::new(
        fec_version,
        &row.row_type,
        schedule_type.to_sqlite_tablename().as_str(),
    );
    tx.execute(&rt.create_sql(), [])?;
    Ok(ItemizationValue {
        statement: rt.insert_statement(tx),
        column_types: rt.column_types,
    })
}

fn prepare_form_type_statement<'a>(
    fec_version: &str,
    row: &FilingRow,
    form_type: &str,
    tx: &'a Transaction,
) -> anyhow::Result<ItemizationValue<'a>> {
    let rt = RecordTable::new(fec_version, &row.row_type, form_type);
    tx.execute(&rt.create_sql(), [])?;
    Ok(ItemizationValue {
        statement: rt.insert_statement(tx),
        column_types: rt.column_types,
    })
}

fn export_itemizations<R: Read>(
    tx: &mut Transaction,
    mut filing: Filing<R>,
    pb: &ItemizationProgressBar,
) -> Result<(), rusqlite::Error> {
    let mut itemizations_statements: HashMap<ItemizationKey, ItemizationValue> = HashMap::new();

    while let Some(r) = filing.next_row() {
        let r = r.unwrap();
        pb.update(&r);

        let v = match form_type_schedule_type(&r.row_type) {
            Some(schedule_type) => {
                match itemizations_statements
                    .get_mut(&ItemizationKey::Schedule(schedule_type.clone()))
                {
                    Some(v) => v,
                    None => {
                        let value = prepare_schedule_statement(
                            &filing.header.fec_version,
                            &r,
                            &schedule_type,
                            tx,
                        )
                        .unwrap();

                        // Insert and return a mutable reference to the value in one operation
                        itemizations_statements
                            .entry(ItemizationKey::Schedule(schedule_type.clone()))
                            .or_insert(value)
                    }
                }
            }
            None => {
                match itemizations_statements.get_mut(&ItemizationKey::FormType(r.row_type.clone()))
                {
                    Some(v) => v,
                    // TODO handle form_type
                    None => {
                        let value = prepare_form_type_statement(
                            &filing.header.fec_version,
                            &r,
                            &r.row_type,
                            tx,
                        )
                        .unwrap();

                        // Insert and return a mutable reference to the value in one operation
                        itemizations_statements
                            .entry(ItemizationKey::FormType(r.row_type.clone()))
                            .or_insert(value)
                    }
                }
            }
        };

        insert_filing_row(&filing.filing_id, &v.column_types, r, &mut v.statement).unwrap();
    }
    Ok(())
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

fn insert_filing_metadata(
    tx: &mut Transaction,
    filing: &Filing<impl Read>,
) -> rusqlite::Result<()> {
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

    let rt = RecordTable::new(
        &filing.header.fec_version,
        &filing.cover.form_type,
        // strip any lagging "A", "N", or "T", or return form_type
        form_type,
    );
    tx.execute(&rt.create_sql(), [])?;
    let mut stmt = rt.insert_statement(tx);
    let params = rt.prep_row(
        &filing.filing_id,
        &filing.cover.record,
        stmt.parameter_count(),
    );
    stmt.execute(params_from_iter(params))?;
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
    tx.execute(CREATE_FILINGS_SQL, [])
        .context("Error initializing filing schema")?;

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
                    mb.println(format!("Error fetching filing: {:?}", e));
                    todo!();
                }
            }
        };
        nfilings += 1;
        match insert_filing_metadata(&mut tx, &filing) {
            Ok(_) => {}
            Err(e) => {
                mb.println(format!(
                    "Error inserting filing metadata for FEC-{}: {:?}",
                    filing.filing_id, e
                ));
                continue;
            }
        }

        if !args.cover_only {
            let pb = ItemizationProgressBar::new(&mb, &filing);
            export_itemizations(&mut tx, filing, &pb).context("Error exporting itemizations")?;
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
