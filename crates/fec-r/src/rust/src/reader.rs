//! Reading a filing's rows into per-family tables, in plain Rust (no R API).
//! `crate::robj` turns the result into R vectors.
//!
//! Three kinds of table:
//! - **typed** ([`TypedTable`]): the family's typed struct, flattened by
//!   `fec_parser::columnar` into [`ColumnBuilder`]s;
//! - **raw** ([`RawTable`]): the filing version's mapping columns, used for
//!   every family in `raw = TRUE` mode and, in typed mode, for rows with no
//!   typed struct or no layout (fallback);
//! - **other** (a [`RawTable`] too): rows `record_family()` doesn't
//!   recognize, as `row_type`, `field_1`, ….
//!
//! Raw tables keep the parsed records and build columns only at conversion
//! time, so a row costs one `StringRecord`, not one `String` per cell.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use csv::StringRecord;
use fec_parser::columnar::{new_builders, ColKind, ColumnBuilder};
use fec_parser::itemizations::{record_family, Itemization};
use fec_parser::mappings::{column_names_for_field, DATE_COLUMNS, FLOAT_COLUMNS};
use fec_parser::{Filing, FilingRow, FilingRowReadError};

use crate::names::{raw_table_name, table_name, OTHER_TABLE};

/// How a table's columns were made; the `table_kinds` value in R.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableKind {
    Typed,
    Raw,
    Other,
}

impl TableKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TableKind::Typed => "typed",
            TableKind::Raw => "raw",
            TableKind::Other => "other",
        }
    }
}

/// A family's typed rows: one builder per flattened column.
pub struct TypedTable {
    pub names: Vec<String>,
    pub builders: Vec<ColumnBuilder>,
}

impl TypedTable {
    fn new(family: &str) -> Self {
        // `family` comes from `Itemization::family()`, so it always has columns.
        let defs = Itemization::family_columns(family).unwrap_or_default();
        TypedTable {
            names: defs.iter().map(|d| d.name.clone()).collect(),
            builders: new_builders(&defs, 0),
        }
    }

    pub fn nrow(&self) -> usize {
        self.builders.first().map_or(0, ColumnBuilder::len)
    }
}

/// Rows kept as records, with columns resolved per row type ("layout").
///
/// The column set is the union of every layout's names in first-seen order
/// (two layouts in one family, e.g. `SC2` 8.4 vs 8.5, share the names they
/// have in common), then `extra_1`, … for fields past a row's layout. A cell
/// a row doesn't have is `NA`.
pub struct RawTable {
    pub names: Vec<String>,
    pub kinds: Vec<ColKind>,
    index: HashMap<String, usize>,
    /// Per layout: the column of each of its fields.
    layouts: Vec<Vec<usize>>,
    /// Row type (as filed, trimmed) → layout.
    layout_of: HashMap<String, usize>,
    /// Extra field `k` (0-based, past the row's layout) → column.
    extra_cols: Vec<usize>,
    extra_prefix: &'static str,
    /// `other` tables: a fixed one-column layout (`row_type`), all text.
    other: bool,
    pub rows: Vec<(StringRecord, usize)>,
}

impl RawTable {
    fn new_raw() -> Self {
        Self::empty("extra_", false)
    }

    fn new_other() -> Self {
        Self::empty("field_", true)
    }

    fn empty(extra_prefix: &'static str, other: bool) -> Self {
        RawTable {
            names: Vec::new(),
            kinds: Vec::new(),
            index: HashMap::new(),
            layouts: Vec::new(),
            layout_of: HashMap::new(),
            extra_cols: Vec::new(),
            extra_prefix,
            other,
            rows: Vec::new(),
        }
    }

    pub fn nrow(&self) -> usize {
        self.rows.len()
    }

    fn add_column(&mut self, name: String, kind: ColKind) -> usize {
        let j = self.names.len();
        self.index.insert(name.clone(), j);
        self.names.push(name);
        self.kinds.push(kind);
        j
    }

    /// The column called `name`, added if new.
    fn column(&mut self, name: &str, kind: ColKind) -> usize {
        match self.index.get(name) {
            Some(&j) => j,
            None => self.add_column(name.to_owned(), kind),
        }
    }

