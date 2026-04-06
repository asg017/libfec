pub mod sqlite_docs;

use crate::{
    cache::bulk::{candidates, committee},
    cli::ExportArgs,
    sourcer::{FilingSourcer, ItemizationProgressBar},
};
use anyhow::Context;
use colored::Colorize;
use csv::StringRecord;
use fec_parser::{
    mappings::{DATE_COLUMNS, FLOAT_COLUMNS},
    schedules::{form_type_schedule_type, ScheduleType},
    try_format_fec_date, Filing, FilingRow,
};
use indicatif::{HumanDuration, MultiProgress};
use rusqlite::{
    params_from_iter,
    types::{ToSqlOutput, Value},
    Connection, Statement, ToSql, Transaction,
};
use std::{collections::HashMap, hash::Hash, io::Read, path::PathBuf, time::Instant};

const CREATE_FILINGS_SQL: &str = r#"
  CREATE TABLE IF NOT EXISTS libfec_filings(
    /**
     * 
     * 
     */

    --- Unique numeric identifier for this filing, assigned by the FEC, ex 1884420
    filing_id TEXT PRIMARY KEY NOT NULL,
    
    --- Version of the FEC filing format, ex '8.4'
    fec_version TEXT NOT NULL,
    
    --- Name of the software that produced this filing, ex 'NetFile'
    software_name TEXT NOT NULL,

    --- Version of the software that produced this filing, ex '2022451'
    software_version TEXT NOT NULL,

    --- If this filing is an amendment, the report_id of the original filing, otherwise null. ex 1884419
    report_id TEXT,

    --- Sequential number of amendments
    report_number TEXT,

    --- Any header comments provided by the filer
    comment TEXT,

    --- Form type of the cover record, ex 'F3'
    cover_record_form TEXT NOT NULL,
    cover_record_form_amendment_indicator TEXT,

    filer_id TEXT NOT NULL,
    filer_name TEXT NOT NULL,
    report_code TEXT,
    coverage_from_date TEXT,
    coverage_through_date TEXT
  )
"#;

const INSERT_FILING_SQL: &str = r#"
  INSERT INTO libfec_filings VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?)
"#;

#[derive(Clone, Copy)]
enum FieldFormat {
    Text,
    Float,
    Date,
}

#[derive(Clone)]
enum FieldValue {
    Text(String),
    Float(f64),
    Date(String),
}

impl ToSql for FieldValue {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        match self {
            FieldValue::Text(v) => Ok(ToSqlOutput::Owned(Value::Text(v.to_owned()))),
            FieldValue::Date(v) => Ok(ToSqlOutput::Owned(Value::Text(v.to_owned()))),
            FieldValue::Float(v) => Ok(ToSqlOutput::Owned(Value::Real(*v))),
        }
    }
}

struct Warning {
    _message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ItemizationKey {
    Schedule(ScheduleType),
    FormType(String),
}

struct ItemizationValue<'a> {
    record_table: RecordTable,
    insert_statement: Statement<'a>,
}

struct RecordTable {
    column_names: Vec<String>,
    column_types: Vec<FieldFormat>,
    suffix: String,
    current_mapping: Vec<String>,
    legacy_field_mapping: HashMap<String, Vec<Option<usize>>>,
}

