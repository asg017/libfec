use std::path::PathBuf;

use derive_builder::Builder;
use fec_api::Office;
use rusqlite::Connection;
use anyhow::Result;
use jiff::civil::DateTime;
use rusqlite::{OptionalExtension, Transaction};
use std::{
    io::{BufWriter, Cursor, Read},
    str::FromStr,
};
use ureq::{http::Response, Body};


static SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS candidate_cycles(
  year INTEGER PRIMARY KEY,
  modified_at TEXT,
  last_checked_at TEXT
);

CREATE TABLE IF NOT EXISTS candidates(
  cycle INTEGER,-- REFERENCES candidate_cycles(year) ON DELETE CASCADE,
  candidate_id TEXT,
  name,
  party_affiliation,
  election_year INTEGER,
  state,
  office,
  district,
  incumbent_challenger_status,
  status,
  principal_campaign_committee,
  address_street1,
  address_street2,
  address_city,
  address_state,
  address_zip,

  UNIQUE(cycle, candidate_id)
);
CREATE INDEX IF NOT EXISTS idx_candidate_cycle_candidate_id ON candidates(cycle, candidate_id);

"#;


fn csv_reader_from_response(response: Response<Body>, name: &str) -> csv::Reader<Cursor<Vec<u8>>> {
    let mut buffer = Cursor::new(Vec::new());
    std::io::copy(&mut response.into_body().into_reader(), &mut BufWriter::new(&mut buffer)).unwrap();
    
    let mut archive = zip::ZipArchive::new(buffer).unwrap();
    let mut txt_file = archive.by_name(name).unwrap();
    let mut cn_contents = Vec::new();
    txt_file.read_to_end(&mut cn_contents).unwrap();

    csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'|')
        .from_reader(Cursor::new(cn_contents))
}

fn write_candidate_rows(tx: &mut Transaction, year: u16, response: Response<Body>) {
    let mut stmt = tx
        .prepare("INSERT INTO candidates VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .unwrap();
    let mut rdr = csv_reader_from_response(response, "cn.txt");
    
    
    for result in rdr.records() {
        let record = result.unwrap();
        stmt.execute(rusqlite::params![
            year,
            record.get(0),
            record.get(1),
            record.get(2),
            record.get(3),
            record.get(4),
            record.get(5),
            record.get(6),
            record.get(7),
            record.get(8),
            record.get(9),
            record.get(10),
            record.get(11),
            record.get(12),
            record.get(13),
            record.get(14),
        ])
        .unwrap();
    }
}
fn sync_cycle(mut tx: Transaction, year: u16) {
    let result = tx
        .query_row(
            "select year, modified_at, last_checked_at from candidate_cycles where year = ?",
            [year],
            |row| {
                Ok((
                    row.get::<usize, u16>(0).expect("1st row to exist"),
                    row.get::<usize, String>(1).expect("2nd row to exist"),
                    row.get::<usize, String>(2).expect("3rd row to exist"),
                ))
            },
        )
        .optional()
        .unwrap();

    // if there is already data for the given year, and the Last-Modified header
    // is recent (within the last 30 minutes), skip the update
    if let Some((year, modified, last_checked_at)) = &result {
        let _last_modified = jiff::fmt::rfc2822::parse(modified).unwrap();
        let last_checked_at = DateTime::from_str(last_checked_at)
            .unwrap()
            .in_tz("UTC")
            .unwrap()
            .timestamp();
        let minutes_since = jiff::Timestamp::now()
            .since(last_checked_at)
            .unwrap()
            .total(jiff::Unit::Second)
            .unwrap();
        println!("{last_checked_at} {} ", minutes_since);

        tx.execute(
            "UPDATE candidate_cycles SET last_checked_at = datetime('now') WHERE year = ?",
            [year],
        )
        .unwrap();
        if minutes_since < 30.0 {
            println!(
                "Skipping {year} as it was last checked {} minutes ago.",
                minutes_since
            );
            return;
        }
    }

    let config = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(5)))
        .build();

    let request = config.new_agent().get(format!(
        "https://www.fec.gov/files/bulk-downloads/{}/cn{}.zip",
        year,
        year.to_string()[year.to_string().len() - 2..].to_string()
    ));

    let request = if let Some((_, modified, _)) = &result {
        request.header("If-Modified-Since", modified)
    } else {
        request
    };

    let response = request.call().unwrap();
    if response.status() == 304 {
        println!("No changes for {year}, skipping.");
        return;
    } else if response.status() != 200 {
        panic!(
            "Failed to fetch candidates for {year}: {}",
            response.status()
        );
    }
    println!("changes...");

    let last_modified = response
        .headers()
        .get("Last-Modified")
        .unwrap()
        .to_str()
        .unwrap();
    tx.execute(
        "INSERT OR REPLACE INTO candidate_cycles (year, modified_at, last_checked_at) VALUES (?, ?, datetime('now'))",
        rusqlite::params![year, last_modified],
    )
    .unwrap();

    tx.execute("DELETE FROM candidates WHERE cycle = ?", [year.to_string()])
        .unwrap();

    write_candidate_rows(&mut tx, year, response);
    tx.commit().unwrap();
}

#[derive(Debug, Clone, Builder)]
pub struct ResolveCandidateParams {
    cycle: u16,
    office: Option<Office>,
    state: Option<String>,
    district: Option<String>,
}
pub fn resolve_candidate_committees(bulk_db_path: &PathBuf, params: ResolveCandidateParams) -> Result<Vec<String>>{
  let mut db = Connection::open(bulk_db_path)?;
  db.execute_batch(SCHEMA)?;
  sync_cycle(db.transaction().unwrap(), params.cycle);
  let mut stmt = db
        .prepare(
            r#"
      SELECT 
        principal_campaign_committee 
      FROM candidates 
      WHERE cycle = :cycle
        AND election_year = cast(:cycle as text)
        AND principal_campaign_committee != ''
        AND if(:office is null, true, office = :office)
        AND if(:state is null, true, state = :state)
        AND if(:district is null, true, cast(district as text) = :district)
      "#,
        )
        .unwrap();
    Ok(stmt.query_map(
        rusqlite::named_params! {
          ":cycle": params.cycle,
          ":office": params.office.map(|o| match o {
            Office::House => "H",
            Office::Senate => "S",
            Office::President => "P",
          }),
          ":state": params.state,
          ":district": params.district
        },
        |row| {
            let committee_id: String = row.get(0)?;
            Ok(committee_id)
        },
    )
    .unwrap()
    .collect::<Result<Vec<String>, _>>()
    .unwrap() )
}