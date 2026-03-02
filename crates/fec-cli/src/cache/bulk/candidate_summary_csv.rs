use super::utils::{sync_item, BulkDataItem, BulkFormat};
use anyhow::Result;
use rusqlite::Transaction;
use std::sync::LazyLock;

static ITEM: LazyLock<BulkDataItem> = LazyLock::new(|| BulkDataItem {
    table_name: "candidate_summary_csv".to_owned(),
    url_scheme: "https://www.fec.gov/files/bulk-downloads/$YEAR/candidate_summary_$YEAR.csv"
        .to_string(),
    schema: SCHEMA.to_string(),
    data_file_name: String::new(),
    column_count: 50,
    fts_schema: None,
    format: BulkFormat::Csv,
});

static SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS candidate_summary_csv(
  cycle INTEGER,
  link_image,
  cand_name,
  cand_id,
  cand_office,
  cand_office_st,
  cand_office_dist,
  cand_party_affiliation,
  cand_incumbent_challenger_open_seat,
  total_receipt,
  total_disbursement,
  cash_on_hand_cop,
  debt_owed_by_committee,
  coverage_end_date,
  cand_street_1,
  cand_street_2,
  cand_city,
  cand_state,
  cand_zip,
  individual_itemized_contribution,
  individual_unitemized_contribution,
  individual_contribution,
  other_committee_contribution,
  party_committee_contribution,
  cand_contribution,
  total_contribution,
  transfer_from_other_auth_committee,
  cand_loan,
  other_loan,
  total_loan,
  offsets_to_operating_expenditure,
  offsets_to_fundraising,
  offsets_to_leagal_accounting,
  other_receipts,
  operating_expenditure,
  exempt_legal_accounting_disbursement,
  fundraising_disbursement,
  transfer_to_other_auth_committee,
  cand_loan_repayment,
  other_loan_repayment,
  total_loan_repayment,
  individual_refund,
  party_committee_refund,
  other_committee_refund,
  total_contribution_refund,
  other_disbursements,
  net_contribution,
  net_operating_expenditure,
  cash_on_hand_bop,
  debt_owe_to_committee,
  coverage_start_date
);
"#;

pub fn export(
    tx: &mut Transaction<'_>,
    year: u16,
    on_progress: Option<&dyn Fn(u64, Option<u64>)>,
) -> Result<()> {
    sync_item(tx, year, &ITEM, on_progress)?;
    Ok(())
}
