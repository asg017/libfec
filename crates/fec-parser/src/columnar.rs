//! Flatten the typed covers ([`crate::covers::Cover`]) and itemizations
//! ([`crate::itemizations::Itemization`]) into typed columns (feature
//! `columnar`).
//!
//! Binding-agnostic: no R, Arrow or other runtime dependency. A binding asks
//! for a table's [`ColumnDef`]s once, allocates one [`ColumnBuilder`] per
//! column, pushes every typed row, then converts the builders into its own
//! vectors (R vectors in `crates/fec-r`; Arrow arrays later).
//!
//! # Column names
//!
//! A table's columns are its struct's fields in declaration order. A nested
//! struct is flattened in place, its field names joined to the parent's with
//! `_`: `contributor` → `name` → `last_name` is `contributor_name_last_name`.
//! No shortening and no renames, so a column name is always the path to the
//! Rust field. The names don't depend on the filing version (one struct reads
//! every layout).
//!
//! # Column types ([`ColKind`])
//!
//! | Rust field | Kind | Value pushed |
//! |---|---|---|
//! | `String`, `&'static str` | [`ColKind::Text`] | the string, `""` kept as `""` |
//! | `Option<String>` | [`ColKind::Text`] | `None` for a blank field |
//! | `f64` | [`ColKind::Float`] | always a value (a blank or garbage main amount is `0.0`) |
//! | `Option<f64>` | [`ColKind::Float`] | `None` for blank or garbage |
//! | `Date`, `Option<Date>` | [`ColKind::Date`] | days since 1970-01-01 |
//! | `bool`, `Option<bool>` | [`ColKind::Bool`] | |
//! | `i16`, `u16` (and `Option`) | [`ColKind::Int`] | widened to `i32` |
//! | a struct deriving `Columnar` | its columns, prefixed | |
//! | `Option<Struct>` | the struct's columns | all `None` when absent |
//! | `Box<T>` | as `T` | |
//! | `Vec<Struct>` | **skipped**: no column | see [`Columnar::skipped`] |
//!
//! `Vec<Struct>` fields are skipped in Phase 0 (R bindings, `todos/r` 02):
//! `Form1::banks`, `Form1M::qualification_candidates`
//! (`Form1MQualification::candidates`),
//! `Form3P::state_allocations_states` (`Form3PStateAllocations::states`) and
//! `ScheduleC::guarantors`. [`Columnar::skipped`] and
//! [`ColumnarEnum::skipped_for`] list them per table.
//!
//! # Driving it from a row loop
//!
//! ```
//! use fec_parser::columnar::{new_builders, ColumnBuilder};
//! use fec_parser::itemizations::Itemization;
//! use std::collections::HashMap;
//!
//! # fn rows() -> Vec<Itemization> { vec![] }
//! // family → (columns, builders)
//! let mut tables: HashMap<&'static str, Vec<ColumnBuilder>> = HashMap::new();
//! for item in rows() {
//!     let builders = tables.entry(item.family()).or_insert_with(|| {
//!         let defs = Itemization::family_columns(item.family()).unwrap();
//!         new_builders(&defs, 0)
//!     });
//!     item.push_columns(builders);
//! }
//! ```

use crate::covers::Cover;
use crate::itemizations::Itemization;
use jiff::civil::Date;

/// The type of a column. Bindings map it to their own vector type (R:
/// `character`, `double`, `Date`, `logical`, `integer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum ColKind {
    /// UTF-8 text.
    Text,
    /// A 64-bit float (amounts, percentages).
    Float,
    /// A calendar date, as days since 1970-01-01 (R's `Date`, Arrow `date32`).
    Date,
    /// A checkbox.
    Bool,
    /// A small integer (a year), widened to `i32`.
    Int,
}

/// One flattened column.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ColumnDef {
    /// The column name: nested field names joined with `_`
    /// (`contributor_name_last_name`).
    pub name: String,
    /// The column's type.
    pub kind: ColKind,
    /// The Rust field's doc comment (the innermost field for a nested column).
    pub doc: &'static str,
}

/// The values of one column, built row by row. One variant per [`ColKind`];
/// `None` is a missing value (R's `NA`).
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnBuilder {
    Text(Vec<Option<String>>),
    Float(Vec<Option<f64>>),
    /// Days since 1970-01-01.
    Date(Vec<Option<i32>>),
    Bool(Vec<Option<bool>>),
    Int(Vec<Option<i32>>),
}

