//// Summarise an FEC filing with the `libfec` package: print its typed cover,
//// then fold once over the rows, matching on each row's typed itemization.
////
////     gleam run -- path/to/filing.fec
////
//// (`make example FEC=path/to/filing.fec` in `crates/fec-gleam` builds the
//// NIF first.)

import argv
import gleam/dict.{type Dict}
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/order
import gleam/string
import gleam/time/calendar.{type Date}
import libfec
import libfec/cover
import libfec/cover/common.{type PersonName} as _
import libfec/error.{type FecError}
import libfec/itemization
import libfec/itemization/common.{type CandidateRef, type Entity} as _
import libfec/itemization/schedule_a.{type ScheduleA}
import libfec/row.{type Row}

pub fn main() -> Nil {
  case argv.load().arguments {
    [path] -> summarise(path)
    _ -> exit_with("usage: gleam run -- <filing.fec>")
  }
}

fn summarise(path: String) -> Nil {
  case libfec.open_with(path, libfec.Options(unknown_rows: libfec.Skip)) {
    Error(e) -> exit_with(path <> ": " <> describe(e))
    Ok(reader) -> {
      print_cover(reader)
      let result = libfec.fold(over: reader, from: new_summary(), with: tally)
      let skipped = libfec.skipped_rows(reader)
      libfec.close(reader)
      case result {
        Ok(summary) -> print_summary(summary, skipped)
        Error(e) -> exit_with(path <> ": " <> describe(e))
      }
    }
  }
}

/// A readable message for each way opening or reading a filing can fail.
fn describe(error: FecError) -> String {
  case error {
    error.IoError(message) -> "can't read the file: " <> message
    error.UnsupportedVersion("") -> "the header has no format version"
    error.UnsupportedVersion(version) -> "unsupported FEC version " <> version
    error.ParseError(line: 0, message:) -> "bad header or cover: " <> message
    error.ParseError(line:, message:) -> at(line) <> message
    error.MissingMapping(row_type:, version:, line:) ->
      at(line) <> "no " <> version <> " columns for row type " <> row_type
    error.Closed -> "the reader was closed"
  }
}

fn at(line: Int) -> String {
  "line " <> count(line) <> ": "
}

// --- The cover --------------------------------------------------------------

/// Prints the header, then a few fields of the typed cover. Every form
/// type has a different record; the ones without a case here (and any
/// form with no typed cover at all) fall back to the common `cover_summary`.
fn print_cover(reader: libfec.Reader) -> Nil {
  let header = reader.header
  io.println(
    "Filing "
    <> reader.filing_id
    <> " (FEC "
    <> header.fec_version
    <> ", "
    <> string.trim(header.software_name <> " " <> header.software_version)
    <> ")",
  )
  case reader.cover {
    Some(cover.Form3X(c)) -> {
      io.println("Form 3X, PAC or party: " <> c.committee_name)
      field("Coverage", period(c.coverage_from_date, c.coverage_through_date))
      field("Total receipts", money(c.summary.line6c_total_receipts.column_a))
      field(
        "Total disbursements",
        money(c.summary.line7_total_disbursements.column_a),
      )
      field(
        "Cash on hand",
        money(c.summary.line8_cash_on_hand_close_of_period.column_a),
      )
    }
    Some(cover.Form3(c)) -> {
      let receipts = c.detailed_summary.receipts.line16_total_receipts
      let disbursements =
        c.detailed_summary.disbursements.line22_total_disbursements
      io.println("Form 3, House or Senate campaign: " <> c.committee_name)
      field("Coverage", period(c.coverage_from_date, c.coverage_through_date))
      field("Total receipts", money(receipts.column_a))
      field("Total disbursements", money(disbursements.column_a))
      field("Cash on hand", money(c.summary.line8_cash_on_hand_close_of_period))
    }
    Some(cover.Form3P(c)) -> {
      io.println("Form 3P, presidential campaign: " <> c.committee_name)
      field("Coverage", period(c.coverage_from_date, c.coverage_through_date))
      field("Total receipts", money(c.summary.line7_total_receipts))
      field("Total disbursements", money(c.summary.line9_total_disbursements))
      field("Cash on hand", money(c.summary.line10_cash_on_hand_end_period))
    }
    Some(cover.Form99(c)) -> {
      io.println("Form 99, miscellaneous text: " <> c.committee_name)
      field("Text", first_line(option.unwrap(c.text, "(none)")))
    }
    Some(_) | None -> {
      let s = reader.cover_summary
      io.println("Form " <> s.form_type <> ": " <> s.filer_name)
      field("Coverage", period(s.coverage_from_date, s.coverage_through_date))
    }
  }
}

// --- One pass over the rows -------------------------------------------------

