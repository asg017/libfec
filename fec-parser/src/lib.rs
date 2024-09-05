pub mod mappings;

use csv::{ByteRecordsIntoIter, StringRecord};
use mappings::column_names_for_field;
use std::{
    fs,
    io::{Error as IOError, Read},
    path::{Path, PathBuf},
};
use thiserror::Error;

pub fn try_format_fec_date(value: &str) -> String {
    if value.len() == "YYYYMMDD".len() {
        return format!("{}-{}-{}", &value[0..4], &value[4..6], &value[6..8]);
    }
    value.to_owned()
}

macro_rules! header_get_field {
    ($hdr:expr, $idx:expr, $name:expr) => {
        $hdr.get($idx)
            .ok_or_else(|| FilingHeaderError::MissingField {
                name: $name.to_owned(),
                idx: $idx,
            })?
            .to_string()
    };
}

#[derive(Error, Debug)]
pub enum FilingHeaderError {
    #[error("Missing field '{name:?}' at index {idx:?}")]
    MissingField { name: String, idx: usize },
    #[error("`{0}`")]
    UnsupportedVersion(String),
}
// fields from mappings2.json -> '^hdr$' -> '$[6-8]'
#[derive(Debug)]
pub struct FilingHeader {
    pub header_record: StringRecord,
    pub record_type: String,
    pub ef_type: String,
    pub fec_version: String,
    pub soft_name: String,
    pub soft_ver: String,
    pub report_id: Option<String>,
    pub report_number: Option<String>,
    pub comment: Option<String>,
}

impl FilingHeader {
    fn from_record(hdr: csv::StringRecord) -> Result<Self, FilingHeaderError> {
        let record_type = header_get_field!(hdr, 0, "record_type");
        let ef_type = header_get_field!(hdr, 1, "ef_type");
        let fec_version = header_get_field!(hdr, 2, "fec_version").trim().to_owned();
        match fec_version.as_str() {
            "8.3" | "8.4" => (),
            _ => {
                return Err(FilingHeaderError::UnsupportedVersion(format!(
                    "Unsupported version '{fec_version}', only 8.4 is currently supported."
                )));
            }
        }
        let soft_name = header_get_field!(hdr, 3, "soft_name");
        let soft_ver = header_get_field!(hdr, 4, "soft_ver");
        let report_id = hdr
            .get(5)
            .map(|v| String::from(v.trim()))
            .filter(|v| !String::is_empty(v));
        let report_number = hdr
            .get(6)
            .map(|v| String::from(v.trim()))
            .filter(|v| !String::is_empty(v));
        let comment = hdr
            .get(7)
            .map(|v| String::from(v.trim()))
            .filter(|v| !String::is_empty(v));

        Ok(FilingHeader {
            header_record: hdr,
            record_type,
            ef_type,
            fec_version,
            soft_name,
            soft_ver,
            report_id,
            report_number,
            comment,
        })
    }
}

pub fn report_code_label(report_code: &str) -> &'static str {
    // labels from: https://api.open.fec.gov/developers/#/filings/get_v1_filings_:~:text=(query)-,Name%20of%20report%20where%20the%20underlying%20data%20comes%20from%3A,-%2D%2010D%20Pre%2DElection
    match report_code {
        "10D" => "Pre-Election",
        "10G" => "Pre-General",
        "10P" => "Pre-Primary",
        "10R" => "Pre-Run-Off",
        "10S" => "Pre-Special",
        "12C" => "Pre-Convention",
        "12G" => "Pre-General",
        "12P" => "Pre-Primary",
        "12R" => "Pre-Run-Off",
        "12S" => "Pre-Special",
        "30D" => "Post-Election",
        "30G" => "Post-General",
        "30P" => "Post-Primary",
        "30R" => "Post-Run-Off",
        "30S" => "Post-Special",
        "60D" => "Post-Convention",
        "M1" => "January Monthly",
        "M10" => "October Monthly",
        "M11" => "November Monthly",
        "M12" => "December Monthly",
        "M2" => "February Monthly",
        "M3" => "March Monthly",
        "M4" => "April Monthly",
        "M5" => "May Monthly",
        "M6" => "June Monthly",
        "M7" => "July Monthly",
        "M8" => "August Monthly",
        "M9" => "September Monthly",
        "MY" => "Mid-Year Report",
        "Q1" => "April Quarterly",
        "Q2" => "July Quarterly",
        "Q3" => "October Quarterly",
        "TER" => "Termination Report",
        "YE" => "Year-End",
        "ADJ" => "COMP ADJUST AMEND",
        "CA" => "COMPREHENSIVE AMEND",
        "90S" => "Post Inaugural Supplement",
        "90D" => "Post Inaugural",
        "48" => "48 Hour Notification",
        "24" => "24 Hour Notification",
        "M7S" => "July Monthly/Semi-Annual",
        "MSA" => "Monthly Semi-Annual (MY)",
        "MYS" => "Monthly Year End/Semi-Annual",
        "Q2S" => "July Quarterly/Semi-Annual",
        "QSA" => "Quarterly Semi-Annual (MY)",
        "QYS" => "Quarterly Year End/Semi-Annual",
        "QYE" => "Quarterly Semi-Annual (YE)",
        "QMS" => "Quarterly Mid-Year/ Semi-Annual",
        "MSY" => "Monthly Semi-Annual (YE)",
        _ => "[Unknown report code]",
    }
}
pub struct FilingCover {
    cover_record: StringRecord,
    pub form_type: String,
    pub filer_id: String,
    pub filer_name: String,
    pub report_code: Option<String>,
    pub coverage_from_date: Option<String>,
    pub coverage_through_date: Option<String>,
}

