use crate::cli::RssArgs;
use crate::commands::export::sqlite;
use crate::sourcer::FilingSourcer;
use crate::tui::filing_detail::{
    render_filing_detail, FilingDetail, FilingDetailAction, FilingDetailState,
};
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use jiff::Timestamp;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::HashSet;
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use super::app::{App, CopyOption, SearchMode};
use super::export::open_or_create_export_db;
use super::render::ui;

/// Returns the current FEC election cycle (current year rounded up to even).
fn current_cycle() -> u16 {
    let year = jiff::Zoned::now().year() as u16;
    year + (year % 2)
}

/// Watch mode: interactive TUI with auto-refresh
pub fn run_watch_mode(
    mut sourcer: FilingSourcer,
    args: &RssArgs,
    since_ts: Option<Timestamp>,
) -> Result<()> {
    // Set up export database if -x flag is provided
    let (export_db, exported_ids) = if let Some(ref export_path) = args.export {
        let mut db = open_or_create_export_db(export_path)?;
        sqlite::init_schema(&mut db)?;

        // Import bulk candidate/committee data before entering the TUI
        if args.include_all_bulk {
            import_bulk_data(&mut sourcer, &mut db)?;
        }

        let ids = sqlite::get_existing_filing_ids(&db).unwrap_or_default();
        (Some(db), ids)
    } else {
        (None, HashSet::new())
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(args.clone(), sourcer, export_db, exported_ids, since_ts);

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {err:?}");
        return Err(err);
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    loop {
        // Check if it's time to fetch
        if Instant::now() >= app.next_fetch {
            app.fetch()?;
        }

        // Process one pending export (if any)
        if app.has_pending_exports() {
            app.process_one_export();
        }

        // Draw UI
        terminal.draw(|f| ui(f, app))?;

        if app.should_exit {
            return Ok(());
        }

        // Clear status message after displaying (but not during active exports)
        if app.status_message.is_some() && !app.has_pending_exports() {
            app.status_message = None;
        }

        // Use shorter poll timeout when exporting to keep UI responsive
        let poll_timeout = if app.has_pending_exports() {
            Duration::from_millis(10)
        } else {
            Duration::from_secs(1)
        };

        // Poll for events
        if event::poll(poll_timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Handle copy menu input separately
                if app.copy_menu_open {
                    handle_copy_menu_input(app, key.code);
                    app.last_key = None;
                    continue;
                }

                // Check for Ctrl+C
                if key.code == KeyCode::Char('c')
                    && key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    app.should_exit = true;
                    continue;
                }

                // Handle search typing mode
                if app.search_mode == SearchMode::Typing {
                    handle_search_input(app, key.code);
                    app.last_key = None;
                    continue;
                }

                handle_main_input(terminal, app, key.code)?;
                app.last_key = Some(key.code);
            }
        }
    }
}

fn handle_copy_menu_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('y') => {
            app.copy_menu_open = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.copy_menu_prev();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.copy_menu_next();
        }
        KeyCode::Enter => {
            let option = CopyOption::all()[app.copy_menu_selection];
            app.copy_selected(option);
        }
        KeyCode::Char('1') => app.copy_selected(CopyOption::FilingId),
        KeyCode::Char('2') => app.copy_selected(CopyOption::CommitteeId),
        KeyCode::Char('3') => app.copy_selected(CopyOption::RssGuid),
        _ => {}
    }
}

fn handle_search_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.cancel_search(),
        KeyCode::Enter => app.lock_search(),
        KeyCode::Backspace => app.search_pop_char(),
        KeyCode::Up => app.select_previous(),
        KeyCode::Down => app.select_next(),
        KeyCode::Char(c) => app.search_push_char(c),
        _ => {}
    }
}

