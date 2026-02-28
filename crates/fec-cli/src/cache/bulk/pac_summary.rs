/**
 * > "The PAC summary file contains one record for each PAC and party
 * >  committee registered with the FEC. It includes summary financial
 * >  information such as total receipts, disbursements, cash on hand, etc."
 *
 * https://www.fec.gov/campaign-finance-data/pac-and-party-summary-file-description/
 *
 * Sample: https://www.fec.gov/files/bulk-downloads/2026/webk26.zip
 *
 */
use super::utils::{sync_item, BulkDataItem};
use anyhow::Result;
use rusqlite::{Connection, Transaction};
use std::sync::LazyLock;

static ITEM: LazyLock<BulkDataItem> = LazyLock::new(|| BulkDataItem {
    table_name: "pac_summary".to_owned(),
    url_scheme: "https://www.fec.gov/files/bulk-downloads/$YEAR/webk$YEAR2.zip".to_string(),
    schema: SCHEMA.to_string(),
    data_file_name: "webk$YEAR2.txt".to_string(),
    column_count: 27,
});

static SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS pac_summary(
  cycle INTEGER,
  committee_id TEXT,
  name,
  committee_type,
  designation,
  filing_frequency,
  total_receipts FLOAT,
  transfers_from_affiliates FLOAT,
  individual_contributions FLOAT,
  other_committee_contributions FLOAT,
  candidate_contributions FLOAT,
  candidate_loans FLOAT,
  total_loans_received FLOAT,
  total_disbursements FLOAT,
  transfers_to_affiliates FLOAT,
  individual_refunds FLOAT,
  other_committee_refunds FLOAT,
  candidate_loan_repayments FLOAT,
  loan_repayments FLOAT,
  cash_on_hand_beginning FLOAT,
  cash_on_hand_close FLOAT,
  debts_owed_by FLOAT,
  nonfederal_transfers_received FLOAT,
  contributions_to_other_committees FLOAT,
  independent_expenditures FLOAT,
  party_coordinated_expenditures FLOAT,
  nonfederal_share_expenditures FLOAT,
  coverage_end_date,
  UNIQUE(cycle, committee_id)
);
"#;

pub fn export(
    tx: &mut Transaction<'_>,
    year: u16,
    on_progress: Option<&dyn Fn(u64, Option<u64>)>,
) -> Result<()> {
    let result = sync_item(tx, year, &ITEM, on_progress)?;
    println!("pac_summary {year} {result:?}");
    Ok(())
}

pub struct CommitteeFinancialSummary {
    pub total_receipts: f64,
    pub total_disbursements: f64,
    pub cash_on_hand_close: f64,
    pub individual_contributions: f64,
    pub other_committee_contributions: f64,
    pub contributions_to_other_committees: f64,
    pub independent_expenditures: f64,
    pub transfers_from_affiliates: f64,
    pub transfers_to_affiliates: f64,
    pub debts_owed_by: f64,
    pub coverage_end_date: String,
}

pub fn get_pac_summary(
    db: &mut Connection,
    cycle: u16,
    committee_id: &str,
    on_progress: Option<&dyn Fn(u64, Option<u64>)>,
) -> Result<Option<CommitteeFinancialSummary>> {
    let mut tx = db.transaction()?;
    let _result = sync_item(&mut tx, cycle, &ITEM, on_progress)?;
    tx.commit()?;

    let mut stmt = db.prepare(
        "SELECT COALESCE(total_receipts, 0.0),
                COALESCE(total_disbursements, 0.0),
                COALESCE(cash_on_hand_close, 0.0),
                COALESCE(individual_contributions, 0.0),
                COALESCE(other_committee_contributions, 0.0),
                COALESCE(contributions_to_other_committees, 0.0),
                COALESCE(independent_expenditures, 0.0),
                COALESCE(transfers_from_affiliates, 0.0),
                COALESCE(transfers_to_affiliates, 0.0),
                COALESCE(debts_owed_by, 0.0),
                COALESCE(coverage_end_date, '')
         FROM pac_summary
         WHERE cycle = ? AND committee_id = ?",
    )?;

    let result = stmt.query_row(rusqlite::params![cycle, committee_id], |row| {
        Ok(CommitteeFinancialSummary {
            total_receipts: row.get(0)?,
            total_disbursements: row.get(1)?,
            cash_on_hand_close: row.get(2)?,
            individual_contributions: row.get(3)?,
            other_committee_contributions: row.get(4)?,
            contributions_to_other_committees: row.get(5)?,
            independent_expenditures: row.get(6)?,
            transfers_from_affiliates: row.get(7)?,
            transfers_to_affiliates: row.get(8)?,
            debts_owed_by: row.get(9)?,
            coverage_end_date: row.get(10)?,
        })
    });

    match result {
        Ok(summary) => Ok(Some(summary)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
