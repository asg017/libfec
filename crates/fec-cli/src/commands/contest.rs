use crate::cache::bulk::candidate_summary::{get_contest_candidates, ContestCandidate};
use crate::sourcer::{Contest, FilingSourcer};
use crate::tui::filing_detail::format_usd;
use crate::tui::{navigation_popup_help_line, popup_area, HelpBar};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
    Frame, Terminal,
};
use std::collections::HashMap;
use std::io;

struct App {
    candidates: Vec<ContestCandidate>,
    /// Map from candidate_id to principal committee ID
    committee_ids: HashMap<String, String>,
    /// The latest coverage_end_date among all candidates (for dimming stale rows)
    latest_date: String,
    contest_url: String,
    table_state: TableState,
    title: String,
    show_yank_popup: bool,
    yank_selected: usize,
}

impl App {
    fn new(
        candidates: Vec<ContestCandidate>,
        committee_ids: HashMap<String, String>,
        contest_url: String,
        title: String,
    ) -> Self {
        let mut table_state = TableState::default();
        if !candidates.is_empty() {
            table_state.select(Some(0));
        }
        let latest_date = candidates
            .iter()
            .map(|c| date_to_sortable(&c.coverage_end_date))
            .max()
            .unwrap_or_default();
        Self {
            candidates,
            committee_ids,
            latest_date,
            contest_url,
            table_state,
            title,
            show_yank_popup: false,
            yank_selected: 0,
        }
    }

    fn next(&mut self) {
        if self.candidates.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => (i + 1).min(self.candidates.len() - 1),
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.candidates.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn first(&mut self) {
        if !self.candidates.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    fn last(&mut self) {
        if !self.candidates.is_empty() {
            self.table_state.select(Some(self.candidates.len() - 1));
        }
    }

    fn selected_candidate(&self) -> Option<&ContestCandidate> {
        self.table_state
            .selected()
            .and_then(|i| self.candidates.get(i))
    }

    fn open_contest_in_browser(&self) {
        let _ = open::that(&self.contest_url);
    }

    fn open_candidate_in_browser(&self) {
        if let Some(c) = self.selected_candidate() {
            let url = format!(
                "https://www.fec.gov/data/candidate/{}/",
                c.candidate_id
            );
            let _ = open::that(url);
        }
    }

    fn yank_options(&self) -> Vec<(&str, String)> {
        let Some(c) = self.selected_candidate() else {
            return vec![];
        };
        let mut opts = vec![("Candidate ID", c.candidate_id.clone())];
        if let Some(cid) = self.committee_ids.get(&c.candidate_id) {
            opts.push(("Committee ID", cid.clone()));
        }
        opts
    }

    fn yank_next(&mut self) {
        let count = self.yank_options().len();
        if count > 0 {
            self.yank_selected = (self.yank_selected + 1) % count;
        }
    }

    fn yank_previous(&mut self) {
        let count = self.yank_options().len();
        if count > 0 {
            self.yank_selected = if self.yank_selected == 0 {
                count - 1
            } else {
                self.yank_selected - 1
            };
        }
    }

    fn copy_selected_yank(&self) {
        let options = self.yank_options();
        if let Some((_, value)) = options.get(self.yank_selected) {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(value);
            }
        }
    }
}

fn fec_contest_url(contest: &Contest, cycle: u16) -> String {
    match contest {
        Contest::President => {
            format!("https://www.fec.gov/data/elections/president/{}/", cycle)
        }
        Contest::Senate { state } => {
            format!(
                "https://www.fec.gov/data/elections/senate/{}/{}/",
                state, cycle
            )
        }
        Contest::House { state, district } => {
            format!(
                "https://www.fec.gov/data/elections/house/{}/{}/{}/",
                state, district, cycle
            )
        }
    }
}

fn contest_title(contest: &Contest, cycle: u16) -> String {
    match contest {
        Contest::President => format!("President ({})", cycle),
        Contest::Senate { state } => {
            let name = state_name(state);
            format!("{} Senate ({})", name, cycle)
        }
        Contest::House { state, district } => {
            let name = state_name(state);
            let ordinal = district_ordinal(district);
            format!("{} {} District ({})", name, ordinal, cycle)
        }
    }
}

fn district_ordinal(district: &str) -> String {
    let n: u32 = district.parse().unwrap_or(0);
    if n == 0 {
        return "At-Large".to_string();
    }
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{}{}", n, suffix)
}

fn state_name(code: &str) -> &str {
    match code {
        "AL" => "Alabama",
        "AK" => "Alaska",
        "AZ" => "Arizona",
        "AR" => "Arkansas",
        "CA" => "California",
        "CO" => "Colorado",
        "CT" => "Connecticut",
        "DE" => "Delaware",
        "FL" => "Florida",
        "GA" => "Georgia",
        "HI" => "Hawaii",
        "ID" => "Idaho",
        "IL" => "Illinois",
        "IN" => "Indiana",
        "IA" => "Iowa",
        "KS" => "Kansas",
        "KY" => "Kentucky",
        "LA" => "Louisiana",
        "ME" => "Maine",
        "MD" => "Maryland",
        "MA" => "Massachusetts",
        "MI" => "Michigan",
        "MN" => "Minnesota",
        "MS" => "Mississippi",
        "MO" => "Missouri",
        "MT" => "Montana",
        "NE" => "Nebraska",
        "NV" => "Nevada",
        "NH" => "New Hampshire",
        "NJ" => "New Jersey",
        "NM" => "New Mexico",
        "NY" => "New York",
        "NC" => "North Carolina",
        "ND" => "North Dakota",
        "OH" => "Ohio",
        "OK" => "Oklahoma",
        "OR" => "Oregon",
        "PA" => "Pennsylvania",
        "RI" => "Rhode Island",
        "SC" => "South Carolina",
        "SD" => "South Dakota",
        "TN" => "Tennessee",
        "TX" => "Texas",
        "UT" => "Utah",
        "VT" => "Vermont",
        "VA" => "Virginia",
        "WA" => "Washington",
        "WV" => "West Virginia",
        "WI" => "Wisconsin",
        "WY" => "Wyoming",
        "DC" => "District of Columbia",
        "AS" => "American Samoa",
        "GU" => "Guam",
        "MP" => "Northern Mariana Islands",
        "PR" => "Puerto Rico",
        "VI" => "Virgin Islands",
        _ => code,
    }
}

fn party_color(code: &str) -> Color {
    match code {
        "DEM" | "DFL" | "DNL" | "D/C" => Color::Blue,
        "REP" => Color::Red,
        "LIB" => Color::Yellow,
        "GRE" | "GR" | "IGR" | "PG" | "DCG" | "DGR" => Color::Green,
        _ => Color::Gray,
    }
}

/// Convert MM/DD/YYYY to YYYY-MM-DD for lexicographic comparison
fn date_to_sortable(date: &str) -> String {
    let parts: Vec<&str> = date.split('/').collect();
    if parts.len() == 3 {
        format!("{}-{}-{}", parts[2], parts[0], parts[1])
    } else {
        date.to_string()
    }
}

fn render_title(f: &mut Frame, app: &App, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            &app.title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {} candidates", app.candidates.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    f.render_widget(title, area);
}

fn render_candidates_table(f: &mut Frame, app: &mut App, area: Rect) {
    if app.candidates.is_empty() {
        let msg = Paragraph::new("No candidates found for this contest.")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(msg, area);
        return;
    }

    let header_style = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);
    let header = Row::new(vec![
        Cell::new("Candidate"),
        Cell::new("Party"),
        Cell::new("Receipts"),
        Cell::new("Disbursed"),
        Cell::new("Indiv."),
        Cell::new("COH"),
        Cell::new("Thru"),
    ])
    .style(header_style)
    .height(1);

