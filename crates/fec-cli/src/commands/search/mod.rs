/*!
 * Interactive TUI for searching FEC candidates and committees
 *
 * This module provides a terminal user interface (TUI) for searching through FEC candidate
 * and committee data using ratatui. It allows users to search by name and view detailed
 * information about campaigns and committees.
 *
 * ## Features
 *
 * - **Real-time dual search**: As you type, both candidate and committee results are fetched
 *   from the local SQLite bulk data cache and displayed instantly with query timing information.
 *   Both result sets are queried simultaneously.
 *
 * - **Tabbed interface**: Results are displayed in tabs showing "Candidates (N)" and "Committees (N)"
 *   where N is the count of results. The active tab is highlighted in green. Default tab is Candidates.
 *
 * - **Cycle selection**: Users can switch between election cycles (2000-present) to search
 *   candidates and committees in different election years. Default cycle is 2026.
 *
 * - **Focus management**: Three focusable panels (Search, Cycle, Results) with Tab/Shift+Tab
 *   navigation. Green borders indicate the currently focused panel.
 *
 * - **Keyboard navigation**:
 *   - Tab/Shift+Tab: Cycle through panels (Search → Cycle → Results → Search)
 *   - Ctrl+A: Switch to Candidates tab
 *   - Ctrl+B: Switch to Committees tab
 *   - ↑/↓: Navigate results or adjust cycle year (depending on focus)
 *   - Typing: Updates search input from any panel and refocuses Search
 *   - Enter: Select candidate/committee and display detailed information
 *   - PageUp/PageDown: Quick cycle year adjustment from any panel
 *   - q/Esc: Quit the application
 *
 * - **Candidate Results table**: Displays candidate information in a structured table with columns:
 *   - Candidate ID (e.g., P00003392)
 *   - Name (e.g., BIDEN, JOSEPH R JR)
 *   - Election Year
 *   - Office (H=House, S=Senate, P=President)
 *   - State/District (e.g., CA-12 for House, CA for Senate)
 *   - Principal Campaign Committee ID
 *
 * - **Committee Results table**: Displays committee information in a structured table with columns:
 *   - Committee ID (e.g., C00401224)
 *   - Name (e.g., BIDEN FOR PRESIDENT)
 *   - Type (committee type code)
 *   - Desig (designation, e.g., P for Principal)
 *   - Party (party affiliation)
 *   - Org (connected organization name)
 *   - Candidate (candidate ID if connected to a candidate)
 *
 * - **Detailed view**: When a candidate/committee is selected (Enter key), the TUI exits and displays
 *   detailed information about the selection.
 *
 * ## Data Source
 *
 * Searches are performed against the local bulk data cache maintained in `.bulk-data.db`.
 * The cache is automatically synced when searching a new cycle. Data comes from FEC's bulk
 * candidate and committee master files.
 *
 * ## Module Structure
 *
 * - `app`: Application state and business logic
 * - `events`: Event loop and keyboard input handling
 * - `ui`: UI coordination and chrome widgets (breadcrumb, search bar, tabs, help)
 * - `candidates_table`: Candidate results table renderer
 * - `committees_table`: Committee results table renderer
 * - `opexp_table`: Operating expenses table renderer
 * - `tests`: Integration tests
 */

mod app;
mod candidates_table;
mod committees_table;
mod events;
mod opexp_table;
mod plain_text;
mod rpc;
mod ui;

#[cfg(test)]
mod tests;

use crate::{cli::SearchArgs, sourcer::FilingSourcer};
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, IsTerminal};

/// Entry point for the search command
pub fn search(mut sourcer: FilingSourcer, args: &SearchArgs) -> Result<()> {
    // RPC mode: JSON-RPC server over stdio
    if args.rpc {
        return rpc::run_rpc_mode(sourcer, args);
    }

    // Non-TTY mode: print plain text table if query is provided
    if !io::stdout().is_terminal() {
        if args.query.is_empty() {
            anyhow::bail!("query required when not running in a terminal");
        }
        return plain_text::print_table(&mut sourcer, args);
    }

    // TUI mode: interactive terminal interface
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new(args.cycle, args.query.clone());
    app.cursor_position = app.input.len();
    app.search(&mut sourcer)?;

    let res = events::run_app(&mut terminal, &mut app, &mut sourcer);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {err:?}");
        return Err(err);
    }

    Ok(())
}

