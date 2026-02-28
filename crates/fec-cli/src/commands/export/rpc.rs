/*!
 * JSON-RPC Mode for Export Command
 *
 * Implements a JSON-RPC 2.0 server over stdio using JSONL protocol.
 * External processes can spawn `libfec export --rpc -o output.db` and interact via newline-delimited JSON.
 *
 * ## Protocol
 *
 * - Request: `{"jsonrpc":"2.0","id":1,"method":"export/start","params":{...}}`
 * - Response: `{"jsonrpc":"2.0","id":1,"result":{...}}`
 * - Notification: `{"jsonrpc":"2.0","method":"export/progress","params":{...}}`
 *
 * ## Methods
 *
 * - `export/start`: Start export with parameters (filings, cycle, cover_only, clobber)
 * - `export/status`: Get current export status and progress
 * - `export/cancel`: Cancel active export
 * - `shutdown`: Gracefully shutdown RPC server
 */

use crate::cache::bulk::{candidate_committee_linkage, candidates, committee};
use crate::cli::ExportArgs;
use crate::sourcer::{process_inputs, FilingSourcer, Item};
use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use super::sqlite;

/// JSON-RPC 2.0 request
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

/// JSON-RPC 2.0 response with result
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 notification (no id)
#[derive(Debug, Serialize)]
struct JsonRpcNotification {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
}

/// Export phases
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum ExportPhase {
    Idle,
    Sourcing,
    DownloadingBulk,
    Exporting,
    Complete,
    Canceled,
    Error,
}

/// Parameters for export/start method
#[derive(Debug, Deserialize)]
struct ExportStartParams {
    /// Optional filings to export (overrides CLI args)
    #[serde(default)]
    filings: Option<Vec<String>>,
    /// Optional cycle year
    #[serde(default)]
    cycle: Option<u16>,
    /// Only export cover records
    #[serde(default)]
    cover_only: Option<bool>,
    /// Overwrite existing database
    #[serde(default)]
    clobber: Option<bool>,
    /// Write export metadata to the database
    #[serde(default)]
    write_metadata: Option<bool>,
    /// Include all bulk data (candidates, committees, linkages) for the specified cycle
    #[serde(default)]
    include_all_bulk: Option<bool>,
}

/// Bulk data type for download tracking
#[derive(Debug, Clone, Copy)]
enum BulkDataType {
    Candidates,
    Committees,
    Linkages,
}

impl BulkDataType {
    fn display_name(&self, cycle: u16) -> String {
        match self {
            BulkDataType::Candidates => format!("{} candidates", cycle),
            BulkDataType::Committees => format!("{} committees", cycle),
            BulkDataType::Linkages => format!("{} linkages", cycle),
        }
    }
}

/// Bulk download task
struct BulkTask {
    cycle: u16,
    data_type: BulkDataType,
    completed: bool,
}

/// Export state tracking
struct ExportState {
    export_id: String,
    phase: ExportPhase,

    // Sourcing
    filing_ids: Vec<Item>,

    // Bulk downloads
    bulk_tasks: Vec<BulkTask>,
    bulk_completed: usize,

    // Export progress
    export_queue: Vec<Item>,
    export_total: usize,
    exported_count: usize,

    // Database
    export_db: Option<Connection>,
    cover_only: bool,

    // Results
    warnings: Vec<String>,
    error_message: Option<String>,

    // Current item being processed
    current_item: Option<String>,

    // Metadata tracking
    write_metadata: bool,
    /// Database autoincrement ID for this export (only set when write_metadata is true)
    metadata_export_id: Option<i64>,
    /// Original inputs provided by the user (for metadata tracking)
    original_inputs: Vec<String>,
}

impl ExportState {
    fn new(export_id: String, cover_only: bool, write_metadata: bool) -> Self {
        ExportState {
            export_id,
            phase: ExportPhase::Idle,
            filing_ids: Vec::new(),
            bulk_tasks: Vec::new(),
            bulk_completed: 0,
            export_queue: Vec::new(),
            export_total: 0,
            exported_count: 0,
            export_db: None,
            cover_only,
            warnings: Vec::new(),
            error_message: None,
            current_item: None,
            write_metadata,
            metadata_export_id: None,
            original_inputs: Vec::new(),
        }
    }

