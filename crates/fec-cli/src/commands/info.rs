//! Info command - Display detailed information about FEC filings and committees
//!
//! This module provides functionality to display detailed information about:
//! - FEC filings (by filing ID)
//! - Committees (by committee ID starting with 'C')
//!
//! For filings, it shows cover sheet information, form details, and optionally
//! a full breakdown of all rows in the filing.
//!
//! For committees, it launches an interactive TUI displaying all available
//! committee information from the bulk data cache.

use crate::tui::{
    candidate_detail::{render_candidate_detail, CandidateDetailAction, CandidateDetailState},
    committee_detail::{render_committee_detail, CommitteeDetailAction, CommitteeDetailState},
    filing_detail::{render_filing_detail, FilingDetail, FilingDetailAction, FilingDetailState},
    truncate_string,
};
use colored::Colorize;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fec_parser::{
    covers::{Cover, Form3PSummary, Form3Summary},
    report_code_label, Filing,
};
use indicatif::{HumanBytes, ProgressBar};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};
use serde_json::Value;
use std::{
    collections::HashMap,
    io::{self, Read},
    time::Duration,
};

use tabled::{
    builder::Builder as TableBuilder,
    settings::{object::Columns as TableColumns, Alignment as TableAlignment, Style as TableStyle},
};

use crate::{
    cli::{CmdInfoFormat, InfoArgs, InfoDisplayMode},
    sourcer::FilingSourcer,
};
struct FilingFormMetadata {
    count: usize,
    bytes: usize,
}

fn form_name(form_type: &str) -> &str {
    let base_form_type = if form_type.ends_with('A') || form_type.ends_with('N') {
        &form_type[..form_type.len() - 1]
    } else {
        form_type
    };

    match base_form_type {
    "F1"  => "Statement of Organization",
    "F1M"  => "Notification of Multicandidate Status",
    "F2"  => "Statement of Candidacy",
    "F24"  => "24/48 Hour Report of Independent Expenditures",
    "F3"  => "Report of Receipts and Disbursements for an Authorized Committee",
    "F3P"  => "Report of Receipts and Disbursements by an Authorized Committee of a Candidate for The Office of President or Vice President",
    "F3L"  => "Report of Contributions Bundled by Lobbyists/Registrants and Lobbyist/Registrant PACs",
    "F3X"  => "Report of Receipts and Disbursements for other than an Authorized Committee",
    "F4"  => "Report of Receipts and Disbursements for a Committee or Organization Supporting a Nomination Convention",
    "F5"  => "Report of Independent Expenditures Made and Contributions Received",
    "F6"  => "48 Hour Notice of Contributions/Loans Received",
    "F7"  => "Report of Communication Costs by Corporations and Membership Organizations",
    "F8"  => "Debt Settlement Plan",
    "F9"  => "24 Hour Notice of Disbursements for Electioneering Communications",
    "F13"  => "Report of Donations Accepted for Inaugural Committee",
    "F99"  => "Miscellaneous Text",
    "FRQ"  => "Request for Additional Information",
    _ => "",
  }
}
use num_format::{Locale, ToFormattedString};
use tabled::{builder::Builder, settings::Style};

fn format_usd(amount: f64) -> String {
    let rounded = (amount * 100.0).round() as i64; // convert to cents
    let dollars = rounded / 100;
    let cents = (rounded % 100).abs(); // handle negative cents correctly

    format!("${}.{:02}", dollars.to_formatted_string(&Locale::en), cents)
}

fn print_summary(summary: &Form3PSummary) {
    let mut b = Builder::with_capacity(3, 0);
    b.push_record(["Summary"]);

    let items = vec![
        (
            "6. Cash on Hand at BEGINNING of the Reporting Period",
            summary.line6_cash_on_hand_beginning_period,
        ),
        (
            "7. Total Receipts This Period",
            summary.line7_total_receipts,
        ),
        ("8. Subtotal (6 + 7)", summary.line8_subtotal),
        (
            "9. Total Disbursements This Period",
            summary.line9_total_disbursements,
        ),
        (
            "10. Cash on Hand at CLOSE of the Reporting Period",
            summary.line10_cash_on_hand_end_period,
        ),
        (
            "11. Debts and Obligations Owed TO the Committee",
            summary.line11_debts_owed_to_committee,
        ),
        (
            "12. Debts and Obligations Owed BY the Committee",
            summary.line12_debts_owed_by_committee,
        ),
        (
            "13. Expenditures Subject To Limitation",
            summary.line13_expenditures_subject_to_limits,
        ),
        (
            "14. NET Contributions (Other than Loans)",
            summary.line14_net_contributions_other_than_loans,
        ),
        (
            "15. NET Operating Expenditures",
            summary.line15_net_operating_expenditures,
        ),
    ];
    for (label, value) in items {
        b.push_record([label.to_string(), format_usd(value)]);
    }

    let mut table = b.build();
    table.with(Style::modern());
    table.modify(
        tabled::settings::object::Columns::last(),
        tabled::settings::Alignment::right(),
    );

    // make 1st row (title) span entire width
    table
        .modify((0, 0), tabled::settings::Span::column(2))
        .modify((0, 0), tabled::settings::Alignment::center());
    // border correct bc header row does weird stuff
    table.with(tabled::settings::themes::BorderCorrection::span());
    println!("{}", table)
}

