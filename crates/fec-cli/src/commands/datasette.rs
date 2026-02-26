use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};

use crate::cli::{DatasetteArgs, ExportArgs};
use crate::commands::export::sqlite::cmd_export_sqlite;
use crate::sourcer::{Contest, FilingSourcer};

enum DatasetteInput {
    DbFile(PathBuf),
    Filing(String),
    Committee(String),
    Contest(Contest),
}

fn classify_input(input: &str) -> DatasetteInput {
    // Check if it's an existing .db file
    let path = PathBuf::from(input);
    if path.extension().and_then(|s| s.to_str()) == Some("db") && path.exists() {
        return DatasetteInput::DbFile(path);
    }

    // Try contest shorthand
    if let Ok(Some(contest)) = Contest::from_arg(input) {
        return DatasetteInput::Contest(contest);
    }

    // Try committee pattern (C followed by 8 chars)
    if input.len() == 9 && input.starts_with('C') && input[1..].chars().all(|c| c.is_ascii_digit())
    {
        return DatasetteInput::Committee(input.to_string());
    }

    // Default to filing ID (strip FEC- prefix if present)
    let id = input.strip_prefix("FEC-").unwrap_or(input);
    DatasetteInput::Filing(id.to_string())
}

fn find_available_port(start: u16) -> Result<u16> {
    for port in start..start.saturating_add(100) {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Ok(port);
        }
    }
    Err(anyhow!(
        "Could not find an available port in range {}-{}",
        start,
        start + 100
    ))
}

fn build_url(input: &DatasetteInput, db_name: &str, port: u16) -> String {
    let base = format!("http://127.0.0.1:{port}/{db_name}");
    match input {
        DatasetteInput::Filing(id) => format!("{base}/-/libfec/filing/{id}"),
        DatasetteInput::Committee(id) => format!("{base}/-/libfec/committee/{id}"),
        DatasetteInput::Contest(contest) => {
            let params = match contest {
                Contest::President => "office=P".to_string(),
                Contest::Senate { state } => format!("state={state}&office=S"),
                Contest::House { state, district } => {
                    format!("state={state}&office=H&district={district}")
                }
            };
            format!("{base}/-/libfec/contest?{params}")
        }
        DatasetteInput::DbFile(_) => format!("{base}/-/libfec"),
    }
}

fn wait_for_server(port: u16) -> bool {
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    false
}

pub fn datasette(sourcer: FilingSourcer, args: DatasetteArgs) -> Result<()> {
    let first_input = classify_input(&args.inputs[0]);

    let (db_path, _tmp_dir) = match &first_input {
        DatasetteInput::DbFile(path) => (path.clone(), None),
        _ => {
            let tmp_dir = tempfile::TempDir::new().context("Failed to create temp directory")?;
            let db_path = tmp_dir.path().join("fec-datasette.db");

            let mut api = args.api.clone();
            // Default election to 2026 for contest inputs if unset
            if matches!(first_input, DatasetteInput::Contest(_)) && api.election.is_none() {
                api.election = Some(2026);
            }
            // Ensure cycle is set so include_all_bulk has data to include
            if api.cycle.is_none() {
                api.cycle = Some(vec![api.election.unwrap_or(2026)]);
            }

            let export_args = ExportArgs {
                filings: args.inputs.clone(),
                format: None,
                target: None,
                output: Some(db_path.clone()),
                output_directory: None,
                clobber: false,
                cover_only: args.cover_only,
                rpc: false,
                write_metadata: true,
                include_all_bulk: true,
                api,
            };

            cmd_export_sqlite(sourcer, db_path.clone(), export_args)?;
            (db_path, Some(tmp_dir))
        }
    };

    let db_name = db_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("fec-datasette");

    let port = find_available_port(args.port)?;
    let target_url = build_url(&first_input, db_name, port);

    eprintln!("Starting datasette on port {port}...");

    let port_str = port.to_string();
    let mut child = Command::new("uvx")
        .args([
            "--prerelease=allow",
            "--with",
            "datasette-libfec",
            "datasette",
            db_path.to_str().unwrap(),
            "-s",
            "permissions.datasette_libfec_access",
            "true",
            "-p",
            &port_str,
        ])
        .spawn()
        .context("Failed to start datasette. Is `uvx` installed? https://docs.astral.sh/uv/getting-started/installation/")?;

    if wait_for_server(port) {
        eprintln!("Opening {target_url}");
        let _ = open::that(&target_url);
    } else {
        eprintln!("Warning: datasette server did not become ready within 10s");
        eprintln!("Try opening manually: {target_url}");
    }

    // Block until datasette exits (Ctrl+C propagates to child)
    child.wait().context("Error waiting for datasette process")?;

    Ok(())
}
