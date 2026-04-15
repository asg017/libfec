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
    /// When set, only return candidates whose election is in this year.
    /// When None, return all candidates in the cycle regardless of election_year.
    #[builder(default)]
    pub election_year: Option<u16>,
    #[builder(default)]
    pub office: Option<Office>,
    #[builder(default)]
    pub state: Option<String>,
    #[builder(default)]
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

pub(crate) fn query_candidate_principal_campaign_committees(
    db: &Connection,
    params: ResolveCandidateParams,
) -> Result<Vec<String>> {
    let sql = r#"
      SELECT
        principal_campaign_committee
      FROM libfec_candidates
      WHERE cycle = :cycle
        AND if(:election_year is null, true, election_year = cast(:election_year as text))
        AND principal_campaign_committee != ''
        AND if(:office is null, true, office = :office)
        AND if(:state is null, true, state = :state)
        AND if(:district is null, true, cast(district as text) = :district)
      "#;
    let params = rusqlite::named_params! {
      ":cycle": params.cycle,
      ":election_year": params.election_year,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_flags::FilingsApiFlags;
    use clap::Parser;
    use insta::assert_snapshot;
    use rusqlite::Connection;
    use std::str::FromStr;

    fn test_db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(SCHEMA).unwrap();
        db.execute_batch(
            r#"
            INSERT INTO libfec_candidates (cycle, candidate_id, name, election_year, state, office, district, principal_campaign_committee)
            VALUES
              (2026, 'H0CA12001', 'Alice Smith', 2026, 'CA', 'H', '12', 'C00000001'),
              (2026, 'H0CA12002', 'Bob Jones', 2026, 'CA', 'H', '12', 'C00000002'),
              (2026, 'S0CA00001', 'Carol White', 2026, 'CA', 'S', '00', 'C00000003'),
              (2026, 'H0TX07001', 'Dave Brown', 2026, 'TX', 'H', '07', 'C00000004'),
              (2026, 'H0CA41001', 'Eve Green', 2026, 'CA', 'H', '41', 'C00000005'),
              (2026, 'P80000001', 'Frank Pres', 2026, 'US', 'P', '00', 'C00000006'),
              (2026, 'H0CA12003', 'No Committee', 2026, 'CA', 'H', '12', '');
        "#,
        )
        .unwrap();
        db
    }

    /// The 2028 cycle file contains candidates for both 2027 special elections
    /// and 2028 regular elections. --cycle returns all, --election filters by year.
    fn test_db_multi_cycle() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(SCHEMA).unwrap();
        db.execute_batch(
            r#"
            INSERT INTO libfec_candidates (cycle, candidate_id, name, election_year, state, office, district, principal_campaign_committee)
            VALUES
              -- 2026 cycle: House race only
              (2026, 'H0CA12001', 'House Rep 2026', 2026, 'CA', 'H', '12', 'C00100001'),
              -- 2028 cycle, election_year=2028: regular Senate + House races
              (2028, 'S0CA00001', 'Senate Candidate A', 2028, 'CA', 'S', '00', 'C00200001'),
              (2028, 'S0CA00002', 'Senate Candidate B', 2028, 'CA', 'S', '00', 'C00200002'),
              (2028, 'H0CA12002', 'House Rep 2028', 2028, 'CA', 'H', '12', 'C00200003'),
              -- 2028 cycle, election_year=2027: special election (mid-cycle vacancy)
              (2028, 'H0CA05001', 'Special Election Candidate', 2027, 'CA', 'H', '05', 'C00200004');
        "#,
        )
        .unwrap();
        db
    }

    fn resolve(db: &Connection, args: &str) -> String {
        let argv: Vec<&str> = std::iter::once("test")
            .chain(args.split_whitespace())
            .collect();
        let flags = FilingsApiFlags::parse_from(argv);

        let cycle = flags
            .election
            .map(|y| y + (y % 2)) // bulk data only for even-year cycles
            .or_else(|| flags.cycle.as_ref().and_then(|c| c.first().copied()))
            .expect("test must provide --cycle or --election");

        let params = ResolveCandidateParamsBuilder::default()
            .cycle(cycle)
            .election_year(flags.election)
            .state(flags.state.clone())
            .district(flags.district.clone())
            .office(
                flags
                    .office
                    .as_deref()
                    .map(|o| Office::from_str(o).unwrap()),
            )
            .build()
            .unwrap();

        let mut ids = query_candidate_principal_campaign_committees(db, params).unwrap();
        ids.sort();

        if ids.is_empty() {
            "(no results)".to_string()
        } else {
            ids.join("\n")
        }
    }

    // All CA candidates: 3 House (districts 12 and 41) + 1 Senate
    #[test]
    fn test_state_ca() {
        assert_snapshot!(resolve(&test_db(), "--cycle 2026 --state CA"), @r"
        C00000001
        C00000002
        C00000003
        C00000005
        ");
    }

    // Only House candidates in CA (excludes Carol's Senate seat)
    #[test]
    fn test_state_ca_office_house() {
        assert_snapshot!(resolve(&test_db(), "--cycle 2026 --state CA --office H"), @r"
        C00000001
        C00000002
        C00000005
        ");
    }

    // Narrows to CA district 12 only (Alice + Bob, not Eve in district 41)
    #[test]
    fn test_state_ca_district_12() {
        assert_snapshot!(resolve(&test_db(), "--cycle 2026 --state CA --district 12"), @r"
        C00000001
        C00000002
        ");
    }

    // Only TX candidate (Dave)
    #[test]
    fn test_state_tx() {
        assert_snapshot!(resolve(&test_db(), "--cycle 2026 --state TX"), @"C00000004");
    }

    // Presidential candidate only (office filter, no state)
    #[test]
    fn test_office_president() {
        assert_snapshot!(resolve(&test_db(), "--cycle 2026 --office P"), @"C00000006");
    }

    // No state/office/district filters: all 6 candidates (excludes empty-committee row)
    #[test]
    fn test_no_filters() {
        assert_snapshot!(resolve(&test_db(), "--cycle 2026"), @r"
        C00000001
        C00000002
        C00000003
        C00000004
        C00000005
        C00000006
        ");
    }

    // No CA senate race exists in 2026 cycle
    #[test]
    fn test_cycle_2026_ca_senate() {
        assert_snapshot!(resolve(&test_db_multi_cycle(), "--cycle 2026 --state CA --office S"), @"(no results)");
    }

    // CA senate race exists in 2028 — two candidates
    #[test]
    fn test_cycle_2028_ca_senate() {
        assert_snapshot!(resolve(&test_db_multi_cycle(), "--cycle 2028 --state CA --office S"), @r"
        C00200001
        C00200002
        ");
    }

    // 2026 CA has only a House rep (no Senate that cycle)
    #[test]
    fn test_cycle_2026_ca_all() {
        assert_snapshot!(resolve(&test_db_multi_cycle(), "--cycle 2026 --state CA"), @"C00100001");
    }

    // --cycle returns ALL candidates in that cycle, regardless of election_year
    // (includes the 2027 special election candidate)
    #[test]
    fn test_cycle_2028_ca_all() {
        assert_snapshot!(resolve(&test_db_multi_cycle(), "--cycle 2028 --state CA"), @r"
        C00200001
        C00200002
        C00200003
        C00200004
        ");
    }

    // --election filters to only candidates whose election_year matches
    // (excludes the 2027 special election candidate)
    #[test]
    fn test_election_2028_ca_all() {
        assert_snapshot!(resolve(&test_db_multi_cycle(), "--election 2028 --state CA"), @r"
        C00200001
        C00200002
        C00200003
        ");
    }

    // --election 2028 with senate filter
    #[test]
    fn test_election_2028_ca_senate() {
        assert_snapshot!(resolve(&test_db_multi_cycle(), "--election 2028 --state CA --office S"), @r"
        C00200001
        C00200002
        ");
    }

    // --election 2027 (odd year special election): rounds up to cycle 2028,
    // then filters election_year=2027 to find only the special election candidate
    #[test]
    fn test_election_2027_ca_special() {
        assert_snapshot!(resolve(&test_db_multi_cycle(), "--election 2027 --state CA"), @"C00200004");
    }
}
