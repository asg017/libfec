# Export RPC Mode

The `libfec export --rpc -o <path.db>` command provides a JSON-RPC 2.0 server for programmatic control of FEC filing exports over stdio using a JSONL (newline-delimited JSON) protocol.

## Overview

External processes can spawn `libfec export --rpc -o output.db` and interact with it by:
- Sending JSON-RPC requests to stdin
- Reading JSON-RPC responses and notifications from stdout

This enables:
- Programmatic export of FEC filings to SQLite
- Real-time progress notifications during multi-filing exports
- Incremental exports (automatically skips already-exported filings)
- Bulk data downloads (candidates, committees, linkages)
- Integration with other tools and languages

## Starting the RPC Server

```bash
libfec export --rpc -o /path/to/output.db
```

**Required flags:**
- `--rpc`: Enable RPC mode
- `-o <path>`: Output SQLite database path (must have `.db` extension)

**Optional flags:**
- `--clobber`: Allow overwriting existing database (can also be set per-request)
- `--cover-only`: Only export cover records by default (can be overridden per-request)
- `--write-metadata`: Write export metadata to the database (export ID, input mappings, filing list)

## Protocol

### Message Format

All messages are JSONL - one JSON object per line.

#### Request (stdin → server)
```json
{"jsonrpc":"2.0","id":1,"method":"export/start","params":{...}}
```

#### Response (server → stdout)
```json
{"jsonrpc":"2.0","id":1,"result":{...}}
```

#### Error Response (server → stdout)
```json
{"jsonrpc":"2.0","id":1,"error":{"code":-32001,"message":"Export already in progress"}}
```

#### Notification (server → stdout, no id)
```json
{"jsonrpc":"2.0","method":"export/progress","params":{...}}
```

## JSON-RPC Methods

### `export/start` - Start Export Operation

Start a new export operation with the specified parameters.

**Parameters:**
- `filings` (array of strings, optional): Filing IDs, committee IDs, or candidate IDs to export
- `cycle` (integer, optional): Election cycle year (e.g., 2024) - used when resolving committees/candidates
- `cover_only` (bool, optional): Only export cover records, not itemizations (default: false)
- `clobber` (bool, optional): Overwrite existing database content (default: false)
- `write_metadata` (bool, optional): Write export metadata to the database (default: false)

**Filing ID formats supported:**
- Numeric filing IDs: `"1884420"`, `"1884421"`
- Committee IDs: `"C00401224"` (9 characters starting with C)
- Candidate IDs: `"P00009423"`, `"H6CA12345"`, `"S2NY00001"` (9 characters starting with P/H/S)

**Example Request - Export specific filings:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "export/start",
  "params": {
    "filings": ["1884420", "1884421", "1884422"],
    "cover_only": true
  }
}
```

**Example Request - Export all filings for a committee:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "export/start",
  "params": {
    "filings": ["C00401224"],
    "cycle": 2024
  }
}
```

**Example Request - Export with clobber:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "export/start",
  "params": {
    "filings": ["1884420"],
    "clobber": true
  }
}
```

**Example Request - Export with metadata tracking:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "export/start",
  "params": {
    "filings": ["C00401224"],
    "cycle": 2024,
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
    "export_id": "export-550e8400-e29b-41d4-a716-446655440000",
    "status": "started"
  }
}
```

**Success Response (with `write_metadata: true`):**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "export_id": "export-550e8400-e29b-41d4-a716-446655440000",
    "status": "started",
    "metadata_export_id": 1
  }
}
```

The `metadata_export_id` is the auto-incrementing primary key from the `libfec_exports` table, useful for querying export metadata.

**Error Codes:**
- `-32001`: Export already in progress
- `-32002`: Failed to open database or invalid output path
- `-32003`: Failed to resolve filings
- `-32602`: Invalid parameter format

### `export/status` - Get Current Export Status

Get the current status of the export operation.

**Parameters:** None (empty object)

**Example Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "export/status",
  "params": {}
}
```

**Response When Idle:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "export_id": null,
    "phase": "idle"
  }
}
```

**Response When Sourcing (resolving filing IDs):**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "export_id": "export-550e8400-...",
    "phase": "sourcing"
  }
}
```

**Response When Downloading Bulk Data:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "export_id": "export-550e8400-...",
    "phase": "downloading_bulk",
    "completed": 2,
    "total": 6,
    "current": "2024 committees"
  }
}
```

**Response When Exporting:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "export_id": "export-550e8400-...",
    "phase": "exporting",
    "completed": 50,
    "total": 200,
    "current_filing_id": "1884420"
  }
}
```

**Response When Complete:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "export_id": "export-550e8400-...",
    "phase": "complete",
    "total_exported": 200,
    "warnings": ["Filing 1234567: HTTP 403 Forbidden"]
  }
}
```

**Response When Complete (with metadata):**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "export_id": "export-550e8400-...",
    "phase": "complete",
    "total_exported": 200,
    "warnings": [],
    "metadata_export_id": 1
  }
}
```

