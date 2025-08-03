mod excel;
mod sqlite;
use anyhow::anyhow;
use excel::cmd_export_excel;
use sqlite::cmd_export_sqlite;
use std::error::Error;

pub fn export(args: crate::cli::ExportArgs) -> Result<(), Box<dyn Error>> {
    match args.output.extension().map(|s| s.to_str()).flatten() {
        Some("db") => cmd_export_sqlite(args),
        Some("xlsx") => cmd_export_excel(args),
        Some(_) | None => Err(anyhow!("asdf").into()),
    }
}
