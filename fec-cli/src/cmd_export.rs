use fec_parser::{
    mappings::{DATE_COLUMNS, FLOAT_COLUMNS},
    try_format_fec_date, Filing, FilingRowReadError,
};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use rusqlite::{
    params_from_iter,
    types::{ToSqlOutput, Value},
    Connection, Statement, ToSql, Transaction,
};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Read,
    path::Path,
    time::{Duration, Instant},
};
use thiserror::Error;

use crate::sourcer::FilingSourcer;

#[derive(Error, Debug)]
pub enum CmdExportError {
    #[error("`{0}`: {1}")]
    SqliteError(String, #[source] rusqlite::Error),
    #[error("asdf")]
    NextInvalid(#[source] FilingRowReadError),
}

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

struct Entry<'a> {
    statement: Statement<'a>,
    field_formats: Vec<FieldFormat>,
}

fn export_itemizations_by_form_type<R: Read>(
    mut filing: Filing<R>,
    tx: &mut Transaction,
    pb: &ProgressBar,
) -> Result<(), rusqlite::Error> {
    let mut stmt_map: HashMap<String, Entry> = HashMap::new();
    while let Some(r) = filing.next_row() {
        let r = r.unwrap();
        pb.set_position(r.record.position().unwrap().byte());

        let entry = match stmt_map.get_mut(&r.row_type) {
            Some(stmt) => stmt,
            None => {
                let column_names = fec_parser::mappings::column_names_for_field(
                    &r.row_type,
                    &filing.header.fec_version,
                )
                .unwrap();

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

                let columns_defs: Vec<String> = column_names
                    .iter()
                    .zip(column_types.clone())
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
                sql += r.row_type.as_ref();
                sql += "](\n  ";
                sql += "filing_id text references libfec_filings(filing_id),\n  ";
                sql += columns_defs.join(",\n  ").as_str();
                sql += "\n)";

                tx.execute(&sql, [])?;

                let sql = format!(
                    "INSERT INTO libfec_{} VALUES ({})",
                    r.row_type,
                    vec!["?"; column_names.len() + 1].join(",")
                );

                let statement = tx.prepare(&sql)?;
                stmt_map.insert(
                    r.row_type.clone(),
                    Entry {
                        statement,
                        field_formats: column_types,
                    },
                );

                stmt_map
                    .get_mut(&r.row_type)
                    .expect("retrieve statement that was just inserted")
            }
        };

        let mut vals: Vec<FieldValue> = r
            .record
            .iter()
            .enumerate()
            .map(|(idx, field)| match entry.field_formats.get(idx) {
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

        vals.insert(0, FieldValue::Text(filing.filing_id.clone()));
        if vals.len() == entry.statement.parameter_count() + 1 {
            vals.pop();
        }

        while vals.len() < entry.statement.parameter_count() {
            vals.push(FieldValue::Text("".to_owned()));
        }
        if vals.len() > entry.statement.parameter_count() {
            pb.println(format!(
                "Warning too long at {}:{}, {} vs {}!",
                filing.filing_id,
                r.record.position().map(|p| p.line()).unwrap_or(0),
                vals.len(),
                entry.statement.parameter_count()
            ));
            vals.truncate(entry.statement.parameter_count());
        }
        entry.statement.execute(params_from_iter(vals))?;
        entry.statement.clear_bindings();
    }
    Ok(())
}
fn export_schedule_a<R: Read>(
    mut filing: Filing<R>,
    tx: &mut Transaction,
    pb: &ProgressBar,
) -> Result<(), rusqlite::Error> {
    let column_names = fec_parser::mappings::column_names_for_field("SA", "8.4").unwrap();

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

    let columns_defs: Vec<String> = column_names
        .iter()
        .zip(column_types.clone())
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
    sql += "schedule_a";
    sql += "](\n  ";
    sql += "filing_id text references libfec_filings(filing_id),\n  ";
    sql += columns_defs.join(",\n  ").as_str();
    sql += "\n)";

    tx.execute(&sql, [])?;

    let sql = format!(
        "INSERT INTO libfec_{} VALUES ({})",
        "schedule_a",
        vec!["?"; column_names.len() + 1].join(",")
    );

    let mut statement = tx.prepare(&sql)?;

    while let Some(r) = filing.next_row() {
        let r = r.unwrap();
        pb.set_position(r.record.position().unwrap().byte());
        if !r.row_type.starts_with("SA") {
            continue;
        }

        let mut vals: Vec<FieldValue> = r
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

        vals.insert(0, FieldValue::Text(filing.filing_id.clone()));
        if vals.len() == statement.parameter_count() + 1 {
            vals.pop();
        }

        while vals.len() < statement.parameter_count() {
            vals.push(FieldValue::Text("".to_owned()));
        }
        if vals.len() > statement.parameter_count() {
            pb.println(format!(
                "Warning too long at {}:{}, {} vs {}!",
                filing.filing_id,
                r.record.position().map(|p| p.line()).unwrap_or(0),
                vals.len(),
                statement.parameter_count()
            ));
            vals.truncate(statement.parameter_count());
        }
        statement.execute(params_from_iter(vals))?;
        statement.clear_bindings();
    }
    Ok(())
}

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
    report_code TEXT NOT NULL,
    coverage_from_date TEXT NOT NULL,
    coverage_through_date TEXT NOT NULL
  )
"#;

const INSERT_FILING_SQL: &str = r#"
  INSERT INTO libfec_filings VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)
"#;

pub enum CmdExportTarget {
    ByFormType,
    ScheduleA,
}

pub fn cmd_export(
    filing_paths: Vec<String>,
    db: &str,
    target: CmdExportTarget,
) -> Result<(), CmdExportError> {
    let filing_sourcer = FilingSourcer::new();
    let t0 = Instant::now();
    let mut db = Connection::open(db).map_err(|e| {
        CmdExportError::SqliteError(format!("Error connecting to database {db}"), e)
    })?;

    let mut tx = db.transaction().unwrap();
    tx.execute(CREATE_FILINGS_SQL, []).unwrap();
    let mb = MultiProgress::new();
    let pb_files = if filing_paths.len() > 1 {
        let pb_files = mb.add(ProgressBar::new(filing_paths.len() as u64));
        pb_files.set_style(BAR_FILES_STYLE.clone());
        pb_files.enable_steady_tick(Duration::from_millis(100));
        Some(pb_files)
    } else {
        None
    };

    for filing_path in filing_paths {
        //pb_files.set_message(filing_path.clone());
        let filing = filing_sourcer.resolve(&filing_path);
        let pb_file = mb.add(ProgressBar::new(filing.source_length.unwrap() as u64));
        pb_file.set_style(BAR_FILE_STYLE.clone());
        let filing_id = filing.filing_id.clone();
        pb_file.set_message(format!(
            "FEC-{} ({} {} {} to {})",
            filing_id,
            filing.cover.filer_name,
            filing.cover.report_code,
            filing.cover.coverage_from_date,
            filing.cover.coverage_through_date
        ));
        tx.execute(
            INSERT_FILING_SQL,
            rusqlite::params![
                &filing.filing_id,
                &filing.header.fec_version,
                &filing.header.soft_name,
                &filing.header.soft_ver,
                &filing.header.report_id,
                &filing.header.report_number,
                &filing.header.comment,
                &filing.cover.form_type,
                &filing.cover.filer_id,
                &filing.cover.filer_name,
                &filing.cover.report_code,
                &filing.cover.coverage_from_date,
                &filing.cover.coverage_through_date,
            ],
        )
        .unwrap();
        match target {
            CmdExportTarget::ByFormType => {
                export_itemizations_by_form_type(filing, &mut tx, &pb_file).map_err(|e| {
                    CmdExportError::SqliteError(format!("Error inserting filing {filing_id}"), e)
                })?;
            }
            CmdExportTarget::ScheduleA => {
                export_schedule_a(filing, &mut tx, &pb_file).unwrap();
            }
        }

        if let Some(pb_files) = &pb_files {
            pb_files.inc(1);
        }
    }
    tx.commit().unwrap();
    if let Some(pb_files) = &pb_files {
        pb_files.finish_and_clear();
    }

    println!("Finished in {}", HumanDuration(Instant::now() - t0));
    println!("{:?}", db.path());
    Ok(())
}
