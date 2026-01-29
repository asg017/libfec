# Search RPC Mode

The `libfec search --rpc` command provides a JSON-RPC 2.0 server for programmatic searching of FEC candidates and committees over stdio using a JSONL (newline-delimited JSON) protocol.

## Overview

External processes can spawn `libfec search --rpc` and interact with it by:
- Sending JSON-RPC requests to stdin
- Reading JSON-RPC responses from stdout

This enables:
- Programmatic search of FEC bulk data
- Integration with other tools and languages
- Building custom search interfaces

## Quick Start

```bash
# Start the RPC server
libfec search --rpc --cycle 2024

# Send a search request (in another terminal or via pipe)
echo '{"jsonrpc":"2.0","id":1,"method":"search/query","params":{"query":"biden"}}' | libfec search --rpc
```

## Protocol

### Message Format

All messages are JSONL - one JSON object per line.

#### Request (stdin → server)
```json
{"jsonrpc":"2.0","id":1,"method":"search/query","params":{...}}
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
{"jsonrpc":"2.0","method":"ready","params":{...}}
```

## Startup

When the RPC server starts, it sends a `ready` notification:

```json
{"jsonrpc":"2.0","method":"ready","params":{"version":"0.0.18","default_cycle":2026}}
```

**Ready Parameters:**
- `version` (string): libfec version
- `default_cycle` (number): Default election cycle from CLI args

## JSON-RPC Methods

### `search/query` - Search Candidates and Committees

Search for candidates and committees by name.

**Parameters:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | Yes | Search query string |
| `cycle` | number | No | Election cycle year (uses default if omitted) |
| `limit` | number | No | Max results per category (default: 100) |

**Example Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "search/query",
  "params": {
    "query": "biden",
    "cycle": 2024,
    "limit": 50
  }
}
```

**Success Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "cycle": 2024,
    "query": "biden",
    "candidate_count": 1,
    "committee_count": 8,
    "candidates": [
      {
        "candidate_id": "P80000722",
        "name": "BIDEN, JOSEPH R JR",
        "election_year": 2024,
        "office": "P",
        "state": "US",
        "district": "00",
        "principal_campaign_committee": "C00703975"
      }
    ],
    "committees": [
      {
        "committee_id": "C00762435",
        "name": "THE PEOPLE FOR THE REMOVAL OF JOE BIDEN PAC",
        "committee_type": "O",
        "designation": "U",
        "party_affiliation": "",
        "connected_org_name": "",
        "candidate_id": null
      }
    ]
  }
}
```

**Candidate Object Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `candidate_id` | string | FEC candidate ID (e.g., "P80000722") |
| `name` | string | Candidate name |
| `election_year` | number | Election year |
| `office` | string | Office: "P" (President), "S" (Senate), "H" (House) |
| `state` | string | State code or "US" for president |
| `district` | string | District number ("00" for president/senate) |
| `principal_campaign_committee` | string\|null | Primary committee ID |

**Committee Object Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `committee_id` | string | FEC committee ID (e.g., "C00703975") |
| `name` | string | Committee name |
| `committee_type` | string | Type code (see Committee Types below) |
| `designation` | string | Designation code |
| `party_affiliation` | string | Party code (e.g., "DEM", "REP") |
| `connected_org_name` | string | Connected organization name |
| `candidate_id` | string\|null | Associated candidate ID |

---

### `search/candidate` - Get Candidate Details

Get detailed information about a specific candidate.

**Parameters:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `candidate_id` | string | Yes | FEC candidate ID |
| `cycle` | number | No | Election cycle year |

**Example Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "search/candidate",
  "params": {
    "candidate_id": "P80000722",
    "cycle": 2024
  }
}
```

**Success Response (found):**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "found": true,
    "cycle": 2024,
    "candidate": {
      "candidate_id": "P80000722",
      "name": "BIDEN, JOSEPH R JR",
      "party_affiliation": "DEM",
      "election_year": 2024,
      "state": "US",
      "office": "P",
      "district": "00",
      "principal_campaign_committee": "C00703975",
      "address": {
        "street1": "PO BOX 58174",
        "street2": "",
        "city": "PHILADELPHIA",
        "state": "PA",
        "zip": "19102"
      }
    }
  }
}
```

**Response (not found):**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "found": false,
    "cycle": 2024,
    "candidate_id": "P00000000"
  }
}
```

---

### `search/committee` - Get Committee Details

Get detailed information about a specific committee.

**Parameters:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `committee_id` | string | Yes | FEC committee ID |
| `cycle` | number | No | Election cycle year |

**Example Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "search/committee",
  "params": {
    "committee_id": "C00703975",
    "cycle": 2024
  }
}
```

**Success Response (found):**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "found": true,
    "cycle": 2024,
    "committee": {
      "committee_id": "C00703975",
      "name": "HARRIS FOR PRESIDENT",
      "treasurer_name": "SPENCER, KEANA",
      "designation": "P",
      "committee_type": "P",
      "party_affiliation": "DEM",
      "filing_frequency": "M",
      "interest_group_category": "",
      "connected_org_name": "HARRIS VICTORY FUND",
      "candidate_id": "P00009423",
      "address": {
        "street1": "PO BOX 58174",
        "street2": "",
        "city": "PHILADELPHIA",
        "state": "PA",
        "zip": "19102"
      },
      "fec_url": "https://www.fec.gov/data/committee/C00703975/"
    }
  }
}
```

**Response (not found):**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "found": false,
    "cycle": 2024,
    "committee_id": "C00000000"
  }
}
```

---

### `shutdown` - Gracefully Exit

Shutdown the RPC server.

**Parameters:** None

