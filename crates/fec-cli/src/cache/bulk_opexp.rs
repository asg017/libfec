/**
 * > "This file contains disbursements reported on FEC Form 3 Line 17,
 *  FEC Form 3P Line 23, and FEC Form 3X Lines 21(a)(i), 21(a)(ii) and 21(b)."
 * https://www.fec.gov/campaign-finance-data/operating-expenditures-file-description/
 *
 * Sample: https://www.fec.gov/files/bulk-downloads/2026/oppexp26.zip
 *
 */
use crate::cache::bulk_utils::{sync_item, BulkDataItem};
use anyhow::{Context, Result};
use derive_builder::Builder;
use fec_api::Office;
use rusqlite::{Connection, Transaction};
use std::sync::LazyLock;

static ITEM: LazyLock<BulkDataItem> = LazyLock::new(|| BulkDataItem {
    table_name: "operating_expenses".to_owned(),
    url_scheme: "https://www.fec.gov/files/bulk-downloads/$YEAR/oppexp$YEAR2.zip".to_string(),
    schema: SCHEMA.to_string(),
    data_file_name: "oppexp.txt".to_string(),
    column_count: 25,
});

static SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS operating_expenses(
  cycle INTEGER,
  committee_id,
  amendment_indicator,
  report_year INTEGER,
  report_type,
  image_number,
  line_number,
  form_type_code,
  schedule_type_code,
  name,
  city,
  state,
  zip_code,
  transaction_date,
  transaction_amount float,
  transaction_pgi,
  purpose,
  category,
  category_description,
  memo_code,
  memo_text,
  entity_type,
  sub_id integer,
  filing_id integer,
  transaction_id,
  back_reference_transaction_id,
  UNIQUE(cycle, sub_id)
);

"#;

pub fn export(tx: &mut Transaction<'_>, year: u16) -> Result<()> {
    let result = sync_item(tx, year, &ITEM)?;
    println!("opeexp {year} {result:?}");
    Ok(())
}
