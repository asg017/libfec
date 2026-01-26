use crate::commands::export::sqlite;
use crate::sourcer::FilingSourcer;
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;

/// Open or create the export SQLite database
pub fn open_or_create_export_db(path: &PathBuf) -> Result<Connection> {
    Connection::open(path)
        .with_context(|| format!("Could not open or create database at {:?}", path))
}

/// Export a single filing by ID using the sourcer
pub fn export_filing_by_id(
    sourcer: &FilingSourcer,
    db: &mut Connection,
    filing_id: &str,
    cover_only: bool,
) -> Result<()> {
    let filing = sourcer.resolve_from_user_argument(filing_id)?;
    sqlite::export_single_filing(db, filing, cover_only)?;
    Ok(())
}
