/*!
 * JSON-RPC Mode for Search Command
 *
 * Implements a JSON-RPC 2.0 server over stdio using JSONL protocol.
 * External processes can spawn `libfec search --rpc` and interact via newline-delimited JSON.
 *
 * ## Protocol
 *
 * - Request: `{"jsonrpc":"2.0","id":1,"method":"search/query","params":{...}}`
 * - Response: `{"jsonrpc":"2.0","id":1,"result":{...}}`
 *
 * ## Methods
 *
 * - `search/query`: Search candidates and committees by name
 * - `search/candidate`: Get detailed candidate information by ID
 * - `search/committee`: Get detailed committee information by ID
 * - `shutdown`: Gracefully shutdown RPC server
 */

use crate::cache::bulk::{candidates, committee};
use crate::cli::SearchArgs;
use crate::sourcer::FilingSourcer;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{self, BufRead, Write};

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

/// Parameters for search/query method
#[derive(Debug, Deserialize)]
struct SearchQueryParams {
    /// Search query string
    query: String,
    /// Election cycle year (optional, overrides CLI default)
    #[serde(default)]
    cycle: Option<u16>,
    /// Maximum results per category (optional, default 100)
    #[serde(default)]
    limit: Option<usize>,
}

/// Parameters for search/candidate method
#[derive(Debug, Deserialize)]
struct CandidateDetailParams {
    /// Candidate ID (e.g., "P00003392")
    candidate_id: String,
    /// Election cycle year (optional, overrides CLI default)
    #[serde(default)]
    cycle: Option<u16>,
}

/// Parameters for search/committee method
#[derive(Debug, Deserialize)]
struct CommitteeDetailParams {
    /// Committee ID (e.g., "C00401224")
    committee_id: String,
    /// Election cycle year (optional, overrides CLI default)
    #[serde(default)]
    cycle: Option<u16>,
}

/// Serializable candidate search result
#[derive(Debug, Serialize)]
struct CandidateResult {
    candidate_id: String,
    name: String,
    election_year: u16,
    office: String,
    state: String,
    district: String,
    principal_campaign_committee: Option<String>,
}

impl From<candidates::CandidateSearchResult> for CandidateResult {
    fn from(c: candidates::CandidateSearchResult) -> Self {
        CandidateResult {
            candidate_id: c.candidate_id,
            name: c.name,
            election_year: c.election_year,
            office: c.office,
            state: c.state,
            district: c.district,
            principal_campaign_committee: c.principal_campaign_committee,
        }
    }
}

/// Serializable candidate detail
#[derive(Debug, Serialize)]
struct CandidateDetailResult {
    candidate_id: String,
    name: String,
    party_affiliation: String,
    election_year: u16,
    state: String,
    office: String,
    district: String,
    principal_campaign_committee: Option<String>,
    address: AddressResult,
}

/// Serializable address
#[derive(Debug, Serialize)]
struct AddressResult {
    street1: String,
    street2: String,
    city: String,
    state: String,
    zip: String,
}

impl From<candidates::CandidateDetail> for CandidateDetailResult {
    fn from(c: candidates::CandidateDetail) -> Self {
        CandidateDetailResult {
            candidate_id: c.candidate_id,
            name: c.name,
            party_affiliation: c.party_affiliation,
            election_year: c.election_year,
            state: c.state,
            office: c.office,
            district: c.district,
            principal_campaign_committee: c.principal_campaign_committee,
            address: AddressResult {
                street1: c.address_street1,
                street2: c.address_street2,
                city: c.address_city,
                state: c.address_state,
                zip: c.address_zip,
            },
        }
    }
}

/// Serializable committee search result
#[derive(Debug, Serialize)]
struct CommitteeResult {
    committee_id: String,
    name: String,
    committee_type: String,
    designation: String,
    party_affiliation: String,
    connected_org_name: String,
    candidate_id: Option<String>,
}

impl From<committee::CommitteeSearchResult> for CommitteeResult {
    fn from(c: committee::CommitteeSearchResult) -> Self {
        CommitteeResult {
            committee_id: c.committee_id,
            name: c.name,
            committee_type: c.committee_type,
            designation: c.designation,
            party_affiliation: c.party_affiliation,
            connected_org_name: c.connected_org_name,
            candidate_id: c.candidate_id,
        }
    }
}

