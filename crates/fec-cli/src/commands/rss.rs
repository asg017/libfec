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

use crate::cli::RssArgs;
use crate::rss::{self, ActiveFilters, Item, format_countdown, format_duration_ago};
use crate::sourcer::FilingSourcer;
use crate::tui::filing_detail::{FilingDetail, FilingDetailState, render_filing_detail};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use jiff::{Timestamp, Zoned};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};
use tabled::{
    builder::Builder as TableBuilder,
    settings::Style as TableStyle,
};

/// Copy menu options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopyOption {
    FilingId,
    CommitteeId,
    RssGuid,
}

impl CopyOption {
    fn all() -> &'static [CopyOption] {
        &[CopyOption::FilingId, CopyOption::CommitteeId, CopyOption::RssGuid]
    }

    fn label(&self) -> &'static str {
        match self {
            CopyOption::FilingId => "Filing ID",
            CopyOption::CommitteeId => "Committee ID",
            CopyOption::RssGuid => "RSS GUID",
        }
    }
}

/// Application state for the RSS TUI
struct App {
    /// The fetched feed items
    items: Vec<Item>,
    /// Feed title
    feed_title: String,
    /// Last-Modified timestamp from server
    last_modified: Option<Timestamp>,
    /// When we last fetched
    last_fetch: Instant,
    /// When to fetch next
    next_fetch: Instant,
    /// Refresh interval
    interval: Duration,
    /// Max items to display
    limit: usize,
    /// Table selection state
    table_state: TableState,
    /// Whether to exit
    should_exit: bool,
    /// Error message to display
    error: Option<String>,
    /// Command-line args for fetching
    args: RssArgs,
    /// Active filters being used
    active_filters: ActiveFilters,
    /// URL used to fetch data
    feed_url: String,
    /// Filing sourcer for loading filing details
    sourcer: FilingSourcer,
    /// Last key pressed (for `gg` detection)
    last_key: Option<KeyCode>,
    /// Copy menu state
    copy_menu_open: bool,
    /// Copy menu selection index
    copy_menu_selection: usize,
    /// Status message to show briefly
    status_message: Option<String>,
}

impl App {
    fn new(args: RssArgs, sourcer: FilingSourcer) -> Self {
        let now = Instant::now();
        let interval = args.interval;
        let limit = args.limit;
        let (feed_url, _) = rss::build_feed_url(&args);
        Self {
            items: Vec::new(),
            feed_title: String::new(),
            last_modified: None,
            last_fetch: now,
            next_fetch: now, // Fetch immediately
            interval: Duration::from_secs(interval),
            limit,
            table_state: TableState::default(),
            should_exit: false,
            error: None,
            args,
            active_filters: ActiveFilters::default(),
            feed_url,
            sourcer,
            last_key: None,
            copy_menu_open: false,
            copy_menu_selection: 0,
            status_message: None,
        }
    }

