use super::utils::{sync_item, BulkDataItem, BulkFormat};
use anyhow::Result;
use rusqlite::Transaction;
use std::sync::LazyLock;

static ITEM: LazyLock<BulkDataItem> = LazyLock::new(|| BulkDataItem {
    table_name: "form2_filers".to_owned(),
    url_scheme: "https://www.fec.gov/files/bulk-downloads/$YEAR/Form2Filer_$YEAR.csv".to_string(),
    schema: SCHEMA.to_string(),
    data_file_name: String::new(),
    column_count: 16,
    fts_schema: None,
    format: BulkFormat::Csv,
});

static SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS form2_filers(
  cycle INTEGER,
  candidate_id,
  candidate_name,
  party,
  party_code,
  candidate_office,
  candidate_office_code,
  candidate_office_state,
  candidate_office_state_code,
  candidate_office_district,
  city,
  state,
  zip,
  election_year,
  receipt_date,
  report_year,
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