    /// Get status as JSON value
    fn to_status_json(&self) -> serde_json::Value {
        match &self.phase {
            ExportPhase::Idle => json!({
                "export_id": serde_json::Value::Null,
                "phase": "idle"
            }),
            ExportPhase::Sourcing => json!({
                "export_id": self.export_id,
                "phase": "sourcing"
            }),
            ExportPhase::DownloadingBulk => json!({
                "export_id": self.export_id,
                "phase": "downloading_bulk",
                "completed": self.bulk_completed,
                "total": self.bulk_tasks.len(),
                "current": self.current_item
            }),
            ExportPhase::Exporting => json!({
                "export_id": self.export_id,
                "phase": "exporting",
                "completed": self.exported_count,
                "total": self.export_total,
                "current_filing_id": self.current_item
            }),
            ExportPhase::Complete => {
                let mut result = json!({
                    "export_id": self.export_id,
                    "phase": "complete",
                    "total_exported": self.exported_count,
                    "warnings": self.warnings
                });
                // Include metadata_export_id when metadata tracking is enabled
                if let Some(metadata_id) = self.metadata_export_id {
                    result["metadata_export_id"] = json!(metadata_id);
                }
                result
            }
            ExportPhase::Canceled => json!({
                "export_id": self.export_id,
                "phase": "canceled"
            }),
            ExportPhase::Error => json!({
                "export_id": self.export_id,
                "phase": "error",
                "error_message": self.error_message
            }),
        }
    }
}

/// Main RPC mode entry point
pub fn run_rpc_mode(mut sourcer: FilingSourcer, args: ExportArgs) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdin_lines = stdin.lock().lines();
    let mut stdout_lock = stdout.lock();

    let mut state: Option<ExportState> = None;

    // Send ready notification
    send_notification(
        &mut stdout_lock,
        "ready",
        json!({ "version": env!("CARGO_PKG_VERSION") }),
    )?;

    loop {
        // Process work if in progress
        if let Some(ref mut export_state) = state {
            match export_state.phase {
                ExportPhase::DownloadingBulk => {
                    // Download one bulk file per iteration
                    if let Err(e) = process_bulk_download(export_state, &mut sourcer) {
                        export_state.phase = ExportPhase::Error;
                        export_state.error_message = Some(e.to_string());
                    }
                    send_progress_notification(&mut stdout_lock, export_state)?;
                }
                ExportPhase::Exporting => {
                    // Export up to 10 filings per iteration
                    match process_exports(export_state, &sourcer, 10) {
                        Ok(processed) => {
                            if processed > 0 {
                                send_progress_notification(&mut stdout_lock, export_state)?;
                            }
                        }
                        Err(e) => {
                            export_state.phase = ExportPhase::Error;
                            export_state.error_message = Some(e.to_string());
                            send_progress_notification(&mut stdout_lock, export_state)?;
                        }
                    }
                }
                _ => {}
            }
        }

        // Read next request
        match stdin_lines.next() {
            Some(Ok(line)) => {
                let (response, should_exit) =
                    handle_request(&line, &mut state, &mut sourcer, &args)?;
                send_response(&mut stdout_lock, response)?;
                if should_exit {
                    break;
                }
            }
            Some(Err(e)) => {
                return Err(anyhow::anyhow!("Stdin error: {}", e));
            }
            None => {
                // Stdin closed
                break;
            }
        }
    }

    Ok(())
}

