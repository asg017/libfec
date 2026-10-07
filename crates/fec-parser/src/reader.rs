//! Record sources for the two body delimiters, plus the first-line sniffing
//! and `/* Header` block parsing that decide between them.
//!
//! FS (0x1C) files keep the exact `csv::Reader` path the parser has always
//! used, over the untouched byte stream: the first-line peek is replayed in
//! front of the reader, nothing is consumed, so records and their positions
//! are unchanged. Comma files (electronic 1.x–5.3) are read one physical line
//! at a time by [`LineRecords`].

use crate::format::{is_begin_text, is_end_text};
use csv::{ByteRecord, ByteRecordsIntoIter, Position};
use indexmap::IndexMap;
use std::io::{self, BufRead, BufReader, Chain, Cursor, Read};

/// The input stream with the peeked prefix put back in front of it.
pub(crate) type Source<R> = Chain<Cursor<Vec<u8>>, R>;

/// Stop peeking for the first newline after this many bytes.
const PEEK_LIMIT: usize = 1 << 20;

/// Buffer size for line-based readers.
const LINE_BUFFER: usize = 64 * 1024;

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

/// What the first line says about the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sniffed {
    /// Starts with `/*` (after an optional UTF-8 BOM and whitespace).
    /// Like the other variants, judged on the first non-blank line.
    LegacyBlock,
    /// First line contains an FS (0x1C).
    Fs,
    /// Anything else: a comma-delimited `HDR` line.
    Comma,
}

/// Sniff the first non-blank line of `prefix` (or, if no line of it is
/// complete, whatever follows its leading blank lines).
pub(crate) fn sniff(prefix: &[u8]) -> Sniffed {
    let first_line = first_content_line(prefix).unwrap_or_else(|| {
        let start = prefix.iter().rposition(|b| is_eol(*b)).map_or(0, |i| i + 1);
        &prefix[start..]
    });
    let start = first_line
        .strip_prefix(b"\xEF\xBB\xBF".as_slice())
        .unwrap_or(first_line)
        .trim_ascii_start();
    if start.starts_with(b"/*") {
        Sniffed::LegacyBlock
    } else if first_line.contains(&0x1c) {
        Sniffed::Fs
    } else {
        Sniffed::Comma
    }
}

/// A parsed `/* Header` ... `/* End Header` block (electronic 1.x / 2.x).
#[derive(Debug, Default)]
pub(crate) struct LegacyBlock {
    /// `key = value` lines before `Schedule_Counts:`, in file order, keys as
    /// written. The first occurrence of a key wins.
    pub fields: IndexMap<String, String>,
    /// `key = value` lines after `Schedule_Counts:`.
    pub schedule_counts: IndexMap<String, String>,
    /// Bytes consumed, including both `/*` lines and any blank lines before
    /// the first.
    pub bytes: u64,
    /// Lines consumed, including both `/*` lines and any blank lines before
    /// the first.
    pub lines: u64,
}

impl LegacyBlock {
    /// Value of `key`, compared case-insensitively (`FEC_Ver_#` /
    /// `FEC_VER_#`).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }
}

