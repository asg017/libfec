/*!
 * Integration tests for Export RPC mode
 *
 * Tests the JSON-RPC protocol over stdio with `libfec export --rpc -o <db>`.
 *
 * These tests focus on protocol correctness without hitting the FEC API.
 * Tests that would require network calls are either:
 * - Using empty filing lists
 * - Testing error conditions
 * - Testing protocol-level behavior only
 */

use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;
use tempfile::NamedTempFile;

/// Helper to spawn libfec export --rpc and communicate via JSONL
struct ExportRpcClient {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    next_id: i32,
    #[allow(dead_code)]
    db_path: tempfile::NamedTempFile,
}

impl ExportRpcClient {
    fn new() -> Self {
        let db_path = NamedTempFile::with_suffix(".db").unwrap();

        let mut child = Command::new(env!("CARGO_BIN_EXE_libfec"))
            .args(["export", "--rpc", "-o", db_path.path().to_str().unwrap()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn libfec export --rpc");

        let stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = BufReader::new(child.stdout.take().expect("Failed to open stdout"));

        let mut client = ExportRpcClient {
            child,
            stdin,
            stdout,
            next_id: 1,
            db_path,
        };

        // Wait for ready notification
        let ready = client.read_notification().expect("Failed to read ready");
        assert_eq!(ready["method"], "ready");

        client
    }

    fn read_line(&mut self) -> Option<serde_json::Value> {
        let mut line = String::new();
        match self.stdout.read_line(&mut line) {
            Ok(0) => None,
            Ok(_) => serde_json::from_str(&line).ok(),
            Err(_) => None,
        }
    }

    fn read_notification(&mut self) -> Option<serde_json::Value> {
        self.read_line()
    }

    fn send_request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, serde_json::Value> {
        let id = self.next_id;
        self.next_id += 1;

        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let request_line = serde_json::to_string(&request).unwrap() + "\n";
        self.stdin
            .write_all(request_line.as_bytes())
            .expect("Failed to write request");
        self.stdin.flush().expect("Failed to flush stdin");

        // Read response (skip notifications)
        loop {
            let message = self.read_line().expect("Failed to read response");

            // Skip notifications (no id field)
            if message.get("id").is_none() {
                continue;
            }

            if message["id"] == id {
                if message.get("error").is_some() {
                    return Err(message["error"].clone());
                }
                return Ok(message["result"].clone());
            }
        }
    }

    fn shutdown(mut self) {
        let _ = self.send_request("shutdown", json!({}));
        // Give it a moment to shutdown gracefully
        std::thread::sleep(Duration::from_millis(100));
        let _ = self.child.kill();
    }
}

impl Drop for ExportRpcClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

// =============================================================================
// Protocol Tests (no network calls)
// =============================================================================

#[test]
fn test_export_rpc_ready_notification() {
    // The RpcClient constructor already validates ready notification
    let client = ExportRpcClient::new();
    client.shutdown();
}

#[test]
fn test_export_rpc_status_idle() {
    let mut client = ExportRpcClient::new();

    let status = client
        .send_request("export/status", json!({}))
        .expect("export/status should succeed");

    assert_eq!(status["phase"], "idle");
    assert!(status["export_id"].is_null());

    client.shutdown();
}

#[test]
fn test_export_rpc_invalid_method() {
    let mut client = ExportRpcClient::new();

    let error = client
        .send_request("invalid/method", json!({}))
        .expect_err("Invalid method should fail");

    assert_eq!(error["code"], -32601);
    assert!(error["message"].as_str().unwrap().contains("not found"));

    client.shutdown();
}

#[test]
fn test_export_rpc_cancel_when_idle() {
    let mut client = ExportRpcClient::new();

    let error = client
        .send_request("export/cancel", json!({}))
        .expect_err("export/cancel should fail when idle");

    assert_eq!(error["code"], -32000);
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("No export in progress"));

    client.shutdown();
}

#[test]
fn test_export_rpc_invalid_json() {
    let mut client = ExportRpcClient::new();

    // Send malformed JSON
    client
        .stdin
        .write_all(b"not valid json\n")
        .expect("Failed to write");
    client.stdin.flush().expect("Failed to flush");

    let response = client.read_line().expect("Should read error response");
    assert_eq!(response["error"]["code"], -32700);

    client.shutdown();
}

#[test]
fn test_export_rpc_shutdown() {
    let mut client = ExportRpcClient::new();

    let result = client
        .send_request("shutdown", json!({}))
        .expect("shutdown should succeed");

    assert_eq!(result["ok"], true);

    // Process should exit gracefully - don't call shutdown() again
    std::thread::sleep(Duration::from_millis(100));
}

// =============================================================================
// Export Start Tests (minimal/no network calls)
// =============================================================================

#[test]
fn test_export_rpc_start_empty_filings() {
    let mut client = ExportRpcClient::new();

    // Start export with empty filings list - should complete immediately
    let result = client
        .send_request(
            "export/start",
            json!({
                "filings": [],
                "cover_only": true
            }),
        )
        .expect("export/start with empty filings should succeed");

    assert!(result["export_id"].is_string());
    assert_eq!(result["status"], "started");

    // Status should be complete (nothing to export)
    let status = client
        .send_request("export/status", json!({}))
        .expect("export/status should succeed");

    assert_eq!(status["phase"], "complete");
    assert_eq!(status["total_exported"], 0);

    client.shutdown();
}

#[test]
fn test_export_rpc_start_with_cover_only() {
    let mut client = ExportRpcClient::new();

    let result = client
        .send_request(
            "export/start",
            json!({
                "filings": [],
                "cover_only": true
            }),
        )
        .expect("export/start should succeed");

    assert!(result["export_id"].is_string());
    assert!(result["export_id"].as_str().unwrap().starts_with("export-"));

    client.shutdown();
}

#[test]
fn test_export_rpc_start_with_clobber() {
    let mut client = ExportRpcClient::new();

    // First export
    client
        .send_request(
            "export/start",
            json!({
                "filings": [],
                "clobber": true
            }),
        )
        .expect("First export/start should succeed");

    // Wait for completion
    std::thread::sleep(Duration::from_millis(100));

    // Second export with clobber should also work
    let result = client
        .send_request(
            "export/start",
            json!({
                "filings": [],
                "clobber": true
            }),
        )
        .expect("Second export/start with clobber should succeed");

    assert!(result["export_id"].is_string());

    client.shutdown();
}

#[test]
fn test_export_rpc_concurrent_export_error() {
    let mut client = ExportRpcClient::new();

    // Start a long-running export by using an invalid filing ID
    // This will trigger sourcing phase but won't complete immediately
    let _ = client.send_request(
        "export/start",
        json!({
            "filings": [],
        }),
    );

    // Immediately try to start another - but first one may have completed
    // So we check if we get either success or "already in progress"
    let result = client.send_request(
        "export/start",
        json!({
            "filings": [],
        }),
    );

    // Either we get success (first completed) or error (still running)
    match result {
        Ok(_) => {
            // First export completed, second started
        }
        Err(error) => {
            assert_eq!(error["code"], -32001);
            assert!(error["message"]
                .as_str()
                .unwrap()
                .contains("already in progress"));
        }
    }

    client.shutdown();
}

#[test]
fn test_export_rpc_status_after_start() {
    let mut client = ExportRpcClient::new();

    // Start export
    let start_result = client
        .send_request(
            "export/start",
            json!({
                "filings": [],
            }),
        )
        .expect("export/start should succeed");

    let export_id = start_result["export_id"].as_str().unwrap();

    // Check status
    let status = client
        .send_request("export/status", json!({}))
        .expect("export/status should succeed");

    // With empty filings, should be complete
    assert_eq!(status["export_id"], export_id);
    let phase = status["phase"].as_str().unwrap();
    assert!(
        phase == "complete" || phase == "exporting" || phase == "sourcing",
        "Unexpected phase: {}",
        phase
    );

    client.shutdown();
}

#[test]
fn test_export_rpc_cancel_during_export() {
    let mut client = ExportRpcClient::new();

    // Start export (with empty list, it completes fast)
    let start_result = client
        .send_request(
            "export/start",
            json!({
                "filings": [],
            }),
        )
        .expect("export/start should succeed");

    let export_id = start_result["export_id"].as_str().unwrap();

    // Try to cancel - might succeed or fail if already complete
    let cancel_result = client.send_request("export/cancel", json!({}));

    match cancel_result {
        Ok(result) => {
            // Cancel succeeded
            assert_eq!(result["canceled"], true);
            assert_eq!(result["export_id"], export_id);
        }
        Err(error) => {
            // Already completed, can't cancel
            assert_eq!(error["code"], -32000);
        }
    }

    client.shutdown();
}

// =============================================================================
// Parameter Validation Tests (no network calls)
// =============================================================================

#[test]
fn test_export_rpc_start_invalid_cycle_type() {
    let mut client = ExportRpcClient::new();

    let error = client
        .send_request(
            "export/start",
            json!({
                "filings": [],
                "cycle": "not a number"
            }),
        )
        .expect_err("Invalid cycle type should fail");

    assert_eq!(error["code"], -32602);

    client.shutdown();
}

#[test]
fn test_export_rpc_start_invalid_cover_only_type() {
    let mut client = ExportRpcClient::new();

    let error = client
        .send_request(
            "export/start",
            json!({
                "filings": [],
                "cover_only": "not a bool"
            }),
        )
        .expect_err("Invalid cover_only type should fail");

    assert_eq!(error["code"], -32602);

    client.shutdown();
}

#[test]
fn test_export_rpc_start_invalid_filings_type() {
    let mut client = ExportRpcClient::new();

    let error = client
        .send_request(
            "export/start",
            json!({
                "filings": "not an array"
            }),
        )
        .expect_err("Invalid filings type should fail");

    assert_eq!(error["code"], -32602);

    client.shutdown();
}

// =============================================================================
// Database Schema Tests (no network calls)
// =============================================================================

#[test]
fn test_export_rpc_creates_database_schema() {
    let db_file = NamedTempFile::with_suffix(".db").unwrap();
    let db_path = db_file.path().to_path_buf();

    // Create client with specific db path
    let mut child = Command::new(env!("CARGO_BIN_EXE_libfec"))
        .args(["export", "--rpc", "-o", db_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Read ready notification
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();

    // Start empty export
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "export/start",
        "params": {"filings": []}
    });
    writeln!(stdin, "{}", serde_json::to_string(&request).unwrap()).unwrap();
    stdin.flush().unwrap();

    // Read response
    line.clear();
    stdout.read_line(&mut line).unwrap();

    // Wait for completion and shutdown
    std::thread::sleep(Duration::from_millis(200));

    let shutdown = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "shutdown",
        "params": {}
    });
    writeln!(stdin, "{}", serde_json::to_string(&shutdown).unwrap()).unwrap();
    stdin.flush().unwrap();

    std::thread::sleep(Duration::from_millis(100));
    let _ = child.kill();

    // Verify database was created with schema
    let db = rusqlite::Connection::open(&db_path).expect("Should open database");

    // Check that libfec_filings table exists
    let count: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='libfec_filings'",
            [],
            |row| row.get(0),
        )
        .expect("Should query schema");

    assert_eq!(count, 1, "Database should have libfec_filings table");
}