/// Serializable committee detail
#[derive(Debug, Serialize)]
struct CommitteeDetailResult {
    committee_id: String,
    name: String,
    treasurer_name: String,
    designation: String,
    committee_type: String,
    party_affiliation: String,
    filing_frequency: String,
    interest_group_category: String,
    connected_org_name: String,
    candidate_id: Option<String>,
    address: AddressResult,
    fec_url: String,
}

impl From<committee::CommitteeDetail> for CommitteeDetailResult {
    fn from(c: committee::CommitteeDetail) -> Self {
        let fec_url = c.fec_url();
        CommitteeDetailResult {
            committee_id: c.committee_id,
            name: c.name,
            treasurer_name: c.treasurer_name,
            designation: c.designation,
            committee_type: c.committee_type,
            party_affiliation: c.party_affiliation,
            filing_frequency: c.filing_frequency,
            interest_group_category: c.interest_group_category,
            connected_org_name: c.connected_org_name,
            candidate_id: c.candidate_id,
            address: AddressResult {
                street1: c.address_street1,
                street2: c.address_street2,
                city: c.address_city,
                state: c.address_state,
                zip: c.address_zip,
            },
            fec_url,
        }
    }
}

/// Main RPC mode entry point
pub fn run_rpc_mode(mut sourcer: FilingSourcer, args: &SearchArgs) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdin_lines = stdin.lock().lines();
    let mut stdout_lock = stdout.lock();

    let default_cycle = args.cycle;

    // Send ready notification
    send_notification(
        &mut stdout_lock,
        "ready",
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "default_cycle": default_cycle
        }),
    )?;

    loop {
        // Read next request
        match stdin_lines.next() {
            Some(Ok(line)) => {
                let (response, should_exit) =
                    handle_request(&line, &mut sourcer, default_cycle)?;
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
    sourcer: &mut FilingSourcer,
    default_cycle: u16,
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
        "search/query" => {
            handle_search_query(request.params, sourcer, default_cycle).map(|r| (r, false))
        }
        "search/candidate" => {
            handle_candidate_detail(request.params, sourcer, default_cycle).map(|r| (r, false))
        }
        "search/committee" => {
            handle_committee_detail(request.params, sourcer, default_cycle).map(|r| (r, false))
        }
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

/// Handle search/query method
fn handle_search_query(
    params: serde_json::Value,
    sourcer: &mut FilingSourcer,
    default_cycle: u16,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let search_params: SearchQueryParams =
        serde_json::from_value(params).map_err(|e| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: Some(json!({ "details": e.to_string() })),
        })?;

    let cycle = search_params.cycle.unwrap_or(default_cycle);
    let limit = search_params.limit.unwrap_or(100);

    if search_params.query.is_empty() {
        return Ok(json!({
            "cycle": cycle,
            "query": "",
            "candidates": [],
            "committees": [],
            "candidate_count": 0,
            "committee_count": 0
        }));
    }

    // Open bulk database
    let mut db = sourcer.cache.open_bulk_data_database().map_err(|e| JsonRpcError {
        code: -32000,
        message: "Database error".to_string(),
        data: Some(json!({ "details": e.to_string() })),
    })?;

    // Search candidates
    let candidate_results =
        candidates::search_candidates(&mut db, cycle, &search_params.query).map_err(|e| {
            JsonRpcError {
                code: -32000,
                message: "Candidate search error".to_string(),
                data: Some(json!({ "details": e.to_string() })),
            }
        })?;

    // Search committees
    let committee_results =
        committee::search_committees(&mut db, cycle, &search_params.query).map_err(|e| {
            JsonRpcError {
                code: -32000,
                message: "Committee search error".to_string(),
                data: Some(json!({ "details": e.to_string() })),
            }
        })?;

    // Convert to serializable results with limit
    let candidates: Vec<CandidateResult> = candidate_results
        .into_iter()
        .take(limit)
        .map(CandidateResult::from)
        .collect();

    let committees: Vec<CommitteeResult> = committee_results
        .into_iter()
        .take(limit)
        .map(CommitteeResult::from)
        .collect();

    Ok(json!({
        "cycle": cycle,
        "query": search_params.query,
        "candidates": candidates,
        "committees": committees,
        "candidate_count": candidates.len(),
        "committee_count": committees.len()
    }))
}

/// Handle search/candidate method
fn handle_candidate_detail(
    params: serde_json::Value,
    sourcer: &mut FilingSourcer,
    default_cycle: u16,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let detail_params: CandidateDetailParams =
        serde_json::from_value(params).map_err(|e| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: Some(json!({ "details": e.to_string() })),
        })?;

    let cycle = detail_params.cycle.unwrap_or(default_cycle);

    // Open bulk database
    let mut db = sourcer.cache.open_bulk_data_database().map_err(|e| JsonRpcError {
        code: -32000,
        message: "Database error".to_string(),
        data: Some(json!({ "details": e.to_string() })),
    })?;

    // Get candidate detail
    let detail =
        candidates::get_candidate_detail(&mut db, cycle, &detail_params.candidate_id)
            .map_err(|e| JsonRpcError {
                code: -32000,
                message: "Candidate lookup error".to_string(),
                data: Some(json!({ "details": e.to_string() })),
            })?;

    match detail {
        Some(candidate) => {
            let result: CandidateDetailResult = candidate.into();
            Ok(json!({
                "found": true,
                "cycle": cycle,
                "candidate": result
            }))
        }
        None => Ok(json!({
            "found": false,
            "cycle": cycle,
            "candidate_id": detail_params.candidate_id
        })),
    }
}

/// Handle search/committee method
fn handle_committee_detail(
    params: serde_json::Value,
    sourcer: &mut FilingSourcer,
    default_cycle: u16,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let detail_params: CommitteeDetailParams =
        serde_json::from_value(params).map_err(|e| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: Some(json!({ "details": e.to_string() })),
        })?;

    let cycle = detail_params.cycle.unwrap_or(default_cycle);

    // Open bulk database
    let mut db = sourcer.cache.open_bulk_data_database().map_err(|e| JsonRpcError {
        code: -32000,
        message: "Database error".to_string(),
        data: Some(json!({ "details": e.to_string() })),
    })?;

    // Get committee detail
    let detail =
        committee::get_committee_detail(&mut db, cycle, &detail_params.committee_id).map_err(
            |e| JsonRpcError {
                code: -32000,
                message: "Committee lookup error".to_string(),
                data: Some(json!({ "details": e.to_string() })),
            },
        )?;

    match detail {
        Some(committee) => {
            let result: CommitteeDetailResult = committee.into();
            Ok(json!({
                "found": true,
                "cycle": cycle,
                "committee": result
            }))
        }
        None => Ok(json!({
            "found": false,
            "cycle": cycle,
            "committee_id": detail_params.committee_id
        })),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_rpc_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"search/query","params":{"query":"biden"}}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "search/query");
    }

    #[test]
    fn test_parse_search_query_params() {
        let json = r#"{"query":"biden","cycle":2024,"limit":50}"#;
        let params: SearchQueryParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.query, "biden");
        assert_eq!(params.cycle, Some(2024));
        assert_eq!(params.limit, Some(50));
    }

    #[test]
    fn test_parse_search_query_params_minimal() {
        let json = r#"{"query":"trump"}"#;
        let params: SearchQueryParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.query, "trump");
        assert!(params.cycle.is_none());
        assert!(params.limit.is_none());
    }

    #[test]
    fn test_parse_candidate_detail_params() {
        let json = r#"{"candidate_id":"P00003392","cycle":2024}"#;
        let params: CandidateDetailParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.candidate_id, "P00003392");
        assert_eq!(params.cycle, Some(2024));
    }

    #[test]
    fn test_parse_committee_detail_params() {
        let json = r#"{"committee_id":"C00401224"}"#;
        let params: CommitteeDetailParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.committee_id, "C00401224");
        assert!(params.cycle.is_none());
    }

    #[test]
    fn test_serialize_json_rpc_response() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: Some(json!({"candidates": [], "committees": []})),
            error: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("candidates"));
        assert!(!json.contains("error"));
    }

    #[test]
    fn test_serialize_json_rpc_error() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
                data: None,
            }),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("-32602"));
        assert!(!json.contains("result"));
    }
}