impl FilingCover {
    fn from_record(fec_version: &str, cover_record: StringRecord) -> Result<Self, String> {
        let form_type = cover_record
            .get(0)
            .ok_or_else(|| "Cover record row contains 0 fields".to_owned())?
            .to_owned();
        let columns = column_names_for_field(form_type.as_str(), fec_version).unwrap();

        let id_idx = columns
            .iter()
            .position(|v| v == "filer_committee_id_number" || v == "candidate_id_number")
            .ok_or_else(|| "asdf".to_owned())?;

        let name_idx = columns
            .iter()
            .position(|v| v == "committee_name" || v == "organization_name")
            .ok_or_else(|| "asdf".to_owned())?;

        let report_code = columns
            .iter()
            .position(|v| v == "report_code")
            .and_then(|idx| cover_record.get(idx).map(|s| s.to_owned()));

        let coverage_from_date = columns
            .iter()
            .position(|v| v == "coverage_from_date")
            .and_then(|idx| cover_record.get(idx).map(|s| s.to_owned()));

        let coverage_through_date = columns
            .iter()
            .position(|v| v == "coverage_through_date")
            .and_then(|idx| cover_record.get(idx).map(|s| s.to_owned()));

        let filer_id = cover_record.get(id_idx).unwrap().to_owned();
        let filer_name = cover_record.get(name_idx).unwrap().to_owned();

        Ok(Self {
            cover_record,
            form_type,
            filer_id,
            filer_name,
            report_code,
            coverage_from_date,
            coverage_through_date,
        })
    }
}

