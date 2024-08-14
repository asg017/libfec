use clap::{Arg, Command};
use colored::Colorize;
use csv::StringRecordsIntoIter;
use rusqlite::{params_from_iter, Connection, Statement};
use std::{
    collections::HashMap,
    fs,
    io::{Error as IOError, Read},
    path::{Path, PathBuf},
};
use thiserror::Error;

// fields from mappings2.json -> '^hdr$' -> '$[6-8]'
#[derive(Debug)]
struct FilingHeader {
    record_type: String,
    ef_type: String,
    fec_version: String,
    soft_name: String,
    soft_ver: String,
    report_id: String,
    report_number: String,
    comment: Option<String>,
}
impl FilingHeader {
    fn from_record(hdr: csv::StringRecord) -> Self {
        let record_type = hdr.get(0).unwrap();
        let ef_type = hdr.get(1).unwrap();
        let fec_version = hdr.get(2).unwrap();
        assert_eq!(fec_version, "8.4");
        let soft_name = hdr.get(3).unwrap();
        let soft_ver = hdr.get(4).unwrap();
        let report_id = hdr.get(5).unwrap();
        let report_number = hdr.get(6).unwrap();
        let comment = hdr.get(7).map(String::from);

        FilingHeader {
            record_type: record_type.to_owned(),
            ef_type: ef_type.to_owned(),
            fec_version: fec_version.to_owned(),
            soft_name: soft_name.to_owned(),
            soft_ver: soft_ver.to_owned(),
            report_id: report_id.to_owned(),
            report_number: report_number.to_owned(),
            comment,
        }
    }
}
struct Filing<R: Read> {
    filing_id: String,
    header: FilingHeader,
    records_iter: StringRecordsIntoIter<R>,
}

impl<R: Read> Filing<R> {
    pub fn from_reader(rdr: R, filing_id: String) -> Self {
        let csv_reader = csv::ReaderBuilder::new()
            .delimiter(b"\x1c"[0])
            .flexible(true)
            .has_headers(false)
            .from_reader(rdr);

        let mut records_iter = csv_reader.into_records();

        let hdr = records_iter.next().unwrap().unwrap();
        assert_eq!(hdr.get(0), Some("HDR"));

        let header = FilingHeader::from_record(hdr);

        Self {
            filing_id,
            header,
            records_iter,
        }
    }

    pub fn from_path(filing_path: &Path) -> Result<Filing<fs::File>, FilingError> {
        let filing_id = filing_path
            .file_stem()
            .map(|v| v.to_string_lossy().into_owned())
            .ok_or_else(|| FilingError::UnknownFilingId(filing_path.to_path_buf()))?;

        let filing_file = std::fs::File::open(filing_path)?;

        Ok(Filing::from_reader(filing_file, filing_id.to_string()))
    }
}

fn write_fastfec_compat<R: Read>(filing: Filing<R>, directory: &std::path::Path) {
    let mut csv_writers: HashMap<String, csv::Writer<fs::File>> = HashMap::new();
    for r in filing.records_iter {
        let r = r.unwrap();
        let first = r.get(0).unwrap();
        if let Some(w) = csv_writers.get_mut(first) {
            w.write_record(&r.clone()).unwrap();
        } else {
            let f = fs::File::create_new(directory.join(format!("{first}.csv"))).unwrap();
            let mut w = csv::WriterBuilder::new()
                .flexible(true)
                .has_headers(false)
                .from_writer(f);

            let column_names =
                fec_parser::column_names_for_field(first, &filing.header.fec_version).unwrap();
            w.write_record(column_names).unwrap();
            w.write_record(&r.clone()).unwrap();
            csv_writers.insert(first.to_owned(), w);
        }
    }
}

