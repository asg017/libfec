use anyhow::Context;
use jiff::civil::DateTime;
use rusqlite::{types::ToSqlOutput, OptionalExtension, Transaction};
/**
 * Most of the "bulk data" files found in https://www.fec.gov/data/browse-data/?tab=bulk-data
 * are zip files with a single '|' delimited text file.
 * These utility function works with those files.
 */
use std::{
    io::{BufWriter, Cursor, Read},
    str::FromStr,
};
use ureq::{http::Response, Body};

/// Wraps a reader to report progress via a callback on each read.
struct ProgressReader<'a, R> {
    inner: R,
    bytes_read: u64,
    total: Option<u64>,
    on_progress: &'a dyn Fn(u64, Option<u64>),
}

impl<'a, R: Read> ProgressReader<'a, R> {
    fn new(inner: R, total: Option<u64>, on_progress: &'a dyn Fn(u64, Option<u64>)) -> Self {
        Self {
            inner,
            bytes_read: 0,
            total,
            on_progress,
        }
    }
}

impl<R: Read> Read for ProgressReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.bytes_read += n as u64;
        (self.on_progress)(self.bytes_read, self.total);
        Ok(n)
    }
}

pub(crate) struct BulkDataItem {
    pub(crate) table_name: String,
    pub(crate) schema: String,
    pub(crate) url_scheme: String,
    pub(crate) data_file_name: String,
    pub(crate) column_count: usize,
}

pub(crate) fn csv_reader_from_response(
    response: Response<Body>,
    name: &str,
    on_progress: Option<&dyn Fn(u64, Option<u64>)>,
) -> anyhow::Result<csv::Reader<Cursor<Vec<u8>>>> {
    let content_length = response
        .headers()
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    let mut reader = response.into_body().into_reader();
    let mut buffer = Cursor::new(Vec::new());
    if let Some(on_progress) = on_progress {
        let mut progress_reader = ProgressReader::new(&mut reader, content_length, on_progress);
        std::io::copy(&mut progress_reader, &mut BufWriter::new(&mut buffer))?;
    } else {
        std::io::copy(&mut reader, &mut BufWriter::new(&mut buffer))?;
    }

    let mut archive = zip::ZipArchive::new(buffer)?;
    let mut txt_file = archive.by_name(name)?;
    let mut cn_contents = Vec::new();
    txt_file.read_to_end(&mut cn_contents)?;

    Ok(csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'|')
        .from_reader(Cursor::new(cn_contents)))
}

pub(crate) fn insert_rows(
    tx: &mut Transaction,
    cycle_year: u16,
    csv_reader: &mut csv::Reader<std::io::Cursor<Vec<u8>>>,
    table_name: &str,
    number_of_columns: usize,
) -> anyhow::Result<()> {
    let sql = format!(
        "INSERT INTO {table_name} VALUES(?, {})",
        (0..number_of_columns)
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",")
    );
    let mut stmt = tx.prepare(&sql)?;

    for result in csv_reader.records() {
        let record = result?;
        let mut params = vec![ToSqlOutput::from(cycle_year)];
        params.extend(record.iter().take(number_of_columns).map(ToSqlOutput::from));
        stmt.execute(rusqlite::params_from_iter(params))?;
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) enum SyncResult {
    //New,
    Updated,
    SkippedRecent,
    SkippedNotModified,
}

struct CycleRow {
    modified_at: String,
    last_checked_at: String,
}

