use clap::{Arg, Command};
use colored::Colorize;
use fec_parser::{Filing, FilingError};
use rusqlite::{params_from_iter, Connection, Statement};
use std::{collections::HashMap, fs, io::Read, path::Path};

fn write_fastfec_compat<R: Read>(mut filing: Filing<R>, directory: &std::path::Path) {
    let mut csv_writers: HashMap<String, csv::Writer<fs::File>> = HashMap::new();
    //for r in filing.next_row() {
    while let Some(r) = filing.next_row() {
        let r = r.unwrap();
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

fn write_sqlite<R: Read>(mut filing: Filing<R>, out_db: &mut Connection) {
    let mut stmt_map: HashMap<String, Statement> = HashMap::new();
    //let tx =out_db.transaction().unwrap();
    while let Some(r) = filing.next_row() {
        let r = r.unwrap();
        //println!("{} {:?}", r.row_type, r.record.position());

        if let Some(stmt) = stmt_map.get_mut(&r.row_type) {
            let mut vals: Vec<String> = r.record.iter().map(|r| r.to_string()).collect();
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
            let mut sql = String::from("CREATE TABLE IF NOT EXISTS [libfec_");
            sql += r.row_type.as_ref();
            sql += "](\n  ";
            sql += "filing_id,\n  ";
            let column_names =
                fec_parser::column_names_for_field(&r.row_type, &filing.header.fec_version)
                    .unwrap();
            sql += column_names.join(",\n  ").as_str();
            sql += "\n)";
            out_db.execute(&sql, []).unwrap();
            let mut stmt = out_db
                .prepare(&format!(
                    "INSERT INTO libfec_{} VALUES ({})",
                    r.row_type,
                    vec!["?"; column_names.len() + 1].join(",")
                ))
                .unwrap();
            let mut vals: Vec<String> = r.record.iter().map(|r| r.to_string()).collect();
            vals.insert(0, filing.filing_id.clone());
            if vals.len() == stmt.parameter_count() + 1 {
                vals.pop();
            }

            while vals.len() < stmt.parameter_count() {
                vals.push("".to_owned());
            }
            stmt.execute(params_from_iter(vals)).unwrap();
            stmt.clear_bindings();
            stmt_map.insert(r.row_type, stmt);
        }
    }
}

fn cmd_fastfec_compat(filing_file: &str, output_directory: &str) -> Result<(), FilingError> {
    let filing = Filing::<fs::File>::from_path(Path::new(filing_file))?;
    let output_directory = std::path::Path::new(output_directory);
    write_fastfec_compat(filing, output_directory);
    Ok(())
}

fn cmd_export(filing_paths: Vec<String>, db: &str) -> Result<(), FilingError> {
    let mut db = Connection::open(db).unwrap();
    db.execute(
        "CREATE TABLE libfec_filings(filing_id text primary key)",
        [],
    )
    .unwrap();

    let pb = indicatif::ProgressBar::new(filing_paths.len() as u64);
    pb.set_style(
        indicatif::ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap(),
    );

    for filing_path in filing_paths {
        pb.set_message(filing_path.clone());
        let filing = Filing::<fs::File>::from_path(Path::new(&filing_path)).unwrap();
        db.execute("INSERT INTO libfec_filings VALUES (?)", [&filing.filing_id])
            .unwrap();
        write_sqlite(filing, &mut db);
        pb.inc(1);
    }
    pb.finish();
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
    if let Some(report_id) = filing.header.report_id {
        println!("{}: {}", "Report ID".bold(), report_id);
    }
    if let Some(report_number) = filing.header.report_number {
        println!("Report #{}", report_number);
    }
    if let Some(comment) = filing.header.comment {
        println!("{}: {}", "Comment".bold(), comment);
    }
    Ok(())
}

fn main() {
    let matches = Command::new("libfec-cli")
        .subcommand(
            Command::new("info").arg(Arg::new("filing-path").help(".fec file to read from")),
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

    let cmd_result = match matches.subcommand() {
        Some(("fastfec-compat", m)) => cmd_fastfec_compat(
            m.get_one::<String>("filing-path").unwrap(),
            m.get_one::<String>("output-directory").unwrap(),
        ),
        Some(("info", m)) => cmd_info(m.get_one::<String>("filing-path").unwrap()),
        Some(("export", m)) => cmd_export(
            m.get_many::<String>("filing-path")
                .unwrap()
                .map(|v| v.to_owned())
                .collect(),
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
