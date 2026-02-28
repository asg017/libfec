/*!
 * JSON-RPC Mode for RSS Command
 *
 * Implements a JSON-RPC 2.0 server over stdio using JSONL protocol.
 * External processes can spawn `libfec rss --rpc` and interact via newline-delimited JSON.
 *
 * ## Protocol
 *
 * - Request: `{"jsonrpc":"2.0","id":1,"method":"sync/start","params":{...}}`
 * - Response: `{"jsonrpc":"2.0","id":1,"result":{...}}`
 * - Notification: `{"jsonrpc":"2.0","method":"sync/progress","params":{...}}`
 *
 * ## Methods
 *
 * - `sync/start`: Start RSS sync with parameters (export_path, cover_only, since, filters)
 * - `sync/status`: Get current sync status and progress
 * - `sync/cancel`: Cancel active sync
 * - `shutdown`: Gracefully shutdown RPC server
 */

use crate::cli::RssArgs;
use crate::commands::export::sqlite;
use crate::rss::{self, Item};
use crate::sourcer::FilingSourcer;
use anyhow::{Context, Result};
use jiff::Timestamp;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

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

/// Parameters for sync/start method
#[derive(Debug, Deserialize)]
struct SyncStartParams {
    export_path: PathBuf,
    #[serde(default)]
    cover_only: bool,
    #[serde(default)]
    since: Option<String>,
    #[serde(default)]
    preset: Option<String>,
    #[serde(default)]
    form_type: Option<String>,
    #[serde(default)]
    committee: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    party: Option<String>,
    #[serde(default)]
    write_metadata: bool,
}

/// RPC phase tracking
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
enum RpcPhase {
    #[allow(dead_code)]
    Idle,
    Fetching,
    Exporting,
    Complete,
    Canceled,
    Error,
}

/// RPC sync state
struct RpcSyncState {
    sync_id: String,
    phase: RpcPhase,
    feed_title: String,
    last_modified: Option<Timestamp>,
    total_items: usize,
    filtered_items: usize,
    export_queue: Vec<String>,
    export_batch_total: usize,
    exported_ids: HashSet<String>,
    export_db: Option<Connection>,
    cover_only: bool,
    latest_filing_id: Option<String>,
    latest_pub_date: Option<Timestamp>,
    error_message: Option<String>,
    // Metadata tracking
    write_metadata: bool,
    metadata_sync_id: Option<i64>,
    /// Map from filing_id to RSS item data for metadata recording
    rss_item_data: std::collections::HashMap<String, Item>,
}

impl RpcSyncState {
    /// Process one pending export from the queue
    fn process_one_export(&mut self, sourcer: &FilingSourcer) -> Result<()> {
        if let Some(filing_id) = self.export_queue.pop() {
            if let Some(ref mut db) = self.export_db {
                let (success, message) = match sourcer.resolve_from_user_argument(&filing_id) {
                    Ok(filing) => match sqlite::export_single_filing(db, filing, self.cover_only) {
                        Ok(_) => {
                            self.exported_ids.insert(filing_id.clone());
                            (true, None)
                        }
                        Err(e) => (false, Some(e.to_string())),
                    },
                    Err(e) => (false, Some(format!("Failed to fetch filing: {}", e))),
                };

                // Record metadata if enabled
                if self.write_metadata {
                    if let Some(metadata_sync_id) = self.metadata_sync_id {
                        let rss_params = if let Some(item) = self.rss_item_data.get(&filing_id) {
                            sqlite::RssFilingParams {
                                rss_pub_date: item.pub_date.map(|ts| ts.to_string()),
                                rss_guid: Some(item.guid.clone()),
                                rss_title: Some(item.title.clone()),
                                committee_id: item.committee_id.clone(),
                                form_type: item.form_type.clone(),
                                coverage_from: item.coverage_from.clone(),
                                coverage_through: item.coverage_through.clone(),
                                report_type: item.report_type.clone(),
                            }
                        } else {
                            sqlite::RssFilingParams::default()
                        };

                        let _ = sqlite::record_rss_filing(
                            db,
                            metadata_sync_id,
                            &filing_id,
                            &rss_params,
                            success,
                            message.as_deref(),
                        );
                    }
                }

                if !success {
                    if let Some(msg) = message {
                        return Err(anyhow::anyhow!("{}", msg));
                    }
                }
            }
        }
        Ok(())
    }

    /// Get export progress (completed, total)
    fn export_progress(&self) -> (usize, usize) {
        let completed = self.export_batch_total - self.export_queue.len();
        (completed, self.export_batch_total)
    }

