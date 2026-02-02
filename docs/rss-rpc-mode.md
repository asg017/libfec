# RSS RPC Mode

The `libfec rss --rpc` command provides a JSON-RPC 2.0 server for programmatic control of RSS sync operations over stdio using a JSONL (newline-delimited JSON) protocol.

## Overview

External processes can spawn `libfec rss --rpc` and interact with it by:
- Sending JSON-RPC requests to stdin
- Reading JSON-RPC responses and notifications from stdout

This enables:
- Programmatic control of RSS feed syncing
- Real-time progress notifications
- Integration with other tools and languages

## Protocol

### Message Format

All messages are JSONL - one JSON object per line.

#### Request (stdin → server)
```json
{"jsonrpc":"2.0","id":1,"method":"sync/start","params":{...}}
```

#### Response (server → stdout)
```json
{"jsonrpc":"2.0","id":1,"result":{...}}
```

#### Error Response (server → stdout)
```json
{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid request"}}
```

#### Notification (server → stdout, no id)
```json
{"jsonrpc":"2.0","method":"sync/progress","params":{...}}
```

## JSON-RPC Methods

### `sync/start` - Start RSS Sync

Start a new RSS sync operation with the specified parameters.

**Parameters:**
- `export_path` (string, required): Path to SQLite database file
- `cover_only` (bool, optional): Only export cover records (default: false)
- `since` (string, optional): Time filter (ISO8601 or relative like "1 day ago")
- `preset` (string, optional): "all", "monthly", "quarterly", "presidential", "congressional", "pac"
- `form_type` (string, optional): Form type filter (e.g., "F3", "F3X", "F99")
- `committee` (string, optional): Committee ID filter
- `state` (string, optional): State code filter (e.g., "CA")
- `party` (string, optional): Party filter (e.g., "DEM", "REP")
- `write_metadata` (bool, optional): Write sync metadata to database (default: false)

**Example Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "sync/start",
  "params": {
    "export_path": "/tmp/filings.db",
    "cover_only": true,
    "since": "1 day ago",
    "preset": "presidential",
    "write_metadata": true
  }
}
```

**Success Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "sync_id": "sync-550e8400-e29b-41d4-a716-446655440000",
    "status": "started"
  }
}
```

**Error Codes:**
- `-32001`: Sync already in progress
- `-32002`: Invalid parameters or failed to initialize database
- `-32602`: Invalid parameter format

### `sync/status` - Get Current Sync Status

Get the current status of the sync operation.

**Parameters:** None (empty object)

**Example Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "sync/status",
  "params": {}
}
```

**Response When Idle:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "sync_id": null,
    "phase": "idle"
  }
}
```

**Response When Exporting:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "sync_id": "sync-550e8400-...",
    "phase": "exporting",
    "feed_title": "FEC Electronic Filing RSS Feed - ALL",
    "last_modified": "2026-01-27T12:00:00Z",
    "total_items": 50,
    "filtered_items": 20,
    "export_progress": {
      "completed": 5,
      "total": 20,
      "current_filing_id": "1234567"
    }
  }
}
```

**Response When Complete:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "sync_id": "sync-550e8400-...",
    "phase": "complete",
    "export_summary": {
      "total_exported": 20,
      "latest_filing_id": "1234999",
      "latest_pub_date": "2026-01-27T11:59:00Z"
    }
  }
}
```

**Phases:**
- `idle`: No sync in progress
- `fetching`: Fetching RSS feed from FEC
- `exporting`: Exporting filings to database
- `complete`: Sync completed successfully
- `canceled`: Sync was canceled
- `error`: Sync failed with error

### `sync/cancel` - Cancel Current Sync

Cancel the active sync operation.

**Parameters:** None (empty object)

**Example Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "sync/cancel",
  "params": {}
}
```

**Success Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "canceled": true,
    "sync_id": "sync-550e8400-..."
  }
}
```

**Error:**
- `-32000`: No sync in progress

### `shutdown` - Shutdown RPC Server

Gracefully shutdown the RPC server.