fn print_summary_form3(summary: &Form3Summary) {
    let mut b = Builder::with_capacity(3, 0);
    b.push_record(["Summary"]);

    let items = vec![
        (
            "6. Total Contributions (Other Than Loans)",
            summary.line6_total_contributions_no_loans,
        ),
        (
            "7. Total Contribution Refunds",
            summary.line7_total_contribution_refunds,
        ),
        (
            "8. Net Contributions (Other Than Loans)",
            summary.line8_net_contributions,
        ),
        (
            "9. Total Operating Expenditures",
            summary.line9_total_operating_expenditures,
        ),
        (
            "10. Total Offset to Operating Expenditures",
            summary.line10_total_offset_to_operating_expenditures,
        ),
        (
            "11. Net Operating Expenditures",
            summary.line11_net_operating_expenditures,
        ),
        (
            "12. Cash on Hand at CLOSE of the Reporting Period",
            summary.line12_cash_on_hand_close_of_period,
        ),
        (
            "13. Debts and Obligations Owed TO the Committee",
            summary.line13_debts_owed_to_committee,
        ),
        (
            "14. Debts and Obligations Owed BY the Committee",
            summary.line14_debts_owed_by_committee,
        ),
    ];
    for (label, value) in items {
        b.push_record([label.to_string(), format_usd(value)]);
    }

    let mut table = b.build();
    table.with(Style::modern());
    table.modify(
        tabled::settings::object::Columns::last(),
        tabled::settings::Alignment::right(),
    );

    // make 1st row (title) span entire width
    table
        .modify((0, 0), tabled::settings::Span::column(2))
        .modify((0, 0), tabled::settings::Alignment::center());
    // border correct bc header row does weird stuff
    table.with(tabled::settings::themes::BorderCorrection::span());
    println!("{}", table)
}

