/**
 * > "The all candidate summary file contains one record including summary financial
 * > information for all candidates who raised or spent money during the period
 * > no matter when they are up for election."
 * 
 * https://www.fec.gov/campaign-finance-data/candidate-master-file-description/
 *
 * Sample: https://www.fec.gov/files/bulk-downloads/2026/weball26.zip
 *
 */
use crate::cache::bulk_utils::{BulkDataItem, sync_item};
use anyhow::{Context, Result};
use derive_builder::Builder;
use fec_api::Office;
use rusqlite::{Connection, Transaction};
use std::{path::PathBuf, sync::LazyLock};

static ITEM: LazyLock<BulkDataItem> = LazyLock::new(|| BulkDataItem {
    table_name: "libfec_candidates".to_owned(),
    url_scheme: "https://www.fec.gov/files/bulk-downloads/$YEAR/cn$YEAR2.zip".to_string(),
    schema: SCHEMA.to_string(),
    data_file_name: "cn.txt".to_string(),
    column_count: 15,
});

static SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS libfec_candidates(
  cycle INTEGER, -- REFERENCES candidate_cycles(year) ON DELETE CASCADE,
  candidate_id TEXT,
  name,
  party_affiliation,
  election_year INTEGER,
  state,
  office,
  district,
  incumbent_challenger_status,
  status,
  principal_campaign_committee,
  address_street1,
  address_street2,
  address_city,
  address_state,
  address_zip,

  UNIQUE(cycle, candidate_id)
);
"#;

#[derive(Debug, Clone, Builder, Default)]
pub struct ResolveCandidateParams {
    cycle: u16,
    office: Option<Office>,
    state: Option<String>,
    district: Option<String>,
}

pub(crate) fn include(
    tx: &mut Transaction,
    bulk_db_path: PathBuf,
    params: &ResolveCandidateParams,
) -> Result<()> {
    tx.execute_batch(SCHEMA)?;

    let bulk_db_str = bulk_db_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Bulk database path is not valid UTF-8: {:?}", bulk_db_path))?;

    if !tx
        .prepare_cached("select 1 from pragma_database_list where name = 'bulk_db'")?
        .exists([])?
    {
        tx.execute(
            "ATTACH DATABASE ? AS bulk_db",
            [bulk_db_str],
        )?;
    }

    let sql = r#"
      INSERT OR REPLACE INTO libfec_candidates
        SELECT *
        FROM bulk_db.libfec_candidates 
        WHERE cycle = :cycle
          AND election_year = cast(:cycle as text)
          AND principal_campaign_committee != ''
          AND if(:office is null, true, office = :office)
          AND if(:state is null, true, state = :state)
          AND if(:district is null, true, cast(district as text) = :district)
      "#;
    let params = rusqlite::named_params! {
      ":cycle": params.cycle,
      ":office": params.office.clone().map(|o| match o {
        Office::House => "H",
        Office::Senate => "S",
        Office::President => "P",
      }),
      ":state": params.state,
      ":district": params.district
    };
    let mut stmt = tx.prepare(sql)?;
    stmt.execute(params)?;
    drop(stmt);
    //tx.execute("DETACH DATABASE bulk_db", [])?;
    Ok(())
}

fn query_candidate_principal_campaign_committees(
    db: &Connection,
    params: ResolveCandidateParams,
) -> Result<Vec<String>> {
    let sql = r#"
      SELECT 
        principal_campaign_committee 
      FROM libfec_candidates 
      WHERE cycle = :cycle
        AND election_year = cast(:cycle as text)
        AND principal_campaign_committee != ''
        AND if(:office is null, true, office = :office)
        AND if(:state is null, true, state = :state)
        AND if(:district is null, true, cast(district as text) = :district)
      "#;
    let params = rusqlite::named_params! {
      ":cycle": params.cycle,
      ":office": params.office.map(|o| match o {
        Office::House => "H",
        Office::Senate => "S",
        Office::President => "P",
      }),
      ":state": params.state,
      ":district": params.district
    };
    let mut stmt = db.prepare(sql)?;
    let committee_ids = stmt
        .query_map(params, |row| {
            let committee_id: String = row.get(0)?;
            Ok(committee_id)
        })?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(committee_ids)
}

pub fn resolve_candidate_principal_campaign_committees(
    mut bulk_db: Connection,
    params: ResolveCandidateParams,
) -> Result<Vec<String>> {
    bulk_db.execute_batch(SCHEMA)?;
    let mut tx = bulk_db
        .transaction()
        .context("Could not start a transaction on the .bulk-data.db database")?;
    sync_item(&mut tx, params.cycle, &ITEM)?;
    tx.commit()?;
    query_candidate_principal_campaign_committees(&bulk_db, params)
}

#[allow(dead_code)]
pub fn search_candidates(
    bulk_db: &mut Connection,
    cycle: u16,
    name_query: &str,
) -> Result<Vec<(String, String)>> {
    bulk_db.execute_batch(SCHEMA)?;
    let mut tx = bulk_db
        .transaction()
        .context("Could not start a transaction on the .bulk-data.db database")?;
    sync_item(&mut tx, cycle, &ITEM)?;
    tx.commit()?;

    let sql = r#"
      SELECT 
        candidate_id,
        name
      FROM libfec_candidates 
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
            let candidate_id: String = row.get(0)?;
            let name: String = row.get(1)?;
            Ok((candidate_id, name))
        })?
        .collect::<Result<Vec<(String, String)>, _>>()?;
    Ok(results)
}


pub fn export(tx: &mut Transaction<'_>, year: u16) -> Result<()> {
    sync_item(tx, year, &ITEM)?;
    Ok(())
}
