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
use super::utils::{build_fts_query, sync_item, BulkDataItem, BulkFormat};
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
    fts_schema: Some(FTS_SCHEMA.to_string()),
    format: BulkFormat::ZipPipeDelimited,
});

static FTS_SCHEMA: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS libfec_candidates_fts USING fts5(
  name,
  content='libfec_candidates',
  content_rowid='rowid'
);
"#;

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
    pub cycle: u16,
    pub office: Option<Office>,
    pub state: Option<String>,
    pub district: Option<String>,
}

pub(crate) fn include(
    tx: &mut Transaction,
    bulk_db_path: PathBuf,
    params: &ResolveCandidateParams,
) -> Result<()> {
    tx.execute_batch(SCHEMA)?;

    let bulk_db_str = bulk_db_path.to_str().ok_or_else(|| {
        anyhow::anyhow!("Bulk database path is not valid UTF-8: {:?}", bulk_db_path)
    })?;

    if !tx
        .prepare_cached("select 1 from pragma_database_list where name = 'bulk_db'")?
        .exists([])?
    {
        tx.execute("ATTACH DATABASE ? AS bulk_db", [bulk_db_str])?;
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
    on_progress: Option<&dyn Fn(u64, Option<u64>)>,
    offline: bool,
) -> Result<Vec<String>> {
    bulk_db.execute_batch(SCHEMA)?;
    let mut tx = bulk_db
        .transaction()
        .context("Could not start a transaction on the .bulk-data.db database")?;
    sync_item(&mut tx, params.cycle, &ITEM, on_progress, offline)?;
    tx.commit()?;
    query_candidate_principal_campaign_committees(&bulk_db, params)
}

#[derive(Debug, Clone)]
pub struct CandidateSearchResult {
    pub candidate_id: String,
    pub name: String,
    pub election_year: u16,
    pub office: String,
    pub state: String,
    pub district: String,
    pub principal_campaign_committee: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CandidateDetail {
    pub candidate_id: String,
    pub name: String,
    pub party_affiliation: String,
    pub election_year: u16,
    pub state: String,
    pub office: String,
    pub district: String,
    #[allow(dead_code)]
    pub incumbent_challenger_status: String,
    #[allow(dead_code)]
    pub status: String,
    pub principal_campaign_committee: Option<String>,
    pub address_street1: String,
    pub address_street2: String,
    pub address_city: String,
    pub address_state: String,
    pub address_zip: String,
}

/// Parses a district query like "CA41", "IL09", "TX01" into (state, district).
/// Returns None if the input doesn't match the pattern.
pub fn search_candidates(
    bulk_db: &mut Connection,
    cycle: u16,
    name_query: &str,
    on_progress: Option<&dyn Fn(u64, Option<u64>)>,
) -> Result<Vec<CandidateSearchResult>> {
    bulk_db.execute_batch(SCHEMA)?;
    let mut tx = bulk_db
        .transaction()
        .context("Could not start a transaction on the .bulk-data.db database")?;
    sync_item(&mut tx, cycle, &ITEM, on_progress, false)?;
    tx.commit()?;

    let fts_query = match build_fts_query(name_query) {
        Some(q) => q,
        None => return Ok(Vec::new()),
    };

    let sql = r#"
      SELECT
        c.candidate_id,
        c.name,
        c.election_year,
        COALESCE(c.office, ''),
        COALESCE(c.state, ''),
        COALESCE(c.district, ''),
        c.principal_campaign_committee
      FROM libfec_candidates_fts fts
      JOIN libfec_candidates c ON c.rowid = fts.rowid
      WHERE fts.name MATCH :fts_query
        AND c.cycle = :cycle
      "#;
    let params = rusqlite::named_params! {
      ":cycle": cycle,
      ":fts_query": fts_query,
    };
    let mut stmt = bulk_db.prepare(sql)?;
    let results = stmt
        .query_map(params, |row| {
            let pcc: Option<String> = row.get(6)?;
            Ok(CandidateSearchResult {
                candidate_id: row.get(0)?,
                name: row.get(1)?,
                election_year: row.get(2)?,
                office: row.get(3)?,
                state: row.get(4)?,
                district: row.get(5)?,
                principal_campaign_committee: pcc.filter(|s| !s.is_empty()),
            })
        })?
        .collect::<Result<Vec<CandidateSearchResult>, _>>()?;
    Ok(results)
}

/// Filter candidates by state and district (e.g., "CA" + "41" for California's 41st district)
pub fn filter_candidates_by_district(
    bulk_db: &mut Connection,
    cycle: u16,
    state: &str,
    district: &str,
    on_progress: Option<&dyn Fn(u64, Option<u64>)>,
) -> Result<Vec<CandidateSearchResult>> {
    bulk_db.execute_batch(SCHEMA)?;
    let mut tx = bulk_db
        .transaction()
        .context("Could not start a transaction on the .bulk-data.db database")?;
    sync_item(&mut tx, cycle, &ITEM, on_progress, false)?;
    tx.commit()?;

    let sql = r#"
      SELECT
        candidate_id,
        name,
        election_year,
        COALESCE(office, ''),
        COALESCE(state, ''),
        COALESCE(district, ''),
        principal_campaign_committee
      FROM libfec_candidates
      WHERE cycle = :cycle
        AND state = :state
        AND district = :district
        AND office = 'H'
      ORDER BY name
      "#;
    let params = rusqlite::named_params! {
      ":cycle": cycle,
      ":state": state,
      ":district": district,
    };
    let mut stmt = bulk_db.prepare(sql)?;
    let results = stmt
        .query_map(params, |row| {
            let pcc: Option<String> = row.get(6)?;
            Ok(CandidateSearchResult {
                candidate_id: row.get(0)?,
                name: row.get(1)?,
                election_year: row.get(2)?,
                office: row.get(3)?,
                state: row.get(4)?,
                district: row.get(5)?,
                principal_campaign_committee: pcc.filter(|s| !s.is_empty()),
            })
        })?
        .collect::<Result<Vec<CandidateSearchResult>, _>>()?;
    Ok(results)
}

pub fn filter_candidates_by_senate(
    bulk_db: &mut Connection,
    cycle: u16,
    state: &str,
    on_progress: Option<&dyn Fn(u64, Option<u64>)>,
) -> Result<Vec<CandidateSearchResult>> {
    bulk_db.execute_batch(SCHEMA)?;
    let mut tx = bulk_db
        .transaction()
        .context("Could not start a transaction on the .bulk-data.db database")?;
    sync_item(&mut tx, cycle, &ITEM, on_progress, false)?;
    tx.commit()?;

    let sql = r#"
      SELECT
        candidate_id,
        name,
        election_year,
        COALESCE(office, ''),
        COALESCE(state, ''),
        COALESCE(district, ''),
        principal_campaign_committee
      FROM libfec_candidates
      WHERE cycle = :cycle
        AND state = :state
        AND office = 'S'
      ORDER BY name
      "#;
    let params = rusqlite::named_params! {
      ":cycle": cycle,
      ":state": state,
    };
    let mut stmt = bulk_db.prepare(sql)?;
    let results = stmt
        .query_map(params, |row| {
            let pcc: Option<String> = row.get(6)?;
            Ok(CandidateSearchResult {
                candidate_id: row.get(0)?,
                name: row.get(1)?,
                election_year: row.get(2)?,
                office: row.get(3)?,
                state: row.get(4)?,
                district: row.get(5)?,
                principal_campaign_committee: pcc.filter(|s| !s.is_empty()),
            })
        })?
        .collect::<Result<Vec<CandidateSearchResult>, _>>()?;
    Ok(results)
}

pub fn get_candidate_detail(
    bulk_db: &mut Connection,
    cycle: u16,
    candidate_id: &str,
    on_progress: Option<&dyn Fn(u64, Option<u64>)>,
) -> Result<Option<CandidateDetail>> {
    bulk_db.execute_batch(SCHEMA)?;
    let mut tx = bulk_db
        .transaction()
        .context("Could not start a transaction on the .bulk-data.db database")?;
    sync_item(&mut tx, cycle, &ITEM, on_progress, false)?;
    tx.commit()?;

    let sql = r#"
      SELECT
        candidate_id,
        name,
        COALESCE(party_affiliation, ''),
        election_year,
        COALESCE(state, ''),
        COALESCE(office, ''),
        COALESCE(district, ''),
        COALESCE(incumbent_challenger_status, ''),
        COALESCE(status, ''),
        principal_campaign_committee,
        COALESCE(address_street1, ''),
        COALESCE(address_street2, ''),
        COALESCE(address_city, ''),
        COALESCE(address_state, ''),
        COALESCE(address_zip, '')
      FROM libfec_candidates
      WHERE cycle = :cycle
        AND candidate_id = :candidate_id
      LIMIT 1
      "#;
    let params = rusqlite::named_params! {
      ":cycle": cycle,
      ":candidate_id": candidate_id,
    };
    let mut stmt = bulk_db.prepare(sql)?;
    let mut results = stmt.query_map(params, |row| {
        let pcc: Option<String> = row.get(9)?;
        Ok(CandidateDetail {
            candidate_id: row.get(0)?,
            name: row.get(1)?,
            party_affiliation: row.get(2)?,
            election_year: row.get(3)?,
            state: row.get(4)?,
            office: row.get(5)?,
            district: row.get(6)?,
            incumbent_challenger_status: row.get(7)?,
            status: row.get(8)?,
            principal_campaign_committee: pcc.filter(|s| !s.is_empty()),
            address_street1: row.get(10)?,
            address_street2: row.get(11)?,
            address_city: row.get(12)?,
            address_state: row.get(13)?,
            address_zip: row.get(14)?,
        })
    })?;

    match results.next() {
        Some(Ok(detail)) => Ok(Some(detail)),
        Some(Err(e)) => Err(e.into()),
        None => Ok(None),
    }
}

pub fn export(
    tx: &mut Transaction<'_>,
    year: u16,
    on_progress: Option<&dyn Fn(u64, Option<u64>)>,
    offline: bool,
) -> Result<()> {
    sync_item(tx, year, &ITEM, on_progress, offline)?;
    Ok(())
}
