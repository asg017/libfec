//! Rust tables → R vectors.
//!
//! Text goes through the raw FFI path: `Strings::new(n)` (protected by
//! extendr), then `Rf_mkCharLenCE(…, CE_UTF8)` + `SET_STRING_ELT` per cell,
//! with `R_NaString` for a missing value. extendr's `Rstr::from` protects
//! every string through a global hash table and was 2.3× slower in the probe.
//!
//! GC safety: every vector we allocate is an extendr `Robj` (on extendr's
//! precious list) until it is stored in the parent list, and each new
//! `CHARSXP` is stored with `SET_STRING_ELT` before the next allocation.
//! `Rf_mkCharLenCE` raises an R error (a longjmp over Rust frames) on an
//! embedded NUL, so NULs are stripped first; fec-parser hands over valid
//! UTF-8, so `CE_UTF8` is right.

use extendr_api::prelude::*;
use extendr_ffi::{cetype_t, R_NaString, Rf_mkCharLenCE, SET_STRING_ELT, SEXP};
use fec_parser::columnar::{ColKind, ColumnBuilder};

use crate::reader::{RawTable, Table, TableKind, TypedTable};

type Result<T> = std::result::Result<T, Error>;

/// A `CHARSXP` for `s`, NULs stripped; `R_NaString` for `None`.
///
/// # Safety
///
/// Allocates: the caller must store the result in a protected vector before
/// allocating again.
unsafe fn charsxp(s: Option<&str>) -> SEXP {
    let Some(s) = s else { return R_NaString };
    let owned;
    let s = if s.as_bytes().contains(&0) {
        owned = s.replace('\0', "");
        owned.as_str()
    } else {
        s
    };
    // A field can't approach 2 GB; R's CHARSXP limit is i32::MAX bytes.
    let len = i32::try_from(s.len()).unwrap_or(i32::MAX);
    Rf_mkCharLenCE(s.as_ptr().cast(), len, cetype_t::CE_UTF8)
}

/// A character vector of `n` values.
pub fn text_vec<'a>(n: usize, values: impl Iterator<Item = Option<&'a str>>) -> Robj {
    let v = Strings::new(n);
    // Safety: `v` is protected by extendr; each CHARSXP is stored right away.
    unsafe {
        let sexp = v.get();
        for (i, s) in values.take(n).enumerate() {
            SET_STRING_ELT(sexp, i as isize, charsxp(s));
        }
    }
    v.into()
}

/// A character vector repeating `s` `n` times (one CHARSXP).
fn rep_text(s: &str, n: usize) -> Robj {
    let v = Strings::new(n);
    // Safety: `v` is protected; `c` is made after it and stored before any
    // other allocation (when n == 0 it is unreferenced garbage, harmless).
    unsafe {
        let sexp = v.get();
        let c = charsxp(Some(s));
        for i in 0..n {
            SET_STRING_ELT(sexp, i as isize, c);
        }
    }
    v.into()
}

/// One scalar string (`NA` for `None`).
pub fn scalar_text(s: Option<&str>) -> Robj {
    text_vec(1, std::iter::once(s))
}

/// A `Date` vector from days since 1970-01-01.
pub fn date_vec(values: impl ExactSizeIterator<Item = Option<i32>>) -> Robj {
    let v =
        Doubles::from_values(values.map(|d| d.map_or(Rfloat::na(), |d| Rfloat::from(d as f64))));
    let mut r: Robj = v.into();
    r.set_class(["Date"]).expect("class on a fresh vector");
    r
}

fn builder_to_robj(b: &ColumnBuilder) -> Robj {
    match b {
        ColumnBuilder::Text(v) => text_vec(v.len(), v.iter().map(|s| s.as_deref())),
        ColumnBuilder::Float(v) => {
            Doubles::from_values(v.iter().map(|x| x.map_or(Rfloat::na(), Rfloat::from))).into()
        }
        ColumnBuilder::Date(v) => date_vec(v.iter().copied()),
        ColumnBuilder::Bool(v) => {
            Logicals::from_values(v.iter().map(|x| x.map_or(Rbool::na(), Rbool::from))).into()
        }
        ColumnBuilder::Int(v) => {
            Integers::from_values(v.iter().map(|x| x.map_or(Rint::na(), Rint::from))).into()
        }
    }
}

fn named_list(names: Vec<String>, values: Vec<Robj>) -> Result<Robj> {
    Ok(List::from_names_and_values(names, values)?.into())
}

fn typed_table(t: &TypedTable, filing_id: &str) -> Result<Robj> {
    let mut names = Vec::with_capacity(t.names.len() + 1);
    let mut cols = Vec::with_capacity(t.names.len() + 1);
    names.push("filing_id".to_owned());
    cols.push(rep_text(filing_id, t.nrow()));
    for (name, b) in t.names.iter().zip(&t.builders) {
        names.push(name.clone());
        cols.push(builder_to_robj(b));
    }
    named_list(names, cols)
}

/// A raw cell: trimmed, blank → `None`.
fn cell(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|s| !s.is_empty())
}

