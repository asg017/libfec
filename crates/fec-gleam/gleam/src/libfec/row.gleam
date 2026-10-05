//// One row (line) of a filing after the cover.

import gleam/option.{type Option}
import libfec/itemization.{type Itemization}
import libfec/value.{type Value}

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
