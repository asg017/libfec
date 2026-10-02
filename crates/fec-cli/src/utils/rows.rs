//! Helpers for exporting itemization rows of any format family.
//!
//! A row's layout depends on its row type *and* the filing's version, and
//! legacy filings (1.x–5.x, paper) contain row types that mappings2.json
//! does not (yet) describe for that version. Exports skip such rows with one
//! warning per row type per filing instead of aborting or panicking.

use std::collections::HashSet;

use indicatif::MultiProgress;

/// Prints a warning the first time an unmapped row type is seen in a filing.
/// Shared across filings; keyed by (filing id, row type).
pub struct UnmappedRows<'a> {
    seen: HashSet<(String, String)>,
    mb: Option<&'a MultiProgress>,
}

impl<'a> UnmappedRows<'a> {
    pub fn new(mb: Option<&'a MultiProgress>) -> Self {
        Self {
            seen: HashSet::new(),
            mb,
        }
    }

    /// Warn about an unmapped row, once per row type per filing; `what`
    /// says what happens to such rows (e.g. "skipping them").
    pub fn warn(&mut self, filing_id: &str, row_type: &str, fec_version: &str, what: &str) {
        if self
            .seen
            .insert((filing_id.to_owned(), row_type.to_owned()))
        {
            let msg = format!(
                "warning: FEC-{filing_id}: no column mapping for row type '{row_type}' in version {fec_version}, {what}"
            );
            warn(self.mb, msg);
        }
    }
}

/// Print a warning above the progress bars, or straight to stderr when
/// there are none: a hidden `MultiProgress` (stderr not a terminal, as in
/// scripts and CI) silently drops `println`.
pub fn warn(mb: Option<&MultiProgress>, msg: impl AsRef<str>) {
    match mb.filter(|mb| !mb.is_hidden()) {
        Some(mb) => {
            let _ = mb.println(msg.as_ref());
        }
        None => eprintln!("{}", msg.as_ref()),
    }
}

/// The error for an export in which every input filing failed to open
/// (each was already reported by [`warn`]), so it exits non-zero instead of
/// writing an empty result.
pub fn all_filings_failed(failed: usize) -> anyhow::Error {
    anyhow::anyhow!(
        "none of the {failed} input filing{} could be read; nothing exported",
        if failed == 1 { "" } else { "s" }
    )
}

/// `YYYY-MM-DD` from a `YYYYMMDD` or `MM/DD/YYYY` date (the latter as written
/// in some legacy and paper filings), or `None` for anything else.
pub fn normalize_fec_date(value: &str) -> Option<String> {
    let v = value.trim();
    let b = v.as_bytes();
    let digits = |s: &[u8]| s.iter().all(u8::is_ascii_digit);
    if b.len() == 8 && digits(b) {
        return Some(format!("{}-{}-{}", &v[0..4], &v[4..6], &v[6..8]));
    }
    let mut parts = v.split('/');
    if let (Some(m), Some(d), Some(y), None) = (parts.next(), parts.next(), parts.next(), parts.next())
    {
        if (1..=2).contains(&m.len())
            && (1..=2).contains(&d.len())
            && y.len() == 4
            && digits(m.as_bytes())
            && digits(d.as_bytes())
            && digits(y.as_bytes())
        {
            return Some(format!("{y}-{m:0>2}-{d:0>2}"));
        }
    }
    None
}

/// A row type usable as a file name: `SC/10` → `SC-10` (as FastFEC does).
pub fn file_stem(row_type: &str) -> String {
    row_type
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(normalize_fec_date("20240131").as_deref(), Some("2024-01-31"));
        assert_eq!(normalize_fec_date("01/31/2024").as_deref(), Some("2024-01-31"));
        assert_eq!(normalize_fec_date("1/3/1998").as_deref(), Some("1998-01-03"));
        assert_eq!(normalize_fec_date(""), None);
        assert_eq!(normalize_fec_date("1/1/98"), None);
        assert_eq!(normalize_fec_date("2024013é"), None);
        assert_eq!(normalize_fec_date("abcdefgh"), None);
    }

    #[test]
    fn file_stems() {
        assert_eq!(file_stem("SC/10"), "SC-10");
        assert_eq!(file_stem("SA11AI"), "SA11AI");
        assert_eq!(file_stem("../x"), "---x");
    }
}