impl ColumnBuilder {
    /// An empty builder for a column of `kind`, with room for `capacity` rows.
    pub fn new(kind: ColKind, capacity: usize) -> Self {
        match kind {
            ColKind::Text => ColumnBuilder::Text(Vec::with_capacity(capacity)),
            ColKind::Float => ColumnBuilder::Float(Vec::with_capacity(capacity)),
            ColKind::Date => ColumnBuilder::Date(Vec::with_capacity(capacity)),
            ColKind::Bool => ColumnBuilder::Bool(Vec::with_capacity(capacity)),
            ColKind::Int => ColumnBuilder::Int(Vec::with_capacity(capacity)),
        }
    }

    /// The column's kind.
    pub fn kind(&self) -> ColKind {
        match self {
            ColumnBuilder::Text(_) => ColKind::Text,
            ColumnBuilder::Float(_) => ColKind::Float,
            ColumnBuilder::Date(_) => ColKind::Date,
            ColumnBuilder::Bool(_) => ColKind::Bool,
            ColumnBuilder::Int(_) => ColKind::Int,
        }
    }

    /// The number of values pushed so far.
    pub fn len(&self) -> usize {
        match self {
            ColumnBuilder::Text(v) => v.len(),
            ColumnBuilder::Float(v) => v.len(),
            ColumnBuilder::Date(v) => v.len(),
            ColumnBuilder::Bool(v) => v.len(),
            ColumnBuilder::Int(v) => v.len(),
        }
    }

    /// Whether no value has been pushed.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// One empty builder per column of `defs`, in order, each with room for
/// `capacity` rows.
pub fn new_builders(defs: &[ColumnDef], capacity: usize) -> Vec<ColumnBuilder> {
    defs.iter()
        .map(|d| ColumnBuilder::new(d.kind, capacity))
        .collect()
}

/// A type that flattens into columns. Derived (`#[derive(Columnar)]` from
/// fec-parser-macros) for every struct reachable from [`Cover`] and
/// [`Itemization`]; implemented here for the leaf types.
///
/// Builders passed to [`Columnar::push`] must be the ones made from this
/// type's [`Columnar::columns`] (e.g. with [`new_builders`]): a builder of
/// the wrong kind panics.
pub trait Columnar {
    /// Append this type's columns to `out`, named under `prefix` (`""` at the
    /// top). `doc` is the doc comment of the field holding the value.
    fn append_columns(prefix: &str, doc: &'static str, out: &mut Vec<ColumnDef>);

    /// Push one value per column, in [`Columnar::columns`] order, into
    /// `builders[*at..]`, advancing `*at` past them.
    fn push(&self, builders: &mut [ColumnBuilder], at: &mut usize);

    /// Push a missing value into every column (an absent `Option<Struct>`).
    fn push_absent(builders: &mut [ColumnBuilder], at: &mut usize);

    /// Append the names of the fields that get no column (`Vec<Struct>` in
    /// Phase 0), named as their column would be.
    fn append_skipped(prefix: &str, out: &mut Vec<String>) {
        let _ = (prefix, out);
    }

    /// The flattened columns, named under `prefix` (`""` for a table).
    fn columns(prefix: &str) -> Vec<ColumnDef> {
        let mut out = Vec::new();
        Self::append_columns(prefix, "", &mut out);
        out
    }

    /// The fields that get no column, named under `prefix`.
    fn skipped(prefix: &str) -> Vec<String> {
        let mut out = Vec::new();
        Self::append_skipped(prefix, &mut out);
        out
    }
}

/// A per-variant dispatch for an enum whose variants each hold one
/// [`Columnar`] payload; derived for [`Itemization`] and [`Cover`]. Each
/// variant is its own table, named by its key: the `#[serde(rename)]` tag
/// where there is one (`Itemization`: `"SA"`, the [`record_family`]), else
/// the variant name (`Cover`: `"Form3X"`).
///
/// [`record_family`]: crate::itemizations::record_family
pub trait ColumnarEnum {
    /// Every variant's key, in declaration order.
    const KEYS: &'static [&'static str];
    /// This value's variant key.
    fn key(&self) -> &'static str;
    /// The columns of the variant keyed `key`; `None` for an unknown key.
    fn columns_for(key: &str) -> Option<Vec<ColumnDef>>;
    /// The skipped fields of the variant keyed `key`; `None` for an unknown key.
    fn skipped_for(key: &str) -> Option<Vec<String>>;
    /// Push this value's payload as one row into `builders`, which must be
    /// made from `Self::columns_for(self.key())`.
    ///
    /// # Panics
    ///
    /// If `builders` don't match the variant's columns (count or kinds).
    fn push_payload(&self, builders: &mut [ColumnBuilder]);
}

impl Itemization {
    /// Every family with a typed struct, in declaration order (`"SA"`,
    /// `"SB"`, …, `"SC2"`): the values [`crate::itemizations::record_family`]
    /// returns that type.
    pub fn families() -> &'static [&'static str] {
        <Self as ColumnarEnum>::KEYS
    }

