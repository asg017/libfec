/*!
 * FEC Calendar Dates Viewer
 *
 * This module provides an interactive TUI for viewing upcoming FEC calendar dates
 * including elections, reporting deadlines, commission meetings, and other events.
 *
 * ## Features
 *
 * - Calendar view with highlighted event dates
 * - List of upcoming events with descriptions
 * - Category filtering (elections, deadlines, meetings, holidays)
 * - Keyboard navigation
 */

use crate::cli::{DatesArgs, DatesFormat};
use crate::sourcer::FilingSourcer;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fec_api::{Api, CalendarDatesArgs};
use jiff::{civil::Date as JiffDate, Zoned};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Table, TableState,
        calendar::{CalendarEventStore, Monthly},
    },
    Frame, Terminal,
};
use std::collections::HashMap;
use std::io::{self, Stdout};
use time::{Date, Month, OffsetDateTime};

/// A calendar event from the FEC API
#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub event_id: i64,
    pub summary: String,
    pub description: String,
    pub start_date: Option<JiffDate>,
    pub end_date: Option<JiffDate>,
    pub category: String,
    pub category_id: i64,
    pub location: Option<String>,
    pub url: Option<String>,
}

impl CalendarEvent {
    fn from_json(value: &serde_json::Value) -> Option<Self> {
        let event_id = value.get("event_id")?.as_i64()?;
        let summary = value
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let description = value
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let category = value
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let category_id = value.get("calendar_category_id").and_then(|v| v.as_i64()).unwrap_or(0);
        let location = value
            .get("location")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let url = value.get("url").and_then(|v| v.as_str()).map(|s| s.to_string());

        let start_date = value
            .get("start_date")
            .and_then(|v| v.as_str())
            .and_then(|s| parse_date(s));
        let end_date = value
            .get("end_date")
            .and_then(|v| v.as_str())
            .and_then(|s| parse_date(s));

        Some(CalendarEvent {
            event_id,
            summary,
            description,
            start_date,
            end_date,
            category,
            category_id,
            location,
            url,
        })
    }

    fn category_color(&self) -> Color {
        match self.category_id {
            36 => Color::Cyan,    // Elections
            21 | 25 | 26 => Color::Magenta, // All deadlines (Reporting, Quarterly, Monthly)
            20 => Color::Blue,    // Commission Meetings
            37 => Color::Red,     // Federal Holidays
            27 => Color::LightCyan, // Pre and Post-Elections
            _ => Color::White,
        }
    }

    fn format_date_range(&self) -> String {
        match (&self.start_date, &self.end_date) {
            (Some(start), Some(end)) if start != end => {
                format!("{} - {}", format_date(start), format_date(end))
            }
            (Some(start), _) => format_date(start),
            _ => "-".to_string(),
        }
    }
}

fn parse_date(s: &str) -> Option<JiffDate> {
    // Try YYYY-MM-DD format
    if let Ok(date) = s.parse::<JiffDate>() {
        return Some(date);
    }
    // Try parsing from datetime string (take first 10 chars)
    if s.len() >= 10 {
        if let Ok(date) = s[..10].parse::<JiffDate>() {
            return Some(date);
        }
    }
    None
}

fn format_date(date: &JiffDate) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month() as u8,
        date.day()
    )
}

/// Application state for the calendar TUI
struct App {
    events: Vec<CalendarEvent>,
    table_state: TableState,
    should_exit: bool,
    error: Option<String>,
    args: DatesArgs,
    current_month: time::Month,
    current_year: i32,
    last_key: Option<KeyCode>,
    /// Events grouped by date for calendar highlighting
    events_by_date: HashMap<(i32, u8, u8), Vec<usize>>,
    /// The API URL used for the last fetch
    api_url: Option<String>,
}

impl App {
    fn new(args: DatesArgs) -> Self {
        let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
        Self {
            events: Vec::new(),
            table_state: TableState::default(),
            should_exit: false,
            error: None,
            args,
            current_month: now.month(),
            current_year: now.year(),
            last_key: None,
            events_by_date: HashMap::new(),
            api_url: None,
        }
    }