/// What the fold adds up. Amounts are in dollars.
type Summary {
  Summary(
    rows: Int,
    /// Schedule A contributions, split three ways: the positive ones,
    /// negative ones (refunds, redesignations), and memo entries, which
    /// detail an amount reported elsewhere and so are not added in.
    receipts: Tally,
    refunds: Tally,
    memo_receipts: Tally,
    largest: Option(Contribution),
    states: Dict(String, Float),
    /// Schedule B, memo entries apart as for Schedule A.
    disbursements: Tally,
    memo_disbursements: Tally,
    payees: Dict(String, Float),
    /// Schedule E independent expenditures.
    supporting: Tally,
    opposing: Tally,
    candidates: Dict(String, Float),
    /// TEXT records, other typed families, and rows with no typed family.
    texts: Int,
    other_typed: Int,
    untyped: Dict(String, Int),
  )
}

type Tally {
  Tally(count: Int, total: Float)
}

type Contribution {
  Contribution(contributor: String, amount: Float)
}

fn new_summary() -> Summary {
  let zero = Tally(count: 0, total: 0.0)
  Summary(
    rows: 0,
    receipts: zero,
    refunds: zero,
    memo_receipts: zero,
    largest: None,
    states: dict.new(),
    disbursements: zero,
    memo_disbursements: zero,
    payees: dict.new(),
    supporting: zero,
    opposing: zero,
    candidates: dict.new(),
    texts: 0,
    other_typed: 0,
    untyped: dict.new(),
  )
}

/// The fold step: one `case` on the row's typed record.
fn tally(s: Summary, row: Row) -> Summary {
  let s = Summary(..s, rows: s.rows + 1)
  case row.itemization {
    Some(itemization.ScheduleA(a)) if a.memo ->
      Summary(..s, memo_receipts: add(s.memo_receipts, a.contribution_amount))
    Some(itemization.ScheduleA(a)) if a.contribution_amount <. 0.0 ->
      Summary(..s, refunds: add(s.refunds, a.contribution_amount))
    Some(itemization.ScheduleA(a)) -> {
      let state = option.unwrap(a.contributor.address.state, "??")
      Summary(
        ..s,
        receipts: add(s.receipts, a.contribution_amount),
        largest: larger(s.largest, a),
        states: add_to(s.states, state, a.contribution_amount),
      )
    }
    Some(itemization.ScheduleB(b)) if b.memo ->
      Summary(
        ..s,
        memo_disbursements: add(s.memo_disbursements, b.expenditure_amount),
      )
    Some(itemization.ScheduleB(b)) ->
      Summary(
        ..s,
        disbursements: add(s.disbursements, b.expenditure_amount),
        payees: add_to(
          s.payees,
          libfec.copy(entity_name(b.payee)),
          b.expenditure_amount,
        ),
      )
    Some(itemization.ScheduleE(e)) -> {
      let amount = e.expenditure_amount
      let candidate = libfec.copy(candidate_name(e.candidate))
      let s = Summary(..s, candidates: add_to(s.candidates, candidate, amount))
      case e.support_oppose_code {
        Some("S") -> Summary(..s, supporting: add(s.supporting, amount))
        Some("O") -> Summary(..s, opposing: add(s.opposing, amount))
        _ -> s
      }
    }
    Some(itemization.Text(_)) -> Summary(..s, texts: s.texts + 1)
    Some(_) -> Summary(..s, other_typed: s.other_typed + 1)
    None ->
      Summary(..s, untyped: dict.upsert(s.untyped, row.row_type, increment))
  }
}

fn add(tally: Tally, amount: Float) -> Tally {
  Tally(count: tally.count + 1, total: tally.total +. amount)
}

fn add_to(
  totals: Dict(String, Float),
  key: String,
  amount: Float,
) -> Dict(String, Float) {
  dict.upsert(totals, key, fn(total) { option.unwrap(total, 0.0) +. amount })
}

fn increment(n: Option(Int)) -> Int {
  option.unwrap(n, 0) + 1
}

/// The larger of the largest contribution so far and `a`. The name is
/// copied: it outlives the batch it was read in.
fn larger(largest: Option(Contribution), a: ScheduleA) -> Option(Contribution) {
  case largest {
    Some(Contribution(amount:, ..)) if amount >=. a.contribution_amount ->
      largest
    _ ->
      Some(Contribution(
        contributor: libfec.copy(entity_name(a.contributor)),
        amount: a.contribution_amount,
      ))
  }
}

/// An organization's name, or a person's "First Last".
fn entity_name(entity: Entity) -> String {
  case entity.organization_name {
    Some(name) -> name
    None -> person_name(entity.name)
  }
}