    /// This row's family (`"SA"` for a [`Itemization::ScheduleA`]).
    pub fn family(&self) -> &'static str {
        ColumnarEnum::key(self)
    }

    /// The columns of a family's table (`"SA"`, `"SC1"`, …; see
    /// [`Itemization::families`]). `None` for a family with no struct (`SI`)
    /// or an unknown one.
    pub fn family_columns(family: &str) -> Option<Vec<ColumnDef>> {
        <Self as ColumnarEnum>::columns_for(family)
    }

    /// Push this row into `builders`, made from
    /// `Itemization::family_columns(self.family())`.
    ///
    /// # Panics
    ///
    /// If `builders` were made for another family.
    pub fn push_columns(&self, builders: &mut [ColumnBuilder]) {
        ColumnarEnum::push_payload(self, builders)
    }
}

impl Cover {
    /// Every cover form with a typed struct (`"Form1"`, `"Form3"`, …): the
    /// variant names, which are also the serialized `form` tag.
    pub fn forms() -> &'static [&'static str] {
        <Self as ColumnarEnum>::KEYS
    }

    /// This cover's form (`"Form3X"` for a [`Cover::Form3X`]).
    pub fn form(&self) -> &'static str {
        ColumnarEnum::key(self)
    }

    /// The columns of a cover form (see [`Cover::forms`]); `None` for an
    /// unknown one.
    pub fn form_columns(form: &str) -> Option<Vec<ColumnDef>> {
        <Self as ColumnarEnum>::columns_for(form)
    }

    /// The columns of this cover's form: `Cover::form_columns(self.form())`.
    pub fn columns(&self) -> Vec<ColumnDef> {
        <Self as ColumnarEnum>::columns_for(self.form()).unwrap_or_default()
    }

    /// Push this cover as one row into `builders`, made from
    /// [`Cover::columns`].
    ///
    /// # Panics
    ///
    /// If `builders` were made for another form.
    pub fn push_columns(&self, builders: &mut [ColumnBuilder]) {
        ColumnarEnum::push_payload(self, builders)
    }
}

/// `prefix_name`, or `name` when `prefix` is empty.
#[doc(hidden)]
pub fn join_name(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}_{name}")
    }
}

/// Called by the derived `push_payload`: every builder got exactly one value.
#[doc(hidden)]
pub fn check_pushed(pushed: usize, builders: usize, key: &str) {
    assert_eq!(
        pushed, builders,
        "columnar: `{key}` pushed {pushed} columns into {builders} builders"
    );
}

