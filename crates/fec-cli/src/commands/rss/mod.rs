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
mod simple;
mod since;
mod watch;

use crate::cli::RssArgs;
use crate::sourcer::FilingSourcer;
use anyhow::Result;

/// Entry point for the RSS command
pub fn rss(sourcer: FilingSourcer, args: &RssArgs) -> Result<()> {
    // Parse and validate --since if provided
    let since_ts = if let Some(ref since_str) = args.since {
        Some(since::parse_since(since_str)?)
    } else {
        None
    };

    if args.watch {
        watch::run_watch_mode(sourcer, args, since_ts)
    } else {
        simple::run_simple_mode(&sourcer, args, since_ts)
    }
}
