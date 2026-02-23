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
use fec_api::{Api, ApiCache, CalendarDatesArgs};
use jiff::{civil::Date as JiffDate, Zoned};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        calendar::{CalendarEventStore, Monthly},
        Block, Borders, Cell, Paragraph, Row, Table, TableState,
    },
    Frame, Terminal,
};
use std::collections::HashMap;
use std::io::{self, Stdout};
use time::{Date, Month};

/// A calendar event from the FEC API
#[derive(Debug, Clone)]
pub struct CalendarEvent {
    #[allow(dead_code)]
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
        let category_id = value
            .get("calendar_category_id")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let location = value
            .get("location")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let url = value
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let start_date = value
            .get("start_date")
            .and_then(|v| v.as_str())
            .and_then(parse_date);
        let end_date = value
            .get("end_date")
            .and_then(|v| v.as_str())
            .and_then(parse_date);

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
            36 => Color::Cyan,              // Elections
            21 | 25 | 26 => Color::Magenta, // All deadlines (Reporting, Quarterly, Monthly)
            20 => Color::Blue,              // Commission Meetings
            37 => Color::Red,               // Federal Holidays
            27 => Color::LightCyan,         // Pre and Post-Elections
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

    /// Extract the election type from the summary by stripping the state prefix.
    /// e.g. "CA Primary Election" -> "Primary Election"
    /// e.g. "TX/18 Special General Election Runoff" -> "Special General Election Runoff"
    /// Returns None for non-elections (category_id != 36).
    fn extract_election_type(&self) -> Option<&str> {
        if self.category_id != 36 {
            return None;
        }
        let summary = self.summary.as_str();
        if let Some(first_word) = summary.split_whitespace().next() {
            // "TX/18 ..." or "CA ..." — strip the first word if it looks like a state prefix
            if first_word.contains('/') {
                // District format like "TX/18"
                if let Some((state_part, _)) = first_word.split_once('/') {
                    if crate::utils::states::is_valid_state_code(state_part) {
                        let rest = summary[first_word.len()..].trim_start();
                        if !rest.is_empty() {
                            return Some(rest);
                        }
                    }
                }
            } else if first_word.len() == 2
                && crate::utils::states::is_valid_state_code(first_word)
            {
                let rest = summary[first_word.len()..].trim_start();
                if !rest.is_empty() {
                    return Some(rest);
                }
            }
        }
        // Fallback: return the whole summary if we can't strip a prefix
        if summary.is_empty() {
            None
        } else {
            Some(summary)
        }
    }

    /// Extract state code(s) from event summary or location
    /// Elections typically have format "XX/## Election Type" or "XX Election Type"
    /// Returns all applicable state codes for the event
    fn extract_state_codes(&self) -> Vec<String> {
        let mut codes = Vec::new();

        // First try to extract from summary
        if let Some(first_word) = self.summary.split_whitespace().next() {
            // Check if it's in format "XX/##" (most reliable pattern)
            if let Some((state_part, _)) = first_word.split_once('/') {
                if crate::utils::states::is_valid_state_code(state_part) {
                    codes.push(state_part.to_uppercase());
                    return codes;
                }
            }

            // Check if first word is exactly 2 letters and a valid state code
            // This avoids false positives like "Primary" -> "PR"
            if first_word.len() == 2 && crate::utils::states::is_valid_state_code(first_word) {
                codes.push(first_word.to_uppercase());
                return codes;
            }
        }

        // Try to extract from location field
        if let Some(ref location) = self.location {
            // Check if location contains comma-separated state codes (e.g., "AR, NC, TX")
            if location.contains(',') {
                for part in location.split(',') {
                    let trimmed = part.trim();
                    if crate::utils::states::is_valid_state_code(trimmed) {
                        codes.push(trimmed.to_uppercase());
                    }
                }
                if !codes.is_empty() {
                    return codes;
                }
            }

            // Check if location is a single state code
            if crate::utils::states::is_valid_state_code(location) {
                codes.push(location.to_uppercase());
                return codes;
            }

            // Check if location is a state name
            if let Some(code) = crate::utils::states::state_code_from_name(location) {
                codes.push(code.to_string());
                return codes;
            }
        }

        codes
    }

    /// Check if this event should be shown for a given state filter
    /// - If no state filter, show all events
    /// - If state filter is set:
    ///   - Always show reporting deadlines (21, 25, 26) as they apply to all states
    ///   - Filter reporting periods (27, 28, 29, 38) by state as they're state-specific
    ///   - Filter elections (36) by state
    fn matches_state_filter(&self, state_filter: Option<&str>) -> bool {
        let Some(filter_state) = state_filter else {
            return true; // No filter, show everything
        };

        // Always show national reporting deadlines regardless of state filter
        // These are monthly/quarterly reports that apply to all committees
        if matches!(self.category_id, 21 | 25 | 26) {
            return true;
        }

        // For state-specific events (elections and reporting periods), check if state matches
        let event_states = self.extract_state_codes();
        if !event_states.is_empty() {
            // Check if any of the event's states match the filter
            event_states
                .iter()
                .any(|state| state.eq_ignore_ascii_case(filter_state))
        } else {
            // If we can't determine state, include it (might be national event)
            true
        }
    }
}

enum DisplayRow {
    Single(usize),
    Grouped {
        event_indices: Vec<usize>,
        date: JiffDate,
        election_type: String,
        states: Vec<String>,
    },
}