/// Handle a single JSON-RPC request
fn handle_request(
    line: &str,
    state: &mut Option<ExportState>,
    sourcer: &mut FilingSourcer,
    base_args: &ExportArgs,
) -> Result<(JsonRpcResponse, bool)> {
    // Parse request
    let request: JsonRpcRequest = match serde_json::from_str(line) {
        Ok(req) => req,
        Err(e) => {
            return Ok((
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: serde_json::Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: "Parse error".to_string(),
                        data: Some(json!({ "details": e.to_string() })),
                    }),
                },
                false,
            ));
        }
    };

    let request_id = request.id.clone().unwrap_or(serde_json::Value::Null);

    // Dispatch method
    let result: Result<(serde_json::Value, bool), JsonRpcError> = match request.method.as_str() {
        "export/start" => {
            handle_export_start(request.params, state, sourcer, base_args).map(|r| (r, false))
        }
        "export/status" => handle_export_status(state).map(|r| (r, false)),
        "export/cancel" => handle_export_cancel(state).map(|r| (r, false)),
        "shutdown" => Ok((json!({ "ok": true }), true)),
        _ => Err(JsonRpcError {
            code: -32601,
            message: "Method not found".to_string(),
            data: Some(json!({ "method": request.method })),
        }),
    };

    match result {
        Ok((result_value, should_exit)) => Ok((
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request_id,
                result: Some(result_value),
                error: None,
            },
            should_exit,
        )),
        Err(error) => Ok((
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request_id,
                result: None,
                error: Some(error),
            },
            false,
        )),
    }
}

