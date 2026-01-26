#![deny(clippy::unwrap_used)]

pub mod covers;
pub mod mappings;
pub mod schedules;

use crate::covers::Cover;
use csv::{ByteRecordsIntoIter, StringRecord};
use indexmap::IndexMap;
use jiff::civil::Date;
use mappings::column_names_for_field;
use std::{fs, io::Read, path::Path};
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

/// > The first record of every electronic file that is submitted to the FEC
/// > must be an HDR record that precedes the main body of the ASCII CSV
/// > (comma separated values) data"
/// > Source: FEC_Format_8.4.pdf, page 3
#[derive(Debug)]
pub struct FilingHeader {
    pub header_record: StringRecord,
    pub record_type: String,
    pub ef_type: String,
    pub fec_version: String,
    pub software_name: String,
    pub software_version: String,
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
            "8.1" | "8.2" | "8.3" | "8.4" | "8.5" => (),
            _ => {
                return Err(FilingHeaderError::UnsupportedVersion(format!(
                    "Unsupported version '{fec_version}', only 8.5 is currently supported."
                )));
            }
        }
        let software_name = header_get_field!(hdr, 3, "soft_name");
        let software_version = header_get_field!(hdr, 4, "soft_ver");
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
            software_name,
            software_version,
            report_id,
            report_number,
            comment,
        })
    }
}

pub fn report_code_label(report_code: &str) -> &'static str {
    // labels from: https://api.open.fec.gov/developers/#/filings/get_v1_filings_:~:text=(query)-,Name%20of%20report%20where%20the%20underlying%20data%20comes%20from%3A,-%2D%2010D%20Pre%2DElection
    // also: https://www.fec.gov/campaign-finance-data/report-type-code-descriptions/
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
        "M5" => "May Month        ly",
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

/// > "The second record will be a "cover" record for the particular filing,
/// > (for example, a F3 or and F3X record for a FEC-3 or FEC-3X electronic report)."
pub struct FilingCover {
    pub record: StringRecord,
    pub record_column_names: Vec<String>,
    pub form_type: String,
    pub filer_id: String,
    pub filer_name: String,
    pub report_code: Option<String>,
    pub coverage_from_date: Option<Date>,
    pub coverage_through_date: Option<Date>,
    pub cover_record_kv: IndexMap<String, String>,
    pub cover_data: Option<Cover>,
}

impl FilingCover {
    fn from_record(fec_version: &str, cover_record: StringRecord) -> Result<Self, String> {
        let form_type = cover_record
            .get(0)
            .ok_or_else(|| "Cover record row contains 0 fields".to_owned())?
            .to_owned();

        let columns = column_names_for_field(form_type.as_str(), fec_version)
            .map_err(|e| format!("Error getting column names for form type '{form_type}': {e}"))?;
        let mut cover_record_kv = IndexMap::new();
        for (column_name, field) in columns.iter().zip(cover_record.iter()) {
            cover_record_kv.insert(column_name.to_owned(), field.to_owned());
        }

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

        // provided as '20240901'
        let coverage_from_date = match columns
            .iter()
            .position(|v| v == "coverage_from_date")
            .and_then(|idx| cover_record.get(idx).map(|s| s.to_owned()))
            .map(|s| Date::strptime("%Y%m%d", s))
        {
            Some(Ok(date)) => Some(date),
            None => None,
            // TODO: F5 forms sometimes have a coverage_from_date column but the value is empty? ex FEC-1917549
            Some(Err(_)) => None,
        };

        let coverage_through_date = match columns
            .iter()
            .position(|v| v == "coverage_through_date")
            .and_then(|idx| cover_record.get(idx).map(|s| s.to_owned()))
            .map(|s| Date::strptime("%Y%m%d", s))
        {
            Some(Ok(date)) => Some(date),
            None => None,
            // TODO: F5 forms sometimes have a coverage_from_date column but the value is empty? ex FEC-1917549
            Some(Err(_)) => None,
        };

        let filer_id = cover_record
            .get(id_idx)
            .ok_or_else(|| "Cover record missing filer ID field".to_owned())?
            .to_owned();
        let filer_name = cover_record
            .get(name_idx)
            .ok_or_else(|| "Cover record missing filer name field".to_owned())?
            .to_owned();
        let cover_data = covers::cover_from_form_type(&form_type, &cover_record_kv);
        Ok(Self {
            record: cover_record,
            record_column_names: columns.to_owned(),
            form_type,
            filer_id,
            filer_name,
            report_code,
            coverage_from_date,
            coverage_through_date,
            cover_record_kv,
            cover_data,
        })
    }
}