    /// Get status as JSON value
    fn to_status_json(&self) -> serde_json::Value {
        match &self.phase {
            RpcPhase::Idle => json!({
                "sync_id": serde_json::Value::Null,
                "phase": "idle"
            }),
            RpcPhase::Fetching => json!({
                "sync_id": self.sync_id,
                "phase": "fetching"
            }),
            RpcPhase::Exporting => {
                let (completed, total) = self.export_progress();
                let current_filing_id = self.export_queue.last().cloned();
                json!({
                    "sync_id": self.sync_id,
                    "phase": "exporting",
                    "feed_title": self.feed_title,
                    "last_modified": self.last_modified.map(|ts| ts.to_string()),
                    "total_items": self.total_items,
                    "filtered_items": self.filtered_items,
                    "export_progress": {
                        "completed": completed,
                        "total": total,
                        "current_filing_id": current_filing_id
                    }
                })
            }
            RpcPhase::Complete => json!({
                "sync_id": self.sync_id,
                "phase": "complete",
                "export_summary": {
                    "total_exported": self.export_batch_total,
                    "latest_filing_id": self.latest_filing_id,
                    "latest_pub_date": self.latest_pub_date.map(|ts| ts.to_string())
                }
            }),
            RpcPhase::Canceled => json!({
                "sync_id": self.sync_id,
                "phase": "canceled"
            }),
            RpcPhase::Error => json!({
                "sync_id": self.sync_id,
                "phase": "error",
                "error_message": self.error_message
            }),
        }
    }
}