/// Days from 1970-01-01 to `date` (negative before).
pub fn days_since_epoch(date: Date) -> i32 {
    // Howard Hinnant's `days_from_civil`, exact over jiff's range.
    let y = i32::from(date.year()) - i32::from(date.month() <= 2);
    let m = i32::from(date.month());
    let d = i32::from(date.day());
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn kind_mismatch(builder: &ColumnBuilder, want: ColKind, at: usize) -> ! {
    panic!(
        "columnar: column {at} is {:?}, pushed a {want:?} value",
        builder.kind()
    )
}

macro_rules! leaf {
    ($ty:ty, $kind:ident, |$v:ident| $conv:expr) => {
        impl Columnar for $ty {
            fn append_columns(prefix: &str, doc: &'static str, out: &mut Vec<ColumnDef>) {
                out.push(ColumnDef {
                    name: prefix.to_owned(),
                    kind: ColKind::$kind,
                    doc,
                });
            }
            fn push(&self, builders: &mut [ColumnBuilder], at: &mut usize) {
                let $v = self;
                match &mut builders[*at] {
                    ColumnBuilder::$kind(col) => col.push(Some($conv)),
                    other => kind_mismatch(other, ColKind::$kind, *at),
                }
                *at += 1;
            }
            fn push_absent(builders: &mut [ColumnBuilder], at: &mut usize) {
                match &mut builders[*at] {
                    ColumnBuilder::$kind(col) => col.push(None),
                    other => kind_mismatch(other, ColKind::$kind, *at),
                }
                *at += 1;
            }
        }
    };
}

leaf!(String, Text, |v| v.clone());
leaf!(&'static str, Text, |v| (*v).to_owned());
leaf!(f64, Float, |v| *v);
leaf!(bool, Bool, |v| *v);
leaf!(i16, Int, |v| i32::from(*v));
leaf!(u16, Int, |v| i32::from(*v));
leaf!(Date, Date, |v| days_since_epoch(*v));

impl<T: Columnar> Columnar for Option<T> {
    fn append_columns(prefix: &str, doc: &'static str, out: &mut Vec<ColumnDef>) {
        T::append_columns(prefix, doc, out)
    }
    fn push(&self, builders: &mut [ColumnBuilder], at: &mut usize) {
        match self {
            Some(v) => v.push(builders, at),
            None => T::push_absent(builders, at),
        }
    }
    fn push_absent(builders: &mut [ColumnBuilder], at: &mut usize) {
        T::push_absent(builders, at)
    }
    fn append_skipped(prefix: &str, out: &mut Vec<String>) {
        T::append_skipped(prefix, out)
    }
}

impl<T: Columnar> Columnar for Box<T> {
    fn append_columns(prefix: &str, doc: &'static str, out: &mut Vec<ColumnDef>) {
        T::append_columns(prefix, doc, out)
    }
    fn push(&self, builders: &mut [ColumnBuilder], at: &mut usize) {
        (**self).push(builders, at)
    }
    fn push_absent(builders: &mut [ColumnBuilder], at: &mut usize) {
        T::push_absent(builders, at)
    }
    fn append_skipped(prefix: &str, out: &mut Vec<String>) {
        T::append_skipped(prefix, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::itemizations::ScheduleA;
    use std::collections::HashSet;

    #[test]
    fn days_since_epoch_matches_jiff() {
        let epoch = Date::constant(1970, 1, 1);
        for date in [
            Date::constant(1970, 1, 1),
            Date::constant(1969, 12, 31),
            Date::constant(2000, 2, 29),
            Date::constant(2024, 3, 1),
            Date::constant(1900, 3, 1),
            Date::constant(1, 1, 1),
            Date::constant(9999, 12, 31),
        ] {
            let want = (date - epoch).get_days();
            assert_eq!(days_since_epoch(date), want, "{date}");
        }
    }

    #[test]
    fn schedule_a_columns_start_in_declaration_order() {
        let cols = ScheduleA::columns("");
        let names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names[..3],
            ["form_type", "filer_committee_id", "transaction_id"]
        );
        let kind = |n: &str| cols.iter().find(|c| c.name == n).map(|c| c.kind);
        assert_eq!(kind("contribution_date"), Some(ColKind::Date));
        assert_eq!(kind("contribution_amount"), Some(ColKind::Float));
        assert_eq!(kind("contributor_name_last_name"), Some(ColKind::Text));
        assert_eq!(kind("memo"), Some(ColKind::Bool));
        assert!(ScheduleA::skipped("").is_empty());
    }

    fn assert_unique(what: &str, cols: &[ColumnDef]) {
        let mut seen = HashSet::new();
        for c in cols {
            assert!(
                seen.insert(&c.name),
                "{what}: duplicate column `{}`",
                c.name
            );
        }
    }

    #[test]
    fn no_duplicate_names_in_any_family_or_form() {
        for family in Itemization::families() {
            let cols = Itemization::family_columns(family).unwrap_or_default();
            assert!(!cols.is_empty(), "{family}: no columns");
            assert_unique(family, &cols);
        }
        for form in Cover::forms() {
            let cols = Cover::form_columns(form).unwrap_or_default();
            assert!(!cols.is_empty(), "{form}: no columns");
            assert_unique(form, &cols);
        }
    }

    #[test]
    fn unknown_keys() {
        assert!(Itemization::family_columns("SI").is_none());
        assert!(Cover::form_columns("SA").is_none());
    }

    #[test]
    fn skipped_vec_fields() {
        let skipped: Vec<(&str, Vec<String>)> = Itemization::families()
            .iter()
            .chain(Cover::forms())
            .filter_map(|k| {
                let s = <Itemization as ColumnarEnum>::skipped_for(k)
                    .or_else(|| <Cover as ColumnarEnum>::skipped_for(k))
                    .unwrap_or_default();
                (!s.is_empty()).then_some((*k, s))
            })
            .collect();
        assert_eq!(
            skipped,
            [
                ("SC", vec!["guarantors".to_owned()]),
                ("Form1", vec!["banks".to_owned()]),
                ("Form3P", vec!["state_allocations_states".to_owned()]),
                ("Form1M", vec!["qualification_candidates".to_owned()]),
            ]
        );
    }

    #[test]
    #[should_panic(expected = "columnar")]
    fn wrong_builder_kind_panics() {
        let mut b = vec![ColumnBuilder::new(ColKind::Float, 1)];
        "x".to_owned().push(&mut b, &mut 0);
    }

    #[test]
    fn option_struct_pushes_absent() {
        use crate::covers::PersonName;
        let defs = <Option<PersonName>>::columns("vp");
        assert!(defs.iter().all(|d| d.name.starts_with("vp_")));
        let mut b = new_builders(&defs, 2);
        let mut at = 0;
        None::<PersonName>.push(&mut b, &mut at);
        assert_eq!(at, defs.len());
        assert!(b.iter().all(|c| c.len() == 1));
        assert!(matches!(&b[0], ColumnBuilder::Text(v) if v[0].is_none()));
    }
}
