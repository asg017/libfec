use super::utils::{sync_item, BulkDataItem, BulkFormat};
use anyhow::Result;
use rusqlite::Transaction;
use std::sync::LazyLock;

static ITEM: LazyLock<BulkDataItem> = LazyLock::new(|| BulkDataItem {
    table_name: "independent_expenditures".to_owned(),
    url_scheme: "https://www.fec.gov/files/bulk-downloads/$YEAR/independent_expenditure_$YEAR.csv"
        .to_string(),
    schema: SCHEMA.to_string(),
    data_file_name: String::new(),
    column_count: 23,
    fts_schema: None,
    format: BulkFormat::Csv,
});

static SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS independent_expenditures(
  cycle INTEGER,
  cand_id,
  cand_name,
  spe_id,
  spe_nam,
  ele_type,
  can_office_state,
  can_office_dis,
  can_office,
  cand_pty_aff,
  exp_amo,
  exp_date,
  agg_amo,
  sup_opp,
  pur,
  pay,
  file_num,
  amndt_ind,
  tran_id,
  image_num,
  receipt_dat,
  fec_election_yr,
  prev_file_num,
  dissem_dt
);
"#;

pub fn export(
    tx: &mut Transaction<'_>,
    year: u16,
    on_progress: Option<&dyn Fn(u64, Option<u64>)>,
    offline: bool,
) -> Result<()> {
    sync_item(tx, year, &ITEM, on_progress, offline)?;
    Ok(())
}
