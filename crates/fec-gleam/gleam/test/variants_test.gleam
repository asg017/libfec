//// Every `Cover` variant (15) and every `Itemization` variant (27) decodes
//// through the NIF and matches its constructor.
////
//// Real filings (fec-parser's committed fixtures) for every variant except
//// the Form 9 line items F91–F94, which no fixture has: those come from the
//// synthetic `fixtures/form9_items.fec`. `ScheduleH6` comes from the 7.0
//// legacy fixture.

import gleam/list
import gleam/option.{Some}
import gleam/string
import libfec/cover
import libfec/internal/nif.{type Opened}
import libfec/itemization
import libfec/row.{type Row}
import support

fn covers(file: String) -> Opened {
  support.open(support.parser_fixtures <> "covers/" <> file, False)
}

pub fn cover_form1_test() {
  let o = covers("F1N_1906351.fec")
  let assert Some(cover.Form1(c)) = o.cover
  assert c.form_type == o.cover_summary.form_type
}

pub fn cover_form1m_test() {
  let o = covers("F1MN_1917288.fec")
  let assert Some(cover.Form1M(c)) = o.cover
  assert c.form_type == o.cover_summary.form_type
}

pub fn cover_form13_test() {
  let o = covers("F13N_1904840.fec")
  let assert Some(cover.Form13(c)) = o.cover
  assert c.form_type == o.cover_summary.form_type
}

pub fn cover_form2_test() {
  let o = covers("F2N_1923633.fec")
  let assert Some(cover.Form2(c)) = o.cover
  assert c.form_type == o.cover_summary.form_type
}

pub fn cover_form24_test() {
  let o = covers("F24N_1946204.fec")
  let assert Some(cover.Form24(c)) = o.cover
  assert c.form_type == o.cover_summary.form_type
}

pub fn cover_form3_test() {
  let o = covers("F3N_1918805.fec")
  let assert Some(cover.Form3(c)) = o.cover
  assert c.form_type == o.cover_summary.form_type
}

pub fn cover_form3l_test() {
  let o = covers("F3LN_1902042.fec")
  let assert Some(cover.Form3L(c)) = o.cover
  assert c.form_type == o.cover_summary.form_type
}

pub fn cover_form3p_test() {
  let o = covers("F3PN_1920459.fec")
  let assert Some(cover.Form3P(c)) = o.cover
  assert c.form_type == o.cover_summary.form_type
}

pub fn cover_form3x_test() {
  let o = covers("F3XN_1926068.fec")
  let assert Some(cover.Form3X(c)) = o.cover
  assert c.filer_committee_id == o.cover_summary.filer_id
}

pub fn cover_form4_test() {
  let o = covers("F4N_1901605.fec")
  let assert Some(cover.Form4(c)) = o.cover
  assert c.form_type == o.cover_summary.form_type
}

pub fn cover_form5_test() {
  let o = covers("F5N_1888248.fec")
  let assert Some(cover.Form5(c)) = o.cover
  assert c.form_type == o.cover_summary.form_type
}

pub fn cover_form6_test() {
  let o = covers("F6N_1947008.fec")
  let assert Some(cover.Form6(c)) = o.cover
  assert c.form_type == o.cover_summary.form_type
}

pub fn cover_form7_test() {
  let o = covers("F7N_1884734.fec")
  let assert Some(cover.Form7(c)) = o.cover
  assert c.form_type == o.cover_summary.form_type
}

pub fn cover_form9_test() {
  let o = covers("F9A_2015422.fec")
  let assert Some(cover.Form9(c)) = o.cover
  assert c.form_type == o.cover_summary.form_type
}

pub fn cover_form99_test() {
  let o = covers("F99_1909934.fec")
  let assert Some(cover.Form99(c)) = o.cover
  assert c.form_type == o.cover_summary.form_type
  // The `[BEGINTEXT]` block after the cover is read into the typed cover.
  let assert Some(text) = c.text
  assert text != ""
}

/// The first row of `path` whose row type starts with `prefix`.
fn first(path: String, prefix: String) -> Row {
  let assert Ok(row) =
    list.find(support.rows(path), fn(r) {
      string.starts_with(r.row_type, prefix)
    })
  row
}

fn items(file: String, prefix: String) -> Row {
  first(support.parser_fixtures <> "itemizations/" <> file, prefix)
}