**Response When Error:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "export_id": "export-550e8400-...",
    "phase": "error",
    "error_message": "Database write failed"
  }
}
```

### `export/cancel` - Cancel Current Export

Cancel the active export operation.

**Parameters:** None (empty object)

**Example Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "export/cancel",
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
    "export_id": "export-550e8400-..."
  }
}
```

**Error:**
- `-32000`: No export in progress

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

## Export Phases

The export operation progresses through these phases:

| Phase | Description |
|-------|-------------|
| `idle` | No export in progress |
| `sourcing` | Resolving filing IDs (converting committee/candidate IDs to filing IDs) |
| `downloading_bulk` | Downloading bulk data files (candidates, committees, linkages) |
| `exporting` | Exporting individual filings to the database |
| `complete` | Export completed successfully |
| `canceled` | Export was canceled by user |
| `error` | Export failed with an error |

## Progress Notifications

The server automatically sends `export/progress` notifications (no `id` field) during export operations.

**Sourcing phase:**
```json
{"jsonrpc":"2.0","method":"export/progress","params":{"phase":"sourcing"}}
```

**Downloading bulk data:**
```json
{"jsonrpc":"2.0","method":"export/progress","params":{
  "phase":"downloading_bulk",
  "completed":2,
  "total":6,
  "current":"2024 committees"
}}
```

**Exporting filings:**
```json
{"jsonrpc":"2.0","method":"export/progress","params":{
  "phase":"exporting",
  "completed":50,
  "total":200,
  "current_filing_id":"1884420"
}}
```

**Complete:**
```json
{"jsonrpc":"2.0","method":"export/progress","params":{
  "phase":"complete",
  "total_exported":200,
  "warnings":["Filing 1234567: HTTP 403 Forbidden"]
}}
```

**Error:**
```json
{"jsonrpc":"2.0","method":"export/progress","params":{
  "phase":"error",
  "error_message":"Database connection lost"
}}
```

Clients can listen for these notifications to show real-time progress without polling.

## Ready Notification

When the RPC server starts, it sends a `ready` notification:

```json
{"jsonrpc":"2.0","method":"ready","params":{"version":"0.0.18"}}
```

## Incremental Exports

The export RPC mode supports incremental exports automatically:

1. When `export/start` is called, the server queries the database for existing filing IDs
2. Filings that already exist in the database are skipped
3. Only new filings are downloaded and exported

This allows you to run exports repeatedly without re-downloading already-exported filings.

To force a full re-export, use the `clobber` parameter:
```json
{"params": {"clobber": true, "filings": ["C00401224"]}}
```

## Bulk Data

When exporting filings that require candidate/committee resolution (e.g., when you pass a committee ID), the server automatically downloads bulk data files:

- **Candidates**: FEC candidate master file for the cycle
- **Committees**: FEC committee master file for the cycle
- **Linkages**: Candidate-committee linkage file for the cycle

These are downloaded once per cycle and cached. The bulk data is also exported to the output database in tables:
- `libfec_candidates`
- `libfec_committees`
- `libfec_candidate_committee_linkages`

## Example: Python Client

The included Python client (`examples/export_rpc_client.py`) provides an interactive shell for controlling the RPC server.

**Run the interactive shell:**
```bash
python examples/export_rpc_client.py
```

**Example session:**
```
============================================================
libfec Export RPC Interactive Shell
============================================================
Default export path: /tmp/tmpXXXXXX.db

Type 'help' for available commands, 'quit' to exit

Connected to libfec Export RPC server (version 0.0.18)

> help

Available commands:
  export [options]   - Start export operation
                       Options: --filings IDS, --cycle YEAR
                                --cover, --clobber
  status             - Get current export status
  cancel             - Cancel active export
  shutdown           - Shutdown RPC server and exit
  quit/exit/q        - Exit shell

Examples:
  export --filings 1884420,1884421
  export --filings C00401224 --cycle 2024
  export --filings 1884420 --cover

> export --filings 1884420,1884421,1884422

Starting export with: {'filings': ['1884420', '1884421', '1884422']}
Export started: export-a1b2c3d4-...
(Type 'status' to check progress)

  Status: Exporting 1/3 (FEC-1884420)...
  Status: Exporting 2/3 (FEC-1884421)...
  Status: Exporting 3/3 (FEC-1884422)...
  Status: Complete! Exported 3 filings.

> status

{
  "export_id": "export-a1b2c3d4-...",
  "phase": "complete",
  "total_exported": 3,
  "warnings": []
}

> quit
```

**Programmatic API usage:**

