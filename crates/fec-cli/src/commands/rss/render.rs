use crate::rss::format_countdown;
use jiff::Zoned;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
    Frame,
};

use super::app::{App, CopyOption, SearchMode};

pub fn ui(f: &mut Frame, app: &mut App) {
    let has_filters = !app.active_filters.to_display_strings().is_empty();
    let has_search = app.search_mode != SearchMode::Off;

    let mut constraints = vec![Constraint::Length(3)]; // Header
    if has_filters {
        constraints.push(Constraint::Length(1)); // Filters
    }
    if has_search {
        constraints.push(Constraint::Length(1)); // Search bar
    }
    constraints.push(Constraint::Min(1)); // Table
    constraints.push(Constraint::Length(3)); // Footer

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    let mut idx = 0;
    let header_area = areas[idx];
    idx += 1;

    render_header(f, app, header_area);

    if has_filters {
        render_filters(f, app, areas[idx]);
        idx += 1;
    }
    if has_search {
        render_search_bar(f, app, areas[idx]);
        idx += 1;
    }

    render_filings_table(f, app, areas[idx]);
    idx += 1;
    render_footer(f, app, areas[idx]);

    render_status_message(f, app, f.area());

    if app.copy_menu_open {
        render_copy_menu(f, app);
    }
}

pub fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let data_age = app.data_age_display();
    let header_text = if let Some(ref error) = app.error {
        Line::from(vec![
            Span::styled(
                &app.feed_title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(format!("Error: {}", error), Style::default().fg(Color::Red)),
        ])
    } else {
        let mut spans = vec![
            Span::styled(
                if app.feed_title.is_empty() {
                    "FEC RSS Feed"
                } else {
                    &app.feed_title
                },
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Data: {}", data_age),
                Style::default().fg(Color::Gray),
            ),
        ];

        // Add since filter to header if active
        if let Some(since) = app.since_ts {
            let since_zoned = jiff::Zoned::new(since, jiff::tz::TimeZone::UTC);
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("Since: {}", since_zoned),
                Style::default().fg(Color::Yellow),
            ));
        }

        Line::from(spans)
    };

    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).title("FEC RSS Feed"));
    f.render_widget(header, area);
}

pub fn render_filters(f: &mut Frame, app: &App, area: Rect) {
    let filter_strs = app.active_filters.to_display_strings();
    if filter_strs.is_empty() {
        return;
    }

    let filter_spans: Vec<Span> = filter_strs
        .iter()
        .enumerate()
        .flat_map(|(i, s)| {
            let mut spans = vec![Span::styled(s, Style::default().fg(Color::Yellow))];
            if i < filter_strs.len() - 1 {
                spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
            }
            spans
        })
        .collect();

    let filter_line = Line::from(
        std::iter::once(Span::styled("Filters: ", Style::default().fg(Color::Gray)))
            .chain(filter_spans)
            .collect::<Vec<_>>(),
    );
    let filter_widget = Paragraph::new(filter_line);
    f.render_widget(filter_widget, area);
}

pub fn render_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![Span::styled(
        "/",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )];

    match app.search_mode {
        SearchMode::Typing => {
            spans.push(Span::styled(
                &app.search_query,
                Style::default().fg(Color::White),
            ));
            spans.push(Span::styled("█", Style::default().fg(Color::Yellow)));
        }
        SearchMode::Locked => {
            spans.push(Span::styled(
                &app.search_query,
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::styled(
                " (filtered)",
                Style::default().fg(Color::DarkGray),
            ));
        }
        SearchMode::Off => {}
    }

    let search = Paragraph::new(Line::from(spans));
    f.render_widget(search, area);
}