pub(crate) fn sync_item(
    tx: &mut Transaction,
    year: u16,
    item: &BulkDataItem,
    on_progress: Option<&dyn Fn(u64, Option<u64>)>,
) -> anyhow::Result<SyncResult> {
    tx.execute_batch(&item.schema)?;
    tx.execute(
        &format!(
            r#"
      CREATE TABLE IF NOT EXISTS {}_cycles (
        year INTEGER PRIMARY KEY,
        modified_at TEXT NOT NULL,
        last_checked_at TEXT NOT NULL
      )
    "#,
            item.table_name
        ),
        [],
    )?;
    let result: Option<CycleRow> = tx
        .query_row(
            &format!(
                r#"
              SELECT
                modified_at,
                last_checked_at
              FROM {}_cycles
              WHERE year = ?
            "#,
                item.table_name
            ),
            [year],
            |row| {
                Ok(CycleRow {
                    modified_at: row.get::<usize, String>(0)?,
                    last_checked_at: row.get::<usize, String>(1)?,
                })
            },
        )
        .optional()?;

    // if there is already data for the given year, and the Last-Modified header
    // is recent (within the last 30 minutes), skip the update
    if let Some(row) = &result {
        // TODO use this somewhere?
        let _last_modified = jiff::fmt::rfc2822::parse(&row.modified_at)?;
        let last_checked_at = DateTime::from_str(&row.last_checked_at)
            .with_context(|| {
                format!(
                    "failed to parse last_checked_at datetime from {}",
                    &row.last_checked_at
                )
            })?
            .in_tz("UTC")
            .with_context(|| {
                format!(
                    "failed to convert last_checked_at datetime to UTC timezone for {}",
                    &row.last_checked_at
                )
            })?
            .timestamp();
        let minutes_since_last_check = jiff::Timestamp::now()
            .since(last_checked_at)
            .with_context(|| {
                format!(
                    "failed to compute minutes since last_checked_at datetime from {}",
                    &row.last_checked_at
                )
            })?
            .total(jiff::Unit::Minute)
            .expect("Span total calculations for minutes should never overflow");

        if minutes_since_last_check < 30.0 {
            tx.execute(
                &format!(
                    r#"
            UPDATE {}_cycles
            SET last_checked_at = datetime('now')
            WHERE year = ?
          "#,
                    item.table_name
                ),
                [year],
            )
            .with_context(|| {
                format!(
                    "failed to update last_checked_at for {} {}",
                    item.table_name, year
                )
            })?;
            return Ok(SyncResult::SkippedRecent);
        }
    }

    let config = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build();
    let url = item
        .url_scheme
        .replace("$YEAR2", &year.to_string()[year.to_string().len() - 2..])
        .replace("$YEAR", &year.to_string());
    let request = config.new_agent().get(
        &url, //"https://www.fec.gov/files/bulk-downloads/{}/cn{}.zip",
    );

    let request = if let Some(row) = &result {
        request.header("If-Modified-Since", &row.modified_at)
    } else {
        request
    };

    let response = request
        .call()
        .with_context(|| format!("Failed to fetch bulk data zipfile at {}", url.clone()))?;
    if response.status() == 304 {
        tx.execute(
            &format!(
                r#"
          UPDATE {}_cycles
          SET last_checked_at = datetime('now')
          WHERE year = ?;
          "#,
                item.table_name
            ),
            rusqlite::params![year],
        )?;

        return Ok(SyncResult::SkippedNotModified);
    } else if response.status() != 200 {
        panic!(
            "Failed to fetch candidates for {year}: {}",
            response.status()
        );
    }

    let last_modified = response
        .headers()
        .get("Last-Modified")
        .ok_or_else(|| {
            anyhow::anyhow!(
                "response for {} did not include Last-Modified header",
                url.clone()
            )
        })?
        .to_str()
        .with_context(|| format!("failed to parse Last-Modified header from {}", url.clone()))?;
    tx.execute(
        &format!(
            r#"
          INSERT OR REPLACE INTO {}_cycles (year, modified_at, last_checked_at)
            VALUES (?, ?, datetime('now'))
          "#,
            item.table_name
        ),
        rusqlite::params![year, last_modified],
    )
    .with_context(|| {
        format!(
            "failed to insert or replace cycle row for {} {}",
            item.table_name, year
        )
    })?;

    tx.execute(
        &format!("DELETE FROM {} WHERE cycle = ?", item.table_name),
        [year.to_string()],
    )
    .with_context(|| {
        format!(
            "failed to delete existing rows for {} {}",
            item.table_name, year
        )
    })?;

    let data_file_name = item
        .data_file_name
        .replace("$YEAR2", &year.to_string()[year.to_string().len() - 2..])
        .replace("$YEAR", &year.to_string());
    let mut rdr = csv_reader_from_response(response, &data_file_name, on_progress)?;
    insert_rows(tx, year, &mut rdr, &item.table_name, item.column_count)?;
    Ok(SyncResult::Updated)
}