**Parameters:** None (empty object)

**Example Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "shutdown",
  "params": {}
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {"ok": true}
}
```

## Progress Notifications

The server automatically sends `sync/progress` notifications (no `id` field) during sync operations:

```json
{"jsonrpc":"2.0","method":"sync/progress","params":{"phase":"fetching"}}
{"jsonrpc":"2.0","method":"sync/progress","params":{"phase":"exporting","exported_count":1,"total_count":20}}
{"jsonrpc":"2.0","method":"sync/progress","params":{"phase":"complete"}}
```

Clients can listen for these notifications to show real-time progress without polling.

## Ready Notification

When the RPC server starts, it sends a `ready` notification:

```json
{"jsonrpc":"2.0","method":"ready","params":{"version":"0.0.18"}}
```

## Example: Python Interactive Shell

The included Python client (`examples/rss_rpc_client.py`) provides an interactive shell for controlling the RPC server.

**Run the interactive shell:**
```bash
python examples/rss_rpc_client.py
```

**Example session:**
```
============================================================
libfec RSS RPC Interactive Shell
============================================================
Default export path: /tmp/tmpXXXXXX.db

Type 'help' for available commands, 'quit' to exit

Connected to libfec RPC server (version 0.0.18)

> help

Available commands:
  sync [options]     - Start RSS sync
  status             - Get current sync status
  cancel             - Cancel active sync
  shutdown           - Shutdown RPC server and exit
  quit/exit/q        - Exit shell

Examples:
  sync --since "1 day ago" --cover
  sync --path /tmp/custom.db --preset presidential
  sync --form F3P --state CA --since "3 days ago"

> sync --since "1 day ago" --cover

Starting sync with: {...}
Sync started: sync-a1b2c3d4-...
(Type 'status' to check progress)

  Status: Fetching RSS feed...
  Status: Exporting 5/15...
  Status: Complete!

> status

{
  "sync_id": "sync-a1b2c3d4-...",
  "phase": "complete",
  "export_summary": {
    "total_exported": 15,
    "latest_filing_id": "1234567"
  }
}

> quit
```

**Programmatic API usage:**

For automation and scripting, you can also use the client programmatically:

```python
from rss_rpc_client import LibfecRssClient
from pathlib import Path

# Spawn RPC server
client = LibfecRssClient()

# Start sync
result = client.start_sync(
    export_path=Path("/tmp/filings.db"),
    cover_only=True,
    since="1 day ago",
    preset="presidential"
)

# Poll status until complete
while True:
    status = client.get_status()
    if status["phase"] == "complete":
        break

# Shutdown
client.shutdown()
```

## Manual Testing

You can test the RPC mode manually using stdin/stdout:

```bash
# Start RPC mode
libfec rss --rpc

# In another terminal, send commands:
echo '{"jsonrpc":"2.0","id":1,"method":"sync/status","params":{}}' | libfec rss --rpc

