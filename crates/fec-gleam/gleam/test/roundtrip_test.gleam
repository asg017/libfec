//// Field-by-field round trips of every hand-written type, and a sample of
//// the generated ones, through the NIF.
////
//// `fixtures/FEC-424242.fec` is written by `fixtures/make_fixtures.py`:
//// every text column holds "<PREFIX>.<column>", dates and amounts are all
//// distinct, so a swapped pair of fields fails.

import gleam/list
import gleam/option.{None, Some}
import gleam/string
import gleam/time/calendar.{Date}
import libfec/cover
import libfec/cover/common.{Address, DetailedSummaryRow, PersonName}
import libfec/cover_summary.{CoverSummary}
import libfec/header.{Header}
import libfec/internal/nif.{Batch, Eof, Opened}
import libfec/itemization
import libfec/itemization/common as icommon
import libfec/itemization/text.{TextRecord}
import libfec/row.{type Row, Row}
import libfec/value.{Day, Empty, Number, Text}
import support

const path = "test/fixtures/FEC-424242.fec"

fn all_rows() -> List(Row) {
  let #(rows, end) = support.read_all(support.open(path, False), 1000)
  assert end == Eof
  rows
}

fn nth(rows: List(Row), i: Int) -> Row {
  let assert [row, ..] = list.drop(rows, i)
  row
}

/// Every text column of a row holds "<prefix>.<column>"; the columns in
/// `typed` are checked by the caller.
fn assert_text_columns(row: Row, prefix: String, typed: List(String)) -> Nil {
  list.each(row.values, fn(pair) {
    let #(column, value) = pair
    case list.contains(typed, column) {
      True -> Nil
      False -> {
        assert value == Text(prefix <> "." <> column)
      }
    }
  })
}

fn get(row: Row, column: String) {
  let assert Ok(v) = list.key_find(row.values, column)
  v
}

pub fn opened_header_and_summary_test() {
  let Opened(filing_id:, header:, cover_summary:, cover: _, handle: _) =
    support.open(path, False)
  // The file stem, `FEC-` stripped.
  assert filing_id == "424242"
  assert header
    == Header(
      fec_version: "8.4",
      software_name: "SoftName",
      software_version: "1.2.3",
      report_id: Some("FEC-1111"),
      report_number: Some("7"),
      comment: Some("Header comment"),
    )
  assert cover_summary
    == CoverSummary(
      form_type: "F3XN",
      filer_id: "C00424242",
      filer_name: "Distinct PAC",
      report_code: Some("Q1"),
      coverage_from_date: Some(Date(2024, calendar.January, 1)),
      coverage_through_date: Some(Date(2024, calendar.March, 31)),
    )
}

pub fn header_none_fields_test() {
  // 1721696.fec's HDR has an empty report id and no report number/comment.
  let opened = support.open("../../fec-py/tests/fixtures/1721696.fec", False)
  assert opened.header
    == Header(
      fec_version: "8.4",
      software_name: "FECFile",
      software_version: "8.4",
      report_id: None,
      report_number: Some("0"),
      comment: None,
    )
  assert opened.cover_summary.report_code == Some("M8")
}

pub fn typed_cover_test() {
  let opened = support.open(path, False)
  let assert Some(cover.Form3X(c)) = opened.cover
  assert c.form_type == "F3XN"
  assert c.filer_committee_id == "C00424242"
  assert c.committee_name == "Distinct PAC"
  assert c.address
    == Address(
      street_1: Some("F3X.street_1"),
      street_2: Some("F3X.street_2"),
      city: Some("F3X.city"),
      state: Some("F3X.state"),
      zip_code: Some("F3X.zip_code"),
    )
  assert c.change_of_address == True
  assert c.report_code == Some("Q1")
  assert c.election_code == Some("F3X.election_code")
  assert c.election_date == Some(Date(2024, calendar.November, 5))
  assert c.state_of_election == Some("F3X.state_of_election")
  assert c.coverage_from_date == Some(Date(2024, calendar.January, 1))
  assert c.coverage_through_date == Some(Date(2024, calendar.March, 31))
  assert c.qualified_committee == False
  assert c.treasurer
    == PersonName(
      first_name: "F3X.treasurer_first_name",
      last_name: "F3X.treasurer_last_name",
      middle_name: Some("F3X.treasurer_middle_name"),
      prefix: Some("F3X.treasurer_prefix"),
      suffix: Some("F3X.treasurer_suffix"),
    )
  assert c.date_signed == Some(Date(2024, calendar.April, 15))
  let s = c.summary
  assert s.line6a_cash_on_hand_jan_1 == 1050.5
  assert s.line6a_year == Some(2024)
  assert s.line6b_cash_on_hand_beginning_period == 1000.5
  assert s.line6c_total_receipts == DetailedSummaryRow(1001.5, 1051.5)
  // `col_a_debts_to` is "inf": a non-finite typed f64 is 0.0, not a crash.
  assert s.line9_debts_owed_to_committee == 0.0
  assert s.line10_debts_owed_by_committee == 1005.5
  let d = c.detailed_summary
  assert d.receipts.line11a_i_individuals_itemized
    == DetailedSummaryRow(1006.5, 1055.5)
  assert d.net.line38_net_operating_expenditures
    == DetailedSummaryRow(1049.5, 1098.5)
}

