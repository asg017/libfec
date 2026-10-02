//! Helpers for exporting itemization rows of any format family.
//!
//! A row's layout depends on its row type *and* the filing's version, and
//! filings can contain row types that mappings2.json does not describe for
//! that version. Exports skip such rows with one warning per row type per
//! filing instead of aborting or panicking.

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