    fn fetch(&mut self) -> Result<()> {
        match rss::fetch_feed_with_args(&self.args) {
            Ok((result, filters)) => {
                self.feed_title = result.feed.title;
                self.items = result.feed.items;
                self.last_modified = result.last_modified;
                self.last_fetch = Instant::now();
                self.next_fetch = self.last_fetch + self.interval;
                self.error = None;
                self.active_filters = filters;

                // Select first item if available
                if !self.items.is_empty() && self.table_state.selected().is_none() {
                    self.table_state.select(Some(0));
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
                // Still update timing so we retry
                self.last_fetch = Instant::now();
                self.next_fetch = self.last_fetch + self.interval;
            }
        }
        Ok(())
    }

    fn seconds_until_refresh(&self) -> u64 {
        let now = Instant::now();
        if now >= self.next_fetch {
            0
        } else {
            (self.next_fetch - now).as_secs()
        }
    }

    fn data_age_seconds(&self) -> i64 {
        if let Some(last_mod) = self.last_modified {
            let now = Zoned::now();
            now.timestamp().duration_since(last_mod).as_secs()
        } else {
            self.last_fetch.elapsed().as_secs() as i64
        }
    }

    fn select_next(&mut self) {
        let item_count = self.items.len();
        if item_count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= item_count - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn select_previous(&mut self) {
        let item_count = self.items.len();
        if item_count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    item_count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn select_first(&mut self) {
        if !self.items.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    fn select_last(&mut self) {
        if !self.items.is_empty() {
            self.table_state.select(Some(self.items.len() - 1));
        }
    }

    fn get_selected_item(&self) -> Option<&Item> {
        self.table_state.selected().and_then(|i| self.items.get(i))
    }

    fn copy_selected(&mut self, option: CopyOption) {
        let value = self.get_selected_item().and_then(|item| match option {
            CopyOption::FilingId => item.filing_id.clone(),
            CopyOption::CommitteeId => item.committee_id.clone(),
            CopyOption::RssGuid => Some(item.guid.clone()),
        });

        if let Some(text) = value {
            if let Ok(mut ctx) = arboard::Clipboard::new() {
                if ctx.set_text(&text).is_ok() {
                    self.status_message = Some(format!("Copied: {}", text));
                } else {
                    self.status_message = Some("Failed to copy".to_string());
                }
            } else {
                self.status_message = Some("Clipboard unavailable".to_string());
            }
        } else {
            self.status_message = Some("No value to copy".to_string());
        }
        self.copy_menu_open = false;
    }

    fn copy_menu_next(&mut self) {
        let count = CopyOption::all().len();
        self.copy_menu_selection = (self.copy_menu_selection + 1) % count;
    }

    fn copy_menu_prev(&mut self) {
        let count = CopyOption::all().len();
        self.copy_menu_selection = if self.copy_menu_selection == 0 {
            count - 1
        } else {
            self.copy_menu_selection - 1
        };
    }
}

/// Entry point for the RSS command
pub fn rss(sourcer: FilingSourcer, args: &RssArgs) -> Result<()> {
    if args.watch {
        run_watch_mode(sourcer, args)
    } else {
        run_simple_mode(args)
    }
}

/// Simple mode: fetch once and display a table, then exit
fn run_simple_mode(args: &RssArgs) -> Result<()> {
    let (url, _) = rss::build_feed_url(args);
    let (result, filters) = rss::fetch_feed_with_args(args).map_err(|e| anyhow::anyhow!("{}", e))?;
    let now = Zoned::now();

    let mut builder = TableBuilder::new();
    builder.push_record(["Committee", "Form", "Report", "Filing ID", "Age"]);

    for item in result.feed.items.iter().take(args.limit) {
        let committee = item.extract_committee_name();
        let form = item.form_type.as_deref().unwrap_or("-");
        let report = item.report_type.as_deref().unwrap_or("-");
        let filing_id = item.filing_id.as_deref().unwrap_or("-");
        let age = item.time_ago(&now).unwrap_or_else(|| "-".to_string());

        builder.push_record([committee, form, report, filing_id, &age]);
    }

    let table = builder
        .build()
        .with(TableStyle::rounded())
        .to_string();

    println!("{}", result.feed.title);
    if let Some(last_mod) = result.last_modified {
        let age = now.timestamp().duration_since(last_mod).as_secs();
        println!("Data freshness: {}", format_duration_ago(age as i64));
    }

    // Display active filters
    let filter_strs = filters.to_display_strings();
    if !filter_strs.is_empty() {
        println!("Filters: {}", filter_strs.join(" | "));
    }

    println!("URL: {}", url);
    println!();
    println!("{}", table);
    println!("\nShowing {} of {} items", args.limit.min(result.feed.items.len()), result.feed.items.len());

    Ok(())
}

/// Watch mode: interactive TUI with auto-refresh
fn run_watch_mode(sourcer: FilingSourcer, args: &RssArgs) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(args.clone(), sourcer);

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {err:?}");
        return Err(err);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Check if it's time to fetch
        if Instant::now() >= app.next_fetch {
            app.fetch()?;
        }

        // Draw UI
        terminal.draw(|f| ui(f, app))?;

        if app.should_exit {
            return Ok(());
        }

        // Clear status message after displaying
        if app.status_message.is_some() {
            app.status_message = None;
        }

        // Poll for events with 1-second timeout (for countdown updates)
        if event::poll(Duration::from_secs(1))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Handle copy menu input separately
                if app.copy_menu_open {
                    match key.code {
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
                    app.last_key = None;
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.should_exit = true;
                    }
                    KeyCode::Char('r') => {
                        app.fetch()?;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        app.should_exit = true;
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
                            continue;
                        }
                    }
                    KeyCode::Char('y') => {
                        if app.get_selected_item().is_some() {
                            app.copy_menu_open = true;
                            app.copy_menu_selection = 0;
                        }
                    }
                    KeyCode::Enter => {
                        let filing_id = app.get_selected_item()
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
                app.last_key = Some(key.code);
            }
        }
    }
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

fn ui(f: &mut Frame, app: &mut App) {
    let now = Zoned::now();

    // Check if we have filters to display
    let filter_strs = app.active_filters.to_display_strings();
    let has_filters = !filter_strs.is_empty();

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_filters {
            vec![
                Constraint::Length(3), // Header
                Constraint::Length(1), // Filters
                Constraint::Min(1),    // Table
                Constraint::Length(3), // Footer (2 lines)
            ]
        } else {
            vec![
                Constraint::Length(3), // Header
                Constraint::Min(1),    // Table
                Constraint::Length(3), // Footer (2 lines)
            ]
        })
        .split(f.area());

    // Header with title and data freshness
    let data_age = format_duration_ago(app.data_age_seconds());
    let header_text = if let Some(ref error) = app.error {
        Line::from(vec![
            Span::styled(&app.feed_title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(format!("Error: {}", error), Style::default().fg(Color::Red)),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                if app.feed_title.is_empty() { "FEC RSS Feed" } else { &app.feed_title },
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            ),
            Span::raw("  "),
            Span::styled(format!("Data: {}", data_age), Style::default().fg(Color::Gray)),
        ])
    };

    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).title("FEC RSS Feed"));
    f.render_widget(header, layout[0]);

    // Determine which layout indices to use
    let (filter_idx, table_idx, footer_idx) = if has_filters {
        (Some(1), 2, 3)
    } else {
        (None, 1, 2)
    };

    // Display active filters if any
    if let Some(idx) = filter_idx {
        let filter_spans: Vec<Span> = filter_strs
            .iter()
            .enumerate()
            .flat_map(|(i, s)| {
                let mut spans = vec![
                    Span::styled(s, Style::default().fg(Color::Yellow)),
                ];
                if i < filter_strs.len() - 1 {
                    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
                }
                spans
            })
            .collect();

        let filter_line = Line::from(
            std::iter::once(Span::styled("Filters: ", Style::default().fg(Color::Gray)))
                .chain(filter_spans)
                .collect::<Vec<_>>()
        );
        let filter_widget = Paragraph::new(filter_line);
        f.render_widget(filter_widget, layout[idx]);
    }

    // Table of items
    let header_row = Row::new(vec![
        Cell::from("Committee").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Form").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Report").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Filing ID").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Age").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ])
    .height(1);

    let rows: Vec<Row> = app
        .items
        .iter()
        .map(|item| {
            let committee = item.extract_committee_name();
            let form = item.form_type.as_deref().unwrap_or("-");
            let report = item.report_type.as_deref().unwrap_or("-");
            let filing_id = item.filing_id.as_deref().unwrap_or("-");
            let age = item.time_ago(&now).unwrap_or_else(|| "-".to_string());

            // Color code by form type
            let form_color = match form {
                f if f.starts_with("F3P") => Color::Magenta,
                f if f.starts_with("F3X") => Color::Cyan,
                f if f.starts_with("F3") => Color::Green,
                f if f.starts_with("F1") => Color::Yellow,
                f if f.starts_with("F2") => Color::Blue,
                f if f.starts_with("F99") => Color::Gray,
                _ => Color::White,
            };

            Row::new(vec![
                Cell::from(committee.to_string()),
                Cell::from(form.to_string()).style(Style::default().fg(form_color)),
                Cell::from(report.to_string()),
                Cell::from(filing_id.to_string()).style(Style::default().fg(Color::Cyan)),
                Cell::from(age).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let item_count = app.items.len();
    let table_title = if app.error.is_some() {
        "Recent Filings (error fetching)".to_string()
    } else if app.items.is_empty() {
        "Recent Filings (loading...)".to_string()
    } else {
        format!("Recent Filings ({})", item_count)
    };

    let table = Table::new(
        rows,
        [
            Constraint::Min(40),    // Committee
            Constraint::Length(6),  // Form
            Constraint::Length(20), // Report
            Constraint::Length(10), // Filing ID
            Constraint::Length(15), // Age
        ],
    )
    .header(header_row)
    .block(Block::default().borders(Borders::ALL).title(table_title))
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, layout[table_idx], &mut app.table_state);

    // Footer with countdown, help, and URL
    let countdown = format_countdown(app.seconds_until_refresh());
    let shortcut_style = Style::default().bold().fg(Color::White);
    let descrip_style = Style::default().fg(Color::DarkGray);

    let help_line = Line::from(vec![
        Span::styled(format!("Next refresh: {}", countdown), Style::default().fg(Color::Green)),
        Span::raw("  │  "),
        Span::styled("q", shortcut_style),
        Span::styled(" quit  ", descrip_style),
        Span::styled("r", shortcut_style),
        Span::styled(" refresh  ", descrip_style),
        Span::styled("↑↓/gg/G", shortcut_style),
        Span::styled(" nav  ", descrip_style),
        Span::styled("Enter", shortcut_style),
        Span::styled(" open  ", descrip_style),
        Span::styled("y", shortcut_style),
        Span::styled(" copy", descrip_style),
    ]);

    let url_line = Line::from(vec![
        Span::styled(&app.feed_url, Style::default().fg(Color::DarkGray)),
    ]);

    let footer = Paragraph::new(vec![help_line, url_line])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(footer, layout[footer_idx]);

    // Render status message if present
    if let Some(ref msg) = app.status_message {
        let area = f.area();
        let popup_area = Rect {
            x: area.width.saturating_sub(msg.len() as u16 + 4) / 2,
            y: area.height.saturating_sub(3),
            width: (msg.len() as u16 + 4).min(area.width),
            height: 1,
        };
        let status = Paragraph::new(msg.as_str())
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center);
        f.render_widget(Clear, popup_area);
        f.render_widget(status, popup_area);
    }

    // Render copy menu if open
    if app.copy_menu_open {
        render_copy_menu(f, app);
    }
}

fn render_copy_menu(f: &mut Frame, app: &App) {
    let area = f.area();
    let popup_width = 30;
    let popup_height = 7;
    let popup_area = Rect {
        x: area.width.saturating_sub(popup_width) / 2,
        y: area.height.saturating_sub(popup_height) / 2,
        width: popup_width.min(area.width),
        height: popup_height.min(area.height),
    };

    f.render_widget(Clear, popup_area);

    let selected_item = app.get_selected_item();

    let options: Vec<Line> = CopyOption::all()
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let value = selected_item.and_then(|item| match opt {
                CopyOption::FilingId => item.filing_id.as_deref(),
                CopyOption::CommitteeId => item.committee_id.as_deref(),
                CopyOption::RssGuid => Some(item.guid.as_str()),
            }).unwrap_or("-");

            let prefix = if i == app.copy_menu_selection { "▶ " } else { "  " };
            let style = if i == app.copy_menu_selection {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            Line::from(vec![
                Span::styled(format!("{}{}) ", prefix, i + 1), style),
                Span::styled(opt.label(), style),
                Span::styled(": ", Style::default().fg(Color::DarkGray)),
                Span::styled(truncate_str(value, 10), Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();

    let menu = Paragraph::new(options)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Copy (↑↓/Enter or 1-3)")
                .border_style(Style::default().fg(Color::Yellow))
        );

    f.render_widget(menu, popup_area);
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}
