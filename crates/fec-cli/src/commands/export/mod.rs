mod excel;
mod sqlite;
use anyhow::anyhow;
use excel::cmd_export_excel;
use sqlite::cmd_export_sqlite;

use crate::{cli::ExportArgs, sourcer::FilingSourcer};

pub fn export(sourcer: FilingSourcer, args: ExportArgs) -> anyhow::Result<()> {
    let result = match args.output.extension().map(|s| s.to_str()).flatten() {
        Some("db") => cmd_export_sqlite(sourcer, args),
        Some("xlsx") => cmd_export_excel(sourcer, args),
        Some(_) | None => Err(anyhow!("asdf").into()),
    };
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error exporting: {}", e);
            Err(e)
        }
    }
}
