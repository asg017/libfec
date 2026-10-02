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

/// Read from `rdr` until the first line is complete (a `\n` has been seen),
/// the stream ends, or [`PEEK_LIMIT`] bytes have been read. Returns the
/// stream with those bytes put back in front, plus a copy of them.
pub(crate) fn peek_first_line<R: Read>(mut rdr: R) -> io::Result<(Vec<u8>, Source<R>)> {
    let mut prefix = Vec::with_capacity(8 * 1024);
    let mut chunk = [0u8; 8 * 1024];
    while !prefix.contains(&b'\n') && prefix.len() < PEEK_LIMIT {
        let n = match rdr.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        prefix.extend_from_slice(&chunk[..n]);
    }
    let copy = prefix.clone();
    Ok((copy, Cursor::new(prefix).chain(rdr)))
}

/// What the first line says about the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sniffed {
    /// Starts with `/*` (after an optional UTF-8 BOM and whitespace).
    LegacyBlock,
    /// First line contains an FS (0x1C).
    Fs,
    /// Anything else: a comma-delimited `HDR` line.
    Comma,
}

pub(crate) fn sniff(prefix: &[u8]) -> Sniffed {
    let first_line = match prefix.iter().position(|b| *b == b'\n') {
        Some(i) => &prefix[..i],
        None => prefix,
    };
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
    /// Bytes consumed, including both `/*` lines.
    pub bytes: u64,
    /// Lines consumed, including both `/*` lines.
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
/// `/*`.
pub(crate) fn read_legacy_block<B: BufRead>(rdr: &mut B) -> anyhow::Result<LegacyBlock> {
    let mut block = LegacyBlock::default();
    let mut buf = Vec::new();
    let mut in_counts = false;
    loop {
        buf.clear();
        let n = rdr.read_until(b'\n', &mut buf)?;
        if n == 0 {
            anyhow::bail!("unterminated `/* Header` block: no closing `/*` line");
        }
        block.bytes += n as u64;
        block.lines += 1;
        let line = String::from_utf8_lossy(&buf);
        let line = line.trim_start_matches('\u{feff}').trim();
        if line.starts_with("/*") {
            if block.lines == 1 {
                continue;
            }
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

/// One record per physical line of a comma-delimited body.
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
            let n = match self.rdr.read_until(b'\n', &mut self.buf) {
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
    /// used.
    pub(crate) fn fs(source: Source<R>) -> Self {
        Records::Csv(
            csv::ReaderBuilder::new()
                .delimiter(b"\x1c"[0])
                .flexible(true)
                .has_headers(false)
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

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }
}