fn write_sqlite<R: Read>(filing: Filing<R>, out_db: &mut Connection) {
    let mut stmt_map: HashMap<String, Statement> = HashMap::new();
    //let tx =out_db.transaction().unwrap();
    for r in filing.records_iter {
        let r = r.unwrap();
        let first = r.get(0).unwrap();

        //println!("{}", r.)

        if let Some(stmt) = stmt_map.get_mut(first) {
            //println!("{:?}", stmt.expanded_sql());
            let mut vals: Vec<String> = r.iter().map(|r| r.to_string()).collect();
            vals.insert(0, filing.filing_id.clone());
            while vals.len() < stmt.parameter_count() {
                vals.push("".to_owned());
            }
            if vals.len() == stmt.parameter_count() + 1 {
                vals.pop();
            }
            stmt.execute(params_from_iter(vals)).unwrap();
            stmt.clear_bindings();
        } else {
            let mut sql = String::from("CREATE TABLE libfec_");
            sql += first;
            sql += "(\n  ";
            sql += "filing_id,\n  ";
            let column_names =
                fec_parser::column_names_for_field(first, &filing.header.fec_version).unwrap();
            sql += column_names.join(",\n  ").as_str();
            sql += "\n)";
            out_db.execute(&sql, []).unwrap();
            let mut stmt = out_db
                .prepare(&format!(
                    "INSERT INTO libfec_{first} VALUES ({})",
                    vec!["?"; column_names.len() + 1].join(",")
                ))
                .unwrap();
            let mut vals: Vec<String> = r.iter().map(|r| r.to_string()).collect();
            vals.insert(0, filing.filing_id.clone());
            if vals.len() == stmt.parameter_count() + 1 {
                vals.pop();
            }

            while vals.len() < stmt.parameter_count() {
                vals.push("".to_owned());
            }
            stmt.execute(params_from_iter(vals)).unwrap();
            stmt.clear_bindings();
            stmt_map.insert(first.to_owned(), stmt);
        }
    }
}

#[derive(Error, Debug)]
pub enum FilingError {
    #[error("Could not parse filing id from path `{0}`")]
    UnknownFilingId(PathBuf),
    #[error("Could not read FEC file")]
    Read(#[from] IOError),
}

fn cmd_fastfec_compat(filing_file: &str, output_directory: &str) -> Result<(), FilingError> {
    let filing = Filing::<fs::File>::from_path(Path::new(filing_file))?;
    let output_directory = std::path::Path::new(output_directory);
    write_fastfec_compat(filing, output_directory);
    Ok(())
}

fn cmd_export(filing_file: &str, db: &str) -> Result<(), FilingError> {
    let filing = Filing::<fs::File>::from_path(Path::new(filing_file))?;
    let mut db = Connection::open(db).unwrap();
    write_sqlite(filing, &mut db);
    Ok(())
}
fn cmd_info(filing_file: &str) -> Result<(), FilingError> {
    let filing = Filing::<fs::File>::from_path(Path::new(filing_file))?;
    println!("Info {}", filing_file.bold());
    println!("{}", filing.filing_id);
    println!("{}: {}", "FEC Version".bold(), filing.header.fec_version);
    println!(
        "{}: {} ({})",
        "Software".bold(),
        filing.header.soft_name,
        filing.header.soft_ver
    );
    println!("{}: {}", "Report ID".bold(), filing.header.report_id);
    println!("Report #{}", filing.header.report_number);
    if let Some(comment) = filing.header.comment {
        println!("{}: {}", "Comment".bold(), comment);
    }
    Ok(())
}

fn main() {
    let matches = Command::new("pacman")
        .subcommand(
            Command::new("info").arg(Arg::new("filing-path").help(".fec file to read from")),
        )
        .subcommand(
            Command::new("export")
                .arg(Arg::new("filing-path").help(".fec file to read from"))
                .arg(Arg::new("db").help("SQLite db to export to")),
        )
        .subcommand(
            Command::new("fastfec-compat")
                .arg(Arg::new("filing-path").help(".fec file to read from"))
                .arg(Arg::new("output-directory").help("directory to write CSV files to")),
        )
        .get_matches();

    let cmd_result = match matches.subcommand() {
        Some(("fastfec-compat", m)) => cmd_fastfec_compat(
            m.get_one::<String>("filing-path").unwrap(),
            m.get_one::<String>("output-directory").unwrap(),
        ),
        Some(("info", m)) => cmd_info(m.get_one::<String>("filing-path").unwrap()),
        Some(("export", m)) => cmd_export(
            m.get_one::<String>("filing-path").unwrap(),
            m.get_one::<String>("db").unwrap(),
        ),
        Some(_) => todo!(),
        None => todo!(),
    };
    match cmd_result {
        Ok(_) => std::process::exit(0),
        Err(_) => std::process::exit(1),
    }
}
