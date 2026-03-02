/*!
 * Landing Page TUI
 *
 * Interactive home screen shown when `fec` is run with no arguments.
 * Displays a menu of available TUI commands (Search, Dates, RSS) and
 * launches the selected command. Returns to the landing page after
 * each command exits.
 */

use crate::cli::{DatesArgs, DatesFormat, RssArgs, RssPreset, SearchArgs};
use crate::sourcer::FilingSourcer;
use crate::tui::HelpBar;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame, Terminal,
};
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuItem {
    Search,
    Dates,
    Rss,
}

impl MenuItem {
    const ALL: [MenuItem; 3] = [MenuItem::Search, MenuItem::Dates, MenuItem::Rss];

    fn title(&self) -> &'static str {
        match self {
            MenuItem::Search => "Search",
            MenuItem::Dates => "Dates",
            MenuItem::Rss => "RSS Feed",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            MenuItem::Search => "Search candidates and committees",
            MenuItem::Dates => "View upcoming FEC calendar dates",
            MenuItem::Rss => "Watch recent FEC filings",
        }
    }

    fn shortcut(&self) -> char {
        match self {
            MenuItem::Search => '1',
            MenuItem::Dates => '2',
            MenuItem::Rss => '3',
        }
    }
}

enum LandingSelection {
    Quit,
    Launch(MenuItem),
}

struct App {
    selected: usize,
}

impl App {
    fn new() -> Self {
        Self { selected: 0 }
    }

    fn select_next(&mut self) {
        self.selected = (self.selected + 1) % MenuItem::ALL.len();
    }

    fn select_previous(&mut self) {
        self.selected = if self.selected == 0 {
            MenuItem::ALL.len() - 1
        } else {
            self.selected - 1
        };
    }

    fn selected_item(&self) -> MenuItem {
        MenuItem::ALL[self.selected]
    }
}

pub fn landing(cache_directory: Option<PathBuf>) -> Result<()> {
    loop {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let selection = run_event_loop(&mut terminal)?;

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        match selection {
            LandingSelection::Quit => return Ok(()),
            LandingSelection::Launch(item) => {
                let sourcer = FilingSourcer::new(cache_directory.clone(), false);
                match item {
                    MenuItem::Search => {
                        let args = SearchArgs {
                            query: String::new(),
                            cycle: 2026,
                            rpc: false,
                        };
                        crate::commands::search(sourcer, &args)?;
                    }
                    MenuItem::Dates => {
                        let args = DatesArgs {
                            category: "elections,deadlines".to_string(),
                            days: 365,
                            limit: 500,
                            state: None,
                            format: DatesFormat::Tui,
                            as_of: None,
                        };
                        crate::commands::dates(sourcer, &args)?;
                    }
                    MenuItem::Rss => {
                        let args = RssArgs {
                            watch: true,
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
                        };
                        crate::commands::rss(sourcer, &args)?;
                    }
                }
            }
        }
    }
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<LandingSelection> {
    let mut app = App::new();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(LandingSelection::Quit);
                    }
                    KeyCode::Char('c')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        return Ok(LandingSelection::Quit);
                    }
                    KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                    KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
                    KeyCode::Enter => {
                        return Ok(LandingSelection::Launch(app.selected_item()));
                    }
                    KeyCode::Char('1') => {
                        return Ok(LandingSelection::Launch(MenuItem::Search));
                    }
                    KeyCode::Char('2') => {
                        return Ok(LandingSelection::Launch(MenuItem::Dates));
                    }
                    KeyCode::Char('3') => {
                        return Ok(LandingSelection::Launch(MenuItem::Rss));
                    }
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Menu (centered)
            Constraint::Length(2), // Help bar
        ]);

    let [content_area, help_area] = f.area().layout(&layout);

    render_menu(f, app, content_area);
    render_help(f, help_area);
}

fn render_menu(f: &mut Frame, app: &App, area: Rect) {
    // ASCII art (3) + blank + tagline + blank + 3 menu items = 9 lines
    let menu_height = 9u16;
    let menu_width = 63u16;

    let vertical = Layout::vertical([Constraint::Length(menu_height)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(menu_width)]).flex(Flex::Center);
    let [centered] = vertical.areas(area);
    let [centered] = horizontal.areas(centered);

    let mut lines = Vec::new();

    // ASCII art with green gradient (dark to bright)
    // courtesy of https://patorjk.com/software/taag/#p=testall&f=Alpha&t=libfec&x=none&v=4&h=4&w=80&we=false
    // "ANSI Compact" i think
    let logo_lines = [
        "▄▄    ▄▄ ▄▄▄▄  ▄▄▄▄▄ ▄▄▄▄▄  ▄▄▄▄",
        "██    ██ ██▄██ ██▄▄  ██▄▄  ██▀▀▀",
        "██▄▄▄ ██ ██▄█▀ ██    ██▄▄▄ ▀████",
    ];
    let gradient = [
        Color::Rgb(0, 140, 60),
        Color::Rgb(0, 190, 80),
        Color::Rgb(0, 240, 100),
    ];
    for (i, (line, color)) in logo_lines.iter().zip(gradient.iter()).enumerate() {
        if i == logo_lines.len() - 1 {
            // Last logo line: append version floated right
            let version = format!("v{}", env!("CARGO_PKG_VERSION"));
            let logo_len = line.chars().count();
            let version_len = version.len();
            let padding = (menu_width as usize).saturating_sub(logo_len + version_len);
            lines.push(Line::from(vec![
                Span::styled(*line, Style::default().fg(*color)),
                Span::raw(" ".repeat(padding)),
                Span::styled(version, Style::default().fg(Color::White)),
            ]));
        } else {
            lines.push(Line::from(Span::styled(*line, Style::default().fg(*color))));
        }
    }

    // Blank separator
    lines.push(Line::from(""));

    // Tagline
    lines.push(Line::from(Span::styled(
        "A CLI for exploring federal campaign finance data from the FEC",
        Style::default().fg(Color::Gray),
    )));

    // Blank separator
    lines.push(Line::from(""));

    // Menu items
    for (i, item) in MenuItem::ALL.iter().enumerate() {
        let is_selected = i == app.selected;

        let indicator = if is_selected { "> " } else { "  " };
        let shortcut = format!("{}", item.shortcut());
        let title = item.title();
        let description = format!("  {}", item.description());

        let style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        lines.push(Line::from(vec![
            Span::styled(indicator, style),
            Span::styled(
                shortcut,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default()),
            Span::styled(title, style),
            Span::styled(description, Style::default().fg(Color::DarkGray)),
        ]));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, centered);
}

fn render_help(f: &mut Frame, area: Rect) {
    HelpBar::new()
        .keys(vec!["↑", "↓", "j", "k"], " navigate  ")
        .item("Enter", " select  ")
        .keys(vec!["1", "2", "3"], " jump  ")
        .item("q", " quit")
        .render(f, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    fn create_app() -> App {
        App::new()
    }

    #[test]
    fn test_default_state() {
        let mut app = create_app();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_second_item_selected() {
        let mut app = create_app();
        app.selected = 1;
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_third_item_selected() {
        let mut app = create_app();
        app.selected = 2;
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_narrow_terminal() {
        let mut app = create_app();
        let mut terminal = Terminal::new(TestBackend::new(60, 20)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_wide_terminal() {
        let mut app = create_app();
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }
}