    fn fetch(&mut self, _sourcer: &FilingSourcer) -> Result<()> {
        let api_key = std::env::var("LIBFEC_API_KEY").unwrap_or_else(|_| "DEMO_KEY".to_string());
        let api = Api::new(api_key.as_str());

        let now = Zoned::now();
        let today = now.date();
        let future = today.checked_add(jiff::Span::new().days(self.args.days as i64))?;

        let category_ids = self.args.category_ids();
        let args = CalendarDatesArgs {
            calendar_category_id: if category_ids.is_empty() { None } else { Some(category_ids) },
            min_start_date: Some(format!(
                "{:04}-{:02}-{:02}",
                today.year(),
                today.month() as u8,
                today.day()
            )),
            max_start_date: Some(format!(
                "{:04}-{:02}-{:02}",
                future.year(),
                future.month() as u8,
                future.day()
            )),
            sort: Some("start_date".to_string()),
        };

        let url = api.calendar_dates_url(args);
        self.api_url = Some(url.0.to_string());
        match fec_api::api_request(&url.0) {
            Ok(response) => {
                self.events = response
                    .result_items
                    .iter()
                    .filter_map(|item| CalendarEvent::from_json(item))
                    .collect();

                // Build events by date index
                self.events_by_date.clear();
                for (idx, event) in self.events.iter().enumerate() {
                    if let Some(ref date) = event.start_date {
                        let key = (date.year() as i32, date.month() as u8, date.day() as u8);
                        self.events_by_date.entry(key).or_default().push(idx);
                    }
                }

                self.error = None;
                if !self.events.is_empty() && self.table_state.selected().is_none() {
                    self.table_state.select(Some(0));
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
        Ok(())
    }

    fn select_next(&mut self) {
        let count = self.events.len();
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => (i + 1) % count,
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn select_previous(&mut self) {
        let count = self.events.len();
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn select_first(&mut self) {
        if !self.events.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    fn select_last(&mut self) {
        if !self.events.is_empty() {
            self.table_state.select(Some(self.events.len() - 1));
        }
    }

    fn next_month(&mut self) {
        self.current_month = self.current_month.next();
        if self.current_month == Month::January {
            self.current_year += 1;
        }
    }

    fn prev_month(&mut self) {
        self.current_month = self.current_month.previous();
        if self.current_month == Month::December {
            self.current_year -= 1;
        }
    }

    fn get_selected_event(&self) -> Option<&CalendarEvent> {
        self.table_state.selected().and_then(|i| self.events.get(i))
    }

    fn build_calendar_events(&self) -> CalendarEventStore {
        let mut store = CalendarEventStore::today(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Blue),
        );

        // Add event dates with their category colors
        for event in &self.events {
            if let Some(ref jiff_date) = event.start_date {
                if let Ok(month) = Month::try_from(jiff_date.month() as u8) {
                    if let Ok(date) =
                        Date::from_calendar_date(jiff_date.year() as i32, month, jiff_date.day() as u8)
                    {
                        store.add(
                            date,
                            Style::default()
                                .fg(event.category_color())
                                .add_modifier(Modifier::BOLD),
                        );
                    }
                }
            }
        }

        store
    }
}

/// Entry point for the dates command
pub fn dates(sourcer: FilingSourcer, args: &DatesArgs) -> Result<()> {
    match args.format {
        DatesFormat::Json => run_json_mode(args),
        DatesFormat::Tui => run_tui_mode(sourcer, args),
    }
}

fn run_json_mode(args: &DatesArgs) -> Result<()> {
    let api_key = std::env::var("LIBFEC_API_KEY").unwrap_or_else(|_| "DEMO_KEY".to_string());
    let api = Api::new(api_key.as_str());

    let now = Zoned::now();
    let today = now.date();
    let future = today.checked_add(jiff::Span::new().days(args.days as i64))?;

    let category_ids = args.category_ids();
    let api_args = CalendarDatesArgs {
        calendar_category_id: if category_ids.is_empty() { None } else { Some(category_ids) },
        min_start_date: Some(format!(
            "{:04}-{:02}-{:02}",
            today.year(),
            today.month() as u8,
            today.day()
        )),
        max_start_date: Some(format!(
            "{:04}-{:02}-{:02}",
            future.year(),
            future.month() as u8,
            future.day()
        )),
        sort: Some("start_date".to_string()),
    };

    let url = api.calendar_dates_url(api_args);
    eprintln!("URL: {}", url.0);

    let response = fec_api::api_request(&url.0)?;
    let json = serde_json::json!({
        "url": url.0.to_string(),
        "count": response.result_items.len(),
        "results": response.result_items,
    });
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

fn run_tui_mode(sourcer: FilingSourcer, args: &DatesArgs) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(args.clone());
    app.fetch(&sourcer)?;

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
        terminal.draw(|f| ui(f, app))?;

        if app.should_exit {
            return Ok(());
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.should_exit = true;
                    }
                    KeyCode::Char('c')
                        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
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
                    KeyCode::Left | KeyCode::Char('h') => {
                        app.prev_month();
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        app.next_month();
                    }
                    KeyCode::Char('o') => {
                        // Open event URL in browser if available
                        if let Some(event) = app.get_selected_event() {
                            if let Some(ref url) = event.url {
                                let _ = open::that(url);
                            }
                        }
                    }
                    KeyCode::Char('O') => {
                        // Open API URL in browser
                        if let Some(ref url) = app.api_url {
                            let _ = open::that(url);
                        }
                    }
                    _ => {}
                }
                app.last_key = Some(key.code);
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(10), // Calendar row (3-4 months)
            Constraint::Length(1),  // Legend
            Constraint::Min(1),     // Events list
            Constraint::Length(5),  // Event details
            Constraint::Length(2),  // Footer
        ])
        .split(f.area());

    // Header
    let category_str = app.args.category_display();

    let header_text = if let Some(ref error) = app.error {
        Line::from(vec![
            Span::styled(
                "FEC Calendar",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(format!("Error: {}", error), Style::default().fg(Color::Red)),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                "FEC Calendar",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{} ({} events)", category_str, app.events.len()),
                Style::default().fg(Color::Gray),
            ),
        ])
    };

    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).title("FEC Calendar Dates"));
    f.render_widget(header, main_layout[0]);

