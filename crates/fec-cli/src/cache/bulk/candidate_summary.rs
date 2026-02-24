/**
 * > "The all candidates file contains one record for each candidate
 * >  who has registered with the FEC or appeared on a ballot list.
 * >  It includes summary financial information."
 *
 * https://www.fec.gov/campaign-finance-data/all-candidates-file-description/
 *
 * Sample: https://www.fec.gov/files/bulk-downloads/2026/weball26.zip
 *
 */
use super::utils::{sync_item, BulkDataItem};
use anyhow::Result;
use rusqlite::{Connection, Transaction};
use std::sync::LazyLock;

static ITEM: LazyLock<BulkDataItem> = LazyLock::new(|| BulkDataItem {
    table_name: "candidate_summary".to_owned(),
    url_scheme: "https://www.fec.gov/files/bulk-downloads/$YEAR/weball$YEAR2.zip".to_string(),
    schema: SCHEMA.to_string(),
    data_file_name: "weball$YEAR2.txt".to_string(),
    column_count: 30,
});

static SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS candidate_summary(
  cycle INTEGER,
  candidate_id TEXT,
  name,
  incumbent_challenger_status,
  party_code,
  party_affiliation,
  total_receipts FLOAT,
  transfers_from_authorized FLOAT,
  total_disbursements FLOAT,
  transfers_to_authorized FLOAT,
  cash_on_hand_beginning FLOAT,
  cash_on_hand_close FLOAT,
  candidate_contributions FLOAT,
  candidate_loans FLOAT,
  other_loans FLOAT,
  candidate_loan_repayments FLOAT,
  other_loan_repayments FLOAT,
  debts_owed_by FLOAT,
  total_individual_contributions FLOAT,
  state,
  district,
  special_election,
  primary_election,
  runoff_election,
  general_election,
  general_election_percent,
  other_committee_contributions FLOAT,
  party_contributions FLOAT,
  coverage_end_date,
  individual_refunds FLOAT,
  committee_refunds FLOAT,
  UNIQUE(cycle, candidate_id)
);
"#;

pub fn export(tx: &mut Transaction<'_>, year: u16) -> Result<()> {
    let result = sync_item(tx, year, &ITEM)?;
    println!("candidate_summary {year} {result:?}");
    Ok(())
}

pub struct ContestCandidate {
    pub candidate_id: String,
    pub name: String,
    pub party_affiliation: String,
    pub incumbent_challenger_status: String,
    pub total_receipts: f64,
    pub total_disbursements: f64,
    pub cash_on_hand_close: f64,
    pub total_individual_contributions: f64,
    pub other_committee_contributions: f64,
    pub debts_owed_by: f64,
    pub coverage_end_date: String,
}

pub fn get_contest_candidates(
    db: &mut Connection,
    cycle: u16,
    office: &str,
    state: Option<&str>,
    district: Option<&str>,
) -> Result<Vec<ContestCandidate>> {
    // Sync bulk data first
    let mut tx = db.transaction()?;
    let result = sync_item(&mut tx, cycle, &ITEM)?;
    println!("candidate_summary {cycle} {result:?}");
    tx.commit()?;

    // Build query based on office type
    // candidate_id format: H8CA41001 (H=House), S8CA00001 (S=Senate), P80000001 (P=President)
    let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match office {
        "H" => {
            let s = state.unwrap_or("");
            let d = district.unwrap_or("");
            (
                "SELECT candidate_id, name, party_affiliation, incumbent_challenger_status,
                        COALESCE(total_receipts, 0.0), COALESCE(total_disbursements, 0.0),
                        COALESCE(cash_on_hand_close, 0.0), COALESCE(total_individual_contributions, 0.0),
                        COALESCE(other_committee_contributions, 0.0), COALESCE(debts_owed_by, 0.0),
                        COALESCE(coverage_end_date, '')
                 FROM candidate_summary
                 WHERE cycle = ? AND candidate_id LIKE 'H%' AND state = ? AND district = ?
                 ORDER BY total_receipts DESC".to_string(),
                vec![
                    Box::new(cycle) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(s.to_string()),
                    Box::new(d.to_string()),
                ],
            )
        }
        "S" => {
            let s = state.unwrap_or("");
            (
                "SELECT candidate_id, name, party_affiliation, incumbent_challenger_status,
                        COALESCE(total_receipts, 0.0), COALESCE(total_disbursements, 0.0),
                        COALESCE(cash_on_hand_close, 0.0), COALESCE(total_individual_contributions, 0.0),
                        COALESCE(other_committee_contributions, 0.0), COALESCE(debts_owed_by, 0.0),
                        COALESCE(coverage_end_date, '')
                 FROM candidate_summary
                 WHERE cycle = ? AND candidate_id LIKE 'S%' AND state = ?
                 ORDER BY total_receipts DESC".to_string(),
                vec![
                    Box::new(cycle) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(s.to_string()),
                ],
            )
        }
        "P" => (
            "SELECT candidate_id, name, party_affiliation, incumbent_challenger_status,
                    COALESCE(total_receipts, 0.0), COALESCE(total_disbursements, 0.0),
                    COALESCE(cash_on_hand_close, 0.0), COALESCE(total_individual_contributions, 0.0),
                    COALESCE(other_committee_contributions, 0.0), COALESCE(debts_owed_by, 0.0),
                    COALESCE(coverage_end_date, '')
             FROM candidate_summary
             WHERE cycle = ? AND candidate_id LIKE 'P%'
             ORDER BY total_receipts DESC".to_string(),
            vec![Box::new(cycle) as Box<dyn rusqlite::types::ToSql>],
        ),
        _ => return Err(anyhow::anyhow!("Invalid office type: {}", office)),
    };

    let mut stmt = db.prepare(&sql)?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(params_refs.as_slice(), |row| {
        Ok(ContestCandidate {
            candidate_id: row.get(0)?,
            name: row.get(1)?,
            party_affiliation: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            incumbent_challenger_status: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            total_receipts: row.get(4)?,
            total_disbursements: row.get(5)?,
            cash_on_hand_close: row.get(6)?,
            total_individual_contributions: row.get(7)?,
            other_committee_contributions: row.get(8)?,
            debts_owed_by: row.get(9)?,
            coverage_end_date: row.get(10)?,
        })
    })?;

    let mut candidates = Vec::new();
    for row in rows {
        candidates.push(row?);
    }

    Ok(candidates)
}