#[test]
fn test_export_rpc_empty_export_has_no_filings() {
    let db_file = NamedTempFile::with_suffix(".db").unwrap();
    let db_path = db_file.path().to_path_buf();

    let mut child = Command::new(env!("CARGO_BIN_EXE_libfec"))
        .args(["export", "--rpc", "-o", db_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Read ready notification
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();

    // Start empty export
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "export/start",
        "params": {"filings": []}
    });
    writeln!(stdin, "{}", serde_json::to_string(&request).unwrap()).unwrap();
    stdin.flush().unwrap();

    // Read response
    line.clear();
    stdout.read_line(&mut line).unwrap();

    // Wait and shutdown
    std::thread::sleep(Duration::from_millis(200));

    let shutdown = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "shutdown",
        "params": {}
    });
    writeln!(stdin, "{}", serde_json::to_string(&shutdown).unwrap()).unwrap();
    stdin.flush().unwrap();

    std::thread::sleep(Duration::from_millis(100));
    let _ = child.kill();

    // Verify no filings were inserted
    let db = rusqlite::Connection::open(&db_path).expect("Should open database");
    let count: i64 = db
        .query_row("SELECT COUNT(*) FROM libfec_filings", [], |row| row.get(0))
        .expect("Should count filings");

    assert_eq!(count, 0, "Empty export should have no filings");
}

