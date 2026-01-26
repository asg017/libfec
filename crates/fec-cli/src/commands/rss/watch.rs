use crate::cli::RssArgs;
use crate::commands::export::sqlite;
use crate::sourcer::FilingSourcer;
use crate::tui::filing_detail::{render_filing_detail, FilingDetail, FilingDetailState};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::HashSet;
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use super::app::{App, CopyOption};
use super::export::open_or_create_export_db;
use super::render::ui;

/// Watch mode: interactive TUI with auto-refresh
pub fn run_watch_mode(sourcer: FilingSourcer, args: &RssArgs) -> Result<()> {
    // Set up export database if -x flag is provided
    let (export_db, exported_ids) = if let Some(ref export_path) = args.export {
        let mut db = open_or_create_export_db(export_path)?;
        sqlite::init_schema(&mut db)?;
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

    let mut app = App::new(args.clone(), sourcer, export_db, exported_ids);

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

fn handle_main_input(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    key: KeyCode,
) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_exit = true;
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
                    Ok(_) => {}
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

fn show_filing_detail(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    sourcer: &mut FilingSourcer,
    filing_id: &str,
) -> Result<()> {
    let filing = sourcer.resolve_from_user_argument(filing_id)?;
    let detail = FilingDetail::from(&filing);
    let mut state = FilingDetailState::new();

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
                break;
            }

            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    if state.show_yank_popup {
                        state.show_yank_popup = false;
                    } else {
                        break;
                    }
                }
                KeyCode::Char('y') => {
                    if !state.show_yank_popup {
                        state.show_yank_popup = true;
                    }
                }
                KeyCode::Char('o') => {
                    let _ = detail.open_in_browser();
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if state.show_yank_popup {
                        state.yank_next(&detail);
                    } else {
                        state.scroll_down();
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if state.show_yank_popup {
                        state.yank_previous(&detail);
                    } else {
                        state.scroll_up();
                    }
                }
                KeyCode::Enter => {
                    if state.show_yank_popup {
                        state.copy_selected(&detail);
                        state.show_yank_popup = false;
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
