/**
 * > "The contributions by committees to candidates file contains each
 * >  contribution or independent expenditure made by a PAC, party committee,
 * >  candidate committee, or other federal committee to a candidate during
 * >  the two-year election cycle."
 *
 * https://www.fec.gov/campaign-finance-data/contributions-committees-candidates-file-description/
 *
 * Sample: https://www.fec.gov/files/bulk-downloads/2026/pas226.zip
 *
 */
use crate::cache::bulk_utils::{sync_item, BulkDataItem};
use anyhow::Result;
use rusqlite::Transaction;
use std::sync::LazyLock;

static ITEM: LazyLock<BulkDataItem> = LazyLock::new(|| BulkDataItem {
    table_name: "committee_contributions_to_candidates".to_owned(),
    url_scheme: "https://www.fec.gov/files/bulk-downloads/$YEAR/pas2$YEAR2.zip".to_string(),
    schema: SCHEMA.to_string(),
    data_file_name: "itpas2.txt".to_string(),
    column_count: 22,
});

static SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS committee_contributions_to_candidates(
  cycle INTEGER,
  committee_id,
  amendment_indicator,
  report_type,
  transaction_pgi,
  image_number,
  transaction_type,
  entity_type,
  name,
  city,
  state,
  zip_code,
  employer,
  occupation,
  transaction_date,
  transaction_amount FLOAT,
  other_id,
  candidate_id,
  transaction_id,
  file_number INTEGER,
  memo_code,
  memo_text,
  sub_id INTEGER,
  UNIQUE(cycle, sub_id)
);

"#;

pub fn export(tx: &mut Transaction<'_>, year: u16) -> Result<()> {
    let result = sync_item(tx, year, &ITEM)?;
    println!("pas2 {year} {result:?}");
    Ok(())
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Pas2SearchResult {
    pub committee_id: String,
    pub name: String,
    pub city: String,
    pub state: String,
    pub transaction_date: String,
    pub transaction_amount: f64,
    pub candidate_id: String,
    pub other_id: String,
    pub transaction_type: String,
    pub file_number: i64,
}

/// Search contributions from committees to candidates by contributor name
#[allow(dead_code)]
pub fn search_contributions_to_candidates(
    conn: &mut rusqlite::Connection,
    cycle: u16,
    query: &str,
) -> Result<Vec<Pas2SearchResult>> {
    // Ensure schema exists and data is synced
    conn.execute_batch(SCHEMA)?;
    let mut tx = conn.transaction()?;
    sync_item(&mut tx, cycle, &ITEM)?;
    tx.commit()?;

    let search_pattern = format!("%{}%", query);

    let mut stmt = conn.prepare(
        "SELECT committee_id, name, city, state, transaction_date,
                transaction_amount, candidate_id, other_id, transaction_type, file_number
         FROM committee_contributions_to_candidates
         WHERE cycle = ?1 AND name LIKE ?2
         ORDER BY transaction_amount DESC
         LIMIT 200",
    )?;

    let results = stmt
        .query_map([cycle.to_string(), search_pattern], |row| {
            Ok(Pas2SearchResult {
                committee_id: row.get(0)?,
                name: row.get(1)?,
                city: row.get(2)?,
                state: row.get(3)?,
                transaction_date: row.get(4)?,
                transaction_amount: row.get(5)?,
                candidate_id: row.get(6)?,
                other_id: row.get(7)?,
                transaction_type: row.get(8)?,
                file_number: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}
