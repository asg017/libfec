/*!
 * Integration tests for RSS RPC mode
 *
 * Tests the JSON-RPC protocol over stdio with `libfec rss --rpc`.
 */

use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;
use tempfile::NamedTempFile;

/// Helper to spawn libfec rss --rpc and communicate via JSONL
struct RpcClient {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    next_id: i32,
}

impl RpcClient {
    fn new() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_libfec"))
            .args(&["rss", "--rpc"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn libfec rss --rpc");

        let stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = BufReader::new(child.stdout.take().expect("Failed to open stdout"));

        let mut client = RpcClient {
            child,
            stdin,
            stdout,
            next_id: 1,
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

            // Skip notifications
            if !message.get("id").is_some() {
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

impl Drop for RpcClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[test]
fn test_rpc_sync_start() {
    let mut client = RpcClient::new();
    let temp_db = NamedTempFile::new().unwrap();

    let result = client
        .send_request(
            "sync/start",
            json!({
                "export_path": temp_db.path(),
                "cover_only": true,
                "since": "7 days ago"
            }),
        )
        .expect("sync/start should succeed");

    assert!(result["sync_id"].is_string());
    assert_eq!(result["status"], "started");

    client.shutdown();
}

#[test]
fn test_rpc_sync_status_idle() {
    let mut client = RpcClient::new();

    let status = client
        .send_request("sync/status", json!({}))
        .expect("sync/status should succeed");

    assert_eq!(status["phase"], "idle");
    assert!(status["sync_id"].is_null());

    client.shutdown();
}

#[test]
fn test_rpc_sync_status_after_start() {
    let mut client = RpcClient::new();
    let temp_db = NamedTempFile::new().unwrap();

    // Start sync
    client
        .send_request(
            "sync/start",
            json!({
                "export_path": temp_db.path(),
                "cover_only": true,
                "since": "7 days ago"
            }),
        )
        .expect("sync/start should succeed");

    // Check status
    let status = client
        .send_request("sync/status", json!({}))
        .expect("sync/status should succeed");

    let phase = status["phase"].as_str().unwrap();
    assert!(
        phase == "fetching" || phase == "exporting" || phase == "complete",
        "Phase should be active: {}",
        phase
    );

    client.shutdown();
}

#[test]
fn test_rpc_concurrent_sync_error() {
    let mut client = RpcClient::new();
    let temp_db1 = NamedTempFile::new().unwrap();
    let temp_db2 = NamedTempFile::new().unwrap();

    // Start first sync
    client
        .send_request(
            "sync/start",
            json!({
                "export_path": temp_db1.path(),
                "cover_only": true
            }),
        )
        .expect("First sync/start should succeed");

    // Try to start second sync while first is active
    let error = client
        .send_request(
            "sync/start",
            json!({
                "export_path": temp_db2.path(),
                "cover_only": true
            }),
        )
        .expect_err("Second sync/start should fail");

    assert_eq!(error["code"], -32001);
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("already in progress"));

    client.shutdown();
}

#[test]
fn test_rpc_invalid_method() {
    let mut client = RpcClient::new();

    let error = client
        .send_request("invalid/method", json!({}))
        .expect_err("Invalid method should fail");

    assert_eq!(error["code"], -32601);
    assert!(error["message"].as_str().unwrap().contains("not found"));

    client.shutdown();
}

#[test]
fn test_rpc_invalid_params() {
    let mut client = RpcClient::new();

    let error = client
        .send_request(
            "sync/start",
            json!({
                // Missing required export_path
                "cover_only": true
            }),
        )
        .expect_err("Invalid params should fail");

    assert_eq!(error["code"], -32602);

    client.shutdown();
}

#[test]
fn test_rpc_cancel_sync() {
    let mut client = RpcClient::new();
    let temp_db = NamedTempFile::new().unwrap();

    // Start sync
    client
        .send_request(
            "sync/start",
            json!({
                "export_path": temp_db.path(),
                "cover_only": true,
                "since": "1 day ago"
            }),
        )
        .expect("sync/start should succeed");

    // Cancel it
    let result = client
        .send_request("sync/cancel", json!({}))
        .expect("sync/cancel should succeed");

    assert_eq!(result["canceled"], true);
    assert!(result["sync_id"].is_string());

    client.shutdown();
}

#[test]
fn test_rpc_cancel_when_idle() {
    let mut client = RpcClient::new();

    let error = client
        .send_request("sync/cancel", json!({}))
        .expect_err("sync/cancel should fail when idle");

    assert_eq!(error["code"], -32000);

    client.shutdown();
}

// Commented out due to flakiness - depends on network requests and export timing
// Manual testing with examples/rss_rpc_client.py is recommended
//
// #[test]
// fn test_rpc_full_sync_flow() {
//     let mut client = RpcClient::new();
//     let temp_db = NamedTempFile::new().unwrap();
//
//     // Start sync
//     let start_result = client
//         .send_request(
//             "sync/start",
//             json!({
//                 "export_path": temp_db.path(),
//                 "cover_only": true,
//                 "since": "7 days ago"
//             }),
//         )
//         .expect("sync/start should succeed");
//
//     let sync_id = start_result["sync_id"].as_str().unwrap();
//
//     // Poll status until complete (with timeout)
//     let mut attempts = 0;
//     let max_attempts = 600; // 60 seconds
//
//     loop {
//         std::thread::sleep(Duration::from_millis(100));
//
//         let status = client
//             .send_request("sync/status", json!({}))
//             .expect("sync/status should succeed");
//
//         let phase = status["phase"].as_str().unwrap();
//
//         if phase == "complete" || phase == "error" {
//             assert_eq!(status["sync_id"], sync_id);
//             if phase == "error" {
//                 let error_msg = status.get("error_message").and_then(|v| v.as_str());
//                 panic!("Sync failed with error: {:?}", error_msg);
//             }
//             break;
//         }
//
//         attempts += 1;
//         if attempts >= max_attempts {
//             panic!("Sync did not complete within timeout. Last phase: {}", phase);
//         }
//     }
//
//     // Verify database was created and has schema
//     let db = rusqlite::Connection::open(temp_db.path()).expect("Should open database");
//     let count: i64 = db
//         .query_row(
//             "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='libfec_filings'",
//             [],
//             |row| row.get(0),
//         )
//         .expect("Should query schema");
//     assert_eq!(count, 1, "Database should have libfec_filings table");
//
//     client.shutdown();
// }

#[test]
fn test_rpc_invalid_json() {
    let mut client = RpcClient::new();

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