pub fn render_filings_table(f: &mut Frame, app: &mut App, area: Rect) {
    let now = Zoned::now();

    let header_row = Row::new(vec![
        Cell::from("Committee").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Form").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Report").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Filing ID").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("Age").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ])
    .height(1);

    let rows: Vec<Row> = app
        .filtered_indices
        .iter()
        .filter_map(|&i| app.items.get(i))
        .map(|item| {
            let committee = item.extract_committee_name();
            let form = item.form_type.as_deref().unwrap_or("-");
            let report = item.report_type.as_deref().unwrap_or("-");
            let filing_id = item.filing_id.as_deref().unwrap_or("-");
            let age = item.time_ago(&now).unwrap_or_else(|| "-".to_string());

            // Color code by form type
            let form_color = get_form_color(form);

            Row::new(vec![
                Cell::from(committee.to_string()),
                Cell::from(form.to_string()).style(Style::default().fg(form_color)),
                Cell::from(report.to_string()),
                Cell::from(filing_id.to_string()).style(Style::default().fg(Color::Cyan)),
                Cell::from(age).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let table_title = if app.error.is_some() {
        "Recent Filings (error fetching)".to_string()
    } else if app.items.is_empty() {
        "Recent Filings (loading...)".to_string()
    } else if app.search_mode != SearchMode::Off && !app.search_query.is_empty() {
        format!(
            "Recent Filings ({} of {})",
            app.filtered_indices.len(),
            app.items.len()
        )
    } else {
        format!("Recent Filings ({})", app.items.len())
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

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn get_form_color(form: &str) -> Color {
    match form {
        f if f.starts_with("F3P") => Color::Magenta,
        f if f.starts_with("F3X") => Color::Cyan,
        f if f.starts_with("F3") => Color::Green,
        f if f.starts_with("F1") => Color::Yellow,
        f if f.starts_with("F2") => Color::Blue,
        f if f.starts_with("F99") => Color::Gray,
        _ => Color::White,
    }
}

pub fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let countdown = format_countdown(app.seconds_until_refresh());
    let shortcut_style = Style::default().bold().fg(Color::White);
    let descrip_style = Style::default().fg(Color::DarkGray);

    let mut help_spans = Vec::new();

    if app.has_pending_exports() {
        let (completed, total) = app.export_progress();
        help_spans.push(Span::styled(
            format!("⟳ Exporting {}/{}", completed, total),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        help_spans.push(Span::raw("  │  "));
    }

    help_spans.extend(vec![
        Span::styled(
            format!("Next refresh: {}", countdown),
            Style::default().fg(Color::Green),
        ),
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
        Span::styled(" copy  ", descrip_style),
        Span::styled("/", shortcut_style),
        Span::styled(" search", descrip_style),
    ]);

    let help_line = Line::from(help_spans);
    let url_line = Line::from(vec![Span::styled(
        &app.feed_url,
        Style::default().fg(Color::DarkGray),
    )]);

    let footer = Paragraph::new(vec![help_line, url_line])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(footer, area);
}

pub fn render_status_message(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref msg) = app.status_message {
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
}

pub fn render_copy_menu(f: &mut Frame, app: &App) {
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
            let value = selected_item
                .and_then(|item| match opt {
                    CopyOption::FilingId => item.filing_id.as_deref(),
                    CopyOption::CommitteeId => item.committee_id.as_deref(),
                    CopyOption::RssGuid => Some(item.guid.as_str()),
                })
                .unwrap_or("-");

            let prefix = if i == app.copy_menu_selection {
                "▶ "
            } else {
                "  "
            };
            let style = if i == app.copy_menu_selection {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
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

    let menu = Paragraph::new(options).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Copy (↑↓/Enter or 1-3)")
            .border_style(Style::default().fg(Color::Yellow)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{RssArgs, RssPreset};
    use crate::rss::{ActiveFilters, Item};
    use crate::sourcer::FilingSourcer;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};
    use std::collections::HashSet;
    use std::time::{Duration, Instant};

    fn create_test_items() -> Vec<Item> {
        vec![
            Item {
                title: "ACTBLUE - Form F3X".to_string(),
                link: "https://example.com/1".to_string(),
                description: "Monthly report".to_string(),
                pub_date: None,
                guid: "FEC-12345".to_string(),
                committee_id: Some("C00401224".to_string()),
                filing_id: Some("1234567".to_string()),
                form_type: Some("F3X".to_string()),
                coverage_from: Some("2024-01-01".to_string()),
                coverage_through: Some("2024-01-31".to_string()),
                report_type: Some("M2".to_string()),
            },
            Item {
                title: "BIDEN FOR PRESIDENT - Form F3P".to_string(),
                link: "https://example.com/2".to_string(),
                description: "Quarterly report".to_string(),
                pub_date: None,
                guid: "FEC-12346".to_string(),
                committee_id: Some("C00703975".to_string()),
                filing_id: Some("1234568".to_string()),
                form_type: Some("F3P".to_string()),
                coverage_from: Some("2024-01-01".to_string()),
                coverage_through: Some("2024-03-31".to_string()),
                report_type: Some("Q1".to_string()),
            },
            Item {
                title: "DSCC - Form F3X".to_string(),
                link: "https://example.com/3".to_string(),
                description: "Year-end report".to_string(),
                pub_date: None,
                guid: "FEC-12347".to_string(),
                committee_id: Some("C00042366".to_string()),
                filing_id: Some("1234569".to_string()),
                form_type: Some("F3X".to_string()),
                coverage_from: Some("2023-07-01".to_string()),
                coverage_through: Some("2023-12-31".to_string()),
                report_type: Some("YE".to_string()),
            },
        ]
    }

    /// Test-only App builder for creating controlled test scenarios
    struct TestAppBuilder {
        items: Vec<Item>,
        feed_title: String,
        error: Option<String>,
        active_filters: ActiveFilters,
        feed_url: String,
        copy_menu_open: bool,
        copy_menu_selection: usize,
        status_message: Option<String>,
        export_queue: Vec<String>,
        export_batch_total: usize,
        selected_index: Option<usize>,
        search_query: String,
        search_mode: SearchMode,
    }

    impl Default for TestAppBuilder {
        fn default() -> Self {
            Self {
                items: Vec::new(),
                feed_title: "FEC Electronic Filing RSS Feed".to_string(),
                error: None,
                active_filters: ActiveFilters::default(),
                feed_url: "https://efilingapps.fec.gov/rss/generate".to_string(),
                copy_menu_open: false,
                copy_menu_selection: 0,
                status_message: None,
                export_queue: Vec::new(),
                export_batch_total: 0,
                selected_index: None,
                search_query: String::new(),
                search_mode: SearchMode::Off,
            }
        }
    }

    impl TestAppBuilder {
        fn items(mut self, items: Vec<Item>) -> Self {
            self.items = items;
            self
        }

        fn feed_title(mut self, title: &str) -> Self {
            self.feed_title = title.to_string();
            self
        }

        fn error(mut self, error: &str) -> Self {
            self.error = Some(error.to_string());
            self
        }

        fn filters(mut self, filters: ActiveFilters) -> Self {
            self.active_filters = filters;
            self
        }

        fn copy_menu_open(mut self) -> Self {
            self.copy_menu_open = true;
            self
        }

        fn status_message(mut self, msg: &str) -> Self {
            self.status_message = Some(msg.to_string());
            self
        }

        fn pending_exports(mut self, count: usize) -> Self {
            self.export_queue = vec!["123456".to_string(); count];
            self.export_batch_total = count + 2; // simulate some already done
            self
        }

        fn selected(mut self, index: usize) -> Self {
            self.selected_index = Some(index);
            self
        }

        fn search(mut self, query: &str, mode: SearchMode) -> Self {
            self.search_query = query.to_string();
            self.search_mode = mode;
            self
        }

        fn build(self) -> App {
            let now = Instant::now();
            let mut app = App {
                items: self.items,
                feed_title: self.feed_title,
                last_modified: None,
                last_fetch: now,
                next_fetch: now + Duration::from_secs(300),
                interval: Duration::from_secs(300),
                table_state: ratatui::widgets::TableState::default(),
                should_exit: false,
                error: self.error,
                args: RssArgs {
                    watch: false,
                    interval: 300,
                    limit: 20,
                    preset: RssPreset::All,
                    form_type: None,
                    committee: None,
                    committee_label: None,
                    state: None,
                    party: None,
                    export: None,
                    cover_only: false,
                    since: None,
                    rpc: false,
                    write_metadata: false,
                    include_all_bulk: false,
                },
                active_filters: self.active_filters,
                feed_url: self.feed_url,
                sourcer: FilingSourcer::new(None, false),
                last_key: None,
                copy_menu_open: self.copy_menu_open,
                copy_menu_selection: self.copy_menu_selection,
                status_message: self.status_message,
                export_db: None,
                exported_ids: HashSet::new(),
                cover_only: false,
                export_count: 0,
                export_queue: self.export_queue,
                export_batch_total: self.export_batch_total,
                since_ts: None,
                search_query: self.search_query.clone(),
                search_mode: self.search_mode,
                filtered_indices: Vec::new(),
            };
            app.update_filter();
            if let Some(idx) = self.selected_index {
                app.table_state.select(Some(idx));
            }
            app
        }
    }

    #[test]
    fn test_ui_empty_state() {
        let mut app = TestAppBuilder::default().build();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_ui_with_items() {
        let mut app = TestAppBuilder::default()
            .items(create_test_items())
            .selected(0)
            .build();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_ui_with_error() {
        let mut app = TestAppBuilder::default()
            .error("Connection timeout")
            .build();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_ui_with_filters() {
        let filters = ActiveFilters {
            preset: Some("Presidential".to_string()),
            form_type: Some("F3P".to_string()),
            committee: None,
            state: Some("CA".to_string()),
            party: None,
        };
        let mut app = TestAppBuilder::default()
            .items(create_test_items())
            .filters(filters)
            .selected(0)
            .build();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_ui_with_status_message() {
        let mut app = TestAppBuilder::default()
            .items(create_test_items())
            .status_message("Copied: C00401224")
            .selected(0)
            .build();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_ui_with_pending_exports() {
        let mut app = TestAppBuilder::default()
            .items(create_test_items())
            .pending_exports(3)
            .selected(0)
            .build();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_ui_copy_menu() {
        let mut app = TestAppBuilder::default()
            .items(create_test_items())
            .selected(0)
            .copy_menu_open()
            .build();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_header_normal() {
        let app = TestAppBuilder::default()
            .feed_title("FEC Electronic Filing RSS Feed")
            .build();
        let mut terminal = Terminal::new(TestBackend::new(80, 5)).unwrap();
        terminal.draw(|f| render_header(f, &app, f.area())).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_header_with_error() {
        let app = TestAppBuilder::default()
            .error("Network error: connection refused")
            .build();
        let mut terminal = Terminal::new(TestBackend::new(80, 5)).unwrap();
        terminal.draw(|f| render_header(f, &app, f.area())).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_filters() {
        let filters = ActiveFilters {
            preset: Some("Monthly".to_string()),
            form_type: Some("F3X".to_string()),
            committee: Some("C00401224".to_string()),
            state: None,
            party: Some("DEM".to_string()),
        };
        let app = TestAppBuilder::default().filters(filters).build();
        let mut terminal = Terminal::new(TestBackend::new(80, 3)).unwrap();
        terminal
            .draw(|f| render_filters(f, &app, f.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_footer_normal() {
        let app = TestAppBuilder::default().build();
        let mut terminal = Terminal::new(TestBackend::new(80, 4)).unwrap();
        terminal.draw(|f| render_footer(f, &app, f.area())).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_footer_with_exports() {
        let app = TestAppBuilder::default().pending_exports(5).build();
        let mut terminal = Terminal::new(TestBackend::new(80, 4)).unwrap();
        terminal.draw(|f| render_footer(f, &app, f.area())).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_filings_table() {
        let mut app = TestAppBuilder::default()
            .items(create_test_items())
            .selected(1)
            .build();
        let mut terminal = Terminal::new(TestBackend::new(100, 10)).unwrap();
        terminal
            .draw(|f| render_filings_table(f, &mut app, f.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_filings_table_empty() {
        let mut app = TestAppBuilder::default().build();
        let mut terminal = Terminal::new(TestBackend::new(100, 10)).unwrap();
        terminal
            .draw(|f| render_filings_table(f, &mut app, f.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_copy_menu() {
        let app = TestAppBuilder::default()
            .items(create_test_items())
            .selected(0)
            .copy_menu_open()
            .build();
        let mut terminal = Terminal::new(TestBackend::new(60, 15)).unwrap();
        terminal.draw(|f| render_copy_menu(f, &app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_ui_wide_terminal() {
        let mut app = TestAppBuilder::default()
            .items(create_test_items())
            .selected(0)
            .build();
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_ui_narrow_terminal() {
        let mut app = TestAppBuilder::default()
            .items(create_test_items())
            .selected(0)
            .build();
        let mut terminal = Terminal::new(TestBackend::new(60, 20)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_ui_search_typing() {
        let mut app = TestAppBuilder::default()
            .items(create_test_items())
            .search("act", SearchMode::Typing)
            .selected(0)
            .build();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_ui_search_locked() {
        let mut app = TestAppBuilder::default()
            .items(create_test_items())
            .search("actblue", SearchMode::Locked)
            .selected(0)
            .build();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_ui_search_no_results() {
        let mut app = TestAppBuilder::default()
            .items(create_test_items())
            .search("zzzzz", SearchMode::Typing)
            .build();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }
}