#[derive(Error, Debug)]
pub enum FilingReaderError {
    #[error("No records found in the .fec file")]
    NoRecords,
    #[error("Error reading CSV row")]
    CsvRead(#[from] csv::Error),
    #[error("Missing header as first record")]
    MissingHeader,
    #[error("First field in first record is not 'HDR', found `{0}`")]
    IncorrectHeader(String),
    #[error("Error parsing header")]
    HeaderRead(#[from] FilingHeaderError),
}

#[derive(Error, Debug)]
pub enum FilingError {
    #[error("Could not parse filing id from path `{0}`")]
    UnknownFilingId(PathBuf),
    #[error("Could not read FEC file")]
    Read(#[from] IOError),
    #[error("FEC file error")]
    Reader(#[from] FilingReaderError),
}

pub struct Filing<R: Read> {
    pub filing_id: String,
    pub header: FilingHeader,
    pub cover: FilingCover,
    records_iter: ByteRecordsIntoIter<R>,
    pub source_length: Option<usize>,
}

impl<R: Read> Filing<R> {
    pub fn from_reader(
        rdr: R,
        filing_id: String,
        source_length: Option<usize>,
    ) -> Result<Self, FilingReaderError> {
        let csv_reader = csv::ReaderBuilder::new()
            .delimiter(b"\x1c"[0])
            .flexible(true)
            .has_headers(false)
            .from_reader(rdr);

        let mut records_iter = csv_reader.into_byte_records();

        let hdr = records_iter.next().ok_or(FilingReaderError::NoRecords)??;

        let hdr_record_type = String::from_utf8(
            hdr.get(0)
                .ok_or_else(|| FilingReaderError::MissingHeader)?
                .to_vec(),
        )
        .unwrap();
        if hdr_record_type != "HDR" {
            return Err(FilingReaderError::IncorrectHeader(
                hdr_record_type.to_owned(),
            ));
        }

        let hdr_record = StringRecord::from_byte_record_lossy(hdr);
        let cover_record = records_iter
            .next()
            .expect("2nd record to be cover record")
            .expect("2nd record to exist");
        let header = FilingHeader::from_record(hdr_record)?;
        let cover = FilingCover::from_record(
            &header.fec_version,
            StringRecord::from_byte_record_lossy(cover_record),
        )
        .unwrap();

        Ok(Self {
            filing_id,
            header,
            cover,
            records_iter,
            source_length,
        })
    }

    pub fn from_path(filing_path: &Path) -> Result<Filing<fs::File>, FilingError> {
        let filing_id = filing_path
            .file_stem()
            .map(|v| v.to_string_lossy().into_owned())
            .ok_or_else(|| FilingError::UnknownFilingId(filing_path.to_path_buf()))?;

        let filing_file = std::fs::File::open(filing_path)?;
        let source_length = filing_file.metadata().map(|v| (v.len() as usize)).ok();

        Ok(Filing::from_reader(
            filing_file,
            filing_id.to_string(),
            source_length,
        )?)
    }

    pub fn next_row(&mut self) -> Option<Result<FilingRow, FilingRowReadError>> {
        let (record, original_size) = match self.records_iter.next() {
            Some(Ok(record)) => {
                let n = record.as_slice().len();
                (StringRecord::from_byte_record_lossy(record), n)
            }
            Some(Err(err)) => return Some(Err(FilingRowReadError::CsvError(err))),
            None => return None,
        };

        let row_type = match record.get(0) {
            Some(field) => field.to_owned().replace('/', ""), // idk man, 'SC/12',
            None => {
                return Some(Err(FilingRowReadError::EmptyRecord(
                    record.position().unwrap().line(),
                )));
            }
        };

        if row_type == "[BEGINTEXT]" {
            let mut contents = String::new();
            loop {
                match self.records_iter.next() {
                    Some(Err(e)) => return Some(Err(FilingRowReadError::TextRecordError(e))),
                    Some(Ok(record)) => match record.get(0) {
                        Some(b"[ENDTEXT]") => match self.records_iter.next() {
                            Some(record) => {
                                let record = record.unwrap();
                                let original_size = record.as_slice().len();
                                let record = StringRecord::from_byte_record_lossy(record);
                                let row_type = record.get(0).unwrap().to_owned();
                                return Some(Ok(FilingRow {
                                    row_type,
                                    record,
                                    original_size,
                                }));
                            }
                            None => return None,
                        },
                        Some(_) => {
                            contents += &String::from_utf8_lossy(record.as_slice());
                            contents += "\n";
                        }
                        None => {
                            contents += "\n";
                        }
                    },
                    None => todo!("[BEGINTEXT] did not terminate"),
                }
            }
        }

        Some(Ok(FilingRow {
            row_type,
            record,
            original_size,
        }))
    }
}

#[derive(Error, Debug)]
pub enum FilingRowReadError {
    #[error("Error reading next row from file: `{0}`")]
    CsvError(#[source] csv::Error),
    #[error("Empty record found at line `{0}`")]
    EmptyRecord(u64),
    #[error("Error reading contents of a [BEGINTEXT] record: `{0}`")]
    TextRecordError(#[source] csv::Error),
}

pub struct FilingRow {
    pub row_type: String,
    pub record: StringRecord,
    pub original_size: usize,
}

#[cfg(test)]
mod tests {
    use crate::*;
    use mappings::*;
    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    #[test]
    fn it_works() {
        assert_eq!(FORM_TYPES_SET.len(), FORM_TYPE_VERSIONS_SET.len());
        assert_eq!(FORM_TYPE_VERSIONS_SET.len(), COLUMN_NAMES.len());

        let fec_13360_19 = "SA11A1"; //"SA11A1,C00101766,IND,Kellner^Lawrence,10915 Pifer Way,,Houston,TX,77024,,,\"Continental Airlines, Inc.\",Exec. V.P. & CFO,5000.00,20000510,5000.00,,,,,,,,,,,,,,,,,A,SA11A1.7430";
        assert_eq!(field_idx(fec_13360_19), Some(44));
        assert_eq!(FORM_TYPES[44], "^sa");
        assert_eq!(
            FORM_TYPE_VERSIONS_SET
                .get(44)
                .unwrap()
                .matches("3")
                .iter()
                .next(),
            Some(11)
        );
        let x = &FORM_TYPE_VERSIONS_SET[44];

        assert_eq!(
            COLUMN_NAMES.get(44).unwrap().get(11).unwrap().join(","),
            "form_type,filer_committee_id_number,entity_type,contributor_name,contributor_street_1,contributor_street_2,contributor_city,contributor_state,contributor_zip_code,election_code,election_other_description,contributor_employer,contributor_occupation,contribution_aggregate,contribution_date,contribution_amount,contribution_purpose_code,contribution_purpose_descrip,donor_committee_fec_id,donor_candidate_fec_id,donor_candidate_name,donor_candidate_office,donor_candidate_state,donor_candidate_district,conduit_name,conduit_street1,conduit_street2,conduit_city,conduit_state,conduit_zip_code,memo_code,memo_text_description,amended_cd,transaction_id,back_reference_tran_id_number,back_reference_sched_name,reference_code"
        );
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
}