const LATEST_FEC_VERSION: &str = "8.5";
impl RecordTable {
    fn new(row_type: &str, suffix: &str) -> anyhow::Result<Self> {
        let column_names =
            fec_parser::mappings::column_names_for_field(row_type, LATEST_FEC_VERSION)
                // some forms were *removed* in  8.5, like F3Z1, ex FEC-1890921. TODO Need a better fallback strategy
                .or_else(|_| fec_parser::mappings::column_names_for_field(row_type, "8.4"))
                .with_context(|| {
                    format!(
                        "Error getting mapping column names for field '{}' and fec version '{}'",
                        row_type, LATEST_FEC_VERSION
                    )
                })?
                .to_owned();
        let current_mapping = column_names.clone();
        let column_types: Vec<FieldFormat> = column_names
            .iter()
            .map(|c| {
                if DATE_COLUMNS.contains(c) {
                    FieldFormat::Date
                } else if FLOAT_COLUMNS.contains(c) {
                    FieldFormat::Float
                } else {
                    FieldFormat::Text
                }
            })
            .collect();

        Ok(RecordTable {
            column_names,
            column_types,
            suffix: suffix.to_owned(),
            current_mapping,
            legacy_field_mapping: HashMap::new(),
        })
    }
    fn create_sql(&self) -> String {
        let docs = sqlite_docs::table_docs(&self.suffix.to_ascii_lowercase());

        let mut sql = String::from("CREATE TABLE IF NOT EXISTS [libfec_");
        sql += &self.suffix;
        sql += "](\n";

        if let Some(table_doc) = docs.as_ref().and_then(|d| d.table) {
            for line in table_doc.lines() {
                sql += "  --! ";
                sql += line;
                sql += "\n";
            }
            sql += "\n";
        }

        sql += "  filing_id text references libfec_filings(filing_id),\n";

        let last_idx = self.column_names.len() - 1;
        for (i, (name, col_type)) in self
            .column_names
            .iter()
            .zip(self.column_types.iter())
            .enumerate()
        {
            if let Some(col_doc) = docs.as_ref().and_then(|d| d.column_doc(name)) {
                for line in col_doc.lines() {
                    sql += "  --- ";
                    sql += line;
                    sql += "\n";
                }
            }
            sql += "  ";
            sql += name;
            sql += " ";
            sql += match col_type {
                FieldFormat::Date => "date",
                FieldFormat::Text => "text",
                FieldFormat::Float => "float",
            };
            if i < last_idx {
                sql += ",";
            }
            sql += "\n";
        }

        sql += ")";
        sql
    }
    fn insert_statement<'a>(&self, tx: &'a Transaction) -> anyhow::Result<Statement<'a>> {
        let sql = format!(
            "INSERT INTO libfec_{} VALUES ({})",
            self.suffix,
            vec!["?"; self.column_names.len() + 1].join(",")
        );
        tx.prepare(&sql).with_context(|| {
            format!(
                "Error preparing insert statement for record table libfec_{}",
                self.suffix
            )
        })
    }

    fn insert(
        &mut self,
        insert_stmt: &mut Statement,
        filing_id: &str,
        record: &StringRecord,
        fec_version: &str,
    ) -> anyhow::Result<()> {
        let params = self.prep_row(
            filing_id,
            record,
            insert_stmt.parameter_count(),
            fec_version,
        );
        insert_stmt.execute(params_from_iter(params))?;
        Ok(())
    }
    fn prep_row(
        &mut self,
        filing_id: &str,
        record: &StringRecord,
        n_params: usize,
        fec_version: &str,
    ) -> Vec<FieldValue> {
        let mut warnings = Vec::new();
        let mut values: Vec<FieldValue> = if fec_version == LATEST_FEC_VERSION {
            record
                .iter()
                .enumerate()
                .map(|(idx, field)| match self.column_types.get(idx) {
                    Some(FieldFormat::Text) => FieldValue::Text(field.to_owned()),
                    Some(FieldFormat::Date) => match field.len() {
                        8 => FieldValue::Date(try_format_fec_date(field)),
                        _ => FieldValue::Text(field.to_owned()),
                    },
                    Some(FieldFormat::Float) => match field.parse::<f64>() {
                        Ok(value) => FieldValue::Float(value),
                        Err(_) => FieldValue::Text(field.to_owned()),
                    },
                    None => FieldValue::Text(field.to_owned()),
                })
                .collect()
        } else {
            let legacy_mapping = self
                .legacy_field_mapping
                .entry(fec_version.to_owned())
                .or_insert_with(|| {
                    let legacy_columns =
                        fec_parser::mappings::column_names_for_field(&record[0], fec_version)
                            .unwrap()
                            .to_owned();
                    self.current_mapping
                        .iter()
                        .map(|col_name| legacy_columns.iter().position(|c| c == col_name))
                        .collect::<Vec<Option<usize>>>()
                });
            legacy_mapping
                .iter()
                .enumerate()
                .map(|(idx, &record_idx)| {
                    let field = record_idx
                        .map(|i| record.get(i).unwrap_or(""))
                        .unwrap_or("");
                    match self.column_types.get(idx) {
                        Some(FieldFormat::Text) => FieldValue::Text(field.to_owned()),
                        Some(FieldFormat::Date) => match field.len() {
                            8 => FieldValue::Date(try_format_fec_date(field)),
                            _ => FieldValue::Text(field.to_owned()),
                        },
                        Some(FieldFormat::Float) => match field.parse::<f64>() {
                            Ok(value) => FieldValue::Float(value),
                            Err(_) => FieldValue::Text(field.to_owned()),
                        },
                        None => FieldValue::Text(field.to_owned()),
                    }
                })
                .collect()
        };

        // make sure filing_id is the first column
        values.insert(0, FieldValue::Text(filing_id.to_owned()));

        if values.len() == n_params + 1 {
            values.pop();
        }

        while values.len() < n_params {
            values.push(FieldValue::Text("".to_owned()));
        }
        if values.len() > n_params {
            warnings.push(Warning {
                _message: format!(
                    "Warning too long at {}:{}, {} vs {}!",
                    filing_id,
                    record.position().map(|p| p.line()).unwrap_or(0),
                    values.len(),
                    n_params
                ),
            });
            values.truncate(n_params);
        }
        values
    }
}

fn prepare_schedule_statement<'a>(
    row: &FilingRow,
    schedule_type: &ScheduleType,
    tx: &'a Transaction,
) -> anyhow::Result<ItemizationValue<'a>> {
    let record_table: RecordTable =
        RecordTable::new(&row.row_type, schedule_type.to_sqlite_tablename().as_str())?;
    tx.execute(&record_table.create_sql(), [])?;
    let insert_statement = record_table.insert_statement(tx)?;
    Ok(ItemizationValue {
        record_table,
        insert_statement,
    })
}

