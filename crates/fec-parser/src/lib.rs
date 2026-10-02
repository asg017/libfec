//! Parser for FEC electronic and paper filings (`.fec`) of every format
//! family: see [`mod@format`].
//!
//! # Behaviour changes for 8.x
//!
//! Reading the legacy families (v1–v7, paper) changed a few things that 8.x
//! filings can hit. None occurs in the 48,063 cached 8.x filings (full
//! `wiki/legacy/tools/regress_8x.sh --all` against b1abe2e: 0 diffs), but
//! hand-made or unusual 8.x files see them:
//!
//! - Row types are trimmed: `SA11AI ` → `SA11AI` ([`FilingRow::row_type`];
//!   the record itself is unchanged). Paper P2.3–P3.1 pads them.
//! - `[BEGIN TEXT]`, `[BeginText]` and the other spellings of the text-block
//!   markers start/end text blocks like `[BEGINTEXT]`/`[ENDTEXT]`; before,
//!   only the exact spellings did and the others came back as rows.
//! - A cover record without a committee-name column, or cut short before it,
//!   no longer fails to parse: [`FilingCover::filer_name`] is then the typed
//!   cover's filer name, else empty.
//! - F2S rows get the F2S layout (9 columns) instead of F2's
//!   (`crates/fec-parser-macros/MAPPINGS_CHANGES.md`).
//! - Schedule row types are classified case-insensitively
//!   ([`schedules::form_type_schedule_type`]: `sa11ai` is Schedule A).
//!
//! An 8.x file that starts with blank lines parses as it did before the
//! legacy work (the format sniffer skips them, as the csv reader always did).
//!
//! In fec-cli's exports, 8.x rows of a schedule are now always written in
//! the schedule's 8.5 layout, matched by column name: the single CSV/JSON
//! export (8.0 SE and 8.4 SC2 rows were misaligned positionally), the
//! directory CSV export (whose files took the first row's layout) and the
//! sqlite `libfec_schedule_*` tables (which took the first row type's 8.5
//! layout, e.g. SA3L's in an F3L-only export; SA3L rows now fill the
//! contributor columns, as their 8.5 rows always did positionally).

#![deny(clippy::unwrap_used)]

pub mod covers;
pub mod format;
pub mod mappings;
mod reader;
pub mod schedules;

use crate::covers::Cover;
use crate::format::{is_begin_text, is_end_text, is_supported_version};
pub use crate::format::{Delimiter, HeaderStyle};
use crate::reader::{LegacyBlock, Records, Sniffed};
use csv::{ByteRecord, StringRecord};
use indexmap::IndexMap;
use jiff::civil::Date;
use mappings::column_names_for_field;
use std::iter::Peekable;
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
/// > The first record of every electronic file that is submitted to the FEC
/// > must be an HDR record that precedes the main body of the ASCII CSV
/// > (comma separated values) data"
/// > Source: FEC_Format_8.4.pdf, page 3
///
/// Three header styles exist (see [`HeaderStyle`]); fields a style does not
/// carry are empty strings / `None`:
///
/// | field | `Hdr` (3.x–8.x) | `Paper` (P1.0–P3.4) | `LegacyBlock` (1.x/2.x) |
/// |---|---|---|---|
/// | `record_type` | `HDR` | `HDR` | `/*` (there is no HDR record) |
/// | `ef_type` | `FEC` | empty | empty |
/// | `fec_version` | trimmed | trimmed, e.g. `P3.4` | `FEC_Ver_#` |
/// | `software_name` / `software_version` | as written | name only | `Soft_Name` / `Soft_Ver#` |
/// | `report_id` | yes | P2.2+ | `None` |
/// | `report_number`, `comment` | yes | `None` | `None` |
/// | `name_delimiter` | 3.x–5.x only | `None` | `NameDelim` |
/// | `batch_number` | `None` | yes | `None` |
/// | `received_date` | `None` | P2.6+ | `None` |
/// | `legacy_fields`, `schedule_counts` | empty | empty | the block's lines |
///
/// HDR column positions come from the `^hdr$` entry of mappings2.json for
/// the version, so 3.x–5.x's extra `name_delim` column and paper's layouts
/// are handled; for 6.x–8.x they are the fixed positions 0–7.
#[derive(Debug)]
pub struct FilingHeader {
    /// The HDR record as read. For [`HeaderStyle::LegacyBlock`], a synthesized
    /// record of the block's values (not the schedule counts) in file order.
    pub header_record: StringRecord,
    pub record_type: String,
    pub ef_type: String,
    pub fec_version: String,
    pub software_name: String,
    pub software_version: String,
    pub report_id: Option<String>,
    pub report_number: Option<String>,
    pub comment: Option<String>,
    /// How the header is written.
    pub style: HeaderStyle,
    /// The body's field delimiter.
    pub delimiter: Delimiter,
    /// Sub-delimiter of combined name fields (`Last^First^...`): the 3.x–5.x
    /// HDR `name_delim` column or the `/*` block's `NameDelim`. `None` when
    /// absent or empty; the spec default is `^`.
    pub name_delimiter: Option<String>,
    /// Paper filings: the FEC data-entry batch number.
    pub batch_number: Option<String>,
    /// Paper filings P2.6+: date the FEC received the paper filing.
    pub received_date: Option<String>,
    /// [`HeaderStyle::LegacyBlock`]: every `key = value` line before
    /// `Schedule_Counts:`, keys as written (`FEC_Ver_#`, `Form_Name`,
    /// `FEC_IDnum`, `Control_#`, `Dec/NoDec`, `Date_Fmat`, ...). Empty for
    /// other styles.
    pub legacy_fields: IndexMap<String, String>,
    /// [`HeaderStyle::LegacyBlock`]: the `Schedule_Counts:` lines (row type →
    /// count as written). Empty for other styles.
    pub schedule_counts: IndexMap<String, String>,
}

