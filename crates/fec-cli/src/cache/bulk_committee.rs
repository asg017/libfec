/**
 * > "The committee master file contains one record for each committee registered with the Federal Election Commission. This includes federal political action committees and party committees, campaign committees for presidential, house and senate candidates, as well as groups or organizations who are spending money for or against candidates for federal office."
 * https://www.fec.gov/campaign-finance-data/committee-master-file-description/
 *
 * Sample: https://www.fec.gov/files/bulk-downloads/2026/cm26.zip
 *
 */
use crate::cache::bulk_utils::{sync_item, BulkDataItem};
use anyhow::{Context, Result};
use derive_builder::Builder;
use fec_api::Office;
use rusqlite::{Connection, Transaction};
use std::sync::LazyLock;

static ITEM: LazyLock<BulkDataItem> = LazyLock::new(|| BulkDataItem {
    table_name: "libfec_committees".to_owned(),
    url_scheme: "https://www.fec.gov/files/bulk-downloads/$YEAR/cm$YEAR2.zip".to_string(),
    schema: SCHEMA.to_string(),
    data_file_name: "cm.txt".to_string(),
    column_count: 15,
});

static SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS libfec_committees(
  cycle INTEGER,
  committee_id TEXT,
  name,
  treasurer_name,
  address_street1,
  address_street2,
  address_city,
  address_state,
  address_zip,
  designation,
  committee_type,
  party_affiliation,
  filing_frequency,
  interest_group_category,
  connected_org_name,
  candidate_id,
  UNIQUE(cycle, committee_id)
);
"#;

pub fn export(tx: &mut Transaction<'_>, year: u16) -> Result<()> {
    sync_item(tx, year, &ITEM)?;
    Ok(())
}
