/*!
 * FEC RSS Feed Viewer
 *
 * This module provides both a simple CLI output and an interactive TUI for viewing
 * the FEC's RSS feed of recent electronic filings.
 *
 * ## Features
 *
 * - **Simple mode** (default): Displays a table of recent filings and exits
 * - **Watch mode** (`--watch`): Interactive TUI that auto-refreshes at configurable intervals
 *
 * ## Watch Mode TUI Features
 *
 * - Auto-refresh at configurable interval (default: 5 minutes)
 * - Shows time since each filing was submitted
 * - Shows data freshness from Last-Modified header
 * - Shows countdown to next refresh
 * - Keyboard: `q` to quit, `r` to force refresh
 */

mod app;
mod export;
mod render;
mod rpc;
mod simple;
mod since;
mod watch;

use crate::cli::RssArgs;
use crate::sourcer::FilingSourcer;
use anyhow::{Context, Result};

/// Entry point for the RSS command
pub fn rss(sourcer: FilingSourcer, args: &RssArgs) -> Result<()> {
    let mut args = args.clone();
    resolve_committee_file(&mut args)?;

    // Parse and validate --since if provided
    let since_ts = if let Some(ref since_str) = args.since {
        Some(since::parse_since(since_str)?)
    } else {
        None
    };

    if args.rpc {
        rpc::run_rpc_mode(sourcer, &args, since_ts)
    } else if args.watch {
        watch::run_watch_mode(sourcer, &args, since_ts)
    } else {
        simple::run_simple_mode(&sourcer, &args, since_ts)
    }
}

/// If `--committee` is a `.txt` file path, read it and replace with comma-separated IDs.
/// Lines starting with `#` and empty lines are skipped.
/// Sets `committee_label` to `"filename.txt (N committees)"` for display.
fn resolve_committee_file(args: &mut RssArgs) -> Result<()> {
    if let Some(ref value) = args.committee {
        if value.ends_with(".txt") {
            let filename = std::path::Path::new(value.as_str())
                .file_name()
                .unwrap_or(value.as_ref())
                .to_string_lossy();
            let contents = std::fs::read_to_string(value)
                .with_context(|| format!("Could not read committee file `{}`", value))?;
            let ids: Vec<&str> = contents
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect();
            if ids.is_empty() {
                args.committee = None;
            } else {
                let count = ids.len();
                args.committee_label = Some(format!(
                    "{} ({} committee{})",
                    filename,
                    count,
                    if count == 1 { "" } else { "s" }
                ));
                args.committee = Some(ids.join(","));
            }
        }
    }
    Ok(())
}