    // Render calendar row (3-4 months based on width)
    render_calendar_row(f, app, main_layout[1]);

    // Render legend
    render_legend(f, main_layout[2]);

    // Render events list
    render_events_list(f, app, main_layout[3]);

    // Render event details
    render_event_detail_panel(f, app, main_layout[4]);

    // Footer with help
    let shortcut_style = Style::default().bold().fg(Color::White);
    let descrip_style = Style::default().fg(Color::DarkGray);

    let help_line = Line::from(vec![
        Span::styled("q", shortcut_style),
        Span::styled(" quit  ", descrip_style),
        Span::styled("↑↓/jk", shortcut_style),
        Span::styled(" select  ", descrip_style),
        Span::styled("←→/hl", shortcut_style),
        Span::styled(" month  ", descrip_style),
        Span::styled("gg/G", shortcut_style),
        Span::styled(" first/last  ", descrip_style),
        Span::styled("o", shortcut_style),
        Span::styled(" open  ", descrip_style),
        Span::styled("O", shortcut_style),
        Span::styled(" API", descrip_style),
    ]);

    let footer = Paragraph::new(help_line)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(footer, main_layout[5]);
}

fn render_legend(f: &mut Frame, area: Rect) {
    let legend = Line::from(vec![
        Span::styled("■", Style::default().fg(Color::Blue)),
        Span::styled(" Today  ", Style::default().fg(Color::DarkGray)),
        Span::styled("■", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        Span::styled(" Deadline  ", Style::default().fg(Color::DarkGray)),
        Span::styled("■", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" Election", Style::default().fg(Color::DarkGray)),
    ]);

    let legend_widget = Paragraph::new(legend)
        .alignment(ratatui::layout::Alignment::Right);
    f.render_widget(legend_widget, area);
}

fn render_calendar_row(f: &mut Frame, app: &App, area: Rect) {
    // Determine number of months to show based on width (each calendar is ~22 chars wide)
    let calendar_width = 22u16;
    let num_months = (area.width / calendar_width).max(1).min(6) as usize;

    // Calculate total width used by calendars and padding needed to center
    let total_calendar_width = (num_months as u16) * calendar_width;
    let padding = (area.width.saturating_sub(total_calendar_width)) / 2;

    // Create centered area for calendars
    let centered_area = Rect {
        x: area.x + padding,
        y: area.y,
        width: total_calendar_width,
        height: area.height,
    };

    // Create horizontal layout for calendars
    let constraints: Vec<Constraint> = (0..num_months)
        .map(|_| Constraint::Length(calendar_width))
        .collect();

    let calendar_areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(centered_area);

    let calendar_store = app.build_calendar_events();

    let default_style = Style::default();

    let header_style = Style::default()
        .add_modifier(Modifier::BOLD);

    // Render each month
    let mut current_month = app.current_month;
    let mut current_year = app.current_year;

    for (_i, cal_area) in calendar_areas.iter().enumerate() {
        if let Ok(date) = Date::from_calendar_date(current_year, current_month, 1) {
            let calendar = Monthly::new(date, &calendar_store)
                .show_month_header(header_style)
                .show_weekdays_header(Style::default().fg(Color::Gray))
                .default_style(default_style);
                //.show_surrounding(Style::default().add_modifier(Modifier::DIM));

            f.render_widget(calendar, *cal_area);
        }

        // Advance to next month
        current_month = current_month.next();
        if current_month == Month::January {
            current_year += 1;
        }
    }
}

fn render_events_list(f: &mut Frame, app: &mut App, area: Rect) {
    let header_row = Row::new(vec![
        Cell::from("Date").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Category").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Description").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ])
    .height(1);

    let rows: Vec<Row> = app
        .events
        .iter()
        .map(|event| {
            let date = event.format_date_range();
            let category = &event.category;
            let desc = if event.summary.is_empty() {
                &event.description
            } else {
                &event.summary
            };

            Row::new(vec![
                Cell::from(date),
                Cell::from(category.as_str()).style(Style::default().fg(event.category_color())),
                Cell::from(truncate_str(desc, 60)),
            ])
        })
        .collect();

    let title = format!("Upcoming Events ({})", app.events.len());

    let table = Table::new(
        rows,
        [
            Constraint::Length(16), // Date
            Constraint::Length(24), // Category
            Constraint::Min(30),    // Description
        ],
    )
    .header(header_row)
    .block(Block::default().borders(Borders::ALL).title(title))
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_event_detail_panel(f: &mut Frame, app: &App, area: Rect) {
    let lines = if let Some(event) = app.get_selected_event() {
        let mut lines = Vec::new();

        // Summary/Title
        if !event.summary.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Summary: ", Style::default().fg(Color::Gray)),
                Span::styled(&event.summary, Style::default().fg(Color::White)),
            ]));
        }

        // Location if available
        if let Some(ref loc) = event.location {
            lines.push(Line::from(vec![
                Span::styled("Location: ", Style::default().fg(Color::Gray)),
                Span::styled(loc, Style::default().fg(Color::Cyan)),
            ]));
        }

        // Description (truncated)
        if !event.description.is_empty() {
            let desc = truncate_str(&event.description, (area.width as usize).saturating_sub(15));
            lines.push(Line::from(vec![
                Span::styled("Details: ", Style::default().fg(Color::Gray)),
                Span::styled(desc, Style::default().fg(Color::White)),
            ]));
        }

        lines
    } else {
        vec![Line::from(Span::styled(
            "Select an event to see details",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let border_color = app
        .get_selected_event()
        .map(|e| e.category_color())
        .unwrap_or(Color::DarkGray);

    let detail = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title("Event Details"),
    );

    f.render_widget(detail, area);
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_len.saturating_sub(3)).collect::<String>())
    }
}
