//! The record source for the filing body.
//!
//! FS (0x1C) files keep the exact `csv::Reader` path the parser has always
//! used, over the untouched byte stream: the first-line peek is replayed in
//! front of the reader, nothing is consumed, so records and their positions
//! are unchanged.

use csv::{ByteRecord, ByteRecordsIntoIter};
use std::io::{self, Chain, Cursor, Read};

/// The input stream with the peeked prefix put back in front of it.
pub(crate) type Source<R> = Chain<Cursor<Vec<u8>>, R>;

/// Stop peeking for the first newline after this many bytes.
const PEEK_LIMIT: usize = 1 << 20;

/// Read from `rdr` until the first non-blank line is complete (see
/// [`first_content_line`]), the stream ends, or [`PEEK_LIMIT`] bytes have
/// been read. Returns the stream with those bytes put back in front, plus a
/// copy of them.
pub(crate) fn peek_first_line<R: Read>(mut rdr: R) -> io::Result<(Vec<u8>, Source<R>)> {
    let mut prefix = Vec::with_capacity(8 * 1024);
    let mut chunk = [0u8; 8 * 1024];
    while prefix.len() < PEEK_LIMIT {
        let n = match rdr.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        prefix.extend_from_slice(&chunk[..n]);
        if chunk[..n].iter().any(|b| is_eol(*b)) && first_content_line(&prefix).is_some() {
            break;
        }
    }
    let copy = prefix.clone();
    Ok((copy, Cursor::new(prefix).chain(rdr)))
}

/// Whether a line holds nothing but whitespace (after an optional UTF-8 BOM).
fn is_blank_line(line: &[u8]) -> bool {
    line.strip_prefix(b"\xEF\xBB\xBF".as_slice())
        .unwrap_or(line)
        .trim_ascii()
        .is_empty()
}

fn is_eol(b: u8) -> bool {
    b == b'\n' || b == b'\r'
}

/// The first terminated line of `prefix` that is not blank, without its
/// terminator. Leading blank lines are skipped the way the csv reader of
/// the FS path skips them (a filing may start with an empty line). Lines end
/// at `\n`, `\r\n` or a lone `\r`, as for the csv reader.
fn first_content_line(prefix: &[u8]) -> Option<&[u8]> {
    let mut rest = prefix;
    while let Some(i) = rest.iter().position(|b| is_eol(*b)) {
        let line = &rest[..i];
        if !is_blank_line(line) {
            return Some(line);
        }
        rest = &rest[i + 1..];
    }
    None
}

/// The body reader.
pub(crate) enum Records<R: Read> {
    /// FS files: the csv reader over the whole, unconsumed stream.
    Csv(ByteRecordsIntoIter<Source<R>>),
}

impl<R: Read> Records<R> {
    /// The csv reader for FS files, with the options the parser has always
    /// used. No quoting: the spec forbids `"` in fields, but filings contain
    /// them anyway (`"BUD" SMITH`), and with quoting on a field that starts
    /// with `"` swallows the following delimiters and lines.
    pub(crate) fn fs(source: Source<R>) -> Self {
        Records::Csv(
            csv::ReaderBuilder::new()
                .delimiter(b"\x1c"[0])
                .flexible(true)
                .has_headers(false)
                .quoting(false)
                .from_reader(source)
                .into_byte_records(),
        )
    }
}

impl<R: Read> Iterator for Records<R> {
    type Item = csv::Result<ByteRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Records::Csv(it) => it.next(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peek_reads_past_leading_blank_lines() {
        let input = b"\n\n\nHDR\x1cFEC\nF3XN\n".as_slice();
        let (prefix, mut source) = peek_first_line(input).expect("peek");
        assert_eq!(first_content_line(&prefix), Some(b"HDR\x1cFEC".as_slice()));
        let mut all = Vec::new();
        source.read_to_end(&mut all).expect("read");
        assert_eq!(all, input);
    }
}