    fn layout(&mut self, row_type: &str, fec_version: &str) -> usize {
        if let Some(&id) = self.layout_of.get(row_type) {
            return id;
        }
        let names: Vec<String> = if self.other {
            vec!["row_type".to_owned()]
        } else {
            // No layout for this row type in this version: every field is
            // an extra column, so nothing is lost.
            column_names_for_field(row_type, fec_version)
                .cloned()
                .unwrap_or_default()
        };
        let mut seen = std::collections::HashSet::new();
        let mut cols = Vec::with_capacity(names.len());
        for name in names {
            // A layout that repeats a name gets `name_2`, … so each field
            // keeps its own column.
            let mut unique = name.clone();
            let mut k = 1;
            while !seen.insert(unique.clone()) {
                k += 1;
                unique = format!("{name}_{k}");
            }
            let kind = if self.other {
                ColKind::Text
            } else if DATE_COLUMNS.contains(&unique) {
                ColKind::Date
            } else if FLOAT_COLUMNS.contains(&unique) {
                ColKind::Float
            } else {
                ColKind::Text
            };
            cols.push(self.column(&unique, kind));
        }
        let id = self.layouts.len();
        self.layouts.push(cols);
        self.layout_of.insert(row_type.to_owned(), id);
        id
    }

    fn push(&mut self, row_type: &str, record: StringRecord, fec_version: &str) {
        let id = self.layout(row_type, fec_version);
        let extra = record.len().saturating_sub(self.layouts[id].len());
        while self.extra_cols.len() < extra {
            let name = format!("{}{}", self.extra_prefix, self.extra_cols.len() + 1);
            let j = self.column(&name, ColKind::Text);
            self.extra_cols.push(j);
        }
        self.rows.push((record, id));
    }

    /// Per layout, the record field of each column (`None`: the layout has
    /// no such column). Built once, after the last row.
    pub fn field_maps(&self) -> Vec<Vec<Option<usize>>> {
        self.layouts
            .iter()
            .map(|cols| {
                let mut map = vec![None; self.names.len()];
                for (i, &j) in cols.iter().enumerate() {
                    map[j] = Some(i);
                }
                for (k, &j) in self.extra_cols.iter().enumerate() {
                    // A column can't be both a layout column and an extra.
                    if map[j].is_none() {
                        map[j] = Some(cols.len() + k);
                    }
                }
                map
            })
            .collect()
    }
}

/// One output table, in output order.
pub enum Table {
    Typed(TypedTable),
    Raw(RawTable),
}

/// Collects rows into tables. Feed it rows with [`Collector::push_row`]
/// (in any batches), then call [`Collector::finish`].
pub struct Collector {
    raw_mode: bool,
    fec_version: String,
    name_delimiter: Option<String>,
    /// Families in first-seen order.
    families: Vec<&'static str>,
    typed: HashMap<&'static str, TypedTable>,
    raw: HashMap<&'static str, RawTable>,
    other: Option<RawTable>,
}

impl Collector {
    pub fn new<R: Read>(filing: &Filing<R>, raw_mode: bool) -> Self {
        Collector {
            raw_mode,
            fec_version: filing.header.fec_version.clone(),
            name_delimiter: filing.header.name_delimiter.clone(),
            families: Vec::new(),
            typed: HashMap::new(),
            raw: HashMap::new(),
            other: None,
        }
    }

    fn see(&mut self, family: &'static str) {
        if !self.families.contains(&family) {
            self.families.push(family);
        }
    }

    pub fn push_row(&mut self, row: FilingRow) {
        let Some(family) = record_family(&row.row_type) else {
            self.other.get_or_insert_with(RawTable::new_other).push(
                &row.row_type,
                row.record,
                &self.fec_version,
            );
            return;
        };
        if !self.raw_mode {
            if let Some(item) = Itemization::from_record(
                &row.record,
                &self.fec_version,
                self.name_delimiter.as_deref(),
            ) {
                let key = item.family();
                self.see(key);
                let table = self
                    .typed
                    .entry(key)
                    .or_insert_with(|| TypedTable::new(key));
                // The builders were made from `family_columns(key)`, so
                // `push_columns` can't panic on a mismatch.
                item.push_columns(&mut table.builders);
                return;
            }
        }
        self.see(family);
        self.raw
            .entry(family)
            .or_insert_with(RawTable::new_raw)
            .push(&row.row_type, row.record, &self.fec_version);
    }

