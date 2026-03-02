use super::utils::{sync_item, BulkDataItem, BulkFormat};
use anyhow::Result;
use rusqlite::Transaction;
use std::sync::LazyLock;

static ITEM: LazyLock<BulkDataItem> = LazyLock::new(|| BulkDataItem {
    table_name: "individual_contributions".to_owned(),
    url_scheme: "https://www.fec.gov/files/bulk-downloads/$YEAR/indiv$YEAR2.zip".to_string(),
    schema: SCHEMA.to_string(),
    data_file_name: "by_date/".to_string(),
    column_count: 21,
    fts_schema: None,
    format: BulkFormat::ZipPipeDelimitedDir,
});

pub(crate) static SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS individual_contributions(
  cycle INTEGER,
  committee_id,
  amendment_indicator,
  report_type,
  transaction_pgi,
  image_number,
  transaction_type,
  entity_type,
  name,
  city,
  state,
  zip_code,
  employer,
  occupation,
  transaction_date,
  transaction_amount FLOAT,
  other_id,
  transaction_id,
  file_number INTEGER,
  memo_code,
  memo_text,
  sub_id INTEGER,
  UNIQUE(cycle, sub_id)
);
"#;

pub fn export(
    tx: &mut Transaction<'_>,
    year: u16,
    on_progress: Option<&dyn Fn(u64, Option<u64>)>,
) -> Result<()> {
    sync_item(tx, year, &ITEM, on_progress)?;
    Ok(())
}