/// Handle export/start method
fn handle_export_start(
    params: serde_json::Value,
    state: &mut Option<ExportState>,
    sourcer: &mut FilingSourcer,
    base_args: &ExportArgs,
) -> Result<serde_json::Value, JsonRpcError> {
    // Check if export already in progress
    if let Some(ref export_state) = state {
        if !matches!(
            export_state.phase,
            ExportPhase::Complete | ExportPhase::Canceled | ExportPhase::Error | ExportPhase::Idle
        ) {
            return Err(JsonRpcError {
                code: -32001,
                message: "Export already in progress".to_string(),
                data: None,
            });
        }
    }

    // Parse parameters
    let export_params: ExportStartParams =
        serde_json::from_value(params).map_err(|e| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: Some(json!({ "details": e.to_string() })),
        })?;

    // Generate export ID (UUID for external reference)
    let export_id = format!("export-{}", uuid::Uuid::new_v4());

    // Determine cover_only setting
    let cover_only = export_params.cover_only.unwrap_or(base_args.cover_only);

    // Determine write_metadata setting
    let write_metadata = export_params
        .write_metadata
        .unwrap_or(base_args.write_metadata);

    // Create initial state
    let mut new_state = ExportState::new(export_id.clone(), cover_only, write_metadata);
    new_state.phase = ExportPhase::Sourcing;

    // Get output path
    let output_path = base_args.output.clone().ok_or_else(|| JsonRpcError {
        code: -32002,
        message: "No output path specified".to_string(),
        data: None,
    })?;

    // Handle clobber
    let clobber = export_params.clobber.unwrap_or(base_args.clobber);
    if clobber && output_path.exists() {
        std::fs::remove_file(&output_path).map_err(|e| JsonRpcError {
            code: -32002,
            message: "Failed to remove existing database".to_string(),
            data: Some(json!({ "details": e.to_string() })),
        })?;
    }

    // Open/create database
    let mut db = open_or_create_export_db(&output_path).map_err(|e| JsonRpcError {
        code: -32002,
        message: "Failed to open database".to_string(),
        data: Some(json!({ "details": e.to_string() })),
    })?;

    // Initialize schema
    sqlite::init_schema(&mut db).map_err(|e| JsonRpcError {
        code: -32002,
        message: "Failed to initialize database schema".to_string(),
        data: Some(json!({ "details": e.to_string() })),
    })?;

    // Initialize metadata schema if enabled
    if write_metadata {
        sqlite::init_metadata_schema(&db).map_err(|e| JsonRpcError {
            code: -32002,
            message: "Failed to initialize metadata schema".to_string(),
            data: Some(json!({ "details": e.to_string() })),
        })?;

        // Create export record
        let metadata =
            sqlite::create_export(&db, &export_id, cover_only).map_err(|e| JsonRpcError {
                code: -32002,
                message: "Failed to create export record".to_string(),
                data: Some(json!({ "details": e.to_string() })),
            })?;
        new_state.metadata_export_id = Some(metadata.export_id);
    }

    // Get existing filing IDs for incremental support
    let existing_ids = sqlite::get_existing_filing_ids(&db).unwrap_or_default();

    new_state.export_db = Some(db);

    // Build filings list from params or base_args
    let filings = export_params
        .filings
        .unwrap_or_else(|| base_args.filings.clone());

    // Store original inputs for metadata tracking
    new_state.original_inputs = filings.clone();

    // Build API flags with optional cycle override
    let mut api_flags = base_args.api.clone();
    if let Some(cycle) = export_params.cycle {
        api_flags.election = Some(cycle);
    }

    // Resolve filings using process_inputs
    let processed =
        process_inputs(&filings, api_flags.clone(), sourcer, None).map_err(|e| JsonRpcError {
            code: -32003,
            message: "Failed to resolve filings".to_string(),
            data: Some(json!({ "details": e.to_string() })),
        })?;

    // Store filing IDs
    new_state.filing_ids = processed.queue;

    // Record inputs and their mappings if metadata is enabled
    if write_metadata {
        if let Some(metadata_id) = new_state.metadata_export_id {
            let db = new_state.export_db.as_ref().unwrap();

            // Record each input using the mappings from process_inputs
            for mapping in &processed.input_mappings {
                if let Ok(input_id) = sqlite::record_export_input(
                    db,
                    metadata_id,
                    &mapping.input_type,
                    &mapping.raw_input,
                ) {
                    // Link directly resolved filings to this input
                    for filing_id in &mapping.direct_filings {
                        let _ = sqlite::link_input_to_filing(db, input_id, filing_id);
                    }
                }
            }
        }
    }

    // Determine if we should include all bulk data
    let include_all_bulk = export_params
        .include_all_bulk
        .unwrap_or(base_args.include_all_bulk);

    // Build bulk tasks
    if include_all_bulk {
        // Include all bulk data for the specified cycle(s)
        let cycles: Vec<u16> = if let Some(cycle) = export_params.cycle {
            vec![cycle]
        } else {
            api_flags.cycle.clone().unwrap_or_default()
        };

        for cycle in cycles {
            new_state.bulk_tasks.push(BulkTask {
                cycle,
                data_type: BulkDataType::Candidates,
                completed: false,
            });
            new_state.bulk_tasks.push(BulkTask {
                cycle,
                data_type: BulkDataType::Committees,
                completed: false,
            });
            new_state.bulk_tasks.push(BulkTask {
                cycle,
                data_type: BulkDataType::Linkages,
                completed: false,
            });
        }
    } else {
        // Build bulk tasks from trace (only for resolved candidates)
        for params in &processed.trace.resolve_candidate_params {
            new_state.bulk_tasks.push(BulkTask {
                cycle: params.cycle,
                data_type: BulkDataType::Candidates,
                completed: false,
            });
            new_state.bulk_tasks.push(BulkTask {
                cycle: params.cycle,
                data_type: BulkDataType::Committees,
                completed: false,
            });
            new_state.bulk_tasks.push(BulkTask {
                cycle: params.cycle,
                data_type: BulkDataType::Linkages,
                completed: false,
            });
        }
    }

    // Filter out already-exported IDs and prepare export queue
    new_state.export_queue = new_state
        .filing_ids
        .iter()
        .filter(|item| {
            let id = item_to_filing_id(item);
            !existing_ids.contains(&id)
        })
        .cloned()
        .collect();

    // Reverse queue so we pop from the end (oldest first)
    new_state.export_queue.reverse();
    new_state.export_total = new_state.export_queue.len();

    // Transition phase
    new_state.phase = if !new_state.bulk_tasks.is_empty() {
        ExportPhase::DownloadingBulk
    } else if !new_state.export_queue.is_empty() {
        ExportPhase::Exporting
    } else {
        ExportPhase::Complete
    };

    // Build response with optional metadata_export_id
    let metadata_export_id = new_state.metadata_export_id;

    *state = Some(new_state);

    let mut response = json!({
        "export_id": export_id,
        "status": "started"
    });
    if let Some(metadata_id) = metadata_export_id {
        response["metadata_export_id"] = json!(metadata_id);
    }
    Ok(response)
}

/// Handle export/status method
fn handle_export_status(state: &Option<ExportState>) -> Result<serde_json::Value, JsonRpcError> {
    if let Some(ref export_state) = state {
        Ok(export_state.to_status_json())
    } else {
        Ok(json!({
            "export_id": serde_json::Value::Null,
            "phase": "idle"
        }))
    }
}