/// Consume a `/* Header` block, through the closing line that starts with
/// `/*`. Lines before the opening `/*` line (blank ones, as sniffing only
/// gets here when the first non-blank line starts with `/*`) are consumed
/// and counted too.
pub(crate) fn read_legacy_block<B: BufRead>(rdr: &mut B) -> anyhow::Result<LegacyBlock> {
    let mut block = LegacyBlock::default();
    let mut buf = Vec::new();
    let mut in_counts = false;
    let mut opened = false;
    loop {
        buf.clear();
        let n = read_line(rdr, &mut buf)?;
        if n == 0 {
            anyhow::bail!("unterminated `/* Header` block: no closing `/*` line");
        }
        block.bytes += n as u64;
        block.lines += 1;
        let line = String::from_utf8_lossy(&buf);
        let line = line.trim_start_matches('\u{feff}').trim();
        if !opened {
            // Blank lines before the opening `/* Header` line.
            opened = line.starts_with("/*");
            continue;
        }
        if line.starts_with("/*") {
            return Ok(block);
        }
        if line
            .get(..15)
            .is_some_and(|p| p.eq_ignore_ascii_case("schedule_counts"))
        {
            in_counts = true;
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let target = if in_counts {
            &mut block.schedule_counts
        } else {
            &mut block.fields
        };
        target
            .entry(key.trim().to_owned())
            .or_insert_with(|| value.trim().to_owned());
    }
}

/// Append one line to `buf`, terminator included, and return its length in
/// bytes (0 at the end of the stream). A line ends at `\n`, `\r\n` or a
/// lone `\r` (old Mac files), the terminators the csv reader of the FS path
/// accepts.
fn read_line<B: BufRead>(rdr: &mut B, buf: &mut Vec<u8>) -> io::Result<usize> {
    let mut total = 0;
    loop {
        let available = match rdr.fill_buf() {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        if available.is_empty() {
            return Ok(total);
        }
        let Some(i) = available.iter().position(|b| is_eol(*b)) else {
            let n = available.len();
            buf.extend_from_slice(available);
            rdr.consume(n);
            total += n;
            continue;
        };
        let cr = available[i] == b'\r';
        buf.extend_from_slice(&available[..=i]);
        rdr.consume(i + 1);
        total += i + 1;
        if cr {
            // `\r\n`: the `\n` may be in the next buffer.
            loop {
                match rdr.fill_buf() {
                    Ok(b) if b.first() == Some(&b'\n') => {
                        buf.push(b'\n');
                        rdr.consume(1);
                        total += 1;
                    }
                    Ok(_) => {}
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
                break;
            }
        }
        return Ok(total);
    }
}

/// One record per physical line of a comma-delimited body (lines end at
/// `\n`, `\r\n` or a lone `\r`, see [`read_line`]).
///
/// - Fields are split with csv-core's quote rules (the same as the FS path:
///   `"a,b"` → `a,b`, `""` inside quotes → `"`, `"X" Y` → `X Y`), but a quote
///   never runs past the end of its line.
/// - Blank lines are skipped, except inside a `[BEGINTEXT]` block.
/// - Inside a text block every line, blank or not, is yielded unsplit as a
///   one-field record holding the raw line, so commas and quotes in a letter
///   survive; the `[ENDTEXT]` line is split as usual.
/// - Every record carries a [`Position`]: absolute byte offset of the line in
///   the file (including any consumed `/*` block), 1-based line number, and
///   0-based record index.
pub(crate) struct LineRecords<B> {
    rdr: B,
    buf: Vec<u8>,
    byte: u64,
    line: u64,
    record: u64,
    in_text: bool,
    core: csv_core::Reader,
    out: Vec<u8>,
    ends: Vec<usize>,
}

impl<B: BufRead> LineRecords<B> {
    /// `byte` and `line` are the absolute offset and 1-based number of the
    /// first line `rdr` will return.
    pub(crate) fn new(rdr: B, byte: u64, line: u64) -> Self {
        Self {
            rdr,
            buf: Vec::with_capacity(4 * 1024),
            byte,
            line,
            record: 0,
            in_text: false,
            core: csv_core::ReaderBuilder::new()
                .delimiter(b',')
                .terminator(csv_core::Terminator::Any(b'\n'))
                .build(),
            out: vec![0; 1024],
            ends: vec![0; 64],
        }
    }

    fn split(&mut self, line: &[u8]) -> ByteRecord {
        self.core.reset();
        let mut input = line;
        let mut eof = false;
        let (mut nout, mut nends) = (0, 0);
        loop {
            let (res, i, o, e) =
                self.core
                    .read_record(input, &mut self.out[nout..], &mut self.ends[nends..]);
            input = &input[i..];
            nout += o;
            nends += e;
            match res {
                csv_core::ReadRecordResult::InputEmpty => {
                    if eof {
                        break;
                    }
                    eof = input.is_empty();
                }
                csv_core::ReadRecordResult::OutputFull => {
                    let len = self.out.len();
                    self.out.resize(len * 2, 0);
                }
                csv_core::ReadRecordResult::OutputEndsFull => {
                    let len = self.ends.len();
                    self.ends.resize(len * 2, 0);
                }
                csv_core::ReadRecordResult::Record | csv_core::ReadRecordResult::End => break,
            }
        }
        let mut record = ByteRecord::with_capacity(nout, nends);
        let mut start = 0;
        for &end in &self.ends[..nends] {
            record.push_field(&self.out[start..end]);
            start = end;
        }
        record
    }
}

impl<B: BufRead> Iterator for LineRecords<B> {
    type Item = csv::Result<ByteRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.buf.clear();
            let n = match read_line(&mut self.rdr, &mut self.buf) {
                Ok(0) => return None,
                Ok(n) => n,
                Err(e) => return Some(Err(e.into())),
            };
            let (byte, line) = (self.byte, self.line);
            self.byte += n as u64;
            self.line += 1;

            let mut content = std::mem::take(&mut self.buf);
            if content.last() == Some(&b'\n') {
                content.pop();
            }
            if content.last() == Some(&b'\r') {
                content.pop();
            }

            let record = if self.in_text {
                let trimmed = content.trim_ascii_start();
                let maybe_marker = trimmed.starts_with(b"[") || trimmed.starts_with(b"\"");
                let split = maybe_marker.then(|| self.split(&content));
                match split {
                    Some(r) if r.get(0).is_some_and(is_end_text) => {
                        self.in_text = false;
                        r
                    }
                    _ => {
                        let mut r = ByteRecord::with_capacity(content.len(), 1);
                        r.push_field(&content);
                        r
                    }
                }
            } else if content.trim_ascii().is_empty() {
                self.buf = content;
                continue;
            } else {
                let r = self.split(&content);
                if r.get(0).is_some_and(is_begin_text) {
                    self.in_text = true;
                }
                r
            };
            self.buf = content;

            let mut record = record;
            let mut pos = Position::new();
            pos.set_byte(byte).set_line(line).set_record(self.record);
            record.set_position(Some(pos));
            self.record += 1;
            return Some(Ok(record));
        }
    }
}

/// The body reader for either delimiter.
pub(crate) enum Records<R: Read> {
    /// FS files: the csv reader over the whole, unconsumed stream.
    Csv(ByteRecordsIntoIter<Source<R>>),
    /// Comma files. Boxed: the line reader carries its own buffers.
    Lines(Box<LineRecords<BufReader<Source<R>>>>),
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

    pub(crate) fn lines(rdr: BufReader<Source<R>>, byte: u64, line: u64) -> Self {
        Records::Lines(Box::new(LineRecords::new(rdr, byte, line)))
    }

    pub(crate) fn buffered(source: Source<R>) -> BufReader<Source<R>> {
        BufReader::with_capacity(LINE_BUFFER, source)
    }
}

impl<R: Read> Iterator for Records<R> {
    type Item = csv::Result<ByteRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Records::Csv(it) => it.next(),
            Records::Lines(it) => it.next(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(input: &str) -> Vec<(Vec<String>, u64, u64, u64)> {
        LineRecords::new(input.as_bytes(), 0, 1)
            .map(|r| {
                let r = r.expect("record");
                let p = r.position().expect("position").clone();
                let fields = r
                    .iter()
                    .map(|f| String::from_utf8_lossy(f).into_owned())
                    .collect();
                (fields, p.byte(), p.line(), p.record())
            })
            .collect()
    }

    #[test]
    fn splits_quoted_fields_per_line() {
        let got = lines("HDR,\"FEC\",\"a,b\",,\"x\"\"y\"\r\n\r\n\"SA\",\"open\nnext,line\n");
        assert_eq!(
            got,
            vec![
                (s(&["HDR", "FEC", "a,b", "", "x\"y"]), 0, 1, 0),
                (s(&["SA", "open"]), 27, 3, 1),
                (s(&["next", "line"]), 38, 4, 2),
            ]
        );
    }

    #[test]
    fn text_blocks_keep_raw_lines_and_blanks() {
        let got = lines("F99,C1\n[BeginText]\nHello, \"world\"\n\n  \nBye\n[END TEXT]\n\nSA,1\n");
        let fields: Vec<Vec<String>> = got.iter().map(|g| g.0.clone()).collect();
        assert_eq!(
            fields,
            vec![
                s(&["F99", "C1"]),
                s(&["[BeginText]"]),
                s(&["Hello, \"world\""]),
                s(&[""]),
                s(&["  "]),
                s(&["Bye"]),
                s(&["[END TEXT]"]),
                s(&["SA", "1"]),
            ]
        );
        let line_numbers: Vec<u64> = got.iter().map(|g| g.2).collect();
        assert_eq!(line_numbers, vec![1, 2, 3, 4, 5, 6, 7, 9]);
    }

    #[test]
    fn legacy_block() {
        let input = "/* Header\nFEC_VER_# = 2.02\nSoft_Name = X = Y\nSchedule_Counts:\nSA11A1    = 00004\n/* End Header\nF3XN,C1\n";
        let mut rdr = input.as_bytes();
        let block = read_legacy_block(&mut rdr).expect("block");
        assert_eq!(block.get("fec_ver_#"), Some("2.02"));
        assert_eq!(block.get("Soft_Name"), Some("X = Y"));
        assert_eq!(
            block.schedule_counts.get("SA11A1").map(String::as_str),
            Some("00004")
        );
        assert_eq!(block.lines, 6);
        assert_eq!(block.bytes as usize, input.len() - "F3XN,C1\n".len());
        assert_eq!(rdr, b"F3XN,C1\n");
    }

    #[test]
    fn sniffing() {
        assert_eq!(sniff(b"/* Header\n"), Sniffed::LegacyBlock);
        assert_eq!(sniff(b"\xEF\xBB\xBF /* Header"), Sniffed::LegacyBlock);
        assert_eq!(sniff(b"HDR\x1cFEC\x1c8.4\n"), Sniffed::Fs);
        assert_eq!(sniff(b"HDR,FEC,5.3\nSA\x1c\n"), Sniffed::Comma);
    }

    #[test]
    fn sniffing_skips_leading_blank_lines() {
        assert_eq!(sniff(b"\nHDR\x1cFEC\x1c8.4\n"), Sniffed::Fs);
        assert_eq!(sniff(b"\r\n  \n\nHDR\x1cFEC\x1c8.4"), Sniffed::Fs);
        assert_eq!(sniff(b"\n\n/* Header\n"), Sniffed::LegacyBlock);
        assert_eq!(sniff(b"\nHDR,FEC,5.3\nSA\x1c\n"), Sniffed::Comma);
    }

    #[test]
    fn peek_reads_past_leading_blank_lines() {
        let input = b"\n\n\nHDR\x1cFEC\nF3XN\n".as_slice();
        let (prefix, mut source) = peek_first_line(input).expect("peek");
        assert_eq!(first_content_line(&prefix), Some(b"HDR\x1cFEC".as_slice()));
        let mut all = Vec::new();
        source.read_to_end(&mut all).expect("read");
        assert_eq!(all, input);
    }

    #[test]
    fn lone_cr_ends_lines() {
        let got = lines("HDR,FEC,5.3\rF3XN,C1\r\rSA,1\r\nSB,2\nSC,3\r");
        assert_eq!(
            got,
            vec![
                (s(&["HDR", "FEC", "5.3"]), 0, 1, 0),
                (s(&["F3XN", "C1"]), 12, 2, 1),
                (s(&["SA", "1"]), 21, 4, 2),
                (s(&["SB", "2"]), 27, 5, 3),
                (s(&["SC", "3"]), 32, 6, 4),
            ]
        );
        assert_eq!(sniff(b"\rHDR\x1cFEC\rF3XN"), Sniffed::Fs);
        assert_eq!(sniff(b"HDR,FEC,5.3\rSA\x1c\r"), Sniffed::Comma);
        let mut rdr = "/* Header\rFEC_VER_# = 2.02\r/* End Header\rF3XN,C1\r".as_bytes();
        let block = read_legacy_block(&mut rdr).expect("block");
        assert_eq!((block.get("FEC_Ver_#"), block.lines), (Some("2.02"), 3));
        assert_eq!(rdr, b"F3XN,C1\r");
    }

    #[test]
    fn crlf_split_across_buffers() {
        // A 4-byte buffer puts the `\r` and the `\n` in different fills.
        let input = "SA,1\r\nSB,2\r\n";
        let rdr = BufReader::with_capacity(4, input.as_bytes());
        let got: Vec<(u64, u64)> = LineRecords::new(rdr, 0, 1)
            .map(|r| {
                let p = r.expect("record").position().expect("position").clone();
                (p.byte(), p.line())
            })
            .collect();
        assert_eq!(got, vec![(0, 1), (6, 2)]);
    }

    #[test]
    fn legacy_block_after_blank_lines() {
        let input = "\n  \n/* Header\nFEC_VER_# = 2.02\n/* End Header\nF3XN,C1\n";
        let mut rdr = input.as_bytes();
        let block = read_legacy_block(&mut rdr).expect("block");
        assert_eq!(block.get("FEC_Ver_#"), Some("2.02"));
        assert_eq!(block.lines, 5);
        assert_eq!(block.bytes as usize, input.len() - "F3XN,C1\n".len());
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }
}