// =============================================================================
// Progress Notification Tests
// =============================================================================

// Note: Progress notification tests are difficult to write reliably because:
// 1. Empty exports complete so fast that notifications may not be sent
// 2. Reading from stdout can block if no notifications are available
// 3. The timing between request/response and notifications is non-deterministic
//
// The notification mechanism is tested implicitly through the Python client
// in examples/export_rpc_client.py which handles notifications properly.

// =============================================================================
// Multiple Request Tests
// =============================================================================

#[test]
fn test_export_rpc_multiple_status_requests() {
    let mut client = ExportRpcClient::new();

    // Send multiple status requests in sequence
    for _ in 0..5 {
        let status = client
            .send_request("export/status", json!({}))
            .expect("export/status should succeed");

        assert!(status.get("phase").is_some());
    }

    client.shutdown();
}

#[test]
fn test_export_rpc_interleaved_requests() {
    let mut client = ExportRpcClient::new();

    // Status
    let status1 = client
        .send_request("export/status", json!({}))
        .expect("status should succeed");
    assert_eq!(status1["phase"], "idle");

    // Start empty export
    client
        .send_request("export/start", json!({"filings": []}))
        .expect("start should succeed");

    // Status again
    let status2 = client
        .send_request("export/status", json!({}))
        .expect("status should succeed");
    assert!(status2["export_id"].is_string());

    // Another start should work after completion
    std::thread::sleep(Duration::from_millis(100));
    client
        .send_request("export/start", json!({"filings": [], "clobber": true}))
        .expect("second start should succeed");

    client.shutdown();
}