pub struct Filing<R: Read> {
    pub filing_id: String,
    pub header: FilingHeader,
    pub cover: FilingCover,
    records_iter: ByteRecordsIntoIter<R>,
    pub source_length: usize,
}

impl<R: Read> Filing<R> {
    pub fn from_reader(rdr: R, filing_id: String, source_length: usize) -> anyhow::Result<Self> {
        let csv_reader = csv::ReaderBuilder::new()
            .delimiter(b"\x1c"[0])
            .flexible(true)
            .has_headers(false)
            .from_reader(rdr);

        let mut records_iter = csv_reader.into_byte_records();

        let hdr = records_iter
            .next()
            .ok_or_else(|| anyhow::anyhow!("no header record found"))??;

        let hdr_record_type = String::from_utf8(
            hdr.get(0)
                .ok_or_else(|| anyhow::anyhow!("file missing header"))?
                .to_vec(),
        )
        .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in header record type: {}", e))?;
        if hdr_record_type != "HDR" {
            return Err(anyhow::anyhow!(
                "Incorrect header record type: {hdr_record_type}"
            ));
        }

        let hdr_record = StringRecord::from_byte_record_lossy(hdr);
        let cover_record = records_iter
            .next()
            .ok_or_else(|| anyhow::anyhow!("No cover record found (2nd record missing)"))??;
        let header = FilingHeader::from_record(hdr_record)?;
        let cover = FilingCover::from_record(
            &header.fec_version,
            StringRecord::from_byte_record_lossy(cover_record),
        )
        .map_err(|e| anyhow::anyhow!("Error parsing cover record: {}", e))?;

        Ok(Self {
            filing_id: filing_id
                .strip_prefix("FEC-")
                .unwrap_or(&filing_id)
                .to_owned(),
            header,
            cover,
            records_iter,
            source_length,
        })
    }

    pub fn from_path(filing_path: &Path) -> anyhow::Result<Filing<fs::File>> {
        let filing_id = filing_path
            .file_stem()
            .map(|v| v.to_string_lossy().into_owned())
            .ok_or_else(|| anyhow::anyhow!("Unknown filing id for {:?}", filing_path))?;

        let filing_file = std::fs::File::open(filing_path)?;
        let source_length = filing_file.metadata().map(|v| v.len() as usize)?;

        Filing::from_reader(filing_file, filing_id.to_string(), source_length)
    }

    /// Return the next itemization row in the filing, or None if at end of file.
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
            Some(field) => field.to_owned(),
            None => {
                return Some(Err(FilingRowReadError::EmptyRecord(
                    record.position().map(|p| p.line()).unwrap_or(0),
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
                                let record = match record {
                                    Ok(r) => r,
                                    Err(e) => return Some(Err(FilingRowReadError::CsvError(e))),
                                };
                                let original_size = record.as_slice().len();
                                let record = StringRecord::from_byte_record_lossy(record);
                                let row_type = record
                                    .get(0)
                                    .map(|s| s.to_owned())
                                    .unwrap_or_else(|| String::from(""));
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
                    // Sometimes the file ends without an [ENDTEXT] (ex FEC-1888492), in which we assume the rest of the file is a text entry.
                    None => return None,
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
    #[allow(unused_imports)]
    use crate::*;
}
