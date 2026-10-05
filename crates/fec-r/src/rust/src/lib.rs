use extendr_api::prelude::*;
use fec_parser::columnar::{days_since_epoch, new_builders};
use fec_parser::Filing;

mod names;
mod reader;
mod robj;

type Result<T> = std::result::Result<T, Error>;

/// Version of the bundled fec-parser crate
///
/// Smoke test for the Rust bindings.
///
/// @return A string such as `"0.1.0"`.
/// @export
/// @examples
/// fec_version_info()
#[extendr]
fn fec_version_info() -> &'static str {
    env!("FEC_PARSER_VERSION")
}

/// Read a .fec file into a header, a cover and one table per record family
///
/// Internal: `fec_read()` (R) wraps it. Returns
/// `list(header, cover, cover_info, tables, table_kinds)`:
///
/// - `header`: named list of scalars: `filing_id`, `record_type`, `ef_type`,
///   `fec_version`, `software_name`, `software_version`, `report_id`,
///   `report_number`, `comment`, `style` (`"hdr"`, `"paper"`,
///   `"legacy_block"`), `delimiter` (`"fs"`, `"comma"`), `is_paper`
///   (logical), `name_delimiter`, `batch_number`, `received_date` (character,
///   `NA` when absent).
/// - `cover`: named list of length-1 vectors. Typed mode: the typed cover
///   flattened (`fec_parser::columnar`; `Date`, `double`, `logical`,
///   `integer`, `character`). If the form has no typed struct, and always
///   with `raw = TRUE`: the raw cover record as text (mapping names, blank →
///   `NA`, fields past the layout as `extra_1`, …). A filing always has a
///   cover (a file without one is a `header:` error).
/// - `cover_info`: the form-independent cover fields, named list:
///   `form_type`, `filer_id`, `filer_name`, `report_code` (character),
///   `coverage_from_date`, `coverage_through_date` (`Date`, `NA` when
///   absent), `cover_kind` (`"typed"` or `"raw"`: which `cover` is).
/// - `tables`: named list of tables, each a named list of equal-length
///   vectors whose first column is `filing_id`. Names: one per
///   `record_family()` (`schedule_a`, `schedule_a3l`, `schedule_c1`, `h4`,
///   `f57`, `text`, `schedule_i`, …), in first-seen order; `<name>_raw` for a
///   family's fallback rows when the same family also has typed rows; then
///   `other` (`filing_id`, `row_type`, `field_1`, …, all text) for rows no
///   family matches.
/// - `table_kinds`: named character, same names as `tables`: `"typed"`,
///   `"raw"` (raw mapping columns: every table with `raw = TRUE`, or a typed
///   mode fallback) or `"other"`.
///
/// Errors are R errors whose message starts with `io: `, `header: ` or
/// `parse: line N: ` (`parse: after line N: ` / `parse: ` when the CSV layer
/// gives no line).
///
/// @param path Path to a `.fec` file.
/// @param raw `TRUE` for raw mapping columns in every table.
/// @param n_max Stop after this many itemization rows (`Inf` = all; `0`
///   reads the header and cover only).
/// @noRd
#[extendr]
fn fec_read_impl(path: &str, raw: bool, n_max: f64) -> Result<Robj> {
    let limit = reader::row_limit(n_max).map_err(Error::Other)?;
    let mut filing = reader::open(path).map_err(Error::Other)?;
    let mut collector = reader::Collector::new(&filing, raw);
    reader::read_rows(&mut filing, &mut collector, limit).map_err(Error::Other)?;
    let tables = collector.finish();

    let header = header_list(&filing);
    let (cover, cover_kind) = cover_list(&filing, raw)?;
    let cover_info = cover_info(&filing, cover_kind);
    let (tables, table_kinds) = robj::tables(&tables, &filing.filing_id)?;
    Ok(list!(
        header = header,
        cover = cover,
        cover_info = cover_info,
        tables = tables,
        table_kinds = table_kinds
    )
    .into())
}

fn header_list<R: std::io::Read>(f: &Filing<R>) -> Robj {
    let h = &f.header;
    let t = |s: &str| robj::scalar_text(Some(s));
    let o = |s: &Option<String>| robj::scalar_text(s.as_deref());
    list!(
        filing_id = t(&f.filing_id),
        record_type = t(&h.record_type),
        ef_type = t(&h.ef_type),
        fec_version = t(&h.fec_version),
        software_name = t(&h.software_name),
        software_version = t(&h.software_version),
        report_id = o(&h.report_id),
        report_number = o(&h.report_number),
        comment = o(&h.comment),
        style = t(h.style.as_str()),
        delimiter = t(h.delimiter.as_str()),
        is_paper = h.is_paper(),
        name_delimiter = o(&h.name_delimiter),
        batch_number = o(&h.batch_number),
        received_date = o(&h.received_date)
    )
    .into()
}

/// The cover and whether it is `"typed"` or `"raw"`.
fn cover_list<R: std::io::Read>(f: &Filing<R>, raw: bool) -> Result<(Robj, &'static str)> {
    if !raw {
        if let Some(cover) = &f.cover.cover_data {
            let defs = cover.columns();
            let mut builders = new_builders(&defs, 1);
            // The builders were made from `cover.columns()`: no mismatch panic.
            cover.push_columns(&mut builders);
            let names: Vec<String> = defs.into_iter().map(|d| d.name).collect();
            return Ok((robj::one_row(&names, &builders)?, "typed"));
        }
    }
    let c = &f.cover;
    let kv = c
        .cover_record_kv
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str()));
    let extras = c
        .record
        .iter()
        .skip(c.record_column_names.len())
        .enumerate()
        .map(|(k, v)| (format!("extra_{}", k + 1), v));
    Ok((robj::text_kv(kv.chain(extras))?, "raw"))
}

fn cover_info<R: std::io::Read>(f: &Filing<R>, cover_kind: &str) -> Robj {
    let c = &f.cover;
    let t = |s: &str| robj::scalar_text(Some(s));
    let d = |d: Option<_>| robj::date_vec(std::iter::once(d.map(days_since_epoch)));
    list!(
        form_type = t(&c.form_type),
        filer_id = t(&c.filer_id),
        filer_name = t(&c.filer_name),
        report_code = robj::scalar_text(c.report_code.as_deref()),
        coverage_from_date = d(c.coverage_from_date),
        coverage_through_date = d(c.coverage_through_date),
        cover_kind = t(cover_kind)
    )
    .into()
}

// Macro to generate exports.
// This ensures exported functions are registered with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod libfec;
    fn fec_version_info;
    fn fec_read_impl;
}