**Example Request:**
```json
{"jsonrpc":"2.0","id":99,"method":"shutdown","params":{}}
```

**Response:**
```json
{"jsonrpc":"2.0","id":99,"result":{"ok":true}}
```

## Error Codes

| Code | Name | Description |
|------|------|-------------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid request | Request object invalid |
| -32601 | Method not found | Unknown method name |
| -32602 | Invalid params | Invalid method parameters |
| -32000 | Database error | Error accessing bulk data |

**Example Error Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32602,
    "message": "Invalid params",
    "data": {"details": "missing field `query`"}
  }
}
```

## Reference Tables

### Committee Type Codes

| Code | Description |
|------|-------------|
| P | Presidential |
| H | House |
| S | Senate |
| C | Corporation |
| L | Labor Organization |
| M | Membership Organization |
| T | Trade Association |
| V | Cooperative |
| W | Corporation Without Capital Stock |
| N | PAC - Nonqualified |
| Q | PAC - Qualified |
| I | Independent Expenditure Filer (Person) |
| O | Super PAC (Independent Expenditure-Only) |
| U | Single Candidate Independent Expenditure |
| X | Party - Nonqualified |
| Y | Party - Qualified |
| Z | National Party Nonfederal Account |
| E | Electioneering Communications |
| D | Delegate Committee |

### Committee Designation Codes

| Code | Description |
|------|-------------|
| A | Authorized by a candidate |
| J | Joint fundraising committee |
| P | Principal campaign committee |
| U | Unauthorized |
| B | Lobbyist/Registrant PAC |
| D | Leadership PAC |

### Office Codes

| Code | Description |
|------|-------------|
| P | President |
| S | Senate |
| H | House of Representatives |

## Client Examples

### Bash

```bash
#!/bin/bash

# Start RPC server and keep it running
coproc LIBFEC { libfec search --rpc --cycle 2024; }

# Read ready notification
read -r ready <&${LIBFEC[0]}
echo "Server ready: $ready"

# Send search request
echo '{"jsonrpc":"2.0","id":1,"method":"search/query","params":{"query":"trump"}}' >&${LIBFEC[1]}
read -r response <&${LIBFEC[0]}
echo "Results: $response"

# Shutdown
echo '{"jsonrpc":"2.0","id":2,"method":"shutdown","params":{}}' >&${LIBFEC[1]}
```

### Python

```python
import subprocess
import json

class SearchClient:
    def __init__(self, cycle=2026):
        self.proc = subprocess.Popen(
            ["libfec", "search", "--rpc", "--cycle", str(cycle)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
            bufsize=1
        )
        # Read ready notification
        self.ready = json.loads(self.proc.stdout.readline())
        self.req_id = 0

    def request(self, method, params):
        self.req_id += 1
        req = {"jsonrpc": "2.0", "id": self.req_id, "method": method, "params": params}
        self.proc.stdin.write(json.dumps(req) + "\n")
        self.proc.stdin.flush()
        return json.loads(self.proc.stdout.readline())

    def search(self, query, cycle=None, limit=100):
        params = {"query": query, "limit": limit}
        if cycle:
            params["cycle"] = cycle
        return self.request("search/query", params)

    def get_candidate(self, candidate_id, cycle=None):
        params = {"candidate_id": candidate_id}
        if cycle:
            params["cycle"] = cycle
        return self.request("search/candidate", params)

    def get_committee(self, committee_id, cycle=None):
        params = {"committee_id": committee_id}
        if cycle:
            params["cycle"] = cycle
        return self.request("search/committee", params)

    def close(self):
        self.request("shutdown", {})
        self.proc.wait()

# Usage
client = SearchClient(cycle=2024)
results = client.search("harris")
print(f"Found {results['result']['candidate_count']} candidates")
client.close()
```

### Node.js

```javascript
const { spawn } = require('child_process');
const readline = require('readline');

class SearchClient {
  constructor(cycle = 2026) {
    this.proc = spawn('libfec', ['search', '--rpc', '--cycle', String(cycle)]);
    this.rl = readline.createInterface({ input: this.proc.stdout });
    this.reqId = 0;
    this.pending = new Map();

    this.rl.on('line', (line) => {
      const msg = JSON.parse(line);
      if (msg.id && this.pending.has(msg.id)) {
        this.pending.get(msg.id)(msg);
        this.pending.delete(msg.id);
      }
    });
  }

  request(method, params) {
    return new Promise((resolve) => {
      this.reqId++;
      this.pending.set(this.reqId, resolve);
      const req = { jsonrpc: '2.0', id: this.reqId, method, params };
      this.proc.stdin.write(JSON.stringify(req) + '\n');
    });
  }

  async search(query, cycle, limit = 100) {
    const params = { query, limit };
    if (cycle) params.cycle = cycle;
    return this.request('search/query', params);
  }

  async close() {
    await this.request('shutdown', {});
    this.proc.kill();
  }
}

// Usage
(async () => {
  const client = new SearchClient(2024);
  const results = await client.search('sanders');
  console.log(`Found ${results.result.candidate_count} candidates`);
  await client.close();
})();
```

## CLI Options

```
libfec search --rpc [OPTIONS]

Options:
      --cycle <CYCLE>  Election cycle year to search [default: 2026]
      --rpc            Enable RPC mode for programmatic control
  -h, --help           Print help
```

## See Also

- [RSS RPC Mode](./rss-rpc-mode.md) - RPC mode for RSS feed syncing
- [Export RPC Mode](./export-rpc-mode.md) - RPC mode for filing exports
- [FEC Bulk Data](https://www.fec.gov/data/browse-data/?tab=bulk-data) - Source data