/// Main RPC mode entry point
pub fn run_rpc_mode(
    sourcer: FilingSourcer,
    args: &RssArgs,
    since_ts: Option<Timestamp>,
) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdin_lines = stdin.lock().lines();
    let mut stdout_lock = stdout.lock();

    let mut state: Option<RpcSyncState> = None;

    // Send ready notification
    send_notification(
        &mut stdout_lock,
        "ready",
        json!({ "version": env!("CARGO_PKG_VERSION") }),
    )?;

    loop {
        // Process exports if in progress (process up to 10 at a time before checking for new requests)
        if let Some(ref mut sync_state) = state {
            if matches!(sync_state.phase, RpcPhase::Exporting) {
                let mut processed = 0;
                while !sync_state.export_queue.is_empty() && processed < 10 {
                    match sync_state.process_one_export(&sourcer) {
                        Ok(_) => {
                            processed += 1;
                            // Send progress notification every 5 exports
                            if processed % 5 == 0 {
                                send_progress_notification(&mut stdout_lock, sync_state)?;
                            }
                        }
                        Err(e) => {
                            sync_state.phase = RpcPhase::Error;
                            sync_state.error_message = Some(e.to_string());
                            // Finalize metadata on error
                            finalize_sync_metadata(sync_state);
                            send_progress_notification(&mut stdout_lock, sync_state)?;
                            break;
                        }
                    }
                }

                // Check if we're done
                if sync_state.export_queue.is_empty() && processed > 0 {
                    sync_state.phase = RpcPhase::Complete;
                    // Finalize metadata on completion
                    finalize_sync_metadata(sync_state);
                    send_progress_notification(&mut stdout_lock, sync_state)?;
                }
            }
        }

        // Try to read a request
        match stdin_lines.next() {
            Some(Ok(line)) => {
                let (response, should_exit) =
                    handle_request(&line, &mut state, &sourcer, args, since_ts)?;
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
    state: &mut Option<RpcSyncState>,
    sourcer: &FilingSourcer,
    base_args: &RssArgs,
    base_since_ts: Option<Timestamp>,
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
        "sync/start" => handle_sync_start(request.params, state, sourcer, base_args, base_since_ts)
            .map(|r| (r, false)),
        "sync/status" => handle_sync_status(state).map(|r| (r, false)),
        "sync/cancel" => handle_sync_cancel(state).map(|r| (r, false)),
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

/// Handle sync/start method
fn handle_sync_start(
    params: serde_json::Value,
    state: &mut Option<RpcSyncState>,
    _sourcer: &FilingSourcer,
    base_args: &RssArgs,
    base_since_ts: Option<Timestamp>,
) -> Result<serde_json::Value, JsonRpcError> {
    // Check if sync already in progress
    if let Some(ref sync_state) = state {
        if !matches!(
            sync_state.phase,
            RpcPhase::Complete | RpcPhase::Canceled | RpcPhase::Error
        ) {
            return Err(JsonRpcError {
                code: -32001,
                message: "Sync already in progress".to_string(),
                data: None,
            });
        }
    }

    // Parse parameters
    let sync_params: SyncStartParams =
        serde_json::from_value(params).map_err(|e| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: Some(json!({ "details": e.to_string() })),
        })?;

    // Parse since timestamp if provided
    let since_ts = if let Some(ref since_str) = sync_params.since {
        match super::since::parse_since(since_str) {
            Ok(ts) => Some(ts),
            Err(e) => {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "Invalid since parameter".to_string(),
                    data: Some(json!({ "details": e.to_string() })),
                });
            }
        }
    } else {
        base_since_ts
    };

    // Build args for fetching
    let mut fetch_args = base_args.clone();
    if let Some(ref preset) = sync_params.preset {
        fetch_args.preset = match preset.as_str() {
            "all" => crate::cli::RssPreset::All,
            "monthly" => crate::cli::RssPreset::Monthly,
            "quarterly" => crate::cli::RssPreset::Quarterly,
            "presidential" => crate::cli::RssPreset::Presidential,
            "congressional" => crate::cli::RssPreset::Congressional,
            "pac" => crate::cli::RssPreset::Pac,
            _ => {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "Invalid preset".to_string(),
                    data: Some(json!({ "preset": preset })),
                });
            }
        };
    }
    fetch_args.form_type = sync_params.form_type.clone();
    fetch_args.committee = sync_params.committee.clone();
    fetch_args.state = sync_params.state.clone();
    fetch_args.party = sync_params.party.clone();

    // Generate sync ID
    let sync_id = format!("sync-{}", uuid::Uuid::new_v4());

    // Initialize database
    let (export_db, metadata_sync_id) =
        match super::export::open_or_create_export_db(&sync_params.export_path) {
            Ok(mut db) => {
                if let Err(e) = sqlite::init_schema(&mut db) {
                    return Err(JsonRpcError {
                        code: -32002,
                        message: "Failed to initialize database".to_string(),
                        data: Some(json!({ "details": e.to_string() })),
                    });
                }

                // Initialize metadata schema and create sync record if enabled
                let metadata_id = if sync_params.write_metadata {
                    if let Err(e) = sqlite::init_rss_metadata_schema(&db) {
                        return Err(JsonRpcError {
                            code: -32002,
                            message: "Failed to initialize RSS metadata schema".to_string(),
                            data: Some(json!({ "details": e.to_string() })),
                        });
                    }

                    let preset_str = sync_params.preset.as_deref();
                    let metadata_params = sqlite::RssSyncParams {
                        feed_url: None, // Will be set after fetch
                        feed_last_modified: None,
                        feed_title: None,
                        since_filter: sync_params.since.clone(),
                        preset_filter: preset_str.map(|s| s.to_string()),
                        form_type_filter: sync_params.form_type.clone(),
                        committee_filter: sync_params.committee.clone(),
                        state_filter: sync_params.state.clone(),
                        party_filter: sync_params.party.clone(),
                        cover_only: sync_params.cover_only,
                    };

                    match sqlite::create_rss_sync(&db, &sync_id, &metadata_params) {
                        Ok(metadata) => Some(metadata.sync_id),
                        Err(e) => {
                            return Err(JsonRpcError {
                                code: -32002,
                                message: "Failed to create RSS sync metadata".to_string(),
                                data: Some(json!({ "details": e.to_string() })),
                            });
                        }
                    }
                } else {
                    None
                };

                (Some(db), metadata_id)
            }
            Err(e) => {
                return Err(JsonRpcError {
                    code: -32002,
                    message: "Failed to open database".to_string(),
                    data: Some(json!({ "details": e.to_string() })),
                });
            }
        };

    // Create initial state in fetching phase
    let mut new_state = RpcSyncState {
        sync_id: sync_id.clone(),
        phase: RpcPhase::Fetching,
        feed_title: String::new(),
        last_modified: None,
        total_items: 0,
        filtered_items: 0,
        export_queue: Vec::new(),
        export_batch_total: 0,
        exported_ids: HashSet::new(),
        export_db,
        cover_only: sync_params.cover_only,
        latest_filing_id: None,
        latest_pub_date: None,
        error_message: None,
        write_metadata: sync_params.write_metadata,
        metadata_sync_id,
        rss_item_data: std::collections::HashMap::new(),
    };

    // Fetch feed
    match rss::fetch_feed_with_args(&fetch_args) {
        Ok((result, _filters)) => {
            new_state.feed_title = result.feed.title;
            new_state.last_modified = result.last_modified;
            new_state.total_items = result.feed.items.len();

            // Filter items by --since if provided
            let filtered_items: Vec<Item> = if let Some(since) = since_ts {
                result
                    .feed
                    .items
                    .into_iter()
                    .filter(|item| {
                        item.pub_date
                            .map(|pub_date| pub_date >= since)
                            .unwrap_or(false)
                    })
                    .collect()
            } else {
                result.feed.items
            };

            new_state.filtered_items = filtered_items.len();

            // Load existing filing IDs from database
            if let Some(ref db) = new_state.export_db {
                if let Ok(ids) = sqlite::get_existing_filing_ids(db) {
                    new_state.exported_ids = ids;
                }
            }

            // Queue filings for export and store RSS item data for metadata
            for item in filtered_items.iter() {
                if let Some(ref filing_id) = item.filing_id {
                    if !new_state.exported_ids.contains(filing_id) {
                        new_state.export_queue.push(filing_id.clone());
                        // Store item data for metadata recording
                        if new_state.write_metadata {
                            new_state
                                .rss_item_data
                                .insert(filing_id.clone(), item.clone());
                        }
                    }
                }
            }

            // Reverse queue so we pop from the front (oldest first)
            new_state.export_queue.reverse();
            new_state.export_batch_total = new_state.export_queue.len();

            // Track latest filing
            if let Some(item) = filtered_items.first() {
                new_state.latest_filing_id = item.filing_id.clone();
                new_state.latest_pub_date = item.pub_date;
            }

            // Move to exporting phase (or complete if nothing to export)
            if new_state.export_queue.is_empty() {
                new_state.phase = RpcPhase::Complete;
                finalize_sync_metadata(&mut new_state);
            } else {
                new_state.phase = RpcPhase::Exporting;
            }
        }
        Err(e) => {
            new_state.phase = RpcPhase::Error;
            new_state.error_message = Some(e.to_string());
        }
    }

    *state = Some(new_state);

    Ok(json!({
        "sync_id": sync_id,
        "status": "started"
    }))
}

