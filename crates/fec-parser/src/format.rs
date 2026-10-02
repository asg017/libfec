//! Format families of FEC filings: how the header is written, which delimiter
//! the body uses, and which version strings are accepted.
//!
//! | family | header | body |
//! |---|---|---|
//! | electronic 1.x, 2.x | `/* Header` key = value block ([`HeaderStyle::LegacyBlock`]) | comma |
//! | electronic 3.x, 5.0–5.3 | `HDR,FEC,<ver>,...` ([`HeaderStyle::Hdr`]) | comma |
//! | electronic 6.1–6.4, 7.0, 8.0–8.5 | `HDR␜FEC␜<ver>␜...` ([`HeaderStyle::Hdr`]) | FS (0x1C) |
//! | paper P1.0, P2.2–P2.6, P3.0–P3.4 | `HDR␜P<ver>␜...`, no `FEC` column ([`HeaderStyle::Paper`]) | FS (0x1C) |
//!
//! Sources: `fec-docs/eFilingFormats` (Fec_v1.rtf, FEC_v2.rtf, Fec_v300.rtf,
//! Fec_v500.rtf, FEC_Format_v6.1.pdf App. D) and the paper samples; see
//! `wiki/legacy/FORMATS.md` and `PAPER.md`.

/// How a filing's header is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderStyle {
    /// An `HDR` record with an `FEC` column: electronic 3.x and later.
    Hdr,
    /// A `/* Header` ... `/* End Header` block of `key = value` lines:
    /// electronic 1.x and 2.x. There is no `HDR` record.
    LegacyBlock,
    /// An `HDR` record whose second column is a `P` version (`HDR␜P3.4␜...`):
    /// FEC data entry of a paper filing.
    Paper,
}

impl HeaderStyle {
    /// Stable lowercase name, for bindings and exports.
    pub fn as_str(&self) -> &'static str {
        match self {
            HeaderStyle::Hdr => "hdr",
            HeaderStyle::LegacyBlock => "legacy_block",
            HeaderStyle::Paper => "paper",
        }
    }
}

/// Field delimiter of a filing's body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delimiter {
    /// ASCII file separator, 0x1C: electronic 6.1+ and all paper filings.
    Fs,
    /// Comma, with double-quoted fields: electronic 1.x–5.3.
    Comma,
}

impl Delimiter {
    /// Stable lowercase name, for bindings and exports.
    pub fn as_str(&self) -> &'static str {
        match self {
            Delimiter::Fs => "fs",
            Delimiter::Comma => "comma",
        }
    }
}

/// The version families this parser accepts, for error messages.
pub const SUPPORTED_VERSION_FAMILIES: &str =
    "1.x, 2.x, 3.x, 5.0-5.3, 6.1-6.4, 7.0, 8.0-8.5, P1.0, P2.2-P2.6, P3.0-P3.4";

/// Whether `version` (as written in the header) belongs to a supported
/// version family.
///
/// Matching is on the leading digits of the trimmed string, so `"5.30"`,
/// `"3.01"` and `"6.1 "` are accepted, but `"60"`, `"4.0"` and `"8.6"` are
/// not. Paper versions start with `P` (either case).
pub fn is_supported_version(version: &str) -> bool {
    let v = version.trim();
    let (paper, v) = match v.strip_prefix(['P', 'p']) {
        Some(rest) => (true, rest),
        None => (false, v),
    };
    let (major, rest) = v.split_once('.').unwrap_or((v, ""));
    if major.is_empty()
        || !major.bytes().all(|b| b.is_ascii_digit())
        || !rest.bytes().all(|b| b.is_ascii_digit())
    {
        return false;
    }
    let minor = rest.bytes().next().map(|b| b - b'0');
    matches!(
        (paper, major, minor),
        (false, "1" | "2" | "3", _)
            | (false, "5", Some(0..=3))
            | (false, "6", Some(1..=4))
            | (false, "7", Some(0))
            | (false, "8", Some(0..=5))
            | (true, "1", _)
            | (true, "2", Some(2..=6))
            | (true, "3", Some(0..=4))
    )
}

/// Whether a field is a `[BEGINTEXT]` marker: caseless, optional space
/// between the words, surrounding whitespace ignored (`[BeginText]`,
/// `[BEGIN TEXT]`).
pub(crate) fn is_begin_text(field: &[u8]) -> bool {
    is_marker(field, b"BEGIN")
}

/// Whether a field is an `[ENDTEXT]` marker; see [`is_begin_text`].
pub(crate) fn is_end_text(field: &[u8]) -> bool {
    is_marker(field, b"END")
}

fn is_marker(field: &[u8], word: &[u8]) -> bool {
    let f = field.trim_ascii();
    let Some(inner) = f.strip_prefix(b"[").and_then(|f| f.strip_suffix(b"]")) else {
        return false;
    };
    if inner.len() < word.len() || !inner[..word.len()].eq_ignore_ascii_case(word) {
        return false;
    }
    let rest = &inner[word.len()..];
    let rest = rest.strip_prefix(b" ").unwrap_or(rest);
    rest.eq_ignore_ascii_case(b"TEXT")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_versions() {
        for v in [
            "1", "1.00", "1.02", "2.00", "2.02", "3", "3.0", "3.00", "3.01", "5.00", "5.1", "5.20",
            "5.3", "5.30", "6.1", "6.1 ", " 6.4", "7.0", "7.0 ", "8.0", "8.5", "P1.0", "P1",
            "P2.2", "P2.4", "P2.6", "P3.0", "P3.4", "p3.2",
        ] {
            assert!(is_supported_version(v), "{v:?} should be supported");
        }
        for v in [
            "", "8.6", "4.0", "60", "6.0", "7.1", "5.4", "9.0", "P2.1", "P3.5", "P4.0", "FEC",
            "8.5a", "8", "P", "P.1", "..",
        ] {
            assert!(!is_supported_version(v), "{v:?} should be rejected");
        }
    }

    #[test]
    fn text_markers() {
        for m in [
            "[BEGINTEXT]",
            "[BeginText]",
            "[BEGIN TEXT]",
            " [begintext] \r",
        ] {
            assert!(is_begin_text(m.as_bytes()), "{m:?}");
            assert!(!is_end_text(m.as_bytes()), "{m:?}");
        }
        for m in ["[ENDTEXT]", "[EndText]", "[END TEXT]", "[endtext]\t"] {
            assert!(is_end_text(m.as_bytes()), "{m:?}");
        }
        for m in ["BEGINTEXT", "[BEGINTEXTS]", "[BEGIN  TEXT]", "[END]", ""] {
            assert!(
                !is_begin_text(m.as_bytes()) && !is_end_text(m.as_bytes()),
                "{m:?}"
            );
        }
    }
}