fn prepare_form_type_statement<'a>(
    row: &FilingRow,
    form_type: &str,
    tx: &'a Transaction,
) -> anyhow::Result<ItemizationValue<'a>> {
    let record_table: RecordTable = RecordTable::new(&row.row_type, form_type)?;
    tx.execute(&record_table.create_sql(), [])?;
    let insert_statement = record_table.insert_statement(tx)?;
    Ok(ItemizationValue {
        record_table,
        insert_statement,
    })
}

fn export_itemizations<R: Read>(
    tx: &mut Transaction,
    mut filing: Filing<R>,
    pb: Option<&ItemizationProgressBar>,
) -> anyhow::Result<usize> {
    let mut itemizations_statements: HashMap<ItemizationKey, ItemizationValue> = HashMap::new();
    let mut count = 0;

    while let Some(r) = filing.next_row() {
        let r = r.context("Error reading next row from filing")?;
        if let Some(pb) = pb {
            pb.update(&r)
        }
        count += 1;

        let key = match form_type_schedule_type(&r.row_type) {
            Some(schedule_type) => ItemizationKey::Schedule(schedule_type),
            None => ItemizationKey::FormType(r.row_type.clone()),
        };

        // We need to handle the or_insert_with error case, but HashMap::Entry doesn't
        // support fallible closures directly. So we check if we need to insert first.
        if !itemizations_statements.contains_key(&key) {
            let value = match &key {
                ItemizationKey::Schedule(schedule_type) => {
                    prepare_schedule_statement(&r, schedule_type, tx).with_context(|| {
                        format!(
                            "Error preparing statement for schedule type {:?}",
                            schedule_type
                        )
                    })?
                }
                ItemizationKey::FormType(form_type) => {
                    prepare_form_type_statement(&r, form_type, tx).with_context(|| {
                        format!("Error preparing statement for form type {}", form_type)
                    })?
                }
            };
            itemizations_statements.insert(key.clone(), value);
        }

        let v = itemizations_statements.get_mut(&key).unwrap(); // Safe: we just inserted it above
        v.record_table
            .insert(
                &mut v.insert_statement,
                &filing.filing_id,
                &r.record,
                &filing.header.fec_version,
            )
            .with_context(|| {
                format!(
                    "Error inserting itemization row for filing FEC-{}, line {}",
                    &filing.filing_id,
                    r.record.position().map(|p| p.line()).unwrap_or(0),
                )
            })?;
    }
    Ok(count)
}

pub(crate) fn form_type_parse(form_type: &str) -> (&str, Option<&str>) {
    if let Some(stripped) = form_type.strip_suffix('A') {
        (stripped, Some("A"))
    } else if let Some(stripped) = form_type.strip_suffix('N') {
        (stripped, Some("N"))
    } else if let Some(stripped) = form_type.strip_suffix('T') {
        (stripped, Some("T"))
    } else {
        (form_type, None)
    }
}

// insert row into libfec_filings and the summary record table
fn insert_filing_metadata(tx: &mut Transaction, filing: &Filing<impl Read>) -> anyhow::Result<()> {
    let (form_type, amendment_indicator) = form_type_parse(&filing.cover.form_type);
    tx.execute(
        INSERT_FILING_SQL,
        rusqlite::params![
            &filing.filing_id,
            &filing.header.fec_version,
            &filing.header.software_name,
            &filing.header.software_version,
            &filing.header.report_id,
            &filing.header.report_number,
            &filing.header.comment,
            form_type,
            amendment_indicator,
            &filing.cover.filer_id,
            &filing.cover.filer_name,
            &filing.cover.report_code.clone(),
            &filing.cover.coverage_from_date.clone(),
            &filing.cover.coverage_through_date.clone(),
        ],
    )?;

    let (form_type, _amendment_indicator) = form_type_parse(&filing.cover.form_type);

    let mut rt = RecordTable::new(
        &filing.cover.form_type,
        // strip any lagging "A", "N", or "T", or return form_type
        form_type,
    )
    .with_context(|| {
        format!(
            "Error creating record table for form type '{}' and filing FEC-{}",
            &filing.cover.form_type, &filing.filing_id
        )
    })?;
    tx.execute(&rt.create_sql(), [])?;
    let mut stmt = rt.insert_statement(tx)?;
    rt.insert(
        &mut stmt,
        &filing.filing_id,
        &filing.cover.record,
        &filing.header.fec_version,
    )?;
    Ok(())
}

fn init(tx: &mut Transaction) -> anyhow::Result<()> {
    tx.execute(CREATE_FILINGS_SQL, [])
        .context("Error initializing filing schema")?;
    Ok(())
}

