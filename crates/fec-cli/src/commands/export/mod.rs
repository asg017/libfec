mod dir_csv;
mod excel;
mod single;
mod sqlite;
use anyhow::anyhow;
use excel::cmd_export_excel;
use sqlite::cmd_export_sqlite;

use crate::{
    cli::{ExportArgs, ExportFormat},
    sourcer::FilingSourcer,
};

pub fn export(sourcer: FilingSourcer, args: ExportArgs) -> anyhow::Result<()> {
    let result = match (args.output.clone(), args.output_directory.clone()) {
      (None, None) => {
          Err(anyhow!("Must specify either --output (output to a file) or --output-directory (output multiple files to a directory)"))
      }
      (Some(_), Some(_)) => {
          Err(anyhow!("Cannot specify both --output and --output-directory"))
      }
      (Some(output), None) => {
        if args.clobber && output.exists() {
          std::fs::remove_file(&output)?;
        }
        match output.extension().map(|s| s.to_str()).flatten() {
            Some("db") => cmd_export_sqlite(sourcer, output, args),
            Some("xlsx") => cmd_export_excel(sourcer, output, args),
            Some("csv") => {
              if let Some(target) = args.target.clone() {
                single::cmd_export_single(sourcer, output, args, target, single::SingleOutput::Csv)
              }else {
                Err(anyhow!("Must specify --target when exporting to a single CSV file"))
              }
            },
            Some("json") => {
              if let Some(target) = args.target.clone() {
                single::cmd_export_single(sourcer, output, args, target, single::SingleOutput::Json)
              }else {
                Err(anyhow!("Must specify --target when exporting to a single JSON file"))
              }
            },
            Some(_) | None => Err(anyhow!("Unsupported output format")),
        }
      }
      (None, Some(output_directory)) => {
        std::fs::create_dir_all(&output_directory)?;
        match args.format {
          None => Err(anyhow!("Must specify --format when using --output-directory")),
          Some(ExportFormat::Sqlite) => todo!(),
          Some(ExportFormat::Excel) => todo!(),
          Some(ExportFormat::Csv) => {
            dir_csv::export(sourcer, args, output_directory)
          },
          Some(ExportFormat::Json) => todo!(),
        }
      }
    };
    /*
    let result = match args.output.extension().map(|s| s.to_str()).flatten() {
        Some("db") => cmd_export_sqlite(sourcer, args),
        Some("xlsx") => cmd_export_excel(sourcer, args),
        Some(_) | None => Err(anyhow!("asdf").into()),
    };
     */
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error exporting: {:?}", e);
            Err(e)
        }
    }
}
