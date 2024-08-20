use clap::{Arg, Command};
use colored::Colorize;
use fec_parser::{Filing, FilingError, FilingRowReadError};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use rusqlite::{params_from_iter, Connection, Statement, Transaction};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::Path,
    time::{Duration, Instant},
};
use thiserror::Error;

fn write_fastfec_compat<R: Read>(mut filing: Filing<R>, directory: &std::path::Path) {
    let mut csv_writers: HashMap<String, csv::Writer<fs::File>> = HashMap::new();
    let pb = ProgressBar::new(filing.source_length.unwrap() as u64).with_style(ProgressStyle::with_template(
      "{msg}.fec:\t[{elapsed_precise}] {bar:40.cyan/blue} {eta} {decimal_bytes_per_sec} {decimal_total_bytes} total",
  )
  .unwrap());
    while let Some(r) = filing.next_row() {
        let r = r.unwrap();
        pb.set_position(r.record.position().unwrap().byte());

        if let Some(w) = csv_writers.get_mut(&r.row_type) {
            w.write_record(&r.record.clone()).unwrap();
        } else {
            let f = fs::File::create_new(directory.join(format!("{}.csv", r.row_type))).unwrap();
            let mut w = csv::WriterBuilder::new()
                .flexible(true)
                .has_headers(false)
                .from_writer(f);

            let column_names =
                fec_parser::column_names_for_field(&r.row_type, &filing.header.fec_version)
                    .unwrap();
            w.write_record(column_names).unwrap();
            w.write_record(&r.record.clone()).unwrap();
            csv_writers.insert(r.row_type, w);
        }
    }
}