pub fn cmd_export_sqlite(
    mut sourcer: FilingSourcer,
    path: PathBuf,
    args: ExportArgs,
) -> anyhow::Result<()> {
    let t0 = Instant::now();

    let mut db = crate::cache::open_connection(&path).context(format!(
        "Could not open or create export database at {:?}",
        path
    ))?;
    let mb = MultiProgress::new();

    let mut tx = db
        .transaction()
        .context("Error starting SQLite transaction")?;

    init(&mut tx)?;

    let p = sourcer.cache.bulk_data_database_path();
    let (trace, input_mappings, iter) =
        sourcer.resolve_iterator_from_flags(args.filings, args.api.clone(), Some(&mb))?;

    // Initialize metadata tracking if enabled
    let metadata_export_id = if args.write_metadata {
        // Need to commit the transaction to use metadata functions that require &Connection
        tx.commit()
            .context("Error committing initial transaction")?;

        init_metadata_schema(&db)?;
        let export_uuid = format!("export-{}", uuid::Uuid::new_v4());
        let metadata = create_export(&db, &export_uuid, args.cover_only)?;

        // Record input mappings
        for mapping in &input_mappings {
            if let Ok(input_id) = record_export_input(
                &db,
                metadata.export_id,
                &mapping.input_type,
                &mapping.raw_input,
            ) {
                for filing_id in &mapping.direct_filings {
                    let _ = link_input_to_filing(&db, input_id, filing_id);
                }
            }
        }

        // Start a new transaction for the actual export
        tx = db
            .transaction()
            .context("Error starting export transaction")?;
        Some(metadata.export_id)
    } else {
        None
    };

    // Collect cycles and sync committee bulk data before ATTACHing bulk_db to the export transaction.
    // We open the bulk DB directly via `p` to avoid borrow conflicts with `sourcer` (held by `iter`).
    let committee_cycles: Vec<u16> = if !args.include_all_bulk {
        let mut cycles: Vec<u16> = args.api.cycle.clone().unwrap_or_default();
        for params in &trace.resolve_candidate_params {
            if !cycles.contains(&params.cycle) {
                cycles.push(params.cycle);
            }
        }
        // When committee IDs are specified but no cycle is available, default to the current
        // election cycle so bulk committee data gets synced and included.
        if !trace.committee_ids.is_empty() && cycles.is_empty() {
            let year = jiff::Zoned::now().year() as u16;
            let election_cycle = if year.is_multiple_of(2) {
                year
            } else {
                year + 1
            };
            cycles.push(election_cycle);
        }
        {
            let mut bulk_db = crate::cache::open_connection(&p)
                .with_context(|| format!("Could not open bulk database at {:?}", p))?;
            for &cycle in &cycles {
                let mut bulk_tx = bulk_db.transaction()?;
                committee::export(&mut bulk_tx, cycle, None, false)?;
                bulk_tx.commit()?;
            }
        }
        cycles
    } else {
        vec![]
    };

    if args.include_all_bulk {
        // Include ALL bulk data for the specified cycle(s)
        let cycles = args.api.cycle.clone().unwrap_or_default();
        for cycle in cycles {
            let params = candidates::ResolveCandidateParams {
                cycle,
                office: None,
                state: None,
                district: None,
            };
            candidates::include(&mut tx, p.clone(), &params)
                .with_context(|| format!("Error including candidates for cycle {}", cycle))?;
            committee::include(&mut tx, p.clone(), cycle)
                .with_context(|| format!("Error including committees for cycle {}", cycle))?;
        }
    } else {
        for params in &trace.resolve_candidate_params {
            candidates::include(&mut tx, p.clone(), params).unwrap();
        }
        // Include libfec_committee rows for directly-specified committee IDs
        if !trace.committee_ids.is_empty() {
            let id_strs: Vec<&str> = trace.committee_ids.iter().map(|c| c.as_str()).collect();
            for &cycle in &committee_cycles {
                committee::include_specific(&mut tx, p.clone(), cycle, &id_strs).with_context(
                    || format!("Error including specific committees for cycle {}", cycle),
                )?;
            }
        }
    }

    let mut nfilings = 0;
    let mut skipped_existing = 0usize;

    mb.println(format!(
        "Exporting filings to SQLite database at {:?}",
        path
    ))?;

    for filing in iter {
        let filing = match filing {
            Ok(f) => f,
            Err(e) => {
                if let Some(fec_403) = e.downcast_ref::<crate::sourcer::FecGov403Error>() {
                    eprintln!(
                        "Error fetching filing {} from {}: HTTP 403 Forbidden. This filing may no longer be publicly accessible.",
                        fec_403.filing_id.to_human_readable(), fec_403.url
                    );
                    // TODO save warning somewhere
                    continue;
                } else {
                    let _ = mb.println(format!("Error fetching filing: {:?}", e));
                    todo!();
                }
            }
        };
        let filing_id_str = filing.filing_id.clone();
        nfilings += 1;
        match insert_filing_metadata(&mut tx, &filing) {
            Ok(_) => {
                // Record successful filing in metadata if enabled
                if let Some(export_id) = metadata_export_id {
                    // Commit current work and record filing
                    tx.commit()
                        .context("Error committing transaction for metadata")?;
                    let _ = record_export_filing(&db, export_id, &filing_id_str, true, None);
                    tx = db.transaction().context("Error restarting transaction")?;
                }
            }
            Err(e) => {
                // Check if this is a UNIQUE constraint violation (filing already in DB)
                let is_duplicate = e.chain().any(|cause| {
                    if let Some(rusqlite::Error::SqliteFailure(err, _)) =
                        cause.downcast_ref::<rusqlite::Error>()
                    {
                        err.code == rusqlite::ErrorCode::ConstraintViolation
                    } else {
                        false
                    }
                });

                if is_duplicate {
                    skipped_existing += 1;
                } else {
                    let _ = mb.println(format!(
                        "Error inserting filing metadata for FEC-{}: {:?}",
                        filing.filing_id, e
                    ));
                }
                // Record failed filing in metadata if enabled
                if let Some(export_id) = metadata_export_id {
                    tx.commit()
                        .context("Error committing transaction for metadata")?;
                    let _ = record_export_filing(
                        &db,
                        export_id,
                        &filing_id_str,
                        false,
                        Some(&e.to_string()),
                    );
                    tx = db.transaction().context("Error restarting transaction")?;
                }
                continue;
            }
        }

        if !args.cover_only {
            let pb = ItemizationProgressBar::new(&mb, &filing);
            let filing_id = filing.filing_id.clone();
            export_itemizations(&mut tx, filing, Some(&pb))
                .with_context(|| format!("Error exporting itemizations for FEC-{}", filing_id))?;
        }
    }

    // Include committee records matching exported filings (unless --include-all-bulk already did it)
    for &cycle in &committee_cycles {
        committee::include_from_filings(&mut tx, p.clone(), cycle)?;
    }

    tx.commit().context("Error committing SQLite transaction")?;

    // Finalize metadata if enabled
    if let Some(export_id) = metadata_export_id {
        finalize_export(&db, export_id, "complete", nfilings, None)?;
        println!("Export metadata recorded with export_id: {}", export_id);
    }

    let elapsed = Instant::now() - t0;
    let exported = nfilings - skipped_existing;
    if skipped_existing > 0 {
        println!(
            "Finished exporting {} filings into {} ({} already in DB, skipped), in {}",
            exported,
            path.to_string_lossy().bold(),
            skipped_existing,
            HumanDuration(elapsed)
        );
    } else {
        println!(
            "Finished exporting {} filings into {}, in {}",
            exported,
            path.to_string_lossy().bold(),
            HumanDuration(elapsed)
        );
    }
    Ok(())
}