pub fn row_lines_and_types_test() {
  let rows = all_rows()
  assert list.map(rows, fn(r) { #(r.row_type, r.line) })
    == [
      #("SA11AI", 3),
      #("SB23", 4),
      #("SE", 5),
      #("TEXT", 6),
      #("SA11AI", 7),
      #("SB23", 8),
      #("SA11AI", 9),
      #("SA11AI", 10),
    ]
}

pub fn schedule_a_row_test() {
  let row = nth(all_rows(), 0)
  assert row.row_type == "SA11AI"
  assert row.line == 3
  assert row.extra_fields == ["EXTRA1", "EXTRA2"]
  assert list.length(row.values) == 45
  // Column order: the mapping's.
  let assert [#("form_type", _), #("filer_committee_id_number", _), ..] =
    row.values
  let assert Ok(#(last, _)) = list.last(row.values)
  assert last == "reference_code"
  assert get(row, "form_type") == Text("SA11AI")
  assert get(row, "filer_committee_id_number") == Text("C00424242")
  assert get(row, "entity_type") == Text("IND")
  assert get(row, "contribution_date") == Day(Date(2024, calendar.January, 15))
  assert get(row, "contribution_amount") == Number(250.75)
  assert get(row, "contribution_aggregate") == Number(1000.25)
  assert get(row, "memo_code") == Text("X")
  assert_text_columns(row, "SA", [
    "form_type", "filer_committee_id_number", "entity_type", "contribution_date",
    "contribution_amount", "contribution_aggregate", "memo_code",
  ])

  let assert Some(itemization.ScheduleA(a)) = row.itemization
  assert a.form_type == "SA11AI"
  assert a.filer_committee_id == "C00424242"
  assert a.transaction_id == Some("SA.transaction_id")
  assert a.back_reference_transaction_id
    == Some("SA.back_reference_tran_id_number")
  assert a.back_reference_schedule_name == Some("SA.back_reference_sched_name")
  assert a.contributor
    == icommon.Entity(
      entity_type: Some("IND"),
      organization_name: Some("SA.contributor_organization_name"),
      name: PersonName(
        first_name: "SA.contributor_first_name",
        last_name: "SA.contributor_last_name",
        middle_name: Some("SA.contributor_middle_name"),
        prefix: Some("SA.contributor_prefix"),
        suffix: Some("SA.contributor_suffix"),
      ),
      address: Address(
        street_1: Some("SA.contributor_street_1"),
        street_2: Some("SA.contributor_street_2"),
        city: Some("SA.contributor_city"),
        state: Some("SA.contributor_state"),
        zip_code: Some("SA.contributor_zip_code"),
      ),
    )
  assert a.contributor.address.state == Some("SA.contributor_state")
  assert a.election_code == Some("SA.election_code")
  assert a.election_other_description == Some("SA.election_other_description")
  assert a.contribution_date == Some(Date(2024, calendar.January, 15))
  assert a.contribution_amount == 250.75
  assert a.contribution_aggregate == Some(1000.25)
  assert a.contribution_purpose_code == None
  assert a.contribution_purpose_description
    == Some("SA.contribution_purpose_descrip")
  assert a.contributor_employer == Some("SA.contributor_employer")
  assert a.contributor_occupation == Some("SA.contributor_occupation")
  assert a.donor_committee_fec_id == Some("SA.donor_committee_fec_id")
  assert a.donor_committee_name == Some("SA.donor_committee_name")
  assert a.donor_candidate
    == icommon.CandidateRef(
      fec_id: Some("SA.donor_candidate_fec_id"),
      name: PersonName(
        first_name: "SA.donor_candidate_first_name",
        last_name: "SA.donor_candidate_last_name",
        middle_name: Some("SA.donor_candidate_middle_name"),
        prefix: Some("SA.donor_candidate_prefix"),
        suffix: Some("SA.donor_candidate_suffix"),
      ),
      office: Some("SA.donor_candidate_office"),
      state: Some("SA.donor_candidate_state"),
      district: Some("SA.donor_candidate_district"),
    )
  assert a.conduit_name == Some("SA.conduit_name")
  assert a.conduit_address
    == Address(
      street_1: Some("SA.conduit_street1"),
      street_2: Some("SA.conduit_street2"),
      city: Some("SA.conduit_city"),
      state: Some("SA.conduit_state"),
      zip_code: Some("SA.conduit_zip_code"),
    )
  assert a.memo == True
  assert a.memo_text == Some("SA.memo_text_description")
  assert a.reference_code == Some("SA.reference_code")
  assert a.image_number == None
}

pub fn schedule_b_row_test() {
  let row = nth(all_rows(), 1)
  assert row.row_type == "SB23"
  assert row.line == 4
  assert row.extra_fields == []
  assert list.length(row.values) == 44
  assert get(row, "expenditure_date") == Day(Date(2024, calendar.February, 20))
  assert get(row, "expenditure_amount") == Number(99.25)
  assert get(row, "semi_annual_refunded_bundled_amt") == Empty
  assert get(row, "memo_code") == Empty
  assert_text_columns(row, "SB", [
    "form_type", "filer_committee_id_number", "entity_type", "expenditure_date",
    "expenditure_amount", "semi_annual_refunded_bundled_amt", "memo_code",
  ])
  let assert Some(itemization.ScheduleB(b)) = row.itemization
  assert b.form_type == "SB23"
  assert b.filer_committee_id == "C00424242"
  assert b.transaction_id == Some("SB.transaction_id_number")
  assert b.payee.entity_type == Some("CCM")
  assert b.payee.name.last_name == "SB.payee_last_name"
  assert b.payee.address.zip_code == Some("SB.payee_zip_code")
  assert b.expenditure_date == Some(Date(2024, calendar.February, 20))
  assert b.expenditure_amount == 99.25
  assert b.semi_annual_refunded_bundled_amount == None
  assert b.expenditure_purpose_description
    == Some("SB.expenditure_purpose_descrip")
  assert b.category_code == Some("SB.category_code")
  assert b.beneficiary_committee_fec_id
    == Some("SB.beneficiary_committee_fec_id")
  assert b.beneficiary_candidate.district
    == Some("SB.beneficiary_candidate_district")
  assert b.conduit_address.street_1 == Some("SB.conduit_street_1")
  assert b.memo == False
  assert b.memo_text == Some("SB.memo_text_description")
}

pub fn schedule_e_row_test() {
  let row = nth(all_rows(), 2)
  assert row.row_type == "SE"
  assert row.line == 5
  assert get(row, "dissemination_date") == Day(Date(2024, calendar.March, 5))
  assert get(row, "disbursement_date") == Day(Date(2024, calendar.March, 6))
  assert get(row, "expenditure_amount") == Number(5000.5)
  assert get(row, "calendar_y_t_d_per_election_office") == Number(12_000.25)
  assert get(row, "date_signed") == Day(Date(2024, calendar.March, 7))
  assert_text_columns(row, "SE", [
    "form_type", "filer_committee_id_number", "entity_type",
    "dissemination_date", "disbursement_date", "expenditure_amount",
    "calendar_y_t_d_per_election_office", "support_oppose_code", "date_signed",
    "memo_code",
  ])
  let assert Some(itemization.ScheduleE(e)) = row.itemization
  assert e.form_type == "SE"
  assert e.transaction_id == Some("SE.transaction_id_number")
  assert e.payee.entity_type == Some("ORG")
  assert e.payee_committee_fec_id == Some("SE.payee_cmtte_fec_id_number")
  assert e.dissemination_date == Some(Date(2024, calendar.March, 5))
  assert e.disbursement_date == Some(Date(2024, calendar.March, 6))
  assert e.expenditure_amount == 5000.5
  assert e.calendar_ytd_per_election_office == Some(12_000.25)
  assert e.support_oppose_code == Some("S")
  assert e.candidate.fec_id == Some("SE.candidate_id_number")
  assert e.candidate.name.first_name == "SE.candidate_first_name"
  assert e.candidate.state == Some("SE.candidate_state")
  assert e.signer_name.last_name == "SE.completing_last_name"
  assert e.date_signed == Some(Date(2024, calendar.March, 7))
  assert e.memo == False
  assert e.payee_candidate == None
}

pub fn text_row_test() {
  let row = nth(all_rows(), 3)
  assert row
    == Row(
      row_type: "TEXT",
      line: 6,
      values: [
        #("rec_type", Text("TEXT")),
        #("filer_committee_id_number", Text("TX.filer_committee_id_number")),
        #("transaction_id_number", Text("TX.transaction_id_number")),
        #(
          "back_reference_tran_id_number",
          Text("TX.back_reference_tran_id_number"),
        ),
        #(
          "back_reference_sched_form_name",
          Text("TX.back_reference_sched_form_name"),
        ),
        #("text", Text("Hello text")),
      ],
      itemization: Some(
        itemization.Text(TextRecord(
          form_type: "TEXT",
          filer_committee_id: "TX.filer_committee_id_number",
          transaction_id: Some("TX.transaction_id_number"),
          back_reference_transaction_id: Some(
            "TX.back_reference_tran_id_number",
          ),
          back_reference_schedule_name: Some(
            "TX.back_reference_sched_form_name",
          ),
          text: Some("Hello text"),
        )),
      ),
      extra_fields: [],
    )
}

pub fn value_rule_garbage_and_blank_test() {
  let row = nth(all_rows(), 4)
  // Garbage date and amount: Text, raw and untrimmed.
  assert get(row, "contribution_date") == Text("2024-01-15")
  assert get(row, "contribution_amount") == Text(" 12abc ")
  // Blank (after trimming) amount: Empty.
  assert get(row, "contribution_aggregate") == Empty
  // Text columns: "" is Empty, whitespace is kept.
  assert get(row, "contributor_street_1") == Text("  ")
  assert get(row, "contributor_street_2") == Empty
  let assert Some(itemization.ScheduleA(a)) = row.itemization
  assert a.contribution_date == None
  assert a.contribution_amount == 0.0
  assert a.contribution_aggregate == None
}

pub fn value_rule_short_row_and_padding_test() {
  let row = nth(all_rows(), 5)
  assert row.row_type == "SB23"
  // Padded date and amount parse (trimmed first).
  assert get(row, "expenditure_date") == Day(Date(2024, calendar.February, 21))
  assert get(row, "expenditure_amount") == Number(42.0)
  // Every column of the mapping is present; past the end of the row: Empty.
  assert list.length(row.values) == 44
  let #(present, missing) = list.split(row.values, 21)
  assert list.all(missing, fn(pair) { pair.1 == Empty })
  assert list.all(present, fn(pair) { pair.1 != Empty })
  assert row.extra_fields == []
  let assert Some(itemization.ScheduleB(b)) = row.itemization
  assert b.expenditure_amount == 42.0
  assert b.memo == False
}

pub fn non_finite_floats_test() {
  let rows = all_rows()
  let nan_row = nth(rows, 6)
  assert get(nan_row, "contribution_amount") == Text("nan")
  assert get(nan_row, "contribution_aggregate") == Text("inf")
  let assert Some(itemization.ScheduleA(a)) = nan_row.itemization
  assert a.contribution_amount == 0.0
  assert a.contribution_aggregate == None

  let big_row = nth(rows, 7)
  assert get(big_row, "contribution_amount") == Text("1e999")
  assert get(big_row, "contribution_aggregate") == Text("-inf")
  let assert Some(itemization.ScheduleA(a)) = big_row.itemization
  assert a.contribution_amount == 0.0
  assert a.contribution_aggregate == None
}

pub fn value_helpers_test() {
  assert value.to_string(Text("abc")) == "abc"
  assert value.to_string(Number(1.5)) == "1.5"
  assert value.to_string(Day(Date(987, calendar.February, 3))) == "0987-02-03"
  assert value.to_string(Day(Date(-1, calendar.January, 2))) == "-0001-01-02"
  assert value.to_string(Day(Date(-987, calendar.January, 2))) == "-0987-01-02"
  assert value.to_string(Empty) == ""
  assert value.to_float(Number(2.25)) == Ok(2.25)
  assert value.to_float(Text("2.25")) == Error(Nil)
  assert value.to_float(Empty) == Error(Nil)
  assert value.to_float(Day(Date(2024, calendar.January, 1))) == Error(Nil)
}

pub fn sub_binaries_are_valid_strings_test() {
  // Values and extra fields are sub-binaries of one batch binary; they must
  // behave like any other String.
  let row = nth(all_rows(), 0)
  let assert Text(s) = get(row, "contributor_city")
  assert string.length(s) == string.length("SA.contributor_city")
  assert string.uppercase(s) == "SA.CONTRIBUTOR_CITY"
  assert string.concat(row.extra_fields) == "EXTRA1EXTRA2"
}

pub fn batch_after_eof_test() {
  let opened = support.open(path, False)
  let #(rows, end) = support.read_all(opened, 3)
  assert list.length(rows) == 8
  assert end == Eof
  assert nif.next_batch(opened.handle, 3) == Batch([], Eof)
}