# Or interactively:
libfec rss --rpc
{"jsonrpc":"2.0","id":1,"method":"sync/start","params":{"export_path":"/tmp/test.db","cover_only":true}}
{"jsonrpc":"2.0","id":2,"method":"sync/status","params":{}}
{"jsonrpc":"2.0","id":3,"method":"shutdown","params":{}}
```

## Error Codes

Standard JSON-RPC 2.0 error codes:
- `-32700`: Parse error (invalid JSON)
- `-32601`: Method not found
- `-32602`: Invalid params

Application-specific error codes:
- `-32000`: No sync in progress (for cancel)
- `-32001`: Sync already in progress
- `-32002`: Failed to initialize database or invalid export_path

## Metadata Tracking

When `write_metadata: true` is passed to `sync/start`, the server creates additional tables to track sync operations and per-filing RSS metadata.

### Metadata Tables

**`libfec_rss_syncs`** - Tracks each RSS sync operation:
| Column | Type | Description |
|--------|------|-------------|
| `sync_id` | INTEGER | Auto-incrementing primary key |
| `sync_uuid` | TEXT | UUID string matching the RPC sync_id |
| `created_at` | TEXT | ISO 8601 timestamp when sync started |
| `completed_at` | TEXT | ISO 8601 timestamp when sync completed |
| `feed_url` | TEXT | RSS feed URL used |
| `feed_title` | TEXT | RSS feed title |
| `feed_last_modified` | TEXT | HTTP Last-Modified header |
| `since_filter` | TEXT | --since filter value if used |
| `preset_filter` | TEXT | Preset filter (all, monthly, etc.) |
| `form_type_filter` | TEXT | Form type filter if used |
| `committee_filter` | TEXT | Committee filter if used |
| `state_filter` | TEXT | State filter if used |
| `party_filter` | TEXT | Party filter if used |
| `total_feed_items` | INTEGER | Total items in RSS feed |
| `filtered_items` | INTEGER | Items after --since filter |
| `new_filings_count` | INTEGER | Filings not already in DB |
| `exported_count` | INTEGER | Successfully exported count |
| `cover_only` | INTEGER | 1 if cover-only mode |
| `status` | TEXT | started, complete, error, canceled |
| `error_message` | TEXT | Error message if failed |

**`libfec_rss_filings`** - Tracks each filing with RSS-specific metadata:
| Column | Type | Description |
|--------|------|-------------|
| `sync_id` | INTEGER | Foreign key to libfec_rss_syncs |
| `filing_id` | TEXT | FEC filing ID |
| `rss_pub_date` | TEXT | Publication date from RSS feed |
| `ingested_at` | TEXT | When we processed this item |
| `rss_guid` | TEXT | RSS item GUID |
| `rss_title` | TEXT | RSS item title |
| `committee_id` | TEXT | Committee ID from RSS |
| `form_type` | TEXT | Form type from RSS |
| `coverage_from` | TEXT | Coverage start date |
| `coverage_through` | TEXT | Coverage end date |
| `report_type` | TEXT | Report type from RSS |
| `export_success` | INTEGER | 1 if export succeeded |
| `export_message` | TEXT | Error message if failed |

### Example Metadata Queries

```sql
-- Was this filing imported from RSS? When?
SELECT rss_pub_date, ingested_at, form_type, committee_id
FROM libfec_rss_filings
WHERE filing_id = '1234567';

-- What's the lag between FEC publish time and ingestion?
SELECT filing_id,
       rss_pub_date,
       ingested_at,
       (julianday(ingested_at) - julianday(rss_pub_date)) * 24 * 60 AS lag_minutes
FROM libfec_rss_filings
WHERE sync_id = 1;

-- Summary of a sync operation
SELECT sync_uuid, status, total_feed_items, filtered_items,
       new_filings_count, exported_count, completed_at
FROM libfec_rss_syncs
WHERE sync_uuid = 'sync-550e8400-...';

-- All filings from a specific sync with timing data
SELECT f.filing_id, f.rss_pub_date, f.ingested_at, f.form_type, s.preset_filter
FROM libfec_rss_filings f
JOIN libfec_rss_syncs s ON f.sync_id = s.sync_id
WHERE s.sync_uuid = 'sync-550e8400-...';
```

## Implementation Details

- **Concurrency**: Single sync at a time, single-threaded cooperative multitasking
- **Export Processing**: Processes up to 10 exports per request cycle to avoid blocking
- **Progress Updates**: Sent every 5 exports during the exporting phase
- **State Reuse**: Leverages existing `App` struct patterns for export queue and progress tracking
- **Sync Type**: One-shot syncs only (no auto-refresh like watch mode)

## Testing

Integration tests are located in `crates/fec-cli/tests/rss_rpc_integration.rs`:

```bash
# Run all RPC integration tests
cargo test -p fec-cli --test rss_rpc_integration

# Run specific test
cargo test -p fec-cli --test rss_rpc_integration test_rpc_sync_start
```

Note: Some tests involving full sync flows are commented out due to network dependencies and timing variability. Manual testing with the Python client is recommended for end-to-end verification.
