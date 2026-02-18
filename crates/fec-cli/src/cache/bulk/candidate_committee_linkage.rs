/**
 * > "The candidate-committee linkage file contains information linking the candidate's information to information about his or her committee. "
 *
 * https://www.fec.gov/campaign-finance-data/candidate-committee-linkage-file-description/
 *
 * Sample: https://www.fec.gov/files/bulk-downloads/2026/ccl26.zip
 *
 */
use super::utils::{sync_item, BulkDataItem};
use anyhow::{Context, Result};
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

/// A committee linkage record for a candidate
#[derive(Debug, Clone)]
pub struct CommitteeLinkage {
    pub committee_id: String,
    #[allow(dead_code)]
    pub committee_type: String,
    pub committee_designation: String,
}

impl CommitteeLinkage {
    /// Return a human-readable description of the committee designation
    pub fn designation_description(&self) -> &'static str {
        match self.committee_designation.as_str() {
            "A" => "Authorized by candidate",
            "B" => "Lobbyist/Registrant PAC",
            "D" => "Leadership PAC",
            "J" => "Joint fundraiser",
            "P" => "Principal campaign committee",
            "U" => "Unauthorized",
            _ => "Unknown",
        }
    }

    /// Return a human-readable description of the committee type
    #[allow(dead_code)]
    pub fn type_description(&self) -> &'static str {
        match self.committee_type.as_str() {
            "C" => "Communication cost",
            "D" => "Delegate committee",
            "E" => "Electioneering communication",
            "H" => "House",
            "I" => "Independent expenditor (person or group)",
            "N" => "PAC - Nonqualified",
            "O" => "Independent expenditure-only (Super PACs)",
            "P" => "Presidential",
            "Q" => "PAC - Qualified",
            "S" => "Senate",
            "U" => "Single candidate independent expenditure",
            "V" => "PAC with non-contribution account - Nonqualified",
            "W" => "PAC with non-contribution account - Qualified",
            "X" => "Party - Nonqualified",
            "Y" => "Party - Qualified",
            "Z" => "National party nonfederal account",
            _ => "Unknown",
        }
    }
}

/// Get all committee linkages for a candidate
pub fn get_candidate_committee_linkages(
    bulk_db: &mut rusqlite::Connection,
    cycle: u16,
    candidate_id: &str,
) -> Result<Vec<CommitteeLinkage>> {
    bulk_db.execute_batch(SCHEMA)?;
    let mut tx = bulk_db
        .transaction()
        .context("Could not start a transaction on the .bulk-data.db database")?;
    sync_item(&mut tx, cycle, &ITEM)?;
    tx.commit()?;

    let sql = r#"
      SELECT DISTINCT
        committee_id,
        COALESCE(committee_type, ''),
        COALESCE(committee_designation, '')
      FROM libfec_candidate_committee_linkages
      WHERE cycle = :cycle
        AND candidate_id = :candidate_id
      ORDER BY committee_designation, committee_id
      "#;
    let params = rusqlite::named_params! {
      ":cycle": cycle,
      ":candidate_id": candidate_id,
    };
    let mut stmt = bulk_db.prepare(sql)?;
    let results = stmt
        .query_map(params, |row| {
            Ok(CommitteeLinkage {
                committee_id: row.get(0)?,
                committee_type: row.get(1)?,
                committee_designation: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<CommitteeLinkage>, _>>()?;
    Ok(results)
}
