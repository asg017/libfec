//// One row (line) of a filing after the cover.

import gleam/list
import gleam/option.{type Option}
import libfec/itemization.{type Itemization}
import libfec/value.{type Value}

/// One row of a filing, as `libfec.fold` delivers it.
///
/// The raw fields as filed are not carried (only `extra_fields`, those past
/// the mapped columns): read a column's value with `get`, or the typed
/// record from `itemization`.
pub type Row {
  Row(
    /// The row type as filed, trimmed: `"SA11AI"`, `"SB23"`, `"TEXT"`.
    row_type: String,
    /// The 1-based line of the row in the file (0 if unknown).
    line: Int,
    /// The mapped columns, in column order, with typed values.
    values: List(#(String, Value)),
    /// The typed record; `None` for a row type with no typed family.
    itemization: Option(Itemization),
    /// Raw fields past the mapped columns, as filed.
    extra_fields: List(String),
  )
}

/// The value of a mapped column, by its name in the row type's mapping
/// (`"contribution_amount"`, `"contributor_state"`); `Error(Nil)` if the
/// row type has no such column. A column past the end of a short row is
/// `Ok(Empty)`.
///
/// ```gleam
/// case row.get(row, "contribution_amount") {
///   Ok(value.Number(x)) -> x
///   _ -> 0.0
/// }
/// ```
pub fn get(row: Row, column: String) -> Result(Value, Nil) {
  list.key_find(row.values, column)
}
