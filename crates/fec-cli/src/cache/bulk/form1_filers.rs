use super::utils::{sync_item, BulkDataItem, BulkFormat};
use anyhow::Result;
use rusqlite::Transaction;
use std::sync::LazyLock;

static ITEM: LazyLock<BulkDataItem> = LazyLock::new(|| BulkDataItem {
    table_name: "form1_filers".to_owned(),
    url_scheme: "https://www.fec.gov/files/bulk-downloads/$YEAR/Form1Filer_$YEAR.csv".to_string(),
    schema: SCHEMA.to_string(),
    data_file_name: String::new(),
    column_count: 17,
    fts_schema: None,
    format: BulkFormat::Csv,
});

static SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS form1_filers(
  cycle INTEGER,
  committee_id,
  committee_name,
  committee_street_1,
  committee_street_2,
  committee_city,
  committee_state,
  committee_zip,
  affiliated_committee_name,
  filed_committee_type,
  filed_committee_designation,
  filing_frequency,
  organization_type,
  treasurer_name,
  receipt_date,
  committee_email,
  committee_web_url,
  begin_image_number
);
"#;

pub fn export(
    tx: &mut Transaction<'_>,
    year: u16,
    on_progress: Option<&dyn Fn(u64, Option<u64>)>,
    offline: bool,
) -> Result<()> {
    sync_item(tx, year, &ITEM, on_progress, offline)?;
    Ok(())
}