/// `YYYYMMDD` or `M/D/YYYY` (month and day of 1 or 2 digits, as fec-parser's
/// `%m/%d/%Y` accepts) → days since 1970-01-01; `None` if blank or not a
/// real date (`20230231`).
///
/// Unlike fec-parser, a slash date needs a 4-digit year: the typed reader
/// takes `07/14/23` as the year 23, a leniency not copied here.
fn raw_date(s: &str) -> Option<i32> {
    let b = s.as_bytes();
    let num = |r: &[u8]| -> Option<i32> {
        if r.is_empty() || !r.iter().all(u8::is_ascii_digit) {
            return None;
        }
        Some(r.iter().fold(0, |acc, d| acc * 10 + i32::from(d - b'0')))
    };
    let (y, m, d) = if !b.contains(&b'/') {
        if b.len() != 8 {
            return None;
        }
        (num(&b[..4])?, num(&b[4..6])?, num(&b[6..])?)
    } else {
        let mut parts = b.split(|&c| c == b'/');
        let (m, d, y) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() || m.len() > 2 || d.len() > 2 || y.len() != 4 {
            return None;
        }
        (num(y)?, num(m)?, num(d)?)
    };
    if !(1..=12).contains(&m) {
        return None;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days_in_month = match m {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if d < 1 || d > days_in_month {
        return None;
    }
    // Howard Hinnant's days_from_civil.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

fn raw_table(t: &RawTable, filing_id: &str) -> Result<Robj> {
    let n = t.nrow();
    let maps = t.field_maps();
    let mut names = Vec::with_capacity(t.names.len() + 1);
    let mut cols = Vec::with_capacity(t.names.len() + 1);
    names.push("filing_id".to_owned());
    cols.push(rep_text(filing_id, n));
    for (j, (name, kind)) in t.names.iter().zip(&t.kinds).enumerate() {
        let values = t
            .rows
            .iter()
            .map(|(rec, id)| cell(maps[*id][j].and_then(|i| rec.get(i))));
        let col = match kind {
            ColKind::Date => date_vec(values.map(|s| s.and_then(raw_date))),
            ColKind::Float => Doubles::from_values(values.map(|s| {
                s.and_then(|s| s.parse::<f64>().ok())
                    .map_or(Rfloat::na(), Rfloat::from)
            }))
            .into(),
            _ => text_vec(n, values),
        };
        names.push(name.clone());
        cols.push(col);
    }
    named_list(names, cols)
}

/// `(tables, table_kinds)`: a named list of named column lists, and a named
/// character vector of `"typed"` / `"raw"` / `"other"`.
pub fn tables(tables: &[(String, TableKind, Table)], filing_id: &str) -> Result<(Robj, Robj)> {
    let mut names = Vec::with_capacity(tables.len());
    let mut values = Vec::with_capacity(tables.len());
    for (name, _, table) in tables {
        names.push(name.clone());
        values.push(match table {
            Table::Typed(t) => typed_table(t, filing_id)?,
            Table::Raw(t) => raw_table(t, filing_id)?,
        });
    }
    let list = named_list(names.clone(), values)?;
    let mut kinds = text_vec(
        tables.len(),
        tables.iter().map(|(_, k, _)| Some(k.as_str())),
    );
    kinds.set_names(names)?;
    Ok((list, kinds))
}

/// A one-row set of builders as a named list of length-1 vectors.
pub fn one_row(names: &[String], builders: &[ColumnBuilder]) -> Result<Robj> {
    named_list(
        names.to_vec(),
        builders.iter().map(builder_to_robj).collect(),
    )
}

/// Text key/value pairs as a named list of scalar strings (blank → `NA`).
pub fn text_kv<'a>(pairs: impl Iterator<Item = (String, &'a str)>) -> Result<Robj> {
    let (names, values): (Vec<String>, Vec<Robj>) =
        pairs.map(|(k, v)| (k, scalar_text(cell(Some(v))))).unzip();
    named_list(names, values)
}

#[cfg(test)]
mod tests {
    use super::raw_date;

    #[test]
    fn raw_dates() {
        assert_eq!(raw_date("19700101"), Some(0));
        assert_eq!(raw_date("01/02/1970"), Some(1));
        assert_eq!(raw_date("20240229"), Some(19782));
        assert_eq!(raw_date("20230229"), None);
        assert_eq!(raw_date("20230231"), None);
        assert_eq!(raw_date("20231301"), None);
        assert_eq!(raw_date("00000000"), None);
        assert_eq!(raw_date("ABCDEFGH"), None);
        assert_eq!(raw_date("2023071"), None);
        assert_eq!(raw_date("19691231"), Some(-1));
        assert_eq!(raw_date("12/31/1969"), Some(-1));
    }

    #[test]
    fn raw_slash_dates_take_one_or_two_digit_month_and_day() {
        let jan_2_2003 = raw_date("01/02/2003");
        assert_eq!(jan_2_2003, Some(12_054));
        assert_eq!(raw_date("1/2/2003"), jan_2_2003);
        assert_eq!(raw_date("01/2/2003"), jan_2_2003);
        assert_eq!(raw_date("1/02/2003"), jan_2_2003);
        assert_eq!(raw_date("2/22/2009"), raw_date("20090222"));
        assert_eq!(raw_date("7/14/2023"), raw_date("20230714"));
        assert_eq!(raw_date("2/29/2024"), raw_date("20240229"));
        assert_eq!(raw_date("2/29/2023"), None);
        assert_eq!(raw_date("13/1/2023"), None);
        assert_eq!(raw_date("0/1/2023"), None);
        // The year needs 4 digits; month and day at most 2.
        assert_eq!(raw_date("07/14/23"), None);
        assert_eq!(raw_date("7/4/02023"), None);
        assert_eq!(raw_date("007/4/2023"), None);
        assert_eq!(raw_date("7//2023"), None);
        assert_eq!(raw_date("7/4/2023/1"), None);
        assert_eq!(raw_date("+7/4/2023"), None);
        assert_eq!(raw_date("2023-07-14"), None);
    }
}
