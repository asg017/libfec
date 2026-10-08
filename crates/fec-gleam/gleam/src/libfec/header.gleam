//// The filing's header record (`HDR`, or a legacy `/* Header` block).

import gleam/option.{type Option}

/// The header every filing format has. Fields a format doesn't carry are
/// empty strings / `None` (paper filings have no report number or comment;
/// 1.x/2.x `/* Header` blocks have no report id either).
pub type Header {
  Header(
    /// The format version, trimmed: `"8.4"`, `"P3.4"`, `"2.02"`.
    fec_version: String,
    /// The software that produced the file, as written.
    software_name: String,
    /// Its version, as written (empty for paper filings).
    software_version: String,
    /// `FEC-1234567` for an amendment's original, or the filer's own id.
    report_id: Option(String),
    /// The report number (amendment sequence), as written.
    report_number: Option(String),
    /// The header comment.
    comment: Option(String),
  )
}
