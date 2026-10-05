//// The generated types (`libfec/cover…`, `libfec/itemization…`) are usable
//// from user code: build values by hand and match on them. NIF round-trips
//// of the same types are tested separately.

import gleam/option.{type Option, None, Some}
import libfec/cover/common.{Address}
import libfec/itemization
import libfec/itemization/text.{TextRecord}

pub fn address_test() {
  let address =
    Address(
      street_1: Some("1 Main St"),
      street_2: None,
      city: Some("Springfield"),
      state: Some("IL"),
      zip_code: Some("62701"),
    )
  assert address.city == Some("Springfield")
  assert address.street_2 == None
}

fn text_of(record: itemization.Itemization) -> Option(String) {
  case record {
    itemization.Text(t) -> t.text
    _ -> None
  }
}

pub fn itemization_variant_test() {
  let record =
    itemization.Text(TextRecord(
      form_type: "TEXT",
      filer_committee_id: "C00000000",
      transaction_id: Some("T1"),
      back_reference_transaction_id: None,
      back_reference_schedule_name: Some("F3XN"),
      text: Some("hello"),
    ))
  assert text_of(record) == Some("hello")
}
