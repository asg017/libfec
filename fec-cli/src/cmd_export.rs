use fec_parser::{Filing, FilingRowReadError, DATE_COLUMNS, FLOAT_COLUMNS};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use rusqlite::{
    params_from_iter,
    types::{ToSqlOutput, Value},
    Connection, Statement, ToSql, Transaction,
};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::Path,
    time::{Duration, Instant},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CmdExportError {
    #[error("Error connecting to SQLite database: `{0}`")]
    ConnectionError(#[source] rusqlite::Error),
    #[error("Error inserting data for filing `{0}` to SQLite database: `{1}`")]
    InsertFilingError(String, #[source] SqliteWriteError),
}

#[derive(Error, Debug)]
pub enum SqliteWriteError {
    #[error("Error creating table \"{table_target}\" with SQL {sql}: {source}")]
    CreateTable {
        source: rusqlite::Error,
        sql: String,
        table_target: String,
    },
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

fn write_sqlite<R: Read>(
    mut filing: Filing<R>,
    tx: &mut Transaction,
    pb: &ProgressBar,
) -> Result<(), SqliteWriteError> {
    let mut stmt_map: HashMap<String, Entry> = HashMap::new();
    while let Some(r) = filing.next_row() {
        let r = r.map_err(SqliteWriteError::NextInvalid)?;
        pb.set_position(r.record.position().unwrap().byte());

        let entry = match stmt_map.get_mut(&r.row_type) {
            Some(stmt) => stmt,
            None => {
                let column_names =
                    fec_parser::column_names_for_field(&r.row_type, &filing.header.fec_version)
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

                tx.execute(&sql, [])
                    .map_err(|e| SqliteWriteError::CreateTable {
                        source: e,
                        sql: sql.to_string(),
                        table_target: r.row_type.clone(),
                    })?;

                let sql = format!(
                    "INSERT INTO libfec_{} VALUES ({})",
                    r.row_type,
                    vec!["?"; column_names.len() + 1].join(",")
                );

                let statement = tx.prepare(&sql).unwrap();
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
                    8 => FieldValue::Date(format!(
                        "{}-{}-{}",
                        &field[0..4],
                        &field[4..6],
                        &field[6..8]
                    )),
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
            eprintln!(
                "Warning too long {} vs {}!",
                vals.len(),
                entry.statement.parameter_count()
            );
            vals.truncate(entry.statement.parameter_count());
        }
        entry.statement.execute(params_from_iter(vals)).unwrap();
        entry.statement.clear_bindings();
    }
    Ok(())
}

pub fn cmd_export(filing_paths: Vec<String>, db: &str) -> Result<(), CmdExportError> {
    let t0 = Instant::now();
    let mut db = Connection::open(db).map_err(CmdExportError::ConnectionError)?;

    db.execute(
        "CREATE TABLE libfec_filings(filing_id text primary key)",
        [],
    )
    .unwrap();
    let mut tx = db.transaction().unwrap();
    let mb = MultiProgress::new();
    let pb_files = mb.add(ProgressBar::new(filing_paths.len() as u64));
    pb_files.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap(),
    );
    pb_files.enable_steady_tick(Duration::from_millis(750));

    for filing_path in filing_paths {
        //pb_files.set_message(filing_path.clone());
        let filing = Filing::<fs::File>::from_path(Path::new(&filing_path)).unwrap();
        let pb_file = mb.add(ProgressBar::new(filing.source_length.unwrap() as u64));
        pb_file.set_style(
          indicatif::ProgressStyle::with_template(
              "{msg}.fec:\t[{elapsed_precise}] {bar:40.cyan/blue} {eta} {decimal_bytes_per_sec} {decimal_total_bytes} total",
          )
          .unwrap(),
      );
        let filing_id = filing.filing_id.clone();
        pb_file.set_message(filing_id.clone());
        tx.execute("INSERT INTO libfec_filings VALUES (?)", [&filing.filing_id])
            .unwrap();
        write_sqlite(filing, &mut tx, &pb_file)
            .map_err(|e| CmdExportError::InsertFilingError(filing_id, e))?;
        pb_files.inc(1);
    }
    tx.commit().unwrap();
    pb_files.finish_and_clear();

    println!("Finished in {}", HumanDuration(Instant::now() - t0));
    Ok(())
}
