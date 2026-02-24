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
use rusqlite::Transaction;
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

pub fn export(tx: &mut Transaction<'_>, year: u16) -> Result<()> {
    let result = sync_item(tx, year, &ITEM)?;
    println!("pac_summary {year} {result:?}");
    Ok(())
}