fn handle_main_input(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    key: KeyCode,
) -> Result<()> {
    match key {
        KeyCode::Char('q') => {
            app.should_exit = true;
        }
        KeyCode::Esc => {
            if app.search_mode == SearchMode::Locked {
                app.cancel_search();
            } else {
                app.should_exit = true;
            }
        }
        KeyCode::Char('/') => {
            app.start_search();
        }
        KeyCode::Char('r') => {
            app.fetch()?;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.select_previous();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.select_next();
        }
        KeyCode::Home => {
            app.select_first();
        }
        KeyCode::End | KeyCode::Char('G') => {
            app.select_last();
        }
        KeyCode::Char('g') => {
            if app.last_key == Some(KeyCode::Char('g')) {
                app.select_first();
                app.last_key = None;
                return Ok(());
            }
        }
        KeyCode::Char('y') => {
            if app.get_selected_item().is_some() {
                app.copy_menu_open = true;
                app.copy_menu_selection = 0;
            }
        }
        KeyCode::Enter => {
            let filing_id = app
                .get_selected_item()
                .and_then(|item| item.filing_id.clone());
            if let Some(filing_id) = filing_id {
                match show_filing_detail(terminal, &mut app.sourcer, &filing_id) {
                    Ok(true) => {
                        app.should_exit = true;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        app.status_message = Some(format!("Error: {}", e));
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Returns `Ok(true)` if the user pressed Ctrl+C (force quit).
fn show_filing_detail(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    sourcer: &mut FilingSourcer,
    filing_id: &str,
) -> Result<bool> {
    let filing = sourcer.resolve_from_user_argument(filing_id)?;
    let detail = FilingDetail::from(&filing);
    let mut state = FilingDetailState::new();
    let mut force_quit = false;

    loop {
        terminal.draw(|f| {
            render_filing_detail(f, f.area(), &detail, &state);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Ctrl+C exits immediately
            if key.code == KeyCode::Char('c')
                && key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                force_quit = true;
                break;
            }

            match state.handle_key_event(key, &detail) {
                FilingDetailAction::Exit => break,
                FilingDetailAction::OpenBrowser => {
                    let _ = detail.open_in_browser();
                }
                FilingDetailAction::OpenWebsite { url } => {
                    let _ = open::that(&url);
                }
                FilingDetailAction::ShowFiler { .. } | FilingDetailAction::None => {}
            }
        }
    }

    Ok(force_quit)
}

/// Import all bulk candidate/committee data for the current cycle into the export database.
/// This runs before entering the TUI so download progress is printed to stdout.
fn import_bulk_data(sourcer: &mut FilingSourcer, db: &mut rusqlite::Connection) -> Result<()> {
    use crate::cache::bulk::{candidates, committee};

    let cycle = current_cycle();
    eprintln!(
        "Importing bulk candidate/committee data for cycle {}...",
        cycle
    );

    // Sync bulk data into the cache database (downloads if needed)
    let mut bulk_db = sourcer
        .cache
        .open_bulk_data_database()
        .context("Could not open bulk data database")?;
    let mut bulk_tx = bulk_db
        .transaction()
        .context("Could not start bulk data transaction")?;
    candidates::export(&mut bulk_tx, cycle, None, false)
        .with_context(|| format!("Error syncing candidate data for cycle {}", cycle))?;
    committee::export(&mut bulk_tx, cycle, None, false)
        .with_context(|| format!("Error syncing committee data for cycle {}", cycle))?;
    bulk_tx.commit()?;
    drop(bulk_db);

    // Include bulk data in the export database
    let bulk_db_path = sourcer.cache.bulk_data_database_path();
    let mut tx = db
        .transaction()
        .context("Could not start export transaction for bulk data")?;
    let params = candidates::ResolveCandidateParams {
        cycle,
        office: None,
        state: None,
        district: None,
    };
    candidates::include(&mut tx, bulk_db_path.clone(), &params)
        .with_context(|| format!("Error including candidates for cycle {}", cycle))?;
    committee::include(&mut tx, bulk_db_path, cycle)
        .with_context(|| format!("Error including committees for cycle {}", cycle))?;
    tx.commit()?;

    eprintln!("Bulk data imported.");
    Ok(())
}