/// Whether HDR field 1 is a paper version (`P3.4`) rather than `FEC`.
fn is_paper_version_field(field: &str) -> bool {
    let mut chars = field.trim().chars();
    matches!(chars.next(), Some('P' | 'p')) && chars.next().is_some_and(|c| c.is_ascii_digit())
}

fn unsupported_version(fec_version: &str) -> FilingHeaderError {
    FilingHeaderError::UnsupportedVersion(format!(
        "Unsupported version '{fec_version}', supported versions are {}.",
        format::SUPPORTED_VERSION_FAMILIES
    ))
}

impl FilingHeader {
    /// Whether this is FEC data entry of a paper filing.
    pub fn is_paper(&self) -> bool {
        self.style == HeaderStyle::Paper
    }

    /// Build from an `HDR` record ([`HeaderStyle::Hdr`] or
    /// [`HeaderStyle::Paper`]).
    fn from_record(
        hdr: csv::StringRecord,
        delimiter: Delimiter,
    ) -> Result<Self, FilingHeaderError> {
        let record_type = header_get_field!(hdr, 0, "record_type");
        let style = match hdr.get(1) {
            Some(f) if is_paper_version_field(f) => HeaderStyle::Paper,
            _ => HeaderStyle::Hdr,
        };
        let (ef_type, fec_version) = match style {
            HeaderStyle::Paper => (String::new(), header_get_field!(hdr, 1, "fec_version")),
            _ => (
                header_get_field!(hdr, 1, "ef_type"),
                header_get_field!(hdr, 2, "fec_version"),
            ),
        };
        let fec_version = fec_version.trim().to_owned();
        if !is_supported_version(&fec_version) {
            return Err(unsupported_version(&fec_version));
        }
        let columns = column_names_for_field("hdr", &fec_version)
            .map_err(|_| unsupported_version(&fec_version))?;
        let idx = |name: &str| columns.iter().position(|c| c == name);
        // Required in FS `HDR` records (6.x–8.x, as always); optional in
        // paper and comma ones (136149, a 5.1 file, is just `HDR,FEC,5.1`).
        let strict = style == HeaderStyle::Hdr && delimiter == Delimiter::Fs;
        let raw = |name: &'static str| -> Result<String, FilingHeaderError> {
            match idx(name) {
                Some(i) if strict => Ok(header_get_field!(hdr, i, name)),
                Some(i) => Ok(hdr.get(i).map(str::to_owned).unwrap_or_default()),
                None => Ok(String::new()),
            }
        };
        let opt = |name: &str| {
            idx(name)
                .and_then(|i| hdr.get(i))
                .map(|v| String::from(v.trim()))
                .filter(|v| !String::is_empty(v))
        };
        let software_name = raw("soft_name")?;
        let software_version = raw("soft_ver")?;

