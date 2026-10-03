//! Raw rows as one string plus offsets: every field of every row
//! concatenated, with UTF-16 end offsets (what JS `String.prototype.slice`
//! counts in). No separators, so a field holding `\n` or `\x1c` survives.

use crate::tokens::utf16_len;

/// One batch of raw rows.
#[derive(Debug, Default)]
pub struct Fields {
    /// Every field of every row, concatenated.
    pub text: String,
    /// `ends[k]`: UTF-16 end offset of field `k` within `text`.
    pub ends: Vec<u32>,
    /// `row_ends[i]`: index into `ends` one past row `i`'s last field.
    pub row_ends: Vec<u32>,
    /// `lines[i]`: row `i`'s 1-based physical line.
    pub lines: Vec<u32>,
}

/// Accumulates one batch of raw rows.
#[derive(Default)]
pub struct FieldsWriter {
    batch: Fields,
    off: u32,
}

impl FieldsWriter {
    /// Append one row.
    pub fn push<'a>(&mut self, fields: impl IntoIterator<Item = &'a str>, line: u32) {
        let b = &mut self.batch;
        for f in fields {
            self.off += utf16_len(f);
            b.ends.push(self.off);
            b.text.push_str(f);
        }
        b.row_ends.push(b.ends.len() as u32);
        b.lines.push(line);
    }

    /// Rows so far in this batch.
    pub fn rows(&self) -> usize {
        self.batch.row_ends.len()
    }

    /// Hand the batch over, leaving the writer empty.
    pub fn take(&mut self) -> Fields {
        self.off = 0;
        std::mem::take(&mut self.batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_are_utf16_units() {
        let mut w = FieldsWriter::default();
        w.push(["SA", "Muñoz", "😀x"], 3);
        w.push(["SB", "a\x1cb\nc"], 4);
        assert_eq!(w.rows(), 2);
        let b = w.take();
        assert_eq!(b.text, "SAMuñoz😀xSBa\x1cb\nc");
        assert_eq!(b.ends, [2, 7, 10, 12, 17]);
        assert_eq!(b.row_ends, [3, 5]);
        assert_eq!(b.lines, [3, 4]);
        assert_eq!(w.rows(), 0);
        w.push(["X"], 9);
        assert_eq!(w.take().ends, [1]);
    }
}