#[derive(Error, Debug)]
pub enum SqliteWriteError {
    #[error("INSERT on \"{table_target}\" error: `{source}`")]
    InsertStmt {
        source: rusqlite::Error,
        table_target: String,
    },
    #[error("Error creating table \"{table_target}\" with SQL {sql}: {source}")]
    CreateTable {
        source: rusqlite::Error,
        sql: String,
        table_target: String,
    },
    #[error("asdf")]
    NextInvalid(#[source] FilingRowReadError),
}

fn write_sqlite<R: Read>(
    mut filing: Filing<R>,
    tx: &mut Transaction,
    pb: &ProgressBar,
) -> Result<(), SqliteWriteError> {
    let mut stmt_map: HashMap<String, Statement> = HashMap::new();
    while let Some(r) = filing.next_row() {
        let r = r.map_err(SqliteWriteError::NextInvalid)?;
        //println!("{} {:?}", r.row_type, r.record.position());
        pb.set_position(r.record.position().unwrap().byte());

        if let Some(stmt) = stmt_map.get_mut(&r.row_type) {
            let mut vals: Vec<String> = r.record.iter().map(|r| r.to_string()).collect();
            vals.insert(0, filing.filing_id.clone());
            while vals.len() < stmt.parameter_count() {
                vals.push("".to_owned());
            }
            if vals.len() == stmt.parameter_count() + 1 {
                vals.pop();
            }
            if vals.len() > stmt.parameter_count() {
                pb.println(format!(
                    "Warning too long {} vs {}!",
                    vals.len(),
                    stmt.parameter_count()
                ));
                vals.truncate(stmt.parameter_count());
            }

            stmt.execute(params_from_iter(vals))
                .map_err(|e| SqliteWriteError::InsertStmt {
                    source: e,
                    table_target: r.row_type.clone(),
                })?;
            stmt.clear_bindings();
        } else {
            //println!("{:?}", r.record.position().map(|v| v.byte()));
            let mut sql = String::from("CREATE TABLE IF NOT EXISTS [libfec_");
            sql += r.row_type.as_ref();
            sql += "](\n  ";
            sql += "filing_id,\n  ";
            let column_names =
                fec_parser::column_names_for_field(&r.row_type, &filing.header.fec_version)
                    .unwrap();
            sql += column_names.join(",\n  ").as_str();
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

            let mut stmt = tx.prepare(&sql).unwrap();

            let mut vals: Vec<String> = r.record.iter().map(|r| r.to_string()).collect();
            vals.insert(0, filing.filing_id.clone());
            if vals.len() == stmt.parameter_count() + 1 {
                vals.pop();
            }

            while vals.len() < stmt.parameter_count() {
                vals.push("".to_owned());
            }
            if vals.len() > stmt.parameter_count() {
                eprintln!(
                    "Warning too long {} vs {}!",
                    vals.len(),
                    stmt.parameter_count()
                );
                vals.truncate(stmt.parameter_count());
            }
            stmt.execute(params_from_iter(vals)).unwrap();
            stmt.clear_bindings();
            stmt_map.insert(r.row_type, stmt);
        }
    }
    Ok(())
}

fn cmd_fastfec_compat(filing_file: &str, output_directory: &str) -> Result<(), FilingError> {
    let filing = Filing::<fs::File>::from_path(Path::new(filing_file))?;
    let output_directory = std::path::Path::new(output_directory);
    write_fastfec_compat(filing, output_directory);
    Ok(())
}

#[derive(Error, Debug)]
pub enum CmdExportError {
    #[error("Error connecting to SQLite database: `{0}`")]
    ConnectionError(#[source] rusqlite::Error),
    #[error("Error inserting data for filing `{0}` to SQLite database: `{1}`")]
    InsertFilingError(String, #[source] SqliteWriteError),
}

fn cmd_export(filing_paths: Vec<String>, db: &str) -> Result<(), CmdExportError> {
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

struct FilingFormMetadata {
    count: usize,
    bytes: usize,
}
fn cmd_info(filing_paths: Vec<String>) -> Result<(), FilingError> {
    for filing_path in filing_paths {
        let mut filing = Filing::<fs::File>::from_path(Path::new(&filing_path))?;
        println!("Info {}", filing_path.bold());
        println!("{}", filing.filing_id);
        println!("{}: {}", "FEC Version".bold(), filing.header.fec_version);
        println!(
            "{}: {} ({})",
            "Software".bold(),
            filing.header.soft_name,
            filing.header.soft_ver
        );
        if let Some(ref report_id) = filing.header.report_id {
            println!("{}: {}", "Report ID".bold(), report_id);
        }
        if let Some(ref report_number) = filing.header.report_number {
            println!("Report #{}", report_number);
        }
        if let Some(ref comment) = filing.header.comment {
            println!("{}: {}", "Comment".bold(), comment);
        }

        let mut status: HashMap<String, FilingFormMetadata> = HashMap::new();

        let spinner = ProgressBar::new_spinner().with_message("Summarizing rows...");
        spinner.enable_steady_tick(Duration::from_millis(100));

        while let Some(row) = filing.next_row() {
            let row = row.unwrap();
            if let Some(x) = status.get_mut(&row.row_type) {
                x.count += 1;
                x.bytes += row.record.as_byte_record().as_slice().len();
            } else {
                status.insert(
                    row.row_type.clone(),
                    FilingFormMetadata {
                        count: 1,
                        bytes: row.record.as_byte_record().as_slice().len(),
                    },
                );
            }
        }
        spinner.finish_and_clear();

        let mut x: Vec<_> = status.iter().collect();
        x.sort_by(|a, b| b.1.count.cmp(&a.1.count));
        for (x, y) in x {
            println!(
                "{}: {} rows, {}",
                x.bold(),
                indicatif::HumanCount(y.count as u64),
                indicatif::HumanBytes(y.bytes as u64),
            );
        }
    }

    Ok(())
}

fn main() {
    let matches = Command::new("libfec-cli")
        .subcommand(
            Command::new("info").arg(
                Arg::new("filing-path")
                    .help(".fec file to read from")
                    .num_args(1..),
            ),
        )
        .subcommand(
            Command::new("export")
                .arg(
                    Arg::new("filing-path")
                        .help(".fec file to read from")
                        .num_args(1..),
                )
                .arg(Arg::new("db").short('o').help("SQLite db to export to")),
        )
        .subcommand(
            Command::new("fastfec-compat")
                .arg(Arg::new("filing-path").help(".fec file to read from"))
                .arg(Arg::new("output-directory").help("directory to write CSV files to")),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("fastfec-compat", m)) => {
            cmd_fastfec_compat(
                m.get_one::<String>("filing-path").unwrap(),
                m.get_one::<String>("output-directory").unwrap(),
            )
            .unwrap();
        }
        Some(("info", m)) => {
            cmd_info(
                m.get_many::<String>("filing-path")
                    .unwrap()
                    .map(|v| v.to_owned())
                    .collect(),
            )
            .unwrap();
        }
        Some(("export", m)) => {
            cmd_export(
                m.get_many::<String>("filing-path")
                    .unwrap()
                    .map(|v| v.to_owned())
                    .collect(),
                m.get_one::<String>("db").unwrap(),
            )
            .unwrap();
        }
        Some(_) => todo!(),
        None => todo!(),
    }
}