        Ok(FilingHeader {
            record_type,
            ef_type,
            fec_version,
            software_name,
            software_version,
            report_id: opt("report_id"),
            report_number: opt("report_number"),
            comment: opt("comment"),
            style,
            delimiter,
            name_delimiter: opt("name_delim"),
            batch_number: opt("batch_number"),
            received_date: opt("received_date"),
            legacy_fields: IndexMap::new(),
            schedule_counts: IndexMap::new(),
            header_record: hdr,
        })
    }

    /// Build from a `/* Header` block ([`HeaderStyle::LegacyBlock`]).
    fn from_legacy_block(block: LegacyBlock) -> Result<Self, FilingHeaderError> {
        let fec_version = block
            .get("FEC_Ver_#")
            .ok_or_else(|| {
                FilingHeaderError::UnsupportedVersion(
                    "`/* Header` block has no `FEC_Ver_#` line".to_owned(),
                )
            })?
            .to_owned();
        if !is_supported_version(&fec_version) {
            return Err(unsupported_version(&fec_version));
        }
        let get = |key: &str| block.get(key).map(str::to_owned);
        Ok(FilingHeader {
            header_record: block.fields.values().map(String::as_str).collect(),
            record_type: "/*".to_owned(),
            ef_type: String::new(),
            software_name: get("Soft_Name").unwrap_or_default(),
            software_version: get("Soft_Ver#").unwrap_or_default(),
            report_id: None,
            report_number: None,
            comment: None,
            style: HeaderStyle::LegacyBlock,
            delimiter: Delimiter::Comma,
            name_delimiter: get("NameDelim").filter(|v| !v.is_empty()),
            batch_number: None,
            received_date: None,
            fec_version,
            legacy_fields: block.fields,
            schedule_counts: block.schedule_counts,
        })
    }
}

/// Description of a report code (`Q1` → "April Quarterly"), or
/// `"[Unknown report code]"`.
///
/// The codes are those listed under "Report Codes" in the FEC e-filing
/// specification (FEC_Format_v8.4.pdf p14–15); the descriptions follow it
/// closely but not always word for word, and a few legacy codes (e.g. `ADJ`,
/// `CA`) are not on that list.
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
        // FEC format workbook v8.4, sheet F3X, field 10: "Monthly Year-End reports
        // should be coded with 'MYE'."
        "MYE" => "Monthly Year-End",
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
    /// `name_delimiter`: the header's, for splitting legacy combined names
    /// in the typed cover.
    fn from_record(
        fec_version: &str,
        name_delimiter: Option<&str>,
        cover_record: StringRecord,
    ) -> Result<Self, String> {
        // Trimmed: paper P2.3–P3.1 pads row types (`F7N     `).
        let form_type = cover_record
            .get(0)
            .ok_or_else(|| "Cover record row contains 0 fields".to_owned())?
            .trim()
            .to_owned();

        let columns = column_names_for_field(form_type.as_str(), fec_version)
            .map_err(|e| format!("Error getting column names for form type '{form_type}': {e}"))?;
        // `mappings2.json` names every column uniquely, but if a layout ever
        // repeats a name, the first column keeps it (the same column the
        // positional lookups below find).
        let mut cover_record_kv = IndexMap::new();
        for (column_name, field) in columns.iter().zip(cover_record.iter()) {
            cover_record_kv
                .entry(column_name.to_owned())
                .or_insert_with(|| field.to_owned());
        }
        let field_of = |names: &[&str]| -> Option<&str> {
            columns
                .iter()
                .position(|v| names.contains(&v.as_str()))
                .and_then(|idx| cover_record.get(idx))
        };

        let filer_id = field_of(&["filer_committee_id_number", "candidate_id_number"])
            .ok_or_else(|| {
                format!(
                    "Cover record '{form_type}' (version {fec_version}) has no filer ID \
                     column (filer_committee_id_number or candidate_id_number) \
                     or is too short to contain it"
                )
            })?
            .to_owned();

        // Optional: paper Form 99 layouts have no committee name column, and
        // a record may be cut short before it. Falls back to the typed
        // cover's filer name (Form 5/9 individuals) below, else empty.
        let filer_name = field_of(&["committee_name", "organization_name"])
            .unwrap_or_default()
            .to_owned();

        let report_code = field_of(&["report_code"]).map(|s| s.to_owned());

        // `YYYYMMDD` (v3+ and paper) or `MM/DD/YYYY`; blank or unparsable
        // values are `None` (some F5s have the column but leave it empty,
        // e.g. FEC-1917549).
        let data = covers::fields::Data::new(cover_record_kv.clone(), name_delimiter);
        let coverage_from_date = covers::fields::date(&data, "coverage_from_date");
        let coverage_through_date = covers::fields::date(&data, "coverage_through_date");

        let cover_data = covers::cover_from_form_type(&form_type, &data);
        // Individuals filing F5/F9 leave the organization-name column blank.
        let filer_name = match cover_data.as_ref().and_then(|c| c.filer_name()) {
            Some(name) if filer_name.trim().is_empty() => name,
            _ => filer_name,
        };
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
    records_iter: Peekable<Records<R>>,
    pub source_length: usize,
}