impl DisplayRow {
    fn date(&self, events: &[CalendarEvent]) -> Option<JiffDate> {
        match self {
            DisplayRow::Single(idx) => events[*idx].start_date,
            DisplayRow::Grouped { date, .. } => Some(*date),
        }
    }
}

fn format_state_list(states: &[String]) -> String {
    match states.len() {
        0 => String::new(),
        1 => states[0].clone(),
        2 => format!("{} and {}", states[0], states[1]),
        _ => {
            let (last, rest) = states.split_last().unwrap();
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

fn pluralize_election_type(election_type: &str, count: usize) -> String {
    if count <= 1 {
        return election_type.to_string();
    }
    if election_type.ends_with("Election Runoff") {
        format!("{}s", election_type)
    } else if election_type.ends_with("Election") {
        format!("{}s", election_type)
    } else {
        election_type.to_string()
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

/// Parse `--as-of` into a JiffDate, falling back to real today
fn parse_as_of_date(as_of: &Option<String>) -> JiffDate {
    if let Some(ref date_str) = as_of {
        date_str
            .parse::<JiffDate>()
            .expect("Invalid --as-of date format, expected YYYY-MM-DD")
    } else {
        Zoned::now().date()
    }
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
    /// Display rows (grouped elections + ungrouped events)
    display_rows: Vec<DisplayRow>,
    /// The API URL used for the last fetch
    api_url: Option<String>,
    /// The date to treat as "today" (from --as-of or real today)
    today: JiffDate,
}

impl App {
    fn new(args: DatesArgs) -> Self {
        let today = parse_as_of_date(&args.as_of);
        let current_month = time::Month::try_from(today.month() as u8).unwrap_or(time::Month::January);
        let current_year = today.year() as i32;
        Self {
            events: Vec::new(),
            table_state: TableState::default(),
            should_exit: false,
            error: None,
            args,
            current_month,
            current_year,
            last_key: None,
            events_by_date: HashMap::new(),
            display_rows: Vec::new(),
            api_url: None,
            today,
        }
    }

    fn fetch(&mut self, sourcer: &mut FilingSourcer) -> Result<()> {
        let api_key = std::env::var("LIBFEC_API_KEY").unwrap_or_else(|_| "DEMO_KEY".to_string());
        let api = Api::new(api_key.as_str());

        let today = self.today;
        let future = today.checked_add(jiff::Span::new().days(self.args.days as i64))?;

        let category_ids = self.args.category_ids();
        let args = CalendarDatesArgs {
            calendar_category_id: if category_ids.is_empty() {
                None
            } else {
                Some(category_ids)
            },
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
        match fec_api::api_request_cached(
            &url.0,
            sourcer
                .cache
                .api_cache_mut()
                .map(|c| c as &mut dyn ApiCache),
        ) {
            Ok(response) => {
                // Parse and filter events by state if specified
                let state_filter = self.args.state_code();
                self.events = response
                    .result_items
                    .iter()
                    .filter_map(CalendarEvent::from_json)
                    .filter(|event| event.matches_state_filter(state_filter.as_deref()))
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
                self.build_display_rows();
                if !self.display_rows.is_empty() && self.table_state.selected().is_none() {
                    self.table_state.select(Some(0));
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
        Ok(())
    }

    fn build_display_rows(&mut self) {
        use std::collections::BTreeMap;

        // Separate elections (category_id 36) that have extractable types from other events
        // Key: (start_date, election_type) -> Vec<event_index>
        let mut election_groups: BTreeMap<(JiffDate, String), Vec<usize>> = BTreeMap::new();
        let mut non_election_indices: Vec<usize> = Vec::new();

        for (idx, event) in self.events.iter().enumerate() {
            if let Some(election_type) = event.extract_election_type() {
                if let Some(date) = event.start_date {
                    election_groups
                        .entry((date, election_type.to_string()))
                        .or_default()
                        .push(idx);
                    continue;
                }
            }
            non_election_indices.push(idx);
        }

        let mut rows: Vec<DisplayRow> = Vec::new();

        // Convert election groups into display rows
        for ((date, election_type), indices) in &election_groups {
            if indices.len() == 1 {
                rows.push(DisplayRow::Single(indices[0]));
            } else {
                // Collect unique state codes, deduplicating (e.g. TX/18 + TX/25 -> just TX)
                let mut states: Vec<String> = Vec::new();
                for &idx in indices {
                    for code in self.events[idx].extract_state_codes() {
                        if !states.contains(&code) {
                            states.push(code);
                        }
                    }
                }
                rows.push(DisplayRow::Grouped {
                    event_indices: indices.clone(),
                    date: *date,
                    election_type: election_type.clone(),
                    states,
                });
            }
        }

        // Add non-election events as Single rows
        for idx in non_election_indices {
            rows.push(DisplayRow::Single(idx));
        }

        // Sort all rows by date
        rows.sort_by(|a, b| {
            let date_a = a.date(&self.events);
            let date_b = b.date(&self.events);
            date_a.cmp(&date_b)
        });

        self.display_rows = rows;
    }

    fn select_next(&mut self) {
        let count = self.display_rows.len();
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
        let count = self.display_rows.len();
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
        if !self.display_rows.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    fn select_last(&mut self) {
        if !self.display_rows.is_empty() {
            self.table_state.select(Some(self.display_rows.len() - 1));
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

    fn get_selected_display_row(&self) -> Option<&DisplayRow> {
        self.table_state
            .selected()
            .and_then(|i| self.display_rows.get(i))
    }

    fn build_calendar_events(
        &self,
        start_month: Month,
        start_year: i32,
        num_months: usize,
    ) -> CalendarEventStore {
        let mut store = CalendarEventStore::default();
        let today_jiff = self.today;

        // Highlight "today" in the calendar
        if let Ok(today_month) = Month::try_from(today_jiff.month() as u8) {
            if let Ok(today_time) = Date::from_calendar_date(
                today_jiff.year() as i32,
                today_month,
                today_jiff.day() as u8,
            ) {
                store.add(
                    today_time,
                    Style::default()
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                        .bg(Color::Blue),
                );
            }
        }

        // Dim all past days in displayed months
        let mut month = start_month;
        let mut year = start_year;
        for _ in 0..num_months {
            if Date::from_calendar_date(year, month, 1).is_ok() {
                let days_in_month = month.length(year);
                for day in 1..=days_in_month {
                    if let Ok(date) = Date::from_calendar_date(year, month, day) {
                        if let Ok(jiff_date) =
                            JiffDate::new(year as i16, month as i8, day as i8)
                        {
                            if jiff_date < today_jiff {
                                store.add(
                                    date,
                                    Style::default().add_modifier(Modifier::DIM),
                                );
                            }
                        }
                    }
                }
            }
            month = month.next();
            if month == Month::January {
                year += 1;
            }
        }

        // Add event dates with their category colors (overrides dim for event days)
        for event in &self.events {
            if let Some(ref jiff_date) = event.start_date {
                if let Ok(month) = Month::try_from(jiff_date.month() as u8) {
                    if let Ok(date) = Date::from_calendar_date(
                        jiff_date.year() as i32,
                        month,
                        jiff_date.day() as u8,
                    ) {
                        let is_past = *jiff_date < today_jiff;
                        store.add(
                            date,
                            if is_past {
                                Style::default()
                                    .fg(event.category_color())
                                    .add_modifier(Modifier::DIM)
                            } else {
                                Style::default()
                                    .fg(event.category_color())
                                    .add_modifier(Modifier::BOLD)
                            },
                        );
                    }
                }
            }
        }

        store
    }
}

/// Entry point for the dates command
pub fn dates(mut sourcer: FilingSourcer, args: &DatesArgs) -> Result<()> {
    match args.format {
        DatesFormat::Json => run_json_mode(&mut sourcer, args),
        DatesFormat::Tui => run_tui_mode(sourcer, args),
    }
}

fn run_json_mode(sourcer: &mut FilingSourcer, args: &DatesArgs) -> Result<()> {
    let api_key = std::env::var("LIBFEC_API_KEY").unwrap_or_else(|_| "DEMO_KEY".to_string());
    let api = Api::new(api_key.as_str());

    let today = parse_as_of_date(&args.as_of);
    let future = today.checked_add(jiff::Span::new().days(args.days as i64))?;

    let category_ids = args.category_ids();
    let api_args = CalendarDatesArgs {
        calendar_category_id: if category_ids.is_empty() {
            None
        } else {
            Some(category_ids)
        },
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

    let response = fec_api::api_request_cached(
        &url.0,
        sourcer
            .cache
            .api_cache_mut()
            .map(|c| c as &mut dyn ApiCache),
    )?;

    // Apply state filtering if specified
    let state_filter = args.state_code();
    let filtered_results: Vec<_> = if state_filter.is_some() {
        response
            .result_items
            .iter()
            .filter(|item| {
                // Parse as CalendarEvent to use the filtering logic
                if let Some(event) = CalendarEvent::from_json(item) {
                    event.matches_state_filter(state_filter.as_deref())
                } else {
                    true // Include items we can't parse
                }
            })
            .cloned()
            .collect()
    } else {
        response.result_items.clone()
    };

    let json = serde_json::json!({
        "url": url.0.to_string(),
        "count": filtered_results.len(),
        "results": filtered_results,
    });
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

fn run_tui_mode(mut sourcer: FilingSourcer, args: &DatesArgs) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(args.clone());
    app.fetch(&mut sourcer)?;

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
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
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
                        match app.get_selected_display_row() {
                            Some(DisplayRow::Single(idx)) => {
                                if let Some(ref url) = app.events[*idx].url {
                                    let _ = open::that(url);
                                }
                            }
                            Some(DisplayRow::Grouped { event_indices, .. }) => {
                                // Open first event's URL
                                if let Some(&idx) = event_indices.first() {
                                    if let Some(ref url) = app.events[idx].url {
                                        let _ = open::that(url);
                                    }
                                }
                            }
                            None => {}
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

fn render_header(f: &mut Frame, app: &App, area: Rect) {
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
                format!("{} ({} events)", category_str, app.display_rows.len()),
                Style::default().fg(Color::Gray),
            ),
        ])
    };

    let header = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("FEC Calendar Dates"),
    );
    f.render_widget(header, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
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
    f.render_widget(footer, area);
}

fn ui(f: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(10), // Calendar row (3-4 months)
            Constraint::Length(1),  // Legend
            Constraint::Min(1),     // Events list
            Constraint::Length(5),  // Event details
            Constraint::Length(2),  // Footer
        ]);

    let [header_area, calendar_area, legend_area, events_area, detail_area, footer_area] =
        f.area().layout(&layout);

    render_header(f, app, header_area);
    render_calendar_row(f, app, calendar_area);
    render_legend(f, legend_area);
    render_events_list(f, app, events_area);
    render_event_detail_panel(f, app, detail_area);
    render_footer(f, footer_area);
}

fn render_legend(f: &mut Frame, area: Rect) {
    let legend = Line::from(vec![
        Span::styled("■", Style::default().fg(Color::Blue)),
        Span::styled(" Today  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "■",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Deadline  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "■",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Election", Style::default().fg(Color::DarkGray)),
    ]);

    let legend_widget = Paragraph::new(legend).alignment(ratatui::layout::Alignment::Right);
    f.render_widget(legend_widget, area);
}

fn render_calendar_row(f: &mut Frame, app: &App, area: Rect) {
    // Determine number of months to show based on width (each calendar is ~22 chars wide)
    let calendar_width = 22u16;
    let num_months = (area.width / calendar_width).clamp(1, 6) as usize;

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

    let calendar_store =
        app.build_calendar_events(app.current_month, app.current_year, num_months);

    let default_style = Style::default();

    let header_style = Style::default().add_modifier(Modifier::BOLD);

    // Render each month
    let mut current_month = app.current_month;
    let mut current_year = app.current_year;

    for cal_area in calendar_areas.iter() {
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

    let today = app.today;

    let rows: Vec<Row> = app
        .display_rows
        .iter()
        .map(|display_row| match display_row {
            DisplayRow::Single(idx) => {
                let event = &app.events[*idx];
                let date = event.format_date_range();
                let category = &event.category;
                let desc = if event.summary.is_empty() {
                    &event.description
                } else {
                    &event.summary
                };

                let is_past = event
                    .end_date
                    .as_ref()
                    .or(event.start_date.as_ref())
                    .is_some_and(|d| *d < today);

                if is_past {
                    let dim = Style::default().add_modifier(Modifier::DIM);
                    Row::new(vec![
                        Cell::from(date).style(dim),
                        Cell::from(category.as_str()).style(
                            Style::default()
                                .fg(event.category_color())
                                .add_modifier(Modifier::DIM),
                        ),
                        Cell::from(truncate_str(desc, 60)).style(dim),
                    ])
                } else {
                    Row::new(vec![
                        Cell::from(date),
                        Cell::from(category.as_str())
                            .style(Style::default().fg(event.category_color())),
                        Cell::from(truncate_str(desc, 60)),
                    ])
                }
            }
            DisplayRow::Grouped {
                date,
                election_type,
                states,
                ..
            } => {
                let date_str = format_date(date);
                let type_str =
                    pluralize_election_type(election_type, states.len());
                let desc = format!("{} in {}", type_str, format_state_list(states));
                let is_past = *date < today;
                let color = Color::Cyan; // Elections are always Cyan

                if is_past {
                    let dim = Style::default().add_modifier(Modifier::DIM);
                    Row::new(vec![
                        Cell::from(date_str).style(dim),
                        Cell::from("Election Dates")
                            .style(Style::default().fg(color).add_modifier(Modifier::DIM)),
                        Cell::from(truncate_str(&desc, 60)).style(dim),
                    ])
                } else {
                    Row::new(vec![
                        Cell::from(date_str),
                        Cell::from("Election Dates")
                            .style(Style::default().fg(color)),
                        Cell::from(truncate_str(&desc, 60)),
                    ])
                }
            }
        })
        .collect();

    let title = format!("Upcoming Events ({})", app.display_rows.len());

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
    let (lines, border_color) = match app.get_selected_display_row() {
        Some(DisplayRow::Single(idx)) => {
            let event = &app.events[*idx];
            let mut lines = Vec::new();

            if !event.summary.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("Summary: ", Style::default().fg(Color::Gray)),
                    Span::styled(&event.summary, Style::default().fg(Color::White)),
                ]));
            }

            if let Some(ref loc) = event.location {
                lines.push(Line::from(vec![
                    Span::styled("Location: ", Style::default().fg(Color::Gray)),
                    Span::styled(loc, Style::default().fg(Color::Cyan)),
                ]));
            }

            if !event.description.is_empty() {
                let desc =
                    truncate_str(&event.description, (area.width as usize).saturating_sub(15));
                lines.push(Line::from(vec![
                    Span::styled("Details: ", Style::default().fg(Color::Gray)),
                    Span::styled(desc, Style::default().fg(Color::White)),
                ]));
            }

            (lines, event.category_color())
        }
        Some(DisplayRow::Grouped {
            election_type,
            states,
            date,
            ..
        }) => {
            let type_str = pluralize_election_type(election_type, states.len());
            let lines = vec![
                Line::from(vec![
                    Span::styled("Summary: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{} in {}", type_str, format_state_list(states)),
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Date: ", Style::default().fg(Color::Gray)),
                    Span::styled(format_date(date), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("States: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format_state_list(states),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
            ];

            (lines, Color::Cyan)
        }
        None => {
            let lines = vec![Line::from(Span::styled(
                "Select an event to see details",
                Style::default().fg(Color::DarkGray),
            ))];
            (lines, Color::DarkGray)
        }
    };

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
        format!(
            "{}...",
            s.chars()
                .take(max_len.saturating_sub(3))
                .collect::<String>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_yaml_snapshot;

    fn create_election_event(summary: &str, location: &str, category_id: i64) -> CalendarEvent {
        CalendarEvent {
            event_id: 1,
            summary: summary.to_string(),
            description: "Test event".to_string(),
            start_date: Some("2026-01-01".parse().unwrap()),
            end_date: None,
            category: "Test".to_string(),
            category_id,
            location: Some(location.to_string()),
            url: None,
        }
    }

    #[test]
    fn test_extract_state_code_from_summary_with_slash() {
        let event = create_election_event("TX/18 Special Election", "Texas", 36);
        assert_eq!(event.extract_state_codes(), vec!["TX".to_string()]);
    }

    #[test]
    fn test_extract_state_code_from_summary_simple() {
        let event = create_election_event("CA Primary Election", "California", 36);
        assert_eq!(event.extract_state_codes(), vec!["CA".to_string()]);
    }

    #[test]
    fn test_extract_state_code_from_location() {
        let event = create_election_event("Primary Election", "New York", 36);
        assert_eq!(event.extract_state_codes(), vec!["NY".to_string()]);
    }

    #[test]
    fn test_extract_state_code_no_match() {
        let event = create_election_event("Federal Holiday", "FEC", 37);
        assert!(event.extract_state_codes().is_empty());
    }

    #[test]
    fn test_extract_state_codes_multiple() {
        let event = create_election_event("EC Period Starts", "AR, NC, TX", 28);
        assert_eq!(
            event.extract_state_codes(),
            vec!["AR".to_string(), "NC".to_string(), "TX".to_string()]
        );
    }

    #[test]
    fn test_extract_state_codes_single_code_in_location() {
        let event = create_election_event("EC Period Starts", "MS", 28);
        assert_eq!(event.extract_state_codes(), vec!["MS".to_string()]);
    }

    #[test]
    fn test_matches_state_filter_no_filter() {
        let event = create_election_event("CA Primary", "California", 36);
        assert!(event.matches_state_filter(None));
    }

    #[test]
    fn test_matches_state_filter_election_matches() {
        let event = create_election_event("CA Primary", "California", 36);
        assert!(event.matches_state_filter(Some("CA")));
        assert!(event.matches_state_filter(Some("ca"))); // Case insensitive
    }

    #[test]
    fn test_matches_state_filter_election_no_match() {
        let event = create_election_event("CA Primary", "California", 36);
        assert!(!event.matches_state_filter(Some("TX")));
    }

    #[test]
    fn test_matches_state_filter_reporting_deadline_always_shown() {
        // Reporting deadlines should show regardless of state filter
        let quarterly = create_election_event("Q1 Report Due", "FEC", 25);
        let monthly = create_election_event("Monthly Report Due", "FEC", 26);
        let reporting = create_election_event("Report Due", "FEC", 21);

        assert!(quarterly.matches_state_filter(Some("CA")));
        assert!(monthly.matches_state_filter(Some("TX")));
        assert!(reporting.matches_state_filter(Some("NY")));
    }

    #[test]
    fn test_matches_state_filter_reporting_periods_always_shown() {
        // Reporting periods should show regardless of state filter
        let pre_post = create_election_event("Pre-Election Period", "FEC", 27);
        let ec = create_election_event("EC Period", "FEC", 28);
        let ie = create_election_event("IE Period", "FEC", 29);
        let fea = create_election_event("FEA Period", "FEC", 38);

        assert!(pre_post.matches_state_filter(Some("CA")));
        assert!(ec.matches_state_filter(Some("TX")));
        assert!(ie.matches_state_filter(Some("NY")));
        assert!(fea.matches_state_filter(Some("FL")));
    }

    #[test]
    fn test_matches_state_filter_mixed() {
        // TX election should match TX filter
        let tx_event = create_election_event("TX Primary", "Texas", 36);
        assert!(tx_event.matches_state_filter(Some("TX")));
        assert!(!tx_event.matches_state_filter(Some("CA")));

        // CA election should match CA filter
        let ca_event = create_election_event("CA Primary", "California", 36);
        assert!(ca_event.matches_state_filter(Some("CA")));
        assert!(!ca_event.matches_state_filter(Some("TX")));

        // Reporting deadlines should match any filter
        let deadline = create_election_event("Report Due", "FEC", 21);
        assert!(deadline.matches_state_filter(Some("CA")));
        assert!(deadline.matches_state_filter(Some("TX")));
        assert!(deadline.matches_state_filter(Some("NY")));

        // Reporting periods should match any filter
        let period = create_election_event("IE Period", "FEC", 29);
        assert!(period.matches_state_filter(Some("CA")));
        assert!(period.matches_state_filter(Some("TX")));
        assert!(period.matches_state_filter(Some("NY")));
    }

    /// Create test events representing a mix of elections, deadlines, and reporting periods
    fn create_test_events() -> Vec<CalendarEvent> {
        vec![
            CalendarEvent {
                event_id: 1,
                summary: "CA Primary Election".to_string(),
                description: "Primary election held in CA".to_string(),
                start_date: Some("2026-06-02".parse().unwrap()),
                end_date: None,
                category: "Election Dates".to_string(),
                category_id: 36,
                location: Some("California".to_string()),
                url: None,
            },
            CalendarEvent {
                event_id: 2,
                summary: "TX/18 Special General Election Runoff".to_string(),
                description: "Special general election runoff for Texas's 18th Congressional District".to_string(),
                start_date: Some("2026-01-31".parse().unwrap()),
                end_date: None,
                category: "Election Dates".to_string(),
                category_id: 36,
                location: Some("Texas".to_string()),
                url: None,
            },
            CalendarEvent {
                event_id: 3,
                summary: "NY Primary Election".to_string(),
                description: "Primary election held in NY".to_string(),
                start_date: Some("2026-06-23".parse().unwrap()),
                end_date: None,
                category: "Election Dates".to_string(),
                category_id: 36,
                location: Some("New York".to_string()),
                url: None,
            },
            CalendarEvent {
                event_id: 4,
                summary: "February Monthly Report Due".to_string(),
                description: "February Monthly Report due today".to_string(),
                start_date: Some("2026-02-20".parse().unwrap()),
                end_date: None,
                category: "Monthly".to_string(),
                category_id: 26,
                location: Some("FEC".to_string()),
                url: Some("https://www.fec.gov/help-candidates-and-committees/dates-and-deadlines/2026-reporting-dates/2026-monthly-filers/".to_string()),
            },
            CalendarEvent {
                event_id: 5,
                summary: "April Quarterly Report Due".to_string(),
                description: "April Quarterly Report due today".to_string(),
                start_date: Some("2026-04-15".parse().unwrap()),
                end_date: None,
                category: "Quarterly".to_string(),
                category_id: 25,
                location: Some("FEC".to_string()),
                url: Some("https://www.fec.gov/help-candidates-and-committees/dates-and-deadlines/2026-reporting-dates/2026-quarterly-filers/".to_string()),
            },
            CalendarEvent {
                event_id: 6,
                summary: "CA Pre-Primary Report Due".to_string(),
                description: "Pre-primary report due for California primary".to_string(),
                start_date: Some("2026-05-13".parse().unwrap()),
                end_date: None,
                category: "Pre and Post-Elections".to_string(),
                category_id: 27,
                location: Some("California".to_string()),
                url: None,
            },
            CalendarEvent {
                event_id: 7,
                summary: "24-Hour IE Report Period Begins".to_string(),
                description: "24-Hour report period for CA primary".to_string(),
                start_date: Some("2026-05-22".parse().unwrap()),
                end_date: None,
                category: "IE Periods".to_string(),
                category_id: 29,
                location: Some("CA".to_string()),
                url: None,
            },
            CalendarEvent {
                event_id: 8,
                summary: "EC Period Starts".to_string(),
                description: "EC period for TX primary".to_string(),
                start_date: Some("2026-02-01".parse().unwrap()),
                end_date: None,
                category: "EC Periods".to_string(),
                category_id: 28,
                location: Some("TX".to_string()),
                url: None,
            },
        ]
    }

    #[test]
    fn test_state_filter_ca_snapshot() {
        let events = create_test_events();
        let filtered: Vec<_> = events
            .iter()
            .filter(|e| e.matches_state_filter(Some("CA")))
            .map(|e| {
                serde_json::json!({
                    "summary": e.summary,
                    "category": e.category,
                    "location": e.location,
                    "start_date": e.start_date.as_ref().map(format_date),
                })
            })
            .collect();

        assert_yaml_snapshot!(filtered);
    }

    #[test]
    fn test_state_filter_tx_snapshot() {
        let events = create_test_events();
        let filtered: Vec<_> = events
            .iter()
            .filter(|e| e.matches_state_filter(Some("TX")))
            .map(|e| {
                serde_json::json!({
                    "summary": e.summary,
                    "category": e.category,
                    "location": e.location,
                    "start_date": e.start_date.as_ref().map(format_date),
                })
            })
            .collect();

        assert_yaml_snapshot!(filtered);
    }

    #[test]
    fn test_no_state_filter_snapshot() {
        let events = create_test_events();
        let filtered: Vec<_> = events
            .iter()
            .filter(|e| e.matches_state_filter(None))
            .map(|e| {
                serde_json::json!({
                    "summary": e.summary,
                    "category": e.category,
                    "location": e.location,
                    "start_date": e.start_date.as_ref().map(format_date),
                })
            })
            .collect();

        assert_yaml_snapshot!(filtered);
    }

    #[test]
    fn test_default_categories_include_ec_ie_with_state() {
        use crate::cli::DatesArgs;
        use crate::cli::DatesFormat;

        // With state filter and default categories
        let args_with_state = DatesArgs {
            category: "elections,deadlines".to_string(),
            days: 365,
            limit: 500,
            state: Some("CA".to_string()),
            format: DatesFormat::Tui,
            as_of: None,
        };

        let ids = args_with_state.category_ids();
        assert!(ids.contains(&36)); // Elections
        assert!(ids.contains(&21)); // Reporting Deadlines
        assert!(ids.contains(&25)); // Quarterly
        assert!(ids.contains(&26)); // Monthly
        assert!(ids.contains(&28)); // EC Periods
        assert!(ids.contains(&29)); // IE Periods

        // Without state filter, should not include EC/IE
        let args_without_state = DatesArgs {
            category: "elections,deadlines".to_string(),
            days: 365,
            limit: 500,
            state: None,
            format: DatesFormat::Tui,
            as_of: None,
        };

        let ids = args_without_state.category_ids();
        assert!(ids.contains(&36)); // Elections
        assert!(ids.contains(&21)); // Reporting Deadlines
        assert!(!ids.contains(&28)); // EC Periods should NOT be included
        assert!(!ids.contains(&29)); // IE Periods should NOT be included
    }

    #[test]
    fn test_explicit_categories_override_default() {
        use crate::cli::DatesArgs;
        use crate::cli::DatesFormat;

        // With state filter but explicit categories (not default)
        let args = DatesArgs {
            category: "elections".to_string(),
            days: 365,
            limit: 500,
            state: Some("CA".to_string()),
            format: DatesFormat::Tui,
            as_of: None,
        };

        let ids = args.category_ids();
        assert!(ids.contains(&36)); // Elections
        assert!(!ids.contains(&21)); // Reporting Deadlines should NOT be included
        assert!(!ids.contains(&28)); // EC Periods should NOT be included
        assert!(!ids.contains(&29)); // IE Periods should NOT be included
    }

    // ── extract_election_type tests ──

    #[test]
    fn test_extract_election_type_primary() {
        let event = create_election_event("CA Primary Election", "California", 36);
        assert_eq!(event.extract_election_type(), Some("Primary Election"));
    }

    #[test]
    fn test_extract_election_type_district_prefix() {
        let event =
            create_election_event("TX/18 Special General Election Runoff", "Texas", 36);
        assert_eq!(
            event.extract_election_type(),
            Some("Special General Election Runoff")
        );
    }

    #[test]
    fn test_extract_election_type_convention() {
        let event = create_election_event("National Convention", "DC", 36);
        assert_eq!(event.extract_election_type(), Some("National Convention"));
    }

    #[test]
    fn test_extract_election_type_non_election() {
        let event = create_election_event("Q1 Report Due", "FEC", 26);
        assert_eq!(event.extract_election_type(), None);
    }

    #[test]
    fn test_extract_election_type_empty_summary() {
        let mut event = create_election_event("", "California", 36);
        event.summary = String::new();
        assert_eq!(event.extract_election_type(), None);
    }

    // ── format_state_list tests ──

    #[test]
    fn test_format_state_list_empty() {
        assert_eq!(format_state_list(&[]), "");
    }

    #[test]
    fn test_format_state_list_one() {
        assert_eq!(format_state_list(&["AR".to_string()]), "AR");
    }

    #[test]
    fn test_format_state_list_two() {
        assert_eq!(
            format_state_list(&["AR".to_string(), "NC".to_string()]),
            "AR and NC"
        );
    }

    #[test]
    fn test_format_state_list_three() {
        assert_eq!(
            format_state_list(&["AR".to_string(), "NC".to_string(), "TX".to_string()]),
            "AR, NC, and TX"
        );
    }

    #[test]
    fn test_format_state_list_four() {
        assert_eq!(
            format_state_list(&[
                "AR".to_string(),
                "NC".to_string(),
                "TX".to_string(),
                "CA".to_string()
            ]),
            "AR, NC, TX, and CA"
        );
    }

    // ── pluralize_election_type tests ──

    #[test]
    fn test_pluralize_election_type_singular() {
        assert_eq!(
            pluralize_election_type("Primary Election", 1),
            "Primary Election"
        );
    }

    #[test]
    fn test_pluralize_election_type_plural() {
        assert_eq!(
            pluralize_election_type("Primary Election", 3),
            "Primary Elections"
        );
    }

    #[test]
    fn test_pluralize_election_type_runoff_plural() {
        assert_eq!(
            pluralize_election_type("Special General Election Runoff", 2),
            "Special General Election Runoffs"
        );
    }

    #[test]
    fn test_pluralize_election_type_non_election() {
        assert_eq!(
            pluralize_election_type("National Convention", 3),
            "National Convention"
        );
    }

    // ── Grouping integration tests ──

    fn make_event(
        id: i64,
        summary: &str,
        date: &str,
        category_id: i64,
        category: &str,
    ) -> CalendarEvent {
        CalendarEvent {
            event_id: id,
            summary: summary.to_string(),
            description: format!("Description for {}", summary),
            start_date: Some(date.parse().unwrap()),
            end_date: None,
            category: category.to_string(),
            category_id,
            location: None,
            url: None,
        }
    }

    fn build_test_app(events: Vec<CalendarEvent>) -> App {
        use crate::cli::{DatesArgs, DatesFormat};
        let mut app = App::new(DatesArgs {
            category: "elections,deadlines".to_string(),
            days: 365,
            limit: 500,
            state: None,
            format: DatesFormat::Tui,
            as_of: Some("2026-01-01".to_string()),
        });
        app.events = events;
        app.build_display_rows();
        if !app.display_rows.is_empty() {
            app.table_state.select(Some(0));
        }
        app
    }

    #[test]
    fn test_grouping_same_date_same_type() {
        let app = build_test_app(vec![
            make_event(1, "AR Primary Election", "2026-03-03", 36, "Election Dates"),
            make_event(2, "NC Primary Election", "2026-03-03", 36, "Election Dates"),
            make_event(3, "TX Primary Election", "2026-03-03", 36, "Election Dates"),
        ]);

        assert_eq!(app.display_rows.len(), 1);
        match &app.display_rows[0] {
            DisplayRow::Grouped {
                states,
                election_type,
                event_indices,
                ..
            } => {
                assert_eq!(election_type, "Primary Election");
                assert_eq!(states, &["AR", "NC", "TX"]);
                assert_eq!(event_indices.len(), 3);
            }
            _ => panic!("Expected Grouped row"),
        }
    }

    #[test]
    fn test_grouping_same_date_different_type() {
        let app = build_test_app(vec![
            make_event(1, "AR Primary Election", "2026-03-03", 36, "Election Dates"),
            make_event(
                2,
                "NC General Election",
                "2026-03-03",
                36,
                "Election Dates",
            ),
        ]);

        // Different types on the same date should NOT group
        assert_eq!(app.display_rows.len(), 2);
        assert!(matches!(app.display_rows[0], DisplayRow::Single(_)));
        assert!(matches!(app.display_rows[1], DisplayRow::Single(_)));
    }

    #[test]
    fn test_grouping_different_date_same_type() {
        let app = build_test_app(vec![
            make_event(1, "AR Primary Election", "2026-03-03", 36, "Election Dates"),
            make_event(2, "NC Primary Election", "2026-06-09", 36, "Election Dates"),
        ]);

        // Different dates should NOT group
        assert_eq!(app.display_rows.len(), 2);
        assert!(matches!(app.display_rows[0], DisplayRow::Single(_)));
        assert!(matches!(app.display_rows[1], DisplayRow::Single(_)));
    }

    #[test]
    fn test_grouping_non_elections_never_grouped() {
        let app = build_test_app(vec![
            make_event(1, "Monthly Report Due", "2026-03-03", 26, "Monthly"),
            make_event(2, "Monthly Report Due", "2026-03-03", 26, "Monthly"),
        ]);

        // Non-elections should remain as singles
        assert_eq!(app.display_rows.len(), 2);
        assert!(matches!(app.display_rows[0], DisplayRow::Single(_)));
        assert!(matches!(app.display_rows[1], DisplayRow::Single(_)));
    }

    #[test]
    fn test_grouping_single_election_stays_single() {
        let app = build_test_app(vec![make_event(
            1,
            "AR Primary Election",
            "2026-03-03",
            36,
            "Election Dates",
        )]);

        assert_eq!(app.display_rows.len(), 1);
        assert!(matches!(app.display_rows[0], DisplayRow::Single(0)));
    }

    #[test]
    fn test_grouping_mixed_events_preserve_date_order() {
        let app = build_test_app(vec![
            make_event(1, "Monthly Report Due", "2026-02-20", 26, "Monthly"),
            make_event(2, "AR Primary Election", "2026-03-03", 36, "Election Dates"),
            make_event(3, "NC Primary Election", "2026-03-03", 36, "Election Dates"),
            make_event(4, "TX Primary Election", "2026-03-03", 36, "Election Dates"),
            make_event(5, "Quarterly Report Due", "2026-04-15", 25, "Quarterly"),
        ]);

        assert_eq!(app.display_rows.len(), 3);

        // First: report due on 2026-02-20
        assert!(matches!(app.display_rows[0], DisplayRow::Single(_)));

        // Second: grouped primary elections on 2026-03-03
        match &app.display_rows[1] {
            DisplayRow::Grouped { states, .. } => {
                assert_eq!(states.len(), 3);
            }
            _ => panic!("Expected Grouped row at index 1"),
        }

        // Third: quarterly report on 2026-04-15
        assert!(matches!(app.display_rows[2], DisplayRow::Single(_)));
    }

    #[test]
    fn test_grouping_district_elections_dedup_states() {
        let app = build_test_app(vec![
            make_event(
                1,
                "TX/18 Special General Election",
                "2026-03-03",
                36,
                "Election Dates",
            ),
            make_event(
                2,
                "TX/25 Special General Election",
                "2026-03-03",
                36,
                "Election Dates",
            ),
        ]);

        assert_eq!(app.display_rows.len(), 1);
        match &app.display_rows[0] {
            DisplayRow::Grouped { states, .. } => {
                // TX should appear only once even though there are two TX districts
                assert_eq!(states, &["TX"]);
            }
            _ => panic!("Expected Grouped row"),
        }
    }

    // ── TUI snapshot tests ──

    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_tui_grouped_elections() {
        let mut app = build_test_app(vec![
            make_event(1, "AR Primary Election", "2026-03-03", 36, "Election Dates"),
            make_event(2, "NC Primary Election", "2026-03-03", 36, "Election Dates"),
            make_event(3, "TX Primary Election", "2026-03-03", 36, "Election Dates"),
        ]);

        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_tui_mixed_grouped_and_ungrouped() {
        let mut app = build_test_app(vec![
            make_event(1, "Monthly Report Due", "2026-02-20", 26, "Monthly"),
            make_event(2, "AR Primary Election", "2026-03-03", 36, "Election Dates"),
            make_event(3, "NC Primary Election", "2026-03-03", 36, "Election Dates"),
            make_event(4, "TX Primary Election", "2026-03-03", 36, "Election Dates"),
            make_event(5, "Quarterly Report Due", "2026-04-15", 25, "Quarterly"),
            make_event(
                6,
                "CA General Election",
                "2026-11-03",
                36,
                "Election Dates",
            ),
        ]);

        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_tui_grouped_detail_panel() {
        let mut app = build_test_app(vec![
            make_event(1, "AR Primary Election", "2026-03-03", 36, "Election Dates"),
            make_event(2, "NC Primary Election", "2026-03-03", 36, "Election Dates"),
            make_event(3, "TX Primary Election", "2026-03-03", 36, "Election Dates"),
        ]);
        // Select the grouped row
        app.table_state.select(Some(0));

        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }
}