fn process_filing<R: Read>(
    filing: &mut Filing<R>,
    format: &CmdInfoFormat,
    spinner: &Option<ProgressBar>,
    full: bool,
) {
    if matches!(format, CmdInfoFormat::Human) {
        if !full {
            if let Some(ref spinner) = spinner {
                spinner.finish_and_clear();
            }
        }

        println!(
            "{} {} {} by {} ({})",
            format!("FEC-{}", filing.filing_id).bold(),
            filing.cover.form_type,
            filing
                .cover
                .report_code
                .as_ref()
                .map(|report_code| report_code_label(report_code.as_str()))
                .unwrap_or(""),
            filing.cover.filer_name.bold(),
            filing.cover.filer_id,
        );
        if let (Some(from), Some(through)) = (
            filing.cover.coverage_from_date,
            filing.cover.coverage_through_date,
        ) {
            println!("Covering {:?} to {:?}", from, through,);
        }
        println!();

        println!("{}", form_name(&filing.cover.form_type).dimmed());

        if let Some(ref cover) = filing.cover.cover_data {
            match cover {
                Cover::Form1(form) => {
                    if let Some(date_signed) = form.date_signed {
                        println!(
                            "Signed by {} on {}",
                            form.treasurer.to_string().bold(),
                            date_signed.to_string().bold()
                        );
                    }
                    if let Some(ref candidate) = form.candidate {
                        println!(
                            "Candidate: {} ({}) - {} {}",
                            candidate.full_name().bold(),
                            candidate.candidate_id,
                            candidate.office.as_deref().unwrap_or(""),
                            candidate.state.as_deref().unwrap_or("")
                        );
                    }
                    if let Some(ref committee_type) = form.committee_type {
                        println!("Committee Type: {}", committee_type);
                    }
                }
                Cover::Form3(form) => {
                    println!(
                        "Signed by {} on {}",
                        form.treasurer.to_string().bold(),
                        form.signed.to_string().bold()
                    );
                    print_summary_form3(&form.summary);
                }
                Cover::Form3P(form) => {
                    println!(
                        "Signed by {} on {}",
                        form.treasurer.to_string().bold(),
                        form.signed.to_string().bold()
                    );
                    print_summary(&form.summary);
                }
            }
        }

        println!(
            "{}",
            format!(
                "https://docquery.fec.gov/cgi-bin/forms/{}/{}",
                filing.cover.filer_id, filing.filing_id
            )
            .blue()
        );

        println!(
            "v{} {} filed with {} {}",
            filing.header.fec_version,
            HumanBytes(filing.source_length as u64),
            filing.header.software_name,
            filing.header.software_version
        );

        if let Some(ref report_id) = filing.header.report_id {
            println!("{}: '{}'", "Report ID".bold(), report_id);
        }
        if let Some(ref report_number) = filing.header.report_number {
            println!("Report #{}", report_number);
        }
        if let Some(ref comment) = filing.header.comment {
            println!("{}: '{}'", "Comment".bold(), comment);
        }
    }
    if !full {
        return;
    }

    if let Some(spinner) = spinner {
        spinner.set_message("Summarizing rows…");
    }

    let mut status: HashMap<String, FilingFormMetadata> = HashMap::new();
    while let Some(row) = filing.next_row() {
        let row = row.unwrap();
        if let Some(x) = status.get_mut(&row.row_type) {
            x.count += 1;
            x.bytes += row.original_size;
        } else {
            status.insert(
                row.row_type.clone(),
                FilingFormMetadata {
                    count: 1,
                    bytes: row.original_size,
                },
            );
        }
    }

    if let Some(ref spinner) = spinner {
        spinner.finish_and_clear();
    }

    let mut x: Vec<_> = status.iter().collect();
    x.sort_by(|a, b| b.1.count.cmp(&a.1.count));
    match format {
        CmdInfoFormat::Human => {
            let mut tbl = TableBuilder::new();
            tbl.push_record(["Form Type", "# Rows", "Size"]);
            for (x, y) in x {
                tbl.push_record([
                    x,
                    &indicatif::HumanCount(y.count as u64).to_string(),
                    &indicatif::HumanBytes(y.bytes as u64).to_string(),
                ]);
            }
            let tbl = tbl
                .build()
                .with(TableStyle::modern_rounded())
                .modify(TableColumns::new(1..3), TableAlignment::right())
                .to_string();

            println!("{tbl}");
        }
        CmdInfoFormat::Json => {
            let v = Value::Null;
            println!("{}", v);
        }
    }
}

pub enum InfoInput {
    Filing(String),
    Committee(String),
    Candidate(String),
}

impl InfoInput {
    pub fn from_arg(arg: &str) -> anyhow::Result<InfoInput> {
        if arg.starts_with("C") {
            Ok(InfoInput::Committee(arg.to_string()))
        } else if arg.starts_with("H") || arg.starts_with("P") || arg.starts_with("S") {
            Ok(InfoInput::Candidate(arg.to_string()))
        }
        // if input is FEC-XXXXXX or FECXXXXXXX or XXXXXX (where X is digits)
        else if let Some(stripped) = arg.strip_prefix("FEC-") {
            Ok(InfoInput::Filing(stripped.to_string()))
        } else if let Some(stripped) = arg.strip_prefix("FEC") {
            Ok(InfoInput::Filing(stripped.to_string()))
        } else {
            Err(anyhow::anyhow!(
                "Expected a filling, candidate, or committee ID, got  {}",
                arg
            ))
        }
    }
}