    /// The tables in output order: families in first-seen order (a family
    /// with both typed and fallback rows gives `<name>` then `<name>_raw`),
    /// then `other`.
    pub fn finish(mut self) -> Vec<(String, TableKind, Table)> {
        let mut out = Vec::new();
        for family in &self.families {
            let typed = self.typed.remove(family);
            let has_typed = typed.is_some();
            if let Some(t) = typed {
                out.push((table_name(family), TableKind::Typed, Table::Typed(t)));
            }
            if let Some(t) = self.raw.remove(family) {
                let name = if has_typed {
                    raw_table_name(family)
                } else {
                    table_name(family)
                };
                out.push((name, TableKind::Raw, Table::Raw(t)));
            }
        }
        if let Some(t) = self.other {
            out.push((OTHER_TABLE.to_owned(), TableKind::Other, Table::Raw(t)));
        }
        out
    }
}

/// Pull up to `limit` itemization rows (`None`: all) into `collector`. The
/// limit is checked before `next_row()`, so a row is never pulled and
/// dropped. Returns the number of rows read.
pub fn read_rows<R: Read>(
    filing: &mut Filing<R>,
    collector: &mut Collector,
    limit: Option<usize>,
) -> Result<usize, String> {
    let mut n = 0usize;
    let mut last_line = 0u64;
    while limit.is_none_or(|m| n < m) {
        let Some(row) = filing.next_row() else { break };
        let row = row.map_err(|e| parse_error(&e, last_line))?;
        last_line = row.line;
        collector.push_row(row);
        n += 1;
    }
    Ok(n)
}

/// `parse: line N: …` (`N` from the CSV error, else after the last good row).
fn parse_error(e: &FilingRowReadError, last_line: u64) -> String {
    let line = match e {
        FilingRowReadError::CsvError(e) | FilingRowReadError::TextRecordError(e) => {
            e.position().map(|p| p.line())
        }
        FilingRowReadError::EmptyRecord(line) => Some(*line),
    }
    .filter(|&l| l > 0);
    match line {
        Some(line) => format!("parse: line {line}: {e}"),
        None if last_line > 0 => format!("parse: after line {last_line}: {e}"),
        None => format!("parse: {e}"),
    }
}

/// Open a filing: `io: …` if the file can't be opened, `header: …` if its
/// header or cover can't be read. The filing ID is the file stem (minus a
/// `FEC-` prefix), as `Filing::from_path` does.
pub fn open(path: &str) -> Result<Filing<File>, String> {
    let p = Path::new(path);
    let filing_id = p
        .file_stem()
        .map(|v| v.to_string_lossy().into_owned())
        .ok_or_else(|| format!("io: not a file path: {path}"))?;
    let file = File::open(p).map_err(|e| format!("io: {path}: {e}"))?;
    let meta = file.metadata().map_err(|e| format!("io: {path}: {e}"))?;
    if meta.is_dir() {
        return Err(format!("io: {path}: is a directory"));
    }
    Filing::from_reader(file, filing_id, meta.len() as usize).map_err(|e| {
        // A read failure while reading the header is an I/O error, not a
        // malformed header.
        let io = e.chain().any(|c| {
            c.downcast_ref::<std::io::Error>().is_some()
                || c.downcast_ref::<csv::Error>()
                    .is_some_and(|c| c.is_io_error())
        });
        format!("{}: {e:#}", if io { "io" } else { "header" })
    })
}

/// `n_max` from R (`Inf` by default) as a row limit.
pub fn row_limit(n_max: f64) -> Result<Option<usize>, String> {
    if n_max.is_nan() || n_max < 0.0 {
        return Err(format!("n_max must be a non-negative number, not {n_max}"));
    }
    Ok(if n_max >= usize::MAX as f64 {
        None
    } else {
        Some(n_max as usize)
    })
}