fn candidate_name(candidate: CandidateRef) -> String {
  case person_name(candidate.name), candidate.fec_id {
    "", Some(id) -> id
    "", None -> "(no candidate)"
    name, _ -> name
  }
}

fn person_name(name: PersonName) -> String {
  string.trim(name.first_name <> " " <> name.last_name)
}

// --- The report -------------------------------------------------------------

fn print_summary(s: Summary, skipped: Int) -> Nil {
  io.println("")
  io.println(count(s.rows) <> " rows after the cover")

  case s.receipts.count + s.refunds.count + s.memo_receipts.count {
    0 -> Nil
    _ -> {
      io.println("Schedule A, receipts")
      tally_field("Contributions", s.receipts)
      tally_field("Refunds", s.refunds)
      tally_field("Memo entries (not added)", s.memo_receipts)
      case s.largest {
        Some(Contribution(contributor:, amount:)) ->
          field("Largest", amount_column(amount) <> "  " <> contributor)
        None -> Nil
      }
      top_field("Top states", s.states)
    }
  }

  case s.disbursements.count + s.memo_disbursements.count {
    0 -> Nil
    _ -> {
      io.println("Schedule B, disbursements")
      tally_field("Disbursements", s.disbursements)
      tally_field("Memo entries (not added)", s.memo_disbursements)
      top_field("Top payees", s.payees)
    }
  }

  case s.supporting.count + s.opposing.count {
    0 -> Nil
    _ -> {
      io.println("Schedule E, independent expenditures")
      tally_field("Supporting", s.supporting)
      tally_field("Opposing", s.opposing)
      top_field("Top candidates", s.candidates)
    }
  }

  io.println("Everything else")
  field("TEXT records", count(s.texts))
  field("Other typed rows", count(s.other_typed))
  dict.to_list(s.untyped)
  |> list.each(fn(pair) { field("Untyped " <> pair.0, count(pair.1)) })
  case skipped {
    0 -> Nil
    n -> field("Skipped (no mapping)", count(n))
  }
}

/// `label   $total   (count)`, or nothing for an empty tally.
fn tally_field(label: String, tally: Tally) -> Nil {
  case tally {
    Tally(count: 0, ..) -> Nil
    Tally(count: n, total:) ->
      field(label, amount_column(total) <> "  (" <> count(n) <> ")")
  }
}

/// The five largest totals, one per line, largest first.
fn top_field(label: String, totals: Dict(String, Float)) -> Nil {
  dict.to_list(totals)
  |> list.sort(fn(a, b) {
    float.compare(b.1, a.1) |> order.break_tie(string.compare(a.0, b.0))
  })
  |> list.take(5)
  |> list.index_map(fn(pair, i) {
    let label = case i {
      0 -> label
      _ -> ""
    }
    field(label, amount_column(pair.1) <> "  " <> pair.0)
  })
  Nil
}

fn field(label: String, value: String) -> Nil {
  io.println("  " <> string.pad_end(label, 26, " ") <> value)
}

/// `money(amount)`, right-aligned in a column.
fn amount_column(amount: Float) -> String {
  string.pad_start(money(amount), 18, " ")
}

/// `-$1,234.56`.
fn money(amount: Float) -> String {
  let cents = float.round(amount *. 100.0)
  let sign = case cents < 0 {
    True -> "-"
    False -> ""
  }
  let cents = int.absolute_value(cents)
  sign
  <> "$"
  <> count(cents / 100)
  <> "."
  <> string.pad_start(int.to_string(cents % 100), 2, "0")
}

/// `1,234,567`.
fn count(n: Int) -> String {
  case n < 1000 {
    True -> int.to_string(n)
    False ->
      count(n / 1000)
      <> ","
      <> string.pad_start(int.to_string(n % 1000), 3, "0")
  }
}

fn period(from: Option(Date), through: Option(Date)) -> String {
  case from, through {
    Some(from), Some(through) -> date(from) <> " to " <> date(through)
    _, _ -> "(none)"
  }
}

/// `July 1, 2023`.
fn date(d: Date) -> String {
  calendar.month_to_string(d.month)
  <> " "
  <> int.to_string(d.day)
  <> ", "
  <> int.to_string(d.year)
}

/// The first line of `text`, cut to 60 characters.
fn first_line(text: String) -> String {
  let line = case string.split_once(text, "\n") {
    Ok(#(line, _)) -> line
    Error(Nil) -> text
  }
  case string.length(line) > 60 {
    True -> string.slice(line, 0, 57) <> "..."
    False -> line
  }
}

/// Prints `message` to stderr and exits with status 1.
fn exit_with(message: String) -> Nil {
  io.println_error("summary: " <> message)
  halt(1)
}

@external(erlang, "erlang", "halt")
fn halt(status: Int) -> Nil
