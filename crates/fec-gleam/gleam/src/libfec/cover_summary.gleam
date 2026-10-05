//// The fields every filing's cover record has, whatever its form type.
////
//// The typed cover (`libfec/cover.Cover`, generated) has everything else;
//// this summary also exists for form types with no typed cover.

import gleam/option.{type Option}
import gleam/time/calendar

pub type CoverSummary {
  CoverSummary(
    /// The cover's form type as filed, trimmed: `"F3XN"`, `"F99"`.
    form_type: String,
    /// The filer's FEC ID (committee or candidate).
    filer_id: String,
    /// The committee or organization name; for an individual filing Form 5
    /// or 9, the filer's name. Empty if the layout has none.
    filer_name: String,
    /// The report code (`"Q1"`, `"M8"`, …); `None` if absent or blank.
    report_code: Option(String),
    /// Start of the coverage period, if the form has one and it parses.
    coverage_from_date: Option(calendar.Date),
    /// End of the coverage period, if the form has one and it parses.
    coverage_through_date: Option(calendar.Date),
  )
}