/// Initialize the SQLite database schema for filings
pub fn init_schema(db: &mut Connection) -> anyhow::Result<()> {
    let mut tx = db
        .transaction()
        .context("Error starting SQLite transaction")?;
    init(&mut tx)?;
    tx.commit()?;
    Ok(())
}

/// Get a set of filing IDs that already exist in the database
pub fn get_existing_filing_ids(
    db: &Connection,
) -> anyhow::Result<std::collections::HashSet<String>> {
    let mut stmt = db.prepare("SELECT filing_id FROM libfec_filings")?;
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(ids)
}

/// Export a single filing to the SQLite database
pub fn export_single_filing<R: Read>(
    db: &mut Connection,
    filing: Filing<R>,
    cover_only: bool,
) -> anyhow::Result<()> {
    let mut tx = db
        .transaction()
        .context("Error starting SQLite transaction")?;
    init(&mut tx)?;

    insert_filing_metadata(&mut tx, &filing)?;

    if !cover_only {
        export_itemizations(&mut tx, filing, None)?;
    }

    tx.commit().context("Error committing SQLite transaction")?;
    Ok(())
}

// ============================================================================
// Export Metadata Tracking
// ============================================================================

const CREATE_EXPORTS_SQL: &str = r#"
  CREATE TABLE IF NOT EXISTS libfec_exports(
    --- Auto-incrementing unique identifier for this export operation
    export_id INTEGER PRIMARY KEY AUTOINCREMENT,

    --- UUID string identifier (for RPC mode compatibility)
    export_uuid TEXT NOT NULL,

    --- Timestamp when the export was created (ISO 8601)
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    --- Number of filings exported
    filings_count INTEGER NOT NULL DEFAULT 0,

    --- Whether only cover records were exported (not itemizations)
    cover_only INTEGER NOT NULL DEFAULT 0,

    --- Status of the export: 'started', 'complete', 'error', 'canceled'
    status TEXT NOT NULL DEFAULT 'started',

    --- Error message if status is 'error'
    error_message TEXT
  )
"#;

const CREATE_EXPORT_FILINGS_SQL: &str = r#"
  CREATE TABLE IF NOT EXISTS libfec_export_filings(
    --- Reference to the export operation
    export_id INTEGER NOT NULL REFERENCES libfec_exports(export_id),

    --- The filing ID that was exported
    filing_id TEXT NOT NULL,

    --- Whether this filing was successfully exported
    success INTEGER NOT NULL DEFAULT 1,

    --- Warning or error message for this filing
    message TEXT,

    PRIMARY KEY (export_id, filing_id)
  )
"#;

const CREATE_EXPORT_INPUTS_SQL: &str = r#"
  CREATE TABLE IF NOT EXISTS libfec_export_inputs(
    --- Auto-incrementing unique identifier
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    --- Reference to the export operation
    export_id INTEGER NOT NULL REFERENCES libfec_exports(export_id),

    --- Type of input: 'filing', 'committee', 'candidate', 'contest', 'file', 'url'
    input_type TEXT NOT NULL,

    --- The raw input value as provided by the user
    input_value TEXT NOT NULL,

    --- For contests: the election cycle
    cycle INTEGER,

    --- For contests: the office (president, senate, house)
    office TEXT,

    --- For contests: the state code
    state TEXT,

    --- For contests: the district number
    district TEXT
  )