    let rows: Vec<Row> = app
        .candidates
        .iter()
        .map(|c| {
            let is_stale = date_to_sortable(&c.coverage_end_date) < app.latest_date;

            // Name cell: append gold (I) for incumbents
            let name_cell = if c.incumbent_challenger_status == "I" {
                Cell::from(Line::from(vec![
                    Span::raw(&c.name),
                    Span::styled(" (I)", Style::default().fg(Color::Yellow)),
                ]))
            } else {
                Cell::new(c.name.clone())
            };

            let thru_style = if is_stale {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            Row::new(vec![
                name_cell,
                Cell::new(c.party_affiliation.clone())
                    .style(Style::default().fg(party_color(&c.party_affiliation))),
                Cell::from(Line::from(format_usd(c.total_receipts)).alignment(Alignment::Right)),
                Cell::from(
                    Line::from(format_usd(c.total_disbursements)).alignment(Alignment::Right),
                ),
                Cell::from(
                    Line::from(format_usd(c.total_individual_contributions))
                        .alignment(Alignment::Right),
                ),
                Cell::from(
                    Line::from(format_usd(c.cash_on_hand_close)).alignment(Alignment::Right),
                ),
                Cell::new(c.coverage_end_date.clone()).style(thru_style),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(20),
        Constraint::Length(5),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_help(f: &mut Frame, area: Rect) {
    HelpBar::new()
        .item("q", " quit")
        .text("↑/↓ or j/k navigate")
        .item("g", "/")
        .item("G", " first/last")
        .item("o", " open contest")
        .item("Enter", " open candidate")
        .item("y", " yank")
        .render(f, area);
}

fn render_yank_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup = popup_area(area, 50, 40);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Copy to Clipboard")
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let options = app.yank_options();
    let mut lines = vec![
        Line::from(Span::styled(
            "Select what to copy:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (idx, (label, value)) in options.iter().enumerate() {
        let is_selected = idx == app.yank_selected;
        let prefix = if is_selected { ">> " } else { "   " };
        let style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(Span::styled(
            format!("{}{}: {}", prefix, label, value),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(navigation_popup_help_line());

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn ui(f: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Min(1),    // Candidates table
            Constraint::Length(2), // Help bar
        ]);

    let [title_area, table_area, help_area] = f.area().layout(&layout);

    render_title(f, app, title_area);
    render_candidates_table(f, app, table_area);
    render_help(f, help_area);

    if app.show_yank_popup {
        render_yank_popup(f, app, f.area());
    }
}

/// Look up principal campaign committee IDs for a list of candidates
fn lookup_committee_ids(
    db: &rusqlite::Connection,
    cycle: u16,
    candidates: &[ContestCandidate],
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    // Query the candidates bulk table which has principal_campaign_committee
    let sql = "SELECT candidate_id, principal_campaign_committee
               FROM libfec_candidates
               WHERE cycle = ? AND candidate_id = ? AND principal_campaign_committee IS NOT NULL
                 AND principal_campaign_committee != ''";
    if let Ok(mut stmt) = db.prepare(sql) {
        for c in candidates {
            if let Ok(cid) = stmt.query_row(
                rusqlite::params![cycle, &c.candidate_id],
                |row| row.get::<_, String>(1),
            ) {
                map.insert(c.candidate_id.clone(), cid);
            }
        }
    }
    map
}

pub fn contest(mut sourcer: FilingSourcer, contest: Contest, cycle: u16) -> Result<()> {
    // Determine office/state/district from Contest enum
    let (office, state, district) = match &contest {
        Contest::President => ("P", None, None),
        Contest::Senate { state } => ("S", Some(state.as_str()), None),
        Contest::House { state, district } => ("H", Some(state.as_str()), Some(district.as_str())),
    };

    let title = contest_title(&contest, cycle);
    let contest_url = fec_contest_url(&contest, cycle);

    // Open bulk database and fetch candidates
    let mut db = sourcer.cache.open_bulk_data_database()?;
    let candidates = get_contest_candidates(&mut db, cycle, office, state, district)?;

    // Look up committee IDs from the candidates bulk table (best-effort)
    let committee_ids = lookup_committee_ids(&db, cycle, &candidates);

    // Launch TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(candidates, committee_ids, contest_url, title);

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

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

            if app.show_yank_popup {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('y') => {
                        app.show_yank_popup = false;
                    }
                    KeyCode::Down | KeyCode::Char('j') => app.yank_next(),
                    KeyCode::Up | KeyCode::Char('k') => app.yank_previous(),
                    KeyCode::Enter => {
                        app.copy_selected_yank();
                        app.show_yank_popup = false;
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    KeyCode::Char('g') => app.first(),
                    KeyCode::Char('G') => app.last(),
                    KeyCode::Char('o') => app.open_contest_in_browser(),
                    KeyCode::Enter => app.open_candidate_in_browser(),
                    KeyCode::Char('y') => {
                        app.show_yank_popup = true;
                        app.yank_selected = 0;
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    fn create_test_candidates() -> Vec<ContestCandidate> {
        vec![
            ContestCandidate {
                candidate_id: "H8CA41196".to_string(),
                name: "CALVERT, KEN".to_string(),
                party_affiliation: "REP".to_string(),
                incumbent_challenger_status: "I".to_string(),
                total_receipts: 3245000.0,
                total_disbursements: 2100000.0,
                cash_on_hand_close: 1145000.0,
                total_individual_contributions: 2800000.0,
                other_committee_contributions: 350000.0,
                debts_owed_by: 0.0,
                coverage_end_date: "12/31/2025".to_string(),
            },
            ContestCandidate {
                candidate_id: "H2CA41184".to_string(),
                name: "ROLLINS, WILL".to_string(),
                party_affiliation: "DEM".to_string(),
                incumbent_challenger_status: "C".to_string(),
                total_receipts: 2890000.0,
                total_disbursements: 1950000.0,
                cash_on_hand_close: 940000.0,
                total_individual_contributions: 2600000.0,
                other_committee_contributions: 200000.0,
                debts_owed_by: 15000.0,
                coverage_end_date: "12/31/2025".to_string(),
            },
            ContestCandidate {
                candidate_id: "H6CA41200".to_string(),
                name: "SMITH, JOHN Q".to_string(),
                party_affiliation: "LIB".to_string(),
                incumbent_challenger_status: "C".to_string(),
                total_receipts: 45000.0,
                total_disbursements: 32000.0,
                cash_on_hand_close: 13000.0,
                total_individual_contributions: 40000.0,
                other_committee_contributions: 5000.0,
                debts_owed_by: 0.0,
                coverage_end_date: "06/30/2025".to_string(),
            },
        ]
    }

    #[test]
    fn test_contest_ui() {
        let candidates = create_test_candidates();
        let mut app = App::new(candidates, HashMap::new(), String::new(), "California 41st District (2026)".to_string());
        let mut terminal = Terminal::new(TestBackend::new(100, 20)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_contest_ui_empty() {
        let mut app = App::new(vec![], HashMap::new(), String::new(), "California 41st District (2026)".to_string());
        let mut terminal = Terminal::new(TestBackend::new(100, 20)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_contest_ui_president() {
        let candidates = vec![
            ContestCandidate {
                candidate_id: "P80000722".to_string(),
                name: "HARRIS, KAMALA D.".to_string(),
                party_affiliation: "DEM".to_string(),
                incumbent_challenger_status: "I".to_string(),
                total_receipts: 997000000.0,
                total_disbursements: 950000000.0,
                cash_on_hand_close: 47000000.0,
                total_individual_contributions: 800000000.0,
                other_committee_contributions: 100000000.0,
                debts_owed_by: 0.0,
                coverage_end_date: "12/31/2025".to_string(),
            },
            ContestCandidate {
                candidate_id: "P80000001".to_string(),
                name: "TRUMP, DONALD J.".to_string(),
                party_affiliation: "REP".to_string(),
                incumbent_challenger_status: "C".to_string(),
                total_receipts: 890000000.0,
                total_disbursements: 850000000.0,
                cash_on_hand_close: 40000000.0,
                total_individual_contributions: 700000000.0,
                other_committee_contributions: 90000000.0,
                debts_owed_by: 0.0,
                coverage_end_date: "12/31/2025".to_string(),
            },
        ];
        let mut app = App::new(candidates, HashMap::new(), String::new(), "President (2028)".to_string());
        let mut terminal = Terminal::new(TestBackend::new(100, 12)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_contest_ui_senate() {
        let candidates = vec![
            ContestCandidate {
                candidate_id: "S8CA00001".to_string(),
                name: "FEINSTEIN, DIANNE".to_string(),
                party_affiliation: "DEM".to_string(),
                incumbent_challenger_status: "I".to_string(),
                total_receipts: 5000000.0,
                total_disbursements: 4500000.0,
                cash_on_hand_close: 500000.0,
                total_individual_contributions: 4000000.0,
                other_committee_contributions: 800000.0,
                debts_owed_by: 0.0,
                coverage_end_date: "12/31/2025".to_string(),
            },
        ];
        let mut app = App::new(candidates, HashMap::new(), String::new(), "California Senate (2026)".to_string());
        let mut terminal = Terminal::new(TestBackend::new(100, 12)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_contest_title() {
        assert_eq!(
            contest_title(&Contest::President, 2028),
            "President (2028)"
        );
        assert_eq!(
            contest_title(
                &Contest::Senate {
                    state: "CA".to_string()
                },
                2026
            ),
            "California Senate (2026)"
        );
        assert_eq!(
            contest_title(
                &Contest::House {
                    state: "CA".to_string(),
                    district: "41".to_string()
                },
                2026
            ),
            "California 41st District (2026)"
        );
        assert_eq!(
            contest_title(
                &Contest::House {
                    state: "NY".to_string(),
                    district: "03".to_string()
                },
                2026
            ),
            "New York 3rd District (2026)"
        );
        assert_eq!(
            contest_title(
                &Contest::House {
                    state: "AK".to_string(),
                    district: "00".to_string()
                },
                2026
            ),
            "Alaska At-Large District (2026)"
        );
    }

    #[test]
    fn test_district_ordinal() {
        assert_eq!(district_ordinal("01"), "1st");
        assert_eq!(district_ordinal("02"), "2nd");
        assert_eq!(district_ordinal("03"), "3rd");
        assert_eq!(district_ordinal("04"), "4th");
        assert_eq!(district_ordinal("11"), "11th");
        assert_eq!(district_ordinal("12"), "12th");
        assert_eq!(district_ordinal("13"), "13th");
        assert_eq!(district_ordinal("21"), "21st");
        assert_eq!(district_ordinal("22"), "22nd");
        assert_eq!(district_ordinal("23"), "23rd");
        assert_eq!(district_ordinal("00"), "At-Large");
    }
}