fn form9(prefix: String) -> Row {
  first(support.fixtures <> "form9_items.fec", prefix)
}

pub fn schedule_a_test() {
  let row = items("SA_1907925.fec", "SA")
  let assert Some(itemization.ScheduleA(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_b_test() {
  let row = items("SB_1907931.fec", "SB")
  let assert Some(itemization.ScheduleB(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_c_test() {
  let row = items("SC_1884585.fec", "SC/")
  let assert Some(itemization.ScheduleC(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_c1_test() {
  let row = items("SC1_1935097.fec", "SC1")
  let assert Some(itemization.ScheduleC1(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_c2_test() {
  let row = items("SC2_1883359.fec", "SC2")
  let assert Some(itemization.ScheduleC2(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_d_test() {
  let row = items("SD_1887446.fec", "SD")
  let assert Some(itemization.ScheduleD(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_e_test() {
  let row = items("SE_1883470.fec", "SE")
  let assert Some(itemization.ScheduleE(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_f_test() {
  let row = items("SF_1903343.fec", "SF")
  let assert Some(itemization.ScheduleF(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_h1_test() {
  let row = items("H1_1944956.fec", "H1")
  let assert Some(itemization.ScheduleH1(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_h2_test() {
  let row = items("H2_1904395.fec", "H2")
  let assert Some(itemization.ScheduleH2(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_h3_test() {
  let row = items("H3_1891862.fec", "H3")
  let assert Some(itemization.ScheduleH3(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_h4_test() {
  let row = items("H4_1907825.fec", "H4")
  let assert Some(itemization.ScheduleH4(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_h5_test() {
  let row = items("H5_1893062.fec", "H5")
  let assert Some(itemization.ScheduleH5(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_h6_test() {
  let row = first(support.parser_fixtures <> "legacy/7.0_730663.fec", "H6")
  let assert Some(itemization.ScheduleH6(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_l_test() {
  let row = items("SL_1922499.fec", "SL")
  let assert Some(itemization.ScheduleL(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn schedule_a3l_test() {
  let row = items("SA3L_1887911.fec", "SA3L")
  let assert Some(itemization.ScheduleA3L(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn text_test() {
  let row = items("TEXT_1893403.fec", "TEXT")
  let assert Some(itemization.Text(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn form5_contribution_test() {
  let row = items("F56_1920821.fec", "F56")
  let assert Some(itemization.Form5Contribution(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn form5_expenditure_test() {
  let row = items("F57_1917549.fec", "F57")
  let assert Some(itemization.Form5Expenditure(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn form6_contribution_test() {
  let row = items("F65_1912946.fec", "F65")
  let assert Some(itemization.Form6Contribution(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn form7_communication_test() {
  let row = items("F76_1884734.fec", "F76")
  let assert Some(itemization.Form7Communication(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn form13_donation_test() {
  let row = items("F132_1904840.fec", "F132")
  let assert Some(itemization.Form13Donation(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn form13_refund_test() {
  let row = items("F133_1904839.fec", "F133")
  let assert Some(itemization.Form13Refund(x)) = row.itemization
  assert x.form_type == row.row_type
}

pub fn form9_cover_synthetic_test() {
  let o = support.open(support.fixtures <> "form9_items.fec", False)
  let assert Some(cover.Form9(c)) = o.cover
  assert c.filer_committee_id == "C90009999"
}

pub fn form9_controlling_person_test() {
  let row = form9("F91")
  let assert Some(itemization.Form9ControllingPerson(x)) = row.itemization
  assert x.filer_committee_id == "C90009999"
  assert x.name.last_name == "F91.controller_last_name"
}

pub fn form9_donation_test() {
  let row = form9("F92")
  let assert Some(itemization.Form9Donation(x)) = row.itemization
  assert x.filer_committee_id == "C90009999"
  assert x.form_type == "F92"
}

pub fn form9_disbursement_test() {
  let row = form9("F93")
  let assert Some(itemization.Form9Disbursement(x)) = row.itemization
  assert x.filer_committee_id == "C90009999"
  assert x.form_type == "F93"
}

pub fn form9_candidate_test() {
  let row = form9("F94")
  let assert Some(itemization.Form9Candidate(x)) = row.itemization
  assert x.filer_committee_id == "C90009999"
  assert x.form_type == "F94"
}