"#;

const CREATE_EXPORT_INPUT_FILINGS_SQL: &str = r#"
  CREATE TABLE IF NOT EXISTS libfec_export_input_filings(
    --- Reference to the export input
    input_id INTEGER NOT NULL REFERENCES libfec_export_inputs(id),

    --- The filing ID that this input resolved to
    filing_id TEXT NOT NULL,

    PRIMARY KEY (input_id, filing_id)
  )
"#;

/// Initialize the export metadata schema
pub fn init_metadata_schema(db: &Connection) -> anyhow::Result<()> {
    db.execute(CREATE_EXPORTS_SQL, [])
        .context("Error creating libfec_exports table")?;
    db.execute(CREATE_EXPORT_FILINGS_SQL, [])
        .context("Error creating libfec_export_filings table")?;
    db.execute(CREATE_EXPORT_INPUTS_SQL, [])
        .context("Error creating libfec_export_inputs table")?;
    db.execute(CREATE_EXPORT_INPUT_FILINGS_SQL, [])
        .context("Error creating libfec_export_input_filings table")?;
    Ok(())
}

/// Metadata for an export operation
#[derive(Debug, Clone)]
pub struct ExportMetadata {
    /// Auto-incrementing database ID
    pub export_id: i64,
    /// UUID string (for RPC compatibility)
    #[allow(dead_code)]
    pub export_uuid: String,
}

// Re-export InputType from sourcer for use in metadata recording
pub use crate::sourcer::InputType;

impl InputType {
    /// Get the type name for database storage
    pub fn type_name(&self) -> &'static str {
        match self {
            InputType::Filing => "filing",
            InputType::Committee => "committee",
            InputType::Candidate => "candidate",
            InputType::Contest { .. } => "contest",
            InputType::InputFile => "file",
            InputType::Url => "url",
        }
    }
}

/// Create a new export record and return its metadata
pub fn create_export(
    db: &Connection,
    export_uuid: &str,
    cover_only: bool,
) -> anyhow::Result<ExportMetadata> {
    db.execute(
        "INSERT INTO libfec_exports (export_uuid, cover_only, status) VALUES (?, ?, 'started')",
        rusqlite::params![export_uuid, cover_only as i32],
    )
    .context("Error creating export record")?;

    let export_id = db.last_insert_rowid();

    Ok(ExportMetadata {
        export_id,
        export_uuid: export_uuid.to_string(),
    })
}

/// Record that a filing was exported as part of an export operation
pub fn record_export_filing(
    db: &Connection,
    export_id: i64,
    filing_id: &str,
    success: bool,
    message: Option<&str>,
) -> anyhow::Result<()> {
    db.execute(
        "INSERT OR REPLACE INTO libfec_export_filings (export_id, filing_id, success, message) VALUES (?, ?, ?, ?)",
        rusqlite::params![export_id, filing_id, success as i32, message],
    )
    .context("Error recording export filing")?;
    Ok(())
}

/// Record an input that was used in the export
/// Returns the input ID for linking to filings
pub fn record_export_input(
    db: &Connection,
    export_id: i64,
    input_type: &InputType,
    input_value: &str,
) -> anyhow::Result<i64> {
    match input_type {
        InputType::Contest {
            cycle,
            office,
            state,
            district,
        } => {
            db.execute(
                "INSERT INTO libfec_export_inputs (export_id, input_type, input_value, cycle, office, state, district) VALUES (?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    export_id,
                    input_type.type_name(),
                    input_value,
                    cycle,
                    office,
                    state,
                    district
                ],
            )?;
        }
        _ => {
            db.execute(
                "INSERT INTO libfec_export_inputs (export_id, input_type, input_value) VALUES (?, ?, ?)",
                rusqlite::params![export_id, input_type.type_name(), input_value],
            )?;
        }
    }
    Ok(db.last_insert_rowid())
}

/// Link a filing ID to an input
pub fn link_input_to_filing(db: &Connection, input_id: i64, filing_id: &str) -> anyhow::Result<()> {
    db.execute(
        "INSERT OR IGNORE INTO libfec_export_input_filings (input_id, filing_id) VALUES (?, ?)",
        rusqlite::params![input_id, filing_id],
    )?;
    Ok(())
}

/// Update the export status and filings count
pub fn finalize_export(
    db: &Connection,
    export_id: i64,
    status: &str,
    filings_count: usize,
    error_message: Option<&str>,
) -> anyhow::Result<()> {
    db.execute(
        "UPDATE libfec_exports SET status = ?, filings_count = ?, error_message = ? WHERE export_id = ?",
        rusqlite::params![status, filings_count as i64, error_message, export_id],
    )?;
    Ok(())
}

// ============================================================================
// RSS Sync Metadata Tracking
// ============================================================================