/// Handle export/cancel method
fn handle_export_cancel(
    state: &mut Option<ExportState>,
) -> Result<serde_json::Value, JsonRpcError> {
    if let Some(ref mut export_state) = state {
        let export_id = export_state.export_id.clone();
        export_state.phase = ExportPhase::Canceled;
        export_state.export_queue.clear();
        Ok(json!({
            "canceled": true,
            "export_id": export_id
        }))
    } else {
        Err(JsonRpcError {
            code: -32000,
            message: "No export in progress".to_string(),
            data: None,
        })
    }
}

/// Process bulk data downloads
fn process_bulk_download(state: &mut ExportState, sourcer: &mut FilingSourcer) -> Result<()> {
    if let Some(task) = state.bulk_tasks.iter_mut().find(|t| !t.completed) {
        state.current_item = Some(task.data_type.display_name(task.cycle));

        let db = state
            .export_db
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No database connection"))?;

        // Get the bulk database path and open a connection
        let bulk_db_path = sourcer.cache.bulk_data_database_path();
        let mut bulk_db = sourcer.cache.open_bulk_data_database()?;

        // Create a transaction on the bulk database to sync the data
        let mut bulk_tx = bulk_db
            .transaction()
            .context("Could not start bulk database transaction")?;

        match task.data_type {
            BulkDataType::Candidates => {
                candidates::export(&mut bulk_tx, task.cycle, None)?;
            }
            BulkDataType::Committees => {
                committee::export(&mut bulk_tx, task.cycle, None)?;
            }
            BulkDataType::Linkages => {
                candidate_committee_linkage::export(&mut bulk_tx, task.cycle, None)?;
            }
        }

        bulk_tx.commit()?;

        // Now copy the data to the export database
        let tx = db
            .transaction()
            .context("Could not start export transaction")?;

        // Attach the bulk database to the export database
        let bulk_db_str = bulk_db_path.to_str().ok_or_else(|| {
            anyhow::anyhow!("Bulk database path is not valid UTF-8: {:?}", bulk_db_path)
        })?;

        if !tx
            .prepare_cached("select 1 from pragma_database_list where name = 'bulk_db'")?
            .exists([])?
        {
            tx.execute("ATTACH DATABASE ? AS bulk_db", [bulk_db_str])?;
        }

        // Copy relevant tables based on data type
        match task.data_type {
            BulkDataType::Candidates => {
                tx.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS libfec_candidates AS SELECT * FROM bulk_db.libfec_candidates WHERE 0;
                    INSERT OR REPLACE INTO libfec_candidates SELECT * FROM bulk_db.libfec_candidates WHERE cycle = ?1;
                    "#,
                )?;
                tx.execute(
                    "INSERT OR REPLACE INTO libfec_candidates SELECT * FROM bulk_db.libfec_candidates WHERE cycle = ?",
                    [task.cycle],
                ).ok(); // Ignore errors if table doesn't exist in bulk_db yet
            }
            BulkDataType::Committees => {
                tx.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS libfec_committees AS SELECT * FROM bulk_db.libfec_committees WHERE 0;
                    "#,
                ).ok();
                tx.execute(
                    "INSERT OR REPLACE INTO libfec_committees SELECT * FROM bulk_db.libfec_committees WHERE cycle = ?",
                    [task.cycle],
                ).ok();
            }
            BulkDataType::Linkages => {
                tx.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS libfec_candidate_committee_linkages AS SELECT * FROM bulk_db.libfec_candidate_committee_linkages WHERE 0;
                    "#,
                ).ok();
                tx.execute(
                    "INSERT OR REPLACE INTO libfec_candidate_committee_linkages SELECT * FROM bulk_db.libfec_candidate_committee_linkages WHERE cycle = ?",
                    [task.cycle],
                ).ok();
            }
        }

        tx.commit()?;

        task.completed = true;
        state.bulk_completed += 1;
    }

    // Check if all bulk done
    if state.bulk_tasks.iter().all(|t| t.completed) {
        state.current_item = None;
        state.phase = if state.export_queue.is_empty() {
            ExportPhase::Complete
        } else {
            ExportPhase::Exporting
        };
    }

    Ok(())
}