impl<R: Read> Filing<R> {
    /// Parse the header and cover of a filing in any supported format (see
    /// [`mod@format`]), leaving the reader positioned at the first itemization
    /// row.
    ///
    /// The format is sniffed from the first line: a `/*` line starts a
    /// [`HeaderStyle::LegacyBlock`] header, which is consumed; otherwise the
    /// first line is the `HDR` record, split on FS (0x1C) if it contains one
    /// and on commas if not, and nothing is consumed ahead of the body
    /// reader, so FS files are read exactly as they always have been.
    pub fn from_reader(rdr: R, filing_id: String, source_length: usize) -> anyhow::Result<Self> {
        let (raw_header, mut records_iter) = open_header(rdr)?;
        let (header, cover_record) = match raw_header {
            RawHeader::Built { header, .. } => {
                let cover_record = next_non_blank(&mut records_iter).ok_or_else(|| {
                    anyhow::anyhow!("No cover record found after `/* Header` block")
                })??;
                (*header, cover_record)
            }
            RawHeader::Record { record, delimiter } => {
                // Some 3.00 files have an empty line between HDR and the cover.
                // The cover is read before the HDR record is validated, so
                // errors come out in the order they always have.
                let cover_record = next_non_blank(&mut records_iter).ok_or_else(|| {
                    anyhow::anyhow!("No cover record found (2nd record missing)")
                })??;
                let header = FilingHeader::from_record(record, delimiter)?;
                (header, cover_record)
            }
        };
        let mut cover = FilingCover::from_record(
            &header.fec_version,
            header.name_delimiter.as_deref(),
            StringRecord::from_byte_record_lossy(cover_record),
        )
        .map_err(|e| anyhow::anyhow!("Error parsing cover record: {}", e))?;

        // An F99's message body is not part of its cover record: it is the
        // `[BEGINTEXT]` ... `[ENDTEXT]` block on the lines right after it (FEC
        // format workbook v8.4, sheet F99). Read it now so the typed cover
        // carries it; see `covers::Form99::text`.
        if let Some(Cover::Form99(form)) = cover.cover_data.as_mut() {
            let next_is_text = matches!(
                records_iter.peek(),
                Some(Ok(r)) if r.get(0).is_some_and(is_begin_text)
            );
            if form.text.is_none() && next_is_text {
                records_iter.next();
                form.text = read_text_block(&mut records_iter);
            }
        }

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

        // Trimmed: paper P2.3–P3.1 pads row types (`SB23 `).
        let row_type = match record.get(0) {
            Some(field) => field.trim().to_owned(),
            None => {
                return Some(Err(FilingRowReadError::EmptyRecord(
                    record.position().map(|p| p.line()).unwrap_or(0),
                )));
            }
        };

        if is_begin_text(row_type.as_bytes()) {
            let mut contents = String::new();
            loop {
                match self.records_iter.next() {
                    Some(Err(e)) => return Some(Err(FilingRowReadError::TextRecordError(e))),
                    Some(Ok(record)) => match record.get(0) {
                        Some(f) if is_end_text(f) => match self.records_iter.next() {
                            Some(record) => {
                                let record = match record {
                                    Ok(r) => r,
                                    Err(e) => return Some(Err(FilingRowReadError::CsvError(e))),
                                };
                                let original_size = record.as_slice().len();
                                let record = StringRecord::from_byte_record_lossy(record);
                                let row_type = record
                                    .get(0)
                                    .map(|s| s.trim().to_owned())
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

/// The header as read by [`open_header`], before the cover.
enum RawHeader {
    /// A `/* Header` block, already parsed; `lines` is how many lines it took.
    Built {
        header: Box<FilingHeader>,
        lines: u64,
    },
    /// An `HDR` record, not yet validated.
    Record {
        record: StringRecord,
        delimiter: Delimiter,
    },
}

/// Sniff the format and read the header, leaving the record iterator at the
/// first record after it. Shared by [`Filing::from_reader`] and
/// [`read_header`].
fn open_header<R: Read>(rdr: R) -> anyhow::Result<(RawHeader, Peekable<Records<R>>)> {
    let (prefix, source) = reader::peek_first_line(rdr)?;
    let sniffed = reader::sniff(&prefix);
    drop(prefix);

    match sniffed {
        Sniffed::LegacyBlock => {
            let mut buffered = Records::buffered(source);
            let block = reader::read_legacy_block(&mut buffered)?;
            let (bytes, lines) = (block.bytes, block.lines);
            let header = FilingHeader::from_legacy_block(block)?;
            let records_iter = Records::lines(buffered, bytes, lines + 1).peekable();
            Ok((
                RawHeader::Built {
                    header: Box::new(header),
                    lines,
                },
                records_iter,
            ))
        }
        Sniffed::Fs | Sniffed::Comma => {
            let (delimiter, records) = match sniffed {
                Sniffed::Fs => (Delimiter::Fs, Records::fs(source)),
                _ => (
                    Delimiter::Comma,
                    Records::lines(Records::buffered(source), 0, 1),
                ),
            };
            let mut records_iter = records.peekable();
            let hdr = records_iter
                .next()
                .ok_or_else(|| anyhow::anyhow!("no header record found"))??;

            let hdr_record_type = String::from_utf8(
                hdr.get(0)
                    .ok_or_else(|| anyhow::anyhow!("file missing header"))?
                    .to_vec(),
            )
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in header record type: {}", e))?;
            if !hdr_record_type.trim().eq_ignore_ascii_case("HDR") {
                return Err(anyhow::anyhow!(
                    "Incorrect header record type: {hdr_record_type}"
                ));
            }

            let record = StringRecord::from_byte_record_lossy(hdr);
            Ok((RawHeader::Record { record, delimiter }, records_iter))
        }
    }
}

/// Read only the header of a filing in any supported format, without
/// requiring a cover record after it.
///
/// Returns the header and the number of lines it occupies: the whole
/// `/* Header` ... `/* End Header` block (both `/*` lines included) for
/// [`HeaderStyle::LegacyBlock`], else 1 (the `HDR` record). Uses the same
/// sniffing and header parsing as [`Filing::from_reader`]; meant for callers
/// that parse a filing line by line themselves (e.g. the Python `fecfile`
/// compatibility layer).
pub fn read_header<R: Read>(rdr: R) -> anyhow::Result<(FilingHeader, u64)> {
    match open_header(rdr)?.0 {
        RawHeader::Built { header, lines } => Ok((*header, lines)),
        RawHeader::Record { record, delimiter } => {
            Ok((FilingHeader::from_record(record, delimiter)?, 1))
        }
    }
}

/// Read the body of a `[BEGINTEXT]` block whose marker record has just been
/// consumed, up to and including the `[ENDTEXT]` record (or the end of the
/// file). Fields of a line are re-joined with the FS (`0x1C`) delimiter; in
/// comma files each text line arrives as one unsplit field (see
/// `reader::LineRecords`), so it is kept verbatim, blank lines included.
///
/// The CSV reader skips blank lines, which matter in a letter. A record's
/// reported line number is where the reader *started* looking for it, i.e.
/// it includes the blank lines skipped before it, so the gap between one
/// record's line and the next one's tells how many blank lines preceded the
/// first. Returns `None` for an empty body.
fn read_text_block<I>(records: &mut I) -> Option<String>
where
    I: Iterator<Item = csv::Result<ByteRecord>>,
{
    let line_of = |r: &ByteRecord| r.position().map(|p| p.line());
    let lines_in = |r: &ByteRecord| 1 + r.as_slice().iter().filter(|b| **b == b'\n').count() as u64;
    let mut body = String::new();
    let mut push = |record: &ByteRecord, next: Option<&ByteRecord>| {
        if let (Some(start), Some(next_start)) = (line_of(record), next.and_then(line_of)) {
            let blanks = next_start.saturating_sub(start + lines_in(record));
            for _ in 0..blanks {
                body.push('\n');
            }
        }
        let fields: Vec<&[u8]> = record.iter().collect();
        body.push_str(&String::from_utf8_lossy(&fields.join(&b'\x1c')));
        body.push('\n');
    };

    let mut pending: Option<ByteRecord> = None;
    for record in records.by_ref() {
        let Ok(record) = record else { break };
        if let Some(prev) = pending.take() {
            push(&prev, Some(&record));
        }
        if record.get(0).is_some_and(is_end_text) {
            break;
        }
        pending = Some(record);
    }
    if let Some(prev) = pending {
        push(&prev, None);
    }

    // A stray quote can make the reader swallow the closing marker into a
    // field; drop it if so.
    let body = body.trim_end();
    let body = body.strip_suffix("[ENDTEXT]").unwrap_or(body);
    let body = body.trim_end().trim_start_matches(['\n', '\r']);
    (!body.trim().is_empty()).then(|| body.to_owned())
}

/// Next record that has a non-whitespace field.
fn next_non_blank<I>(records: &mut I) -> Option<csv::Result<ByteRecord>>
where
    I: Iterator<Item = csv::Result<ByteRecord>>,
{
    records.find(|r| match r {
        Ok(r) => r.iter().any(|f| !f.trim_ascii().is_empty()),
        Err(_) => true,
    })
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