const CREATE_RSS_SYNCS_SQL: &str = r#"
  CREATE TABLE IF NOT EXISTS libfec_rss_syncs(
    --- Auto-incrementing unique identifier for this RSS sync operation
    sync_id INTEGER PRIMARY KEY AUTOINCREMENT,

    --- UUID string identifier (for RPC mode compatibility)
    sync_uuid TEXT NOT NULL,

    --- Timestamp when the sync was started (ISO 8601)
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    --- Timestamp when the sync completed (ISO 8601)
    completed_at TEXT,

    --- The RSS feed URL used for this sync
    feed_url TEXT,

    --- HTTP Last-Modified header value from the feed
    feed_last_modified TEXT,

    --- Title of the RSS feed
    feed_title TEXT,

    --- Filters used for this sync
    since_filter TEXT,
    preset_filter TEXT,
    form_type_filter TEXT,
    committee_filter TEXT,
    state_filter TEXT,
    party_filter TEXT,

    --- Counts
    total_feed_items INTEGER,
    filtered_items INTEGER,
    new_filings_count INTEGER NOT NULL DEFAULT 0,
    exported_count INTEGER NOT NULL DEFAULT 0,

    --- Whether only cover records were exported
    cover_only INTEGER NOT NULL DEFAULT 0,

    --- Status of the sync: 'started', 'complete', 'error', 'canceled'
    status TEXT NOT NULL DEFAULT 'started',

    --- Error message if status is 'error'
    error_message TEXT
  )
"#;

const CREATE_RSS_FILINGS_SQL: &str = r#"
  CREATE TABLE IF NOT EXISTS libfec_rss_filings(
    --- Reference to the RSS sync operation
    sync_id INTEGER NOT NULL REFERENCES libfec_rss_syncs(sync_id),

    --- The filing ID that was exported
    filing_id TEXT NOT NULL,

    --- Publication date from RSS feed (ISO 8601)
    rss_pub_date TEXT,

    --- Timestamp when we ingested/processed this item (ISO 8601)
    ingested_at TEXT NOT NULL DEFAULT (datetime('now')),

    --- RSS item metadata
    rss_guid TEXT,
    rss_title TEXT,
    committee_id TEXT,
    form_type TEXT,
    coverage_from TEXT,
    coverage_through TEXT,
    report_type TEXT,

    --- Whether this filing was successfully exported
    export_success INTEGER NOT NULL DEFAULT 1,

    --- Warning or error message for this filing
    export_message TEXT,

    PRIMARY KEY (sync_id, filing_id)
  )
"#;

const CREATE_RSS_FILINGS_INDEX_SQL: &str = r#"
  CREATE INDEX IF NOT EXISTS idx_rss_filings_filing_id
  ON libfec_rss_filings(filing_id)
"#;

/// Initialize the RSS sync metadata schema
pub fn init_rss_metadata_schema(db: &Connection) -> anyhow::Result<()> {
    db.execute(CREATE_RSS_SYNCS_SQL, [])
        .context("Error creating libfec_rss_syncs table")?;
    db.execute(CREATE_RSS_FILINGS_SQL, [])
        .context("Error creating libfec_rss_filings table")?;
    db.execute(CREATE_RSS_FILINGS_INDEX_SQL, [])
        .context("Error creating libfec_rss_filings index")?;
    Ok(())
}

/// Metadata for an RSS sync operation
#[derive(Debug, Clone)]
pub struct RssSyncMetadata {
    /// Auto-incrementing database ID
    pub sync_id: i64,
    /// UUID string (for RPC compatibility)
    #[allow(dead_code)]
    pub sync_uuid: String,
}

/// Parameters for creating an RSS sync record
#[derive(Debug, Default)]
pub struct RssSyncParams {
    pub feed_url: Option<String>,
    pub feed_last_modified: Option<String>,
    pub feed_title: Option<String>,
    pub since_filter: Option<String>,
    pub preset_filter: Option<String>,
    pub form_type_filter: Option<String>,
    pub committee_filter: Option<String>,
    pub state_filter: Option<String>,
    pub party_filter: Option<String>,
    pub cover_only: bool,
}

/// Create a new RSS sync record and return its metadata
pub fn create_rss_sync(
    db: &Connection,
    sync_uuid: &str,
    params: &RssSyncParams,
) -> anyhow::Result<RssSyncMetadata> {
    db.execute(
        r#"INSERT INTO libfec_rss_syncs (
            sync_uuid, feed_url, feed_last_modified, feed_title,
            since_filter, preset_filter, form_type_filter, committee_filter,
            state_filter, party_filter, cover_only, status
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'started')"#,
        rusqlite::params![
            sync_uuid,
            params.feed_url,
            params.feed_last_modified,
            params.feed_title,
            params.since_filter,
            params.preset_filter,
            params.form_type_filter,
            params.committee_filter,
            params.state_filter,
            params.party_filter,
            params.cover_only as i32,
        ],
    )
    .context("Error creating RSS sync record")?;

    let sync_id = db.last_insert_rowid();

    Ok(RssSyncMetadata {
        sync_id,
        sync_uuid: sync_uuid.to_string(),
    })
}

/// Parameters for recording an RSS filing
#[derive(Debug, Default)]
pub struct RssFilingParams {
    pub rss_pub_date: Option<String>,
    pub rss_guid: Option<String>,
    pub rss_title: Option<String>,
    pub committee_id: Option<String>,
    pub form_type: Option<String>,
    pub coverage_from: Option<String>,
    pub coverage_through: Option<String>,
    pub report_type: Option<String>,
}

