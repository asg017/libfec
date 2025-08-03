use crate::sourcer::FilingSourcer;
use colored::Colorize;
use csv::StringRecord;
use fec_parser::{
    mappings::{DATE_COLUMNS, FLOAT_COLUMNS},
    schedules::{form_type_schedule_type, ScheduleType},
    try_format_fec_date, Filing, FilingRow,
};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use rusqlite::{
    params_from_iter,
    types::{ToSqlOutput, Value},
    Connection, Statement, ToSql, Transaction,
};
use std::{
    collections::HashMap,
    error::Error,
    io::Read,
    time::{Duration, Instant},
};
use thiserror::Error;

lazy_static::lazy_static! {
  pub static ref BAR_FILES_STYLE: ProgressStyle =ProgressStyle::with_template(
    "{spinner} {pos}/{len} [{elapsed_precise}]",
  ).expect("valid progress style");
}
lazy_static::lazy_static! {
  pub static ref BAR_FILE_STYLE: ProgressStyle =ProgressStyle::with_template(
    "{msg} {elapsed} {bar:30.cyan/blue}  ({decimal_bytes}/{decimal_total_bytes}, {decimal_bytes_per_sec}) [{eta}]",
  ).expect("valid progress style");
}

const CREATE_FILINGS_SQL: &str = r#"
  CREATE TABLE IF NOT EXISTS libfec_filings(
    filing_id TEXT PRIMARY KEY NOT NULL,
    fec_version TEXT NOT NULL,
    software_name TEXT NOT NULL,
    software_version TEXT NOT NULL,
    report_id TEXT,
    report_number TEXT,
    comment TEXT,
    cover_record_form_type TEXT NOT NULL,
    filer_id TEXT NOT NULL,
    filer_name TEXT NOT NULL,
    report_code TEXT,
    coverage_from_date TEXT,
    coverage_through_date TEXT,
    cover_data JSON
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
    pb: &ProgressBar,
) -> Result<(), rusqlite::Error> {
    let mut itemizations_statements: HashMap<ItemizationKey, ItemizationValue> = HashMap::new();

    while let Some(r) = filing.next_row() {
        let r = r.unwrap();
        if let Some(position) = r.record.position() {
            pb.set_position(position.byte());
        }

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

#[derive(Error, Debug)]
pub enum CmdExportError {
    #[error("`{0}`: {1}")]
    SqliteError(String, #[source] rusqlite::Error),
}

fn insert_filing_metadata(
    tx: &mut Transaction,
    filing: &Filing<impl Read>,
) -> rusqlite::Result<()> {
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
            &filing.cover.form_type,
            &filing.cover.filer_id,
            &filing.cover.filer_name,
            &filing.cover.report_code.clone(),
            &filing.cover.coverage_from_date.clone(),
            &filing.cover.coverage_through_date.clone(),
            serde_json::to_string(&filing.cover.cover_record_kv).unwrap(),
        ],
    )?;

    let rt = RecordTable::new(
        &filing.header.fec_version,
        &filing.cover.form_type,
        // strip any lagging "A", "N", or "T", or return form_type
        &filing.cover.form_type
            .strip_suffix('A')
            .or_else(|| filing.cover.form_type.strip_suffix('N'))
            .or_else(|| filing.cover.form_type.strip_suffix('T'))
            .unwrap_or(&filing.cover.form_type),
    );
    tx.execute(&rt.create_sql(), [])?;
    let mut stmt = rt.insert_statement(tx);
    let params = rt.prep_row(&filing.filing_id, &filing.cover.record, stmt.parameter_count());
    stmt.execute(params_from_iter(params))?;
    Ok(())
}
pub fn cmd_export_sqlite(args: crate::cli::ExportArgs) -> Result<(), Box<dyn Error>> {
    let mut filings = args.filings.clone();
    if args.api.any_provided() {
        filings.extend(args.api.resolve_ids()?);
    }
    let mut db = Connection::open(&args.output).map_err(|e| {
        CmdExportError::SqliteError(format!("Error connecting to database {:?}", args.output), e)
    })?;
    let filing_sourcer = FilingSourcer::new();
    let t0 = Instant::now();

    let mut tx = db.transaction().unwrap();
    tx.execute(CREATE_FILINGS_SQL, []).unwrap();

    let mb = MultiProgress::new();
    let pb_files = if filings.len() > 1 {
        let pb_files = mb.add(ProgressBar::new(filings.len() as u64));
        pb_files.set_style(BAR_FILES_STYLE.clone());
        pb_files.enable_steady_tick(Duration::from_millis(16));
        Some(pb_files)
    } else {
        None
    };

    for filing in &filings {
        let filing = filing_sourcer.resolve(filing);
        let pb_file = mb.add(ProgressBar::new(filing.source_length.unwrap() as u64));
        pb_file.set_style(BAR_FILE_STYLE.clone());

        let filing_id = filing.filing_id.clone();
        pb_file.set_message(format!(
            "{} ({} {})",
            format!("FEC-{}", filing_id).bold(),
            filing.cover.filer_name,
            &filing.cover.report_code.clone().unwrap_or("".to_owned()),
        ));

        insert_filing_metadata(&mut tx, &filing).unwrap();

        if !args.cover_only {
          export_itemizations(&mut tx, filing, &pb_file).unwrap();
        }

        if let Some(pb_files) = &pb_files {
            pb_files.inc(1);
        }
    }
    tx.commit().unwrap();
    if let Some(pb_files) = &pb_files {
        pb_files.finish_and_clear();
    }

    println!(
        "Finished {} files in {}",
        filings.len(),
        HumanDuration(Instant::now() - t0)
    );
    Ok(())
}
