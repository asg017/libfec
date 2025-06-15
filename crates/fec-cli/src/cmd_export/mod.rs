mod sqlite;
mod excel;
use std::error::Error;
use anyhow::anyhow;
use sqlite::cmd_export_sqlite;
use excel::cmd_export_excel;
pub use sqlite::CmdExportTarget;

pub fn cmd_export(
    args: crate::cli::ExportArgs
) -> Result<(), Box<dyn Error>> {
  let mut filings = args.filings.clone();
    if args.api.any_provided() {
      filings.extend(args.api.resolve_ids()?);
    }
    match args.output.extension().map(|s| s.to_str()).flatten() {
      Some("db")  => cmd_export_sqlite(filings, &args.output),
      Some("xlsx")  => cmd_export_excel(filings, &args.output),
      Some(_) | None => Err(anyhow!("asdf").into())
    }
}