/// Record that a filing was exported as part of an RSS sync operation
pub fn record_rss_filing(
    db: &Connection,
    sync_id: i64,
    filing_id: &str,
    params: &RssFilingParams,
    success: bool,
    message: Option<&str>,
) -> anyhow::Result<()> {
    db.execute(
        r#"INSERT OR REPLACE INTO libfec_rss_filings (
            sync_id, filing_id, rss_pub_date, rss_guid, rss_title,
            committee_id, form_type, coverage_from, coverage_through,
            report_type, export_success, export_message
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        rusqlite::params![
            sync_id,
            filing_id,
            params.rss_pub_date,
            params.rss_guid,
            params.rss_title,
            params.committee_id,
            params.form_type,
            params.coverage_from,
            params.coverage_through,
            params.report_type,
            success as i32,
            message,
        ],
    )
    .context("Error recording RSS filing")?;
    Ok(())
}

/// Update the RSS sync status and counts
#[allow(clippy::too_many_arguments)]
pub fn finalize_rss_sync(
    db: &Connection,
    sync_id: i64,
    status: &str,
    total_feed_items: Option<usize>,
    filtered_items: Option<usize>,
    new_filings_count: usize,
    exported_count: usize,
    error_message: Option<&str>,
) -> anyhow::Result<()> {
    db.execute(
        r#"UPDATE libfec_rss_syncs SET
            status = ?,
            completed_at = datetime('now'),
            total_feed_items = ?,
            filtered_items = ?,
            new_filings_count = ?,
            exported_count = ?,
            error_message = ?
        WHERE sync_id = ?"#,
        rusqlite::params![
            status,
            total_feed_items.map(|n| n as i64),
            filtered_items.map(|n| n as i64),
            new_filings_count as i64,
            exported_count as i64,
            error_message,
            sync_id,
        ],
    )?;
    Ok(())
}

/// All known (row_type, table_suffix) pairs for creating the full schema.
/// row_type is a representative form type string that matches the mapping regex.
/// table_suffix is what appears after "libfec_" in the table name.
pub(crate) const ALL_TABLES: &[(&str, &str)] = &[
    // Schedules
    ("SA11", "schedule_a"),
    ("SB21", "schedule_b"),
    ("SC10", "schedule_c"),
    ("SC1", "schedule_c1"),
    ("SC2", "schedule_c2"),
    ("SD9", "schedule_d"),
    ("SE", "schedule_e"),
    ("SF", "schedule_f"),
    // Cover/form records
    ("F1N", "F1N"),
    ("F1M", "F1M"),
    ("F1S", "F1S"),
    ("F2", "F2"),
    ("F24", "F24"),
    ("F3N", "F3N"),
    ("F3LN", "F3LN"),
    ("F3P", "F3P"),
    ("F3PS", "F3PS"),
    ("F3PZ1", "F3PZ1"),
    ("F3PZ2", "F3PZ2"),
    ("F3P31", "F3P31"),
    ("F3S", "F3S"),
    ("F3X", "F3X"),
    ("F3Z", "F3Z"),
    ("F3Z1", "F3Z1"),
    ("F3Z2", "F3Z2"),
    ("F4N", "F4N"),
    ("F5N", "F5N"),
    ("F56", "F56"),
    ("F57", "F57"),
    ("F6", "F6"),
    ("F65", "F65"),
    ("F7N", "F7N"),
    ("F76", "F76"),
    ("F8", "F8"),
    ("F8II", "F8II"),
    ("F8III", "F8III"),
    ("F9", "F9"),
    ("F91", "F91"),
    ("F92", "F92"),
    ("F93", "F93"),
    ("F94", "F94"),
    ("F99", "F99"),
    ("F10", "F10"),
    ("F105", "F105"),
    ("F13N", "F13N"),
    ("F132", "F132"),
    ("F133", "F133"),
    // H schedules
    ("H1", "H1"),
    ("H2", "H2"),
    ("H3", "H3"),
    ("H4", "H4"),
    ("H5", "H5"),
    ("H6", "H6"),
    // Misc
    ("SA3L", "SA3L"),
    ("SI", "SI"),
    ("SL", "SL"),
    ("TEXT", "TEXT"),
];

pub fn cmd_schemaize(path: PathBuf) -> anyhow::Result<()> {
    let mut db = crate::cache::open_connection(&path)
        .context(format!("Could not open or create database at {:?}", path))?;
    let tx = db
        .transaction()
        .context("Error starting SQLite transaction")?;

    tx.execute(CREATE_FILINGS_SQL, [])
        .context("Error creating libfec_filings table")?;

    let mut created = 0;
    for &(row_type, suffix) in ALL_TABLES {
        match RecordTable::new(row_type, suffix) {
            Ok(record_table) => {
                tx.execute(&record_table.create_sql(), [])?;
                created += 1;
            }
            Err(e) => {
                eprintln!("warning: skipping table libfec_{suffix}: {e}");
            }
        }
    }

    tx.commit().context("Error committing transaction")?;
    eprintln!("Created {created} tables in {}", path.to_string_lossy());
    Ok(())
}

#[cfg(test)]
mod test;
