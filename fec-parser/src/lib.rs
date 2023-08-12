use csv::StringRecord;
use regex::{RegexSet, RegexSetBuilder};

use indexmap::IndexMap;
use serde_json::Value;
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Read},
    path::Path,
};

macro_rules! gen_mapping1 {
    ($filename:expr) => {{
        let content = fs::read_to_string($filename).expect("Failed to read file");
        let json: Value = serde_json::from_str(&content).expect("Failed to parse JSON");
        let keys = json.as_object().unwrap().keys();
        let mut result = Vec::new();
        for key in keys {
            result.push(format!(r#"{}"#, key));
        }

        result
    }};
}
macro_rules! gen_mapping2 {
    ($filename:expr) => {{
        let content = fs::read_to_string($filename).expect("Failed to read file");
        let json: Value = serde_json::from_str(&content).expect("Failed to parse JSON");
        let values = json.as_object().unwrap().values();
        let mut result = Vec::new();
        for value in values {
            let keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();

            let set = RegexSetBuilder::new(keys)
                .case_insensitive(true)
                .build()
                .unwrap();
            result.push(set)
        }
        dbg!(&result);
        result
    }};
}
macro_rules! gen_mapping3 {
    ($filename:expr) => {{
        let content = fs::read_to_string($filename).expect("Failed to read file");
        let json: Value = serde_json::from_str(&content).expect("Failed to parse JSON");
        let mut result: Vec<Vec<Vec<String>>> = Vec::new();
        let mapping = json.as_object().unwrap();
        for (_, value) in mapping.iter() {
            let mut r1: Vec<Vec<String>> = vec![];
            for (_, x) in value.as_object().unwrap().iter() {
                let column_names: Vec<String> = x
                    .as_array()
                    .unwrap()
                    .into_iter()
                    .map(|value| value.as_str().unwrap().to_owned())
                    .collect();
                r1.push(column_names);
            }
            result.push(r1);
        }
        dbg!(&result);
        result
    }};
}

fn column_names_for_field<'a>(form_type: &str, fec_version: &str) -> Result<&'a Vec<String>> {
    let idx = field_idx(form_type).unwrap();
    let idx2 = MAPPINGS2
        .get(idx)
        .unwrap()
        .matches(fec_version)
        .iter()
        .next()
        .unwrap();
    let columns = MAPPINGS3.get(idx).unwrap().get(idx2).unwrap();
    Ok(columns)
}

lazy_static::lazy_static! {
  static ref MAPPINGS1: RegexSet = RegexSetBuilder::new(
    // generated with:
    // cat mappings.json | jq -c keys | pbcopy
    gen_mapping1!("src/mappings.json")
)
.case_insensitive(true)
.build()
.unwrap();
}

lazy_static::lazy_static! {
  static ref MAPPINGS2: Vec<RegexSet> = gen_mapping2!("src/mappings.json");
}
lazy_static::lazy_static! {
  static ref MAPPINGS3: Vec<Vec<Vec<String>>> = gen_mapping3!("src/mappings.json");
}

pub fn field_idx(field: &str) -> Option<usize> {
    let matches = MAPPINGS1.matches(field);
    matches.iter().next()
}

pub fn parse() -> String {
    String::from("Hello, world!")
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("file read error")]
    Read(#[from] std::io::Error),
    #[error("invalid format")]
    InvalidFormat,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq)]
pub enum FecFormatVersion {
    V1,
    V2,
}

#[derive(Debug)]
pub struct FecReader<R> {
    pub format_version: FecFormatVersion,
    pub fec_version: String,
    pub headers: IndexMap<String, String>,
    pub csv_reader: csv::Reader<BufReader<R>>,
    record: csv::StringRecord,
}

impl<R: Read> FecReader<R> {
    pub fn new(reader: R) -> Result<FecReader<R>> {
        let mut reader = BufReader::with_capacity(4016, reader);
        let mut first_line = String::new();
        let fec_version;
        let csv_reader;
        let mut headers: IndexMap<String, String> = IndexMap::new();

        let result = reader.read_line(&mut first_line)?;
        if result == 0 {
            return Err(Error::InvalidFormat);
        }
        let format_version = if first_line == "/* Header\n" {
            let mut current_line = String::new();
            loop {
                current_line.clear();
                let result = reader.read_line(&mut current_line)?;
                if result == 0 {
                    todo!("EOF")
                }
                if current_line.starts_with("/* End Header") {
                    match headers.get("FEC_Ver_#") {
                        Some(version) => {
                            fec_version = version.to_owned();
                        }
                        None => return Err(Error::InvalidFormat),
                    }
                    csv_reader = csv::ReaderBuilder::new()
                        .flexible(true)
                        .has_headers(false)
                        .from_reader(reader);

                    break;
                }
                if let Some((key, value)) = current_line.split_once('=') {
                    headers.insert(key.trim().to_owned(), value.trim().to_owned());
                }
            }
            FecFormatVersion::V1
        } else if first_line.starts_with("HDRFEC") {
            todo!("V2");
            FecFormatVersion::V2
        } else {
            return Err(Error::InvalidFormat);
        };
        Ok(FecReader {
            format_version,
            fec_version,
            headers,
            csv_reader,
            record: StringRecord::new(),
        })
    }

    pub fn next_record(&mut self) -> Result<Option<()>> {
        let more = self.csv_reader.read_record(&mut self.record).unwrap();
        if !more {
            return Ok(None);
        }
        println!("{}", self.record.len());
        let x = column_names_for_field(self.record.get(0).unwrap(), &self.fec_version)?;
        println!("{:?}", x);

        Ok(Some(()))
    }
}

impl FecReader<FecReader<File>> {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<FecReader<File>> {
        FecReader::new(File::open(path)?)
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn it_works() {
        let fec_13360_19 = "SA11A1,C00101766,IND,Kellner^Lawrence,10915 Pifer Way,,Houston,TX,77024,,,\"Continental Airlines, Inc.\",Exec. V.P. & CFO,5000.00,20000510,5000.00,,,,,,,,,,,,,,,,,A,SA11A1.7430";
        assert_eq!(field_idx(fec_13360_19), Some(45));
        assert_eq!(
            MAPPINGS2.get(45).unwrap().matches("3").iter().next(),
            Some(4)
        );
        let res = ("form_type,filer_committee_id_number,entity_type,contributor_name,contributor_street_1,contributor_street_2,contributor_city,contributor_state,contributor_zip_code,election_code,election_other_description,contributor_employer,contributor_occupation,contribution_aggregate,contribution_date,contribution_amount,contribution_purpose_code,contribution_purpose_descrip,donor_committee_fec_id,donor_candidate_fec_id,donor_candidate_name,donor_candidate_office,donor_candidate_state,donor_candidate_district,conduit_name,conduit_street1,conduit_street2,conduit_city,conduit_state,conduit_zip_code,memo_code,memo_text_description,amended_cd,transaction_id,back_reference_tran_id_number,back_reference_sched_name,reference_code").split(',').map(|s| s.to_owned()).collect::<Vec<String>>();
        assert_eq!(MAPPINGS3.get(45).unwrap().get(4).unwrap(), &res);

        assert_eq!(MAPPINGS1.len(), MAPPINGS2.len());
        assert_eq!(MAPPINGS2.len(), MAPPINGS3.len());
    }

    #[test]
    fn xxx() {
        let file = File::open("../tests/13360.fec").unwrap();
        let mut reader = BufReader::new(file);

        let mut line = String::with_capacity(1);
        let mut idx = 0;
        loop {
            let n = reader.read_line(&mut line);
            idx += 1;
            if idx >= 17 {
                break;
            }
        }

        let mut csv_reader = csv::ReaderBuilder::new()
            .flexible(true)
            .has_headers(false)
            .from_reader(reader);
        let x = csv_reader.records().next().unwrap().unwrap();
        assert_eq!(x.get(0), Some("F3XA"));
    }

    #[test]
    fn test_reader() {
        let mut reader =
            FecReader::from_path("../tests/13360.fec").expect("Failed to create FecReader");
        assert_eq!(reader.fec_version, "2.02");
        assert_eq!(reader.format_version, FecFormatVersion::V1);
        assert_eq!(
            reader.headers.keys().cloned().collect::<Vec<String>>(),
            vec![
                "FEC_Ver_#",
                "Soft_Name",
                "Soft_Ver#",
                "Dec/NoDec",
                "Date_Fmat",
                "NameDelim",
                "Form_Name",
                "FEC_IDnum",
                "Committee",
                "Control_#",
                "SA11A1",
                "SA17",
                "SB23",
                "SB29"
            ]
        );
        assert_eq!(
            reader.headers.values().cloned().collect::<Vec<String>>(),
            vec!["2.02", "FECfile", "3", "DEC", "CCYYMMDD", "^", "F3XA", "C00101766", "CONTINENTAL AIRLINES INC EMPLOYEE FUND FOR A BETTER AMERICA (FKA CONTINENTAL HOLDINGS PAC)", "K245592Q", "00139", "00001", "00008", "00003"]
        );
        reader.next_record().unwrap();
        assert!(false);
    }
}