```python
from export_rpc_client import LibfecExportClient
from pathlib import Path

# Spawn RPC server
client = LibfecExportClient(
    output_path=Path("/tmp/filings.db"),
    libfec_path="libfec"
)

# Start export
result = client.start_export(
    filings=["1884420", "1884421", "1884422"],
    cover_only=True
)

# Poll status until complete
while True:
    status = client.get_status()
    if status["phase"] in ("complete", "error", "canceled"):
        break

# Shutdown
client.shutdown()
```

## Manual Testing

You can test the RPC mode manually using stdin/stdout:

```bash
# Start RPC mode
libfec export --rpc -o /tmp/test.db

# Server outputs ready notification:
# {"jsonrpc":"2.0","method":"ready","params":{"version":"0.0.18"}}

# Send export/start request:
{"jsonrpc":"2.0","id":1,"method":"export/start","params":{"filings":["1884420"]}}

# Server responds with export_id and sends progress notifications

# Check status:
{"jsonrpc":"2.0","id":2,"method":"export/status","params":{}}

# Shutdown:
{"jsonrpc":"2.0","id":3,"method":"shutdown","params":{}}
```

## Error Codes

Standard JSON-RPC 2.0 error codes:
- `-32700`: Parse error (invalid JSON)
- `-32601`: Method not found
- `-32602`: Invalid params

Application-specific error codes:
- `-32000`: No export in progress (for cancel)
- `-32001`: Export already in progress
- `-32002`: Failed to initialize database or invalid output path
- `-32003`: Failed to resolve filings

## Implementation Details

- **Concurrency**: Single export at a time, single-threaded cooperative multitasking
- **Export Processing**: Processes up to 10 filings per request cycle to avoid blocking stdin
- **Progress Updates**: Sent after each batch of exports during the exporting phase
- **Incremental Support**: Queries database for existing filing IDs before export
- **Bulk Data**: Automatically downloaded and synced when resolving committees/candidates
- **Error Handling**: Individual filing failures are recorded as warnings, export continues

## Database Schema

The exported SQLite database contains:

**Filing metadata:**
- `libfec_filings`: One row per filing with header information

**Itemizations (unless `cover_only` is true):**
- `libfec_schedule_a`: Schedule A (receipts/contributions)
- `libfec_schedule_b`: Schedule B (disbursements)
- `libfec_schedule_c`: Schedule C (loans)
- `libfec_schedule_d`: Schedule D (debts)
- `libfec_schedule_e`: Schedule E (independent expenditures)
- And other schedule/form type tables as needed

**Bulk data (when committees/candidates are resolved):**
- `libfec_candidates`: Candidate master data
- `libfec_committees`: Committee master data
- `libfec_candidate_committee_linkages`: Candidate-committee linkages

**Export metadata (when `write_metadata` is enabled):**
- `libfec_exports`: Export operation records with auto-incrementing `export_id`
- `libfec_export_filings`: Maps export_id to filing_id with success/failure status
- `libfec_export_inputs`: Records the original inputs (filings, committees, candidates, contests)
- `libfec_export_input_filings`: Links inputs to their resolved filing IDs

### Export Metadata Tables

When `--write-metadata` is enabled, the following tables track export operations:

**`libfec_exports`:**
| Column | Type | Description |
|--------|------|-------------|
| `export_id` | INTEGER PRIMARY KEY | Auto-incrementing unique identifier |
| `export_uuid` | TEXT | UUID string (matches `export_id` in JSON responses) |
| `created_at` | TEXT | ISO 8601 timestamp |
| `filings_count` | INTEGER | Number of filings exported |
| `cover_only` | INTEGER | Whether only cover records were exported |
| `status` | TEXT | 'started', 'complete', 'error', 'canceled' |
| `error_message` | TEXT | Error message if status is 'error' |

**`libfec_export_filings`:**
| Column | Type | Description |
|--------|------|-------------|
| `export_id` | INTEGER | Reference to libfec_exports |
| `filing_id` | TEXT | The filing ID that was exported |
| `success` | INTEGER | 1 if successful, 0 if failed |
| `message` | TEXT | Warning or error message |

**`libfec_export_inputs`:**
| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER PRIMARY KEY | Auto-incrementing identifier |
| `export_id` | INTEGER | Reference to libfec_exports |
| `input_type` | TEXT | 'filing', 'committee', 'candidate', 'contest', 'file', 'url' |
| `input_value` | TEXT | The raw input value |
| `cycle` | INTEGER | Election cycle (for contests) |
| `office` | TEXT | Office type (for contests) |
| `state` | TEXT | State code (for contests) |
| `district` | TEXT | District number (for contests) |

**`libfec_export_input_filings`:**
| Column | Type | Description |
|--------|------|-------------|
| `input_id` | INTEGER | Reference to libfec_export_inputs |
| `filing_id` | TEXT | Filing ID that this input resolved to |

## Testing

Unit tests are located in `crates/fec-cli/src/commands/export/rpc.rs`:

```bash
# Run export RPC unit tests
cargo test -p fec-cli export::rpc

# Run all fec-cli tests
cargo test -p fec-cli
```