pub fn info(mut sourcer: FilingSourcer, args: InfoArgs) -> anyhow::Result<()> {
    let spinner = match args.format {
        CmdInfoFormat::Human => {
            let s = ProgressBar::new_spinner();
            s.enable_steady_tick(Duration::from_millis(100));
            Some(s)
        }
        _ => None,
    };
    let inputs = args
        .filings
        .iter()
        .map(|arg| InfoInput::from_arg(arg))
        .collect::<anyhow::Result<Vec<InfoInput>>>()?;
    for input in inputs {
        match input {
            InfoInput::Filing(filing_arg) => {
                let filing = sourcer.resolve_from_user_argument(&filing_arg)?;
                match args.display {
                    InfoDisplayMode::Tui => {
                        let detail = FilingDetail::from(&filing);
                        if let Some(s) = spinner.as_ref() {
                            s.finish_and_clear();
                        }
                        show_filing_detail_tui(detail, "Filing")?;
                    }
                    InfoDisplayMode::Text => {
                        let mut filing = filing;
                        process_filing(&mut filing, &args.format, &spinner, args.full);
                    }
                }
            }
            InfoInput::Committee(committee_id) => {
                // Show committee detail TUI page
                // Default to current cycle (2026) - could be made configurable
                let cycle = 2026;
                match sourcer.cache.open_bulk_data_database() {
                    Ok(mut db) => {
                        match crate::cache::bulk::committee::get_committee_detail(
                            &mut db,
                            cycle,
                            &committee_id,
                        ) {
                            Ok(Some(detail)) => {
                                if let Some(s) = spinner.as_ref() {
                                    s.finish_and_clear();
                                }
                                show_committee_detail_tui(detail, &sourcer)?;
                            }
                            Ok(None) => {
                                println!("Committee {} not found in cycle {}", committee_id, cycle);
                            }
                            Err(e) => {
                                println!("Error fetching committee details: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error opening database: {}", e);
                    }
                }
            }
            InfoInput::Candidate(candidate_id) => {
                // Show candidate detail TUI page
                // Default to current cycle (2026) - could be made configurable
                let cycle = 2026;
                match sourcer.cache.open_bulk_data_database() {
                    Ok(mut db) => {
                        match crate::cache::bulk::candidates::get_candidate_detail(
                            &mut db,
                            cycle,
                            &candidate_id,
                        ) {
                            Ok(Some(detail)) => {
                                // Load linked committees
                                let linkages = crate::cache::bulk::candidate_committee_linkage::get_candidate_committee_linkages(
                                    &mut db,
                                    cycle,
                                    &candidate_id,
                                ).unwrap_or_default();
                                if let Some(s) = spinner.as_ref() {
                                    s.finish_and_clear();
                                }
                                show_candidate_detail_tui(detail, linkages, &sourcer)?;
                            }
                            Ok(None) => {
                                println!("Candidate {} not found in cycle {}", candidate_id, cycle);
                            }
                            Err(e) => {
                                println!("Error fetching candidate details: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error opening database: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

fn render_committee_detail_with_breadcrumb(
    f: &mut Frame,
    detail: &crate::cache::bulk::committee::CommitteeDetail,
    state: &mut CommitteeDetailState,
) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::style::{Color, Style};
    use ratatui::widgets::Paragraph;

    // Layout with breadcrumb at the top
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Breadcrumb
            Constraint::Min(1),    // Main content
        ]);

    let [breadcrumb_area, content_area] = f.area().layout(&layout);

    // Build breadcrumb text
    let name = truncate_string(&detail.name, 50);
    let breadcrumb_text = format!("Info / {}", name);

    let breadcrumb = Paragraph::new(breadcrumb_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(breadcrumb, breadcrumb_area);

    // Render committee detail in the content area
    render_committee_detail(f, content_area, detail, state);
}

fn render_filing_detail_with_breadcrumb(
    f: &mut Frame,
    detail: &FilingDetail,
    state: &FilingDetailState,
    committee_name: &str,
) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::style::{Color, Style};
    use ratatui::widgets::Paragraph;

    // Layout with breadcrumb at the top
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Breadcrumb
            Constraint::Min(1),    // Main content
        ]);

    let [breadcrumb_area, content_area] = f.area().layout(&layout);

    // Build breadcrumb text
    let breadcrumb_text = if committee_name == "Filing" {
        format!("Info / {}", detail.filing_id)
    } else {
        let name = truncate_string(committee_name, 40);
        format!("Info / {} / {}", name, detail.filing_id)
    };

    let breadcrumb = Paragraph::new(breadcrumb_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(breadcrumb, breadcrumb_area);

    // Render filing detail in the content area
    render_filing_detail(f, content_area, detail, state);
}

fn show_committee_detail_tui(
    detail: crate::cache::bulk::committee::CommitteeDetail,
    sourcer: &FilingSourcer,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = CommitteeDetailState::new();

    loop {
        terminal.draw(|f| {
            render_committee_detail_with_breadcrumb(f, &detail, &mut state);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Ctrl+C exits immediately
            if key.code == crossterm::event::KeyCode::Char('c')
                && key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                break;
            }

            match state.handle_key_event(key, &detail) {
                CommitteeDetailAction::Exit => break,
                CommitteeDetailAction::OpenBrowser => {
                    let _ = detail.open_in_browser();
                }
                CommitteeDetailAction::ShowFilingDetail { filing_id } => {
                    // Render the loading state before blocking API call
                    terminal
                        .draw(|f| {
                            render_committee_detail_with_breadcrumb(f, &detail, &mut state);
                        })
                        .unwrap();

                    // Try to resolve and show filing detail
                    match sourcer.resolve_from_user_argument(&filing_id) {
                        Ok(filing) => {
                            let filing_detail = FilingDetail::from(&filing);
                            state.filing_detail_loading = false;

                            // Show filing detail in nested view
                            disable_raw_mode()?;
                            execute!(
                                terminal.backend_mut(),
                                LeaveAlternateScreen
                            )?;

                            // Show filing detail
                            show_filing_detail_tui(filing_detail, &detail.name)?;

                            // Restore committee detail view
                            enable_raw_mode()?;
                            execute!(
                                terminal.backend_mut(),
                                EnterAlternateScreen
                            )?;
                        }
                        Err(e) => {
                            state.filing_detail_loading = false;
                            state.set_filings_error(format!(
                                "Error loading filing {}: {}",
                                filing_id, e
                            ));
                        }
                    }
                }
                CommitteeDetailAction::FetchFilings => {
                    // Render the loading state before blocking API call
                    terminal
                        .draw(|f| {
                            render_committee_detail_with_breadcrumb(f, &detail, &mut state);
                        })
                        .unwrap();
                    state.fetch_filings_for_committee(&detail.committee_id);
                }
                CommitteeDetailAction::None => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn show_candidate_detail_tui(
    detail: crate::cache::bulk::candidates::CandidateDetail,
    linkages: Vec<crate::cache::bulk::candidate_committee_linkage::CommitteeLinkage>,
    sourcer: &crate::sourcer::FilingSourcer,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = CandidateDetailState::new();
    state.set_linked_committees(linkages);

    loop {
        terminal.draw(|f| {
            render_candidate_detail(f, f.area(), &detail, &mut state);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Ctrl+C exits immediately
            if key.code == crossterm::event::KeyCode::Char('c')
                && key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                break;
            }

            match state.handle_key_event(key, &detail) {
                CandidateDetailAction::Exit => break,
                CandidateDetailAction::ShowCommitteeDetail { committee_id: _ } => {
                    // TODO: Navigate to committee detail view
                    // For now, just ignore - would need to refactor to support nested views
                }
                CandidateDetailAction::FetchFilings => {
                    // Render the loading state before blocking API call
                    terminal
                        .draw(|f| {
                            render_candidate_detail(f, f.area(), &detail, &mut state);
                        })
                        .unwrap();
                    state.fetch_filings_for_candidate(&detail.candidate_id);
                }
                CandidateDetailAction::FetchF1Affiliations { committee_id } => {
                    // Render the loading state before blocking API call
                    terminal
                        .draw(|f| {
                            render_candidate_detail(f, f.area(), &detail, &mut state);
                        })
                        .unwrap();
                    state.fetch_f1_affiliations(&committee_id, sourcer);
                }
                CandidateDetailAction::ShowFilingDetail { filing_id: _ } => {
                    // TODO: Navigate to filing detail view
                    // For now, just ignore - would need to refactor to support nested views
                }
                CandidateDetailAction::None => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn show_filing_detail_tui(detail: FilingDetail, committee_name: &str) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = FilingDetailState::new();
    let committee_name = committee_name.to_string();

    loop {
        terminal.draw(|f| {
            render_filing_detail_with_breadcrumb(f, &detail, &state, &committee_name);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Ctrl+C exits immediately
            if key.code == crossterm::event::KeyCode::Char('c')
                && key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                break;
            }

            match state.handle_key_event(key, &detail) {
                FilingDetailAction::Exit => break,
                FilingDetailAction::OpenBrowser => {
                    let _ = detail.open_in_browser();
                }
                FilingDetailAction::ShowFiler { .. } => {
                    // In info context, go back to committee detail (if coming from there)
                    break;
                }
                FilingDetailAction::OpenWebsite { url } => {
                    let _ = open::that(&url);
                }
                FilingDetailAction::None => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    Ok(())
}
