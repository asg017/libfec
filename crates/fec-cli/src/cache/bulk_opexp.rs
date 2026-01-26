/**
 * > "This file contains disbursements reported on FEC Form 3 Line 17,
 * >  FEC Form 3P Line 23, and FEC Form 3X Lines 21(a)(i), 21(a)(ii) and 21(b)."
 *
 * https://www.fec.gov/campaign-finance-data/operating-expenditures-file-description/
 *
 * Sample: https://www.fec.gov/files/bulk-downloads/2026/oppexp26.zip
 *
 */
use crate::cache::bulk_utils::{sync_item, BulkDataItem};
use anyhow::Result;
use rusqlite::Transaction;
use std::sync::LazyLock;

static ITEM: LazyLock<BulkDataItem> = LazyLock::new(|| BulkDataItem {
    table_name: "operating_expenses".to_owned(),
    url_scheme: "https://www.fec.gov/files/bulk-downloads/$YEAR/oppexp$YEAR2.zip".to_string(),
    schema: SCHEMA.to_string(),
    data_file_name: "oppexp.txt".to_string(),
    column_count: 25,
});

static SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS operating_expenses(
  cycle INTEGER,
  committee_id,
  amendment_indicator,
  report_year INTEGER,
  report_type,
  image_number,
  line_number,
  form_type_code,
  schedule_type_code,
  name,
  city,
  state,
  zip_code,
  transaction_date,
  transaction_amount float,
  transaction_pgi,
  purpose,
  category,
  category_description,
  memo_code,
  memo_text,
  entity_type,
  sub_id integer,
  filing_id integer,
  transaction_id,
  back_reference_transaction_id,
  UNIQUE(cycle, sub_id)
);

"#;

pub fn export(tx: &mut Transaction<'_>, year: u16) -> Result<()> {
    let result = sync_item(tx, year, &ITEM)?;
    println!("opeexp {year} {result:?}");
    Ok(())
}

#[derive(Debug, Clone)]
pub struct OpExpSearchResult {
    pub committee_id: String,
    pub name: String,
    pub city: String,
    pub state: String,
    pub transaction_date: String,
    pub transaction_amount: f64,
    pub purpose: String,
    pub filing_id: i64,
}

/// Search operating expenses by recipient name
pub fn search_operating_expenses(
    conn: &mut rusqlite::Connection,
    cycle: u16,
    query: &str,
) -> Result<Vec<OpExpSearchResult>> {
    // Ensure schema exists and data is synced
    conn.execute_batch(SCHEMA)?;
    let mut tx = conn.transaction()?;
    sync_item(&mut tx, cycle, &ITEM)?;
    tx.commit()?;

    let search_pattern = format!("%{}%", query);

    let mut stmt = conn.prepare(
        "SELECT committee_id, name, city, state, transaction_date,
                transaction_amount, purpose, filing_id
         FROM operating_expenses
         WHERE cycle = ?1 AND name LIKE ?2
         ORDER BY transaction_amount DESC
         LIMIT 200",
    )?;

    let results = stmt
        .query_map([cycle.to_string(), search_pattern], |row| {
            Ok(OpExpSearchResult {
                committee_id: row.get(0)?,
                name: row.get(1)?,
                city: row.get(2)?,
                state: row.get(3)?,
                transaction_date: row.get(4)?,
                transaction_amount: row.get(5)?,
                purpose: row.get(6)?,
                filing_id: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}
