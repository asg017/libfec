/**
 * > "The committee master file contains one record for each committee registered with the Federal Election Commission. This includes federal political action committees and party committees, campaign committees for presidential, house and senate candidates, as well as groups or organizations who are spending money for or against candidates for federal office."
 * 
 * https://www.fec.gov/campaign-finance-data/committee-master-file-description/
 *
 * Sample: https://www.fec.gov/files/bulk-downloads/2026/cm26.zip
 *
 */
use crate::cache::bulk_utils::{sync_item, BulkDataItem};
use anyhow::{Context, Result};
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

#[derive(Debug, Clone)]
pub struct CommitteeSearchResult {
    pub committee_id: String,
    pub name: String,
    pub committee_type: String,
    pub designation: String,
    pub party_affiliation: String,
    pub connected_org_name: String,
    pub candidate_id: Option<String>,
}

pub fn search_committees(
    bulk_db: &mut Connection,
    cycle: u16,
    name_query: &str,
) -> Result<Vec<CommitteeSearchResult>> {
    bulk_db.execute_batch(SCHEMA)?;
    let mut tx = bulk_db
        .transaction()
        .context("Could not start a transaction on the .bulk-data.db database")?;
    sync_item(&mut tx, cycle, &ITEM)?;
    tx.commit()?;

    let sql = r#"
      SELECT
        committee_id,
        name,
        COALESCE(committee_type, ''),
        COALESCE(designation, ''),
        COALESCE(party_affiliation, ''),
        COALESCE(connected_org_name, ''),
        candidate_id
      FROM libfec_committees
      WHERE cycle = :cycle
        AND name LIKE '%' || :name_query || '%'
      "#;
    let params = rusqlite::named_params! {
      ":cycle": cycle,
      ":name_query": name_query,
    };
    let mut stmt = bulk_db.prepare(sql)?;
    let results = stmt
        .query_map(params, |row| {
            let cand_id: Option<String> = row.get(6)?;
            Ok(CommitteeSearchResult {
                committee_id: row.get(0)?,
                name: row.get(1)?,
                committee_type: row.get(2)?,
                designation: row.get(3)?,
                party_affiliation: row.get(4)?,
                connected_org_name: row.get(5)?,
                candidate_id: cand_id.filter(|s| !s.is_empty()),
            })
        })?
        .collect::<Result<Vec<CommitteeSearchResult>, _>>()?;
    Ok(results)
}

pub fn export(tx: &mut Transaction<'_>, year: u16) -> Result<()> {
    sync_item(tx, year, &ITEM)?;
    Ok(())
}
