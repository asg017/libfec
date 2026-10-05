//// A column value of a `Row`, typed by the column's kind.

import gleam/float
import gleam/int
import gleam/string
import gleam/time/calendar

/// One field of a row, by column.
///
/// Date columns hold `Day` when the field is a `YYYYMMDD` date, amount
/// columns `Number` when it is a (finite) number. Anything that doesn't
/// parse stays `Text`, exactly as filed, so garbage can't be mistaken for a
/// valid value. `nan` and `inf` in an amount column are garbage too.
pub type Value {
  /// A text column, or garbage in a date or amount column (raw, as filed).
  Text(String)
  /// An amount column that parses.
  Number(Float)
  /// A date column that parses.
  Day(calendar.Date)
  /// An empty field (blank, for a date or amount column), or a column past
  /// the end of a short row.
  Empty
}

/// The value as a string: the text, the number, the date as `YYYY-MM-DD`,
/// or `""` for `Empty`.
pub fn to_string(value: Value) -> String {
  case value {
    Text(s) -> s
    Number(x) -> float.to_string(x)
    Day(d) ->
      year(d.year)
      <> "-"
      <> pad(calendar.month_to_int(d.month), 2)
      <> "-"
      <> pad(d.day, 2)
    Empty -> ""
  }
}

/// The number of a `Number`; `Error(Nil)` for anything else.
pub fn to_float(value: Value) -> Result(Float, Nil) {
  case value {
    Number(x) -> Ok(x)
    _ -> Error(Nil)
  }
}

/// ISO 8601 style: `0042`, `-0001`. Padding the signed string would give `00-1`.
fn year(y: Int) -> String {
  case y < 0 {
    True -> "-" <> pad(int.absolute_value(y), 4)
    False -> pad(y, 4)
  }
}

fn pad(n: Int, width: Int) -> String {
  string.pad_start(int.to_string(n), to: width, with: "0")
}