/// Process filing exports
fn process_exports(
    state: &mut ExportState,
    sourcer: &FilingSourcer,
    batch_size: usize,
) -> Result<usize> {
    let mut processed = 0;

    while !state.export_queue.is_empty() && processed < batch_size {
        let item = state.export_queue.pop().unwrap();
        let filing_id = item_to_filing_id(&item);
        state.current_item = Some(filing_id.clone());

        match resolve_item_to_filing(sourcer, &item) {
            Ok(filing) => {
                let db = state
                    .export_db
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("No database connection"))?;
                match sqlite::export_single_filing(db, filing, state.cover_only) {
                    Ok(_) => {
                        state.exported_count += 1;
                        // Record filing in metadata if enabled
                        if state.write_metadata {
                            if let Some(metadata_id) = state.metadata_export_id {
                                let _ = sqlite::record_export_filing(
                                    db,
                                    metadata_id,
                                    &filing_id,
                                    true,
                                    None,
                                );
                            }
                        }
                    }
                    Err(e) => {
                        // Record warning, continue
                        let warning = format!("Filing {}: {}", filing_id, e);
                        state.warnings.push(warning.clone());
                        // Record failed filing in metadata if enabled
                        if state.write_metadata {
                            if let Some(metadata_id) = state.metadata_export_id {
                                let db = state.export_db.as_ref().unwrap();
                                let _ = sqlite::record_export_filing(
                                    db,
                                    metadata_id,
                                    &filing_id,
                                    false,
                                    Some(&warning),
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                // Record warning, continue
                let warning = format!("Filing {}: {}", filing_id, e);
                state.warnings.push(warning.clone());
                // Record failed filing in metadata if enabled
                if state.write_metadata {
                    if let Some(metadata_id) = state.metadata_export_id {
                        if let Some(ref db) = state.export_db {
                            let _ = sqlite::record_export_filing(
                                db,
                                metadata_id,
                                &filing_id,
                                false,
                                Some(&warning),
                            );
                        }
                    }
                }
            }
        }
        processed += 1;
    }

    // Check completion
    if state.export_queue.is_empty() {
        state.phase = ExportPhase::Complete;
        state.current_item = None;

        // Finalize metadata if enabled
        if state.write_metadata {
            if let (Some(metadata_id), Some(ref db)) = (state.metadata_export_id, &state.export_db)
            {
                let _ = sqlite::finalize_export(
                    db,
                    metadata_id,
                    "complete",
                    state.exported_count,
                    None,
                );
            }
        }
    }

    Ok(processed)
}

/// Convert Item to a filing ID string
fn item_to_filing_id(item: &Item) -> String {
    match item {
        Item::CachedFile(path) => path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default(),
        Item::File(path) => path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default(),
        Item::CustomUrl(url) => url
            .path_segments()
            .and_then(|mut s| s.next_back())
            .map(|s| s.trim_end_matches(".fec").to_string())
            .unwrap_or_default(),
        Item::FilingId(id) => id.to_bare(),
    }
}

/// Resolve an Item to a Filing
fn resolve_item_to_filing(
    sourcer: &FilingSourcer,
    item: &Item,
) -> Result<fec_parser::Filing<Box<dyn std::io::Read>>> {
    match item {
        Item::CachedFile(path) | Item::File(path) => {
            crate::sourcer::resolve_from_path(path.clone())
        }
        Item::CustomUrl(_) | Item::FilingId(_) => {
            let id = item_to_filing_id(item);
            sourcer.resolve_from_user_argument(&id)
        }
    }
}

/// Open or create the export SQLite database
fn open_or_create_export_db(path: &PathBuf) -> Result<Connection> {
    Connection::open(path)
        .with_context(|| format!("Could not open or create database at {:?}", path))
}

/// Send JSON-RPC response to stdout
fn send_response(stdout: &mut dyn Write, response: JsonRpcResponse) -> Result<()> {
    let json = serde_json::to_string(&response).context("Failed to serialize response")?;
    writeln!(stdout, "{}", json)?;
    stdout.flush()?;
    Ok(())
}

/// Send JSON-RPC notification to stdout
fn send_notification(
    stdout: &mut dyn Write,
    method: &str,
    params: serde_json::Value,
) -> Result<()> {
    let notification = JsonRpcNotification {
        jsonrpc: "2.0".to_string(),
        method: method.to_string(),
        params,
    };
    let json = serde_json::to_string(&notification).context("Failed to serialize notification")?;
    writeln!(stdout, "{}", json)?;
    stdout.flush()?;
    Ok(())
}

/// Send progress notification for export state
fn send_progress_notification(stdout: &mut dyn Write, state: &ExportState) -> Result<()> {
    let params = match &state.phase {
        ExportPhase::Sourcing => json!({ "phase": "sourcing" }),
        ExportPhase::DownloadingBulk => json!({
            "phase": "downloading_bulk",
            "completed": state.bulk_completed,
            "total": state.bulk_tasks.len(),
            "current": state.current_item
        }),
        ExportPhase::Exporting => json!({
            "phase": "exporting",
            "completed": state.exported_count,
            "total": state.export_total,
            "current_filing_id": state.current_item
        }),
        ExportPhase::Complete => json!({
            "phase": "complete",
            "total_exported": state.exported_count,
            "warnings": state.warnings
        }),
        ExportPhase::Canceled => json!({ "phase": "canceled" }),
        ExportPhase::Error => json!({
            "phase": "error",
            "error_message": state.error_message
        }),
        _ => return Ok(()), // Don't send notification for idle
    };

    send_notification(stdout, "export/progress", params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_rpc_request() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"method":"export/start","params":{"filings":["1884420"]}}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "export/start");
    }

    #[test]
    fn test_parse_export_start_params() {
        let json = r#"{"filings":["1884420","1884421"],"cover_only":true,"cycle":2024}"#;
        let params: ExportStartParams = serde_json::from_str(json).unwrap();
        assert_eq!(
            params.filings,
            Some(vec!["1884420".to_string(), "1884421".to_string()])
        );
        assert_eq!(params.cover_only, Some(true));
        assert_eq!(params.cycle, Some(2024));
    }

    #[test]
    fn test_parse_export_start_params_minimal() {
        let json = r#"{}"#;
        let params: ExportStartParams = serde_json::from_str(json).unwrap();
        assert!(params.filings.is_none());
        assert!(params.cover_only.is_none());
        assert!(params.cycle.is_none());
        assert!(params.include_all_bulk.is_none());
    }

    #[test]
    fn test_parse_export_start_params_with_include_all_bulk() {
        let json = r#"{"cycle":2024,"include_all_bulk":true}"#;
        let params: ExportStartParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.cycle, Some(2024));
        assert_eq!(params.include_all_bulk, Some(true));
    }

    #[test]
    fn test_serialize_json_rpc_response() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: Some(json!({"export_id": "export-123", "status": "started"})),
            error: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("export-123"));
        assert!(!json.contains("error"));
    }

    #[test]
    fn test_serialize_json_rpc_error() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: None,
            error: Some(JsonRpcError {
                code: -32001,
                message: "Export already in progress".to_string(),
                data: None,
            }),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("-32001"));
        assert!(!json.contains("result"));
    }

    #[test]
    fn test_export_phase_serialization() {
        let phase = ExportPhase::DownloadingBulk;
        let json = serde_json::to_string(&phase).unwrap();
        assert_eq!(json, "\"downloading_bulk\"");

        let phase = ExportPhase::Exporting;
        let json = serde_json::to_string(&phase).unwrap();
        assert_eq!(json, "\"exporting\"");
    }

    #[test]
    fn test_export_state_status_json() {
        let mut state = ExportState::new("export-123".to_string(), false, false);
        state.phase = ExportPhase::Exporting;
        state.exported_count = 10;
        state.export_total = 100;
        state.current_item = Some("1884420".to_string());

        let status = state.to_status_json();
        assert_eq!(status["phase"], "exporting");
        assert_eq!(status["completed"], 10);
        assert_eq!(status["total"], 100);
    }
}