/// Handle sync/status method
fn handle_sync_status(state: &Option<RpcSyncState>) -> Result<serde_json::Value, JsonRpcError> {
    if let Some(ref sync_state) = state {
        Ok(sync_state.to_status_json())
    } else {
        Ok(json!({
            "sync_id": serde_json::Value::Null,
            "phase": "idle"
        }))
    }
}

/// Handle sync/cancel method
fn handle_sync_cancel(state: &mut Option<RpcSyncState>) -> Result<serde_json::Value, JsonRpcError> {
    if let Some(ref mut sync_state) = state {
        let sync_id = sync_state.sync_id.clone();
        sync_state.phase = RpcPhase::Canceled;
        sync_state.export_queue.clear();
        // Finalize metadata on cancellation
        finalize_sync_metadata(sync_state);
        Ok(json!({
            "canceled": true,
            "sync_id": sync_id
        }))
    } else {
        Err(JsonRpcError {
            code: -32000,
            message: "No sync in progress".to_string(),
            data: None,
        })
    }
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

/// Send progress notification for sync state
fn send_progress_notification(stdout: &mut dyn Write, state: &RpcSyncState) -> Result<()> {
    let params = match &state.phase {
        RpcPhase::Fetching => json!({ "phase": "fetching" }),
        RpcPhase::Exporting => {
            let (completed, total) = state.export_progress();
            json!({
                "phase": "exporting",
                "exported_count": completed,
                "total_count": total
            })
        }
        RpcPhase::Complete => json!({ "phase": "complete" }),
        RpcPhase::Canceled => json!({ "phase": "canceled" }),
        RpcPhase::Error => json!({
            "phase": "error",
            "error_message": state.error_message
        }),
        _ => return Ok(()), // Don't send notification for idle
    };

    send_notification(stdout, "sync/progress", params)
}

/// Finalize metadata for a completed/canceled/errored sync
fn finalize_sync_metadata(state: &mut RpcSyncState) {
    if !state.write_metadata {
        return;
    }

    let Some(metadata_sync_id) = state.metadata_sync_id else {
        return;
    };

    let Some(ref db) = state.export_db else {
        return;
    };

    let (status, error_message) = match state.phase {
        RpcPhase::Complete => ("complete", None),
        RpcPhase::Canceled => ("canceled", None),
        RpcPhase::Error => ("error", state.error_message.as_deref()),
        _ => return, // Not in a terminal state
    };

    let exported_count = state.exported_ids.len();
    let new_filings_count = state.export_batch_total;

    let _ = sqlite::finalize_rss_sync(
        db,
        metadata_sync_id,
        status,
        Some(state.total_items),
        Some(state.filtered_items),
        new_filings_count,
        exported_count,
        error_message,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_rpc_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"sync/start","params":{"export_path":"/tmp/test.db"}}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "sync/start");
    }

    #[test]
    fn test_parse_sync_start_params() {
        let json = r#"{"export_path":"/tmp/test.db","cover_only":true,"since":"1 day ago"}"#;
        let params: SyncStartParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.export_path, PathBuf::from("/tmp/test.db"));
        assert!(params.cover_only);
        assert_eq!(params.since, Some("1 day ago".to_string()));
    }

    #[test]
    fn test_serialize_json_rpc_response() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: Some(json!({"sync_id": "sync-123", "status": "started"})),
            error: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("sync-123"));
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
                message: "Sync already in progress".to_string(),
                data: None,
            }),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("-32001"));
        assert!(!json.contains("result"));
    }
}
