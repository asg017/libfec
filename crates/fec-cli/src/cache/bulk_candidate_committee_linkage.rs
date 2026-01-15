/**
 * > "The candidate-committee linkage file contains information linking the candidate's information to information about his or her committee. "
 * 
 * https://www.fec.gov/campaign-finance-data/candidate-committee-linkage-file-description/
 *
 * Sample: https://www.fec.gov/files/bulk-downloads/2026/ccl26.zip
 *
 */
use crate::cache::bulk_utils::{sync_item, BulkDataItem};
use anyhow::Result;
use rusqlite::Transaction;
use std::sync::LazyLock;

static ITEM: LazyLock<BulkDataItem> = LazyLock::new(|| BulkDataItem {
    table_name: "libfec_candidate_committee_linkages".to_owned(),
    url_scheme: "https://www.fec.gov/files/bulk-downloads/$YEAR/ccl$YEAR2.zip".to_string(),
    schema: SCHEMA.to_string(),
    data_file_name: "ccl.txt".to_string(),
    column_count: 7,
});

static SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS libfec_candidate_committee_linkages(
  cycle INTEGER,
  candidate_id,
  candidate_election_year,
  fec_election_year,
  committee_id,
  committee_type,
  committee_designation,
  linkage_id,
  UNIQUE(cycle, linkage_id)
);
"#;

pub fn export(tx: &mut Transaction<'_>, year: u16) -> Result<()> {
    sync_item(tx, year, &ITEM)?;
    Ok(())
}
