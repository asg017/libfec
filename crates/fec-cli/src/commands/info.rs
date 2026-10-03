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
use ratatui::{backend::CrosstermBackend, Terminal};
use serde_json::Value;
use std::{
    collections::HashMap,
    io::{self, IsTerminal, Read},
    time::Duration,
};

use tabled::{
    builder::Builder as TableBuilder,
    settings::{object::Columns as TableColumns, Alignment as TableAlignment, Style as TableStyle},
};

use crate::{
    cli::{CmdInfoFormat, InfoArgs},
    sourcer::{FecFilingId, FilingSourcer},
};
use fec_api::{CandidateId, CommitteeId};
struct FilingFormMetadata {
    count: usize,
    bytes: usize,
}

fn form_name(form_type: &str) -> &'static str {
    match fec_parser::covers::base_form_type(form_type).as_str() {
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
use crate::tui::filing_detail::{f1::affiliated_is_placeholder, format_usd};
use tabled::{builder::Builder, settings::Style};

/// `"Label (CODE)"`, or just the code when there is no label; the same
/// format as the filing detail TUI.
fn code_with_label(code: &str, label: Option<&str>) -> String {
    match label {
        Some(label) => format!("{label} ({code})"),
        None => code.to_string(),
    }
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
    b.push_record(["", "This Period", "Election Cycle-to-Date"]);

    let two_column = vec![
        (
            "6(a). Total Contributions (Other Than Loans)",
            summary.line6a_total_contributions,
        ),
        (
            "6(b). Total Contribution Refunds",
            summary.line6b_total_contribution_refunds,
        ),
        (
            "6(c). Net Contributions (Other Than Loans)",
            summary.line6c_net_contributions,
        ),
        (
            "7(a). Total Operating Expenditures",
            summary.line7a_total_operating_expenditures,
        ),
        (
            "7(b). Total Offsets to Operating Expenditures",
            summary.line7b_total_offsets_to_operating_expenditures,
        ),
        (
            "7(c). Net Operating Expenditures",
            summary.line7c_net_operating_expenditures,
        ),
    ];
    for (label, row) in two_column {
        b.push_record([
            label.to_string(),
            format_usd(row.column_a),
            format_usd(row.column_b),
        ]);
    }
    let one_column = vec![
        (
            "8. Cash on Hand at Close of Reporting Period",
            summary.line8_cash_on_hand_close_of_period,
        ),
        (
            "9. Debts and Obligations Owed TO the Committee",
            summary.line9_debts_owed_to_committee,
        ),
        (
            "10. Debts and Obligations Owed BY the Committee",
            summary.line10_debts_owed_by_committee,
        ),
    ];
    for (label, value) in one_column {
        b.push_record([label.to_string(), format_usd(value), String::new()]);
    }

    let mut table = b.build();
    table.with(Style::modern());
    table.modify(
        tabled::settings::object::Columns::new(1..3),
        tabled::settings::Alignment::right(),
    );

    // make 1st row (title) span entire width
    table
        .modify((0, 0), tabled::settings::Span::column(3))
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
    if matches!(format, CmdInfoFormat::Json) {
        print_filing_json(filing, full);
        return;
    }
    if matches!(format, CmdInfoFormat::Human) {
        if !full {
            if let Some(ref spinner) = spinner {
                spinner.finish_and_clear();
            }
        }

        let report_label = filing
            .cover
            .report_code
            .as_deref()
            .map(report_code_label)
            .filter(|label| *label != "[Unknown report code]")
            .map(|label| format!(" {label}"))
            .unwrap_or_default();
        println!(
            "{} {}{} by {} ({})",
            format!("FEC-{}", filing.filing_id).bold(),
            filing.cover.form_type,
            report_label,
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
                            cover
                                .signer()
                                .map(|s| s.to_string())
                                .unwrap_or_default()
                                .bold(),
                            date_signed.to_string().bold()
                        );
                    }
                    if let Some(ref committee_type) = form.committee_type {
                        println!(
                            "Committee Type: {}",
                            code_with_label(committee_type, form.committee_type_label())
                        );
                    }
                    if let Some(ref candidate) = form.candidate {
                        println!(
                            "Candidate: {}{} - {} {}",
                            candidate.full_name().bold(),
                            candidate
                                .candidate_id
                                .as_deref()
                                .map(|id| format!(" ({id})"))
                                .unwrap_or_default(),
                            candidate
                                .office_label()
                                .or(candidate.office.as_deref())
                                .unwrap_or(""),
                            candidate.state.as_deref().unwrap_or("")
                        );
                    }
                    if let Some(affiliated) = form
                        .affiliated
                        .as_ref()
                        .filter(|a| !affiliated_is_placeholder(a))
                    {
                        println!(
                            "Affiliated: {}{}",
                            affiliated.display_name(),
                            affiliated
                                .relationship_label()
                                .map(|l| format!(" ({l})"))
                                .unwrap_or_default()
                        );
                    }
                    println!("Treasurer: {}", form.treasurer.name);
                }
                Cover::Form3(form) => {
                    if let Some(date_signed) = form.date_signed {
                        println!(
                            "Signed by {} on {}",
                            form.treasurer.to_string().bold(),
                            date_signed.to_string().bold()
                        );
                    }
                    print_summary_form3(&form.summary);
                }
                Cover::Form3P(form) => {
                    if let Some(date_signed) = form.date_signed {
                        println!(
                            "Signed by {} on {}",
                            form.treasurer.to_string().bold(),
                            date_signed.to_string().bold()
                        );
                    }
                    print_summary(&form.summary);
                }
                Cover::Form1M(form) => {
                    if let Some(date_signed) = form.date_signed {
                        println!(
                            "Signed by {} on {}",
                            form.treasurer.to_string().bold(),
                            date_signed.to_string().bold()
                        );
                    }
                    if let Some(ref a) = form.affiliation {
                        println!(
                            "Multicandidate status by affiliation with {} ({}), Form 1 filed {}",
                            a.committee_name.as_deref().unwrap_or("?").bold(),
                            a.committee_id.as_deref().unwrap_or("?"),
                            a.date_form1_filed
                                .map(|d| d.to_string())
                                .unwrap_or_else(|| "?".into())
                        );
                    }
                    if let Some(ref q) = form.qualification {
                        println!(
                            "Multicandidate status by qualification: met requirements on {} ({} candidates, 51st contributor {}, registered {})",
                            q.requirements_met_date
                                .map(|d| d.to_string())
                                .unwrap_or_else(|| "?".into())
                                .bold(),
                            q.candidates.len(),
                            q.fifty_first_contributor_date
                                .map(|d| d.to_string())
                                .unwrap_or_else(|| "?".into()),
                            q.original_registration_date
                                .map(|d| d.to_string())
                                .unwrap_or_else(|| "?".into()),
                        );
                    }
                }
                Cover::Form3X(form) => {
                    if let Some(date_signed) = form.date_signed {
                        println!(
                            "Signed by {} on {}",
                            form.treasurer.to_string().bold(),
                            date_signed.to_string().bold()
                        );
                    }
                    print_summary_form3x(form);
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
        let row = match row {
            Ok(row) => row,
            Err(e) => {
                eprintln!(
                    "warning: FEC-{}: skipping unreadable row: {e}",
                    filing.filing_id
                );
                continue;
            }
        };
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
    x.sort_by_key(|b| std::cmp::Reverse(b.1.count));
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
        // JSON is handled by `print_filing_json` before this point.
        CmdInfoFormat::Json => {}
    }
}

/// `libfec info --format json`: one JSON object per filing with the header,
/// and the generic cover fields, plus per-row-type counts with `--full`.
fn print_filing_json<R: Read>(filing: &mut Filing<R>, full: bool) {
    let header = &filing.header;
    let cover = &filing.cover;
    let mut v = serde_json::json!({
        "filing_id": filing.filing_id,
        "fec_version": header.fec_version,
        "software_name": header.software_name,
        "software_version": header.software_version,
        "report_id": header.report_id,
        "report_number": header.report_number,
        "comment": header.comment,
        "source_length": filing.source_length,
        "form_type": cover.form_type,
        "filer_id": cover.filer_id,
        "filer_name": cover.filer_name,
        "report_code": cover.report_code,
        "coverage_from_date": cover.coverage_from_date.map(|d| d.to_string()),
        "coverage_through_date": cover.coverage_through_date.map(|d| d.to_string()),
    });
    if full {
        let mut rows: std::collections::BTreeMap<String, Value> = Default::default();
        while let Some(row) = filing.next_row() {
            let row = match row {
                Ok(row) => row,
                Err(e) => {
                    eprintln!(
                        "warning: FEC-{}: skipping unreadable row: {e}",
                        filing.filing_id
                    );
                    continue;
                }
            };
            let entry = rows
                .entry(row.row_type.clone())
                .or_insert_with(|| serde_json::json!({"count": 0, "bytes": 0}));
            entry["count"] = (entry["count"].as_u64().unwrap_or(0) + 1).into();
            entry["bytes"] =
                (entry["bytes"].as_u64().unwrap_or(0) + row.original_size as u64).into();
        }
        v["rows"] = serde_json::to_value(rows).unwrap_or(Value::Null);
    }
    println!("{v}");
}

pub enum InfoInput {
    Filing(FecFilingId),
    Committee(CommitteeId),
    Candidate(CandidateId),
}

impl InfoInput {
    pub fn from_arg(arg: &str) -> anyhow::Result<InfoInput> {
        if let Ok(id) = arg.parse::<CommitteeId>() {
            Ok(InfoInput::Committee(id))
        } else if let Ok(id) = arg.parse::<CandidateId>() {
            Ok(InfoInput::Candidate(id))
        } else if let Ok(id) = FecFilingId::from_str(arg) {
            Ok(InfoInput::Filing(id))
        } else {
            Err(anyhow::anyhow!(
                "Expected a filing, candidate, or committee ID, got  {}",
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
                let filing = sourcer.resolve_from_user_argument(&filing_arg.to_bare())?;
                if matches!(args.format, CmdInfoFormat::Human) && io::stdout().is_terminal() {
                    let detail = FilingDetail::from(&filing);
                    if let Some(s) = spinner.as_ref() {
                        s.finish_and_clear();
                    }
                    show_filing_detail_tui(detail)?;
                } else {
                    let mut filing = filing;
                    process_filing(&mut filing, &args.format, &spinner, args.full);
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
                            committee_id.as_str(),
                            None,
                        ) {
                            Ok(Some(detail)) => {
                                let financial_summary =
                                    crate::cache::bulk::pac_summary::get_pac_summary(
                                        &mut db,
                                        cycle,
                                        committee_id.as_str(),
                                        None,
                                    )
                                    .unwrap_or(None);
                                if let Some(s) = spinner.as_ref() {
                                    s.finish_and_clear();
                                }
                                show_committee_detail_tui(detail, financial_summary, &sourcer)?;
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
                            candidate_id.as_str(),
                            None,
                        ) {
                            Ok(Some(detail)) => {
                                // Load linked committees
                                let linkages = crate::cache::bulk::candidate_committee_linkage::get_candidate_committee_linkages(
                                    &mut db,
                                    cycle,
                                    candidate_id.as_str(),
                                    None,
                                ).unwrap_or_default();
                                // Load financial summary
                                let financial_summary =
                                    crate::cache::bulk::candidate_summary::get_candidate_summary(
                                        &mut db,
                                        cycle,
                                        candidate_id.as_str(),
                                        None,
                                    )
                                    .unwrap_or(None);
                                // Load PCC name
                                let pcc_name = detail
                                    .principal_campaign_committee
                                    .as_ref()
                                    .and_then(|pcc_id| {
                                        crate::cache::bulk::committee::get_committee_detail(
                                            &mut db, cycle, pcc_id, None,
                                        )
                                        .ok()
                                        .flatten()
                                        .map(|c| c.name)
                                    });
                                if let Some(s) = spinner.as_ref() {
                                    s.finish_and_clear();
                                }
                                show_candidate_detail_tui(
                                    detail,
                                    linkages,
                                    financial_summary,
                                    pcc_name,
                                    &mut sourcer,
                                )?;
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

fn show_committee_detail_tui(
    detail: crate::cache::bulk::committee::CommitteeDetail,
    financial_summary: Option<crate::cache::bulk::pac_summary::CommitteeFinancialSummary>,
    sourcer: &FilingSourcer,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let _ = run_committee_detail_tui(&mut terminal, &detail, financial_summary, sourcer)?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Returns `Ok(true)` if the user pressed Ctrl+C (force quit).
fn run_committee_detail_tui<B: ratatui::backend::Backend<Error: Send + Sync + 'static>>(
    terminal: &mut Terminal<B>,
    detail: &crate::cache::bulk::committee::CommitteeDetail,
    financial_summary: Option<crate::cache::bulk::pac_summary::CommitteeFinancialSummary>,
    sourcer: &FilingSourcer,
) -> anyhow::Result<bool> {
    let mut state = CommitteeDetailState::new();
    state.set_financial_summary(financial_summary);
    let mut force_quit = false;

    loop {
        terminal.draw(|f| {
            render_committee_detail(f, f.area(), detail, &mut state);
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
                force_quit = true;
                break;
            }

            match state.handle_key_event(key, detail) {
                CommitteeDetailAction::Exit => break,
                CommitteeDetailAction::OpenBrowser => {
                    let _ = detail.open_in_browser();
                }
                CommitteeDetailAction::ShowFilingDetail { filing_id } => {
                    // Render the loading state before blocking API call
                    terminal
                        .draw(|f| {
                            render_committee_detail(f, f.area(), detail, &mut state);
                        })
                        .unwrap();

                    match sourcer.resolve_from_user_argument(&filing_id) {
                        Ok(filing) => {
                            let filing_detail = FilingDetail::from(&filing);
                            state.filing_detail_loading = false;
                            if run_filing_detail_tui(terminal, &filing_detail)? {
                                force_quit = true;
                                break;
                            }
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
                            render_committee_detail(f, f.area(), detail, &mut state);
                        })
                        .unwrap();
                    state.fetch_filings_for_committee(&detail.committee_id);
                }
                CommitteeDetailAction::None => {}
            }
        }
    }

    Ok(force_quit)
}

fn show_candidate_detail_tui(
    detail: crate::cache::bulk::candidates::CandidateDetail,
    linkages: Vec<crate::cache::bulk::candidate_committee_linkage::CommitteeLinkage>,
    financial_summary: Option<crate::cache::bulk::candidate_summary::CandidateFinancialSummary>,
    pcc_name: Option<String>,
    sourcer: &mut crate::sourcer::FilingSourcer,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = CandidateDetailState::new();
    state.set_linked_committees(linkages);
    state.set_financial_summary(financial_summary);
    state.pcc_name = pcc_name;

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
                CandidateDetailAction::ShowCommitteeDetail { committee_id } => {
                    let cycle = 2026;
                    if let Ok(mut db) = sourcer.cache.open_bulk_data_database() {
                        if let Ok(Some(committee)) =
                            crate::cache::bulk::committee::get_committee_detail(
                                &mut db,
                                cycle,
                                &committee_id,
                                None,
                            )
                        {
                            let fin_summary = crate::cache::bulk::pac_summary::get_pac_summary(
                                &mut db,
                                cycle,
                                &committee_id,
                                None,
                            )
                            .unwrap_or(None);
                            if run_committee_detail_tui(
                                &mut terminal,
                                &committee,
                                fin_summary,
                                sourcer,
                            )? {
                                break;
                            }
                        }
                    }
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
                CandidateDetailAction::ShowFilingDetail { filing_id } => {
                    // Render the loading state before blocking API call
                    terminal
                        .draw(|f| {
                            render_candidate_detail(f, f.area(), &detail, &mut state);
                        })
                        .unwrap();
                    match sourcer.resolve_from_user_argument(&filing_id) {
                        Ok(filing) => {
                            let filing_detail = FilingDetail::from(&filing);
                            state.filing_detail_loading = false;
                            if run_filing_detail_tui(&mut terminal, &filing_detail)? {
                                break;
                            }
                        }
                        Err(e) => {
                            state.filing_detail_loading = false;
                            state.filings_error =
                                Some(format!("Error loading filing {}: {}", filing_id, e));
                        }
                    }
                }
                CandidateDetailAction::OpenFecPage => {
                    let url = format!(
                        "https://www.fec.gov/data/candidate/{}/",
                        detail.candidate_id
                    );
                    let _ = open::that(&url);
                }
                CandidateDetailAction::ShowContest {
                    office,
                    state,
                    district,
                } => {
                    use crate::sourcer::Contest;
                    let contest = match office.as_str() {
                        "P" => Some(Contest::President),
                        "S" => Some(Contest::Senate { state }),
                        "H" => Some(Contest::House { state, district }),
                        _ => None,
                    };
                    if let Some(contest) = contest {
                        let cycle = 2026;
                        if crate::commands::contest::run_contest_tui(
                            &mut terminal,
                            sourcer,
                            &contest,
                            cycle,
                        )? {
                            break;
                        }
                    }
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

fn show_filing_detail_tui(detail: FilingDetail) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let _ = run_filing_detail_tui(&mut terminal, &detail)?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Returns `Ok(true)` if the user pressed Ctrl+C (force quit).
fn run_filing_detail_tui<B: ratatui::backend::Backend<Error: Send + Sync + 'static>>(
    terminal: &mut Terminal<B>,
    detail: &FilingDetail,
) -> anyhow::Result<bool> {
    let mut state = FilingDetailState::new();
    let mut force_quit = false;

    loop {
        terminal.draw(|f| {
            render_filing_detail(f, f.area(), detail, &state);
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
                force_quit = true;
                break;
            }

            match state.handle_key_event(key, detail) {
                FilingDetailAction::Exit => break,
                FilingDetailAction::OpenBrowser => {
                    let _ = detail.open_in_browser();
                }
                FilingDetailAction::ShowFiler { .. } => {
                    break;
                }
                FilingDetailAction::OpenWebsite { url } => {
                    let _ = open::that(&url);
                }
                FilingDetailAction::None => {}
            }
        }
    }

    Ok(force_quit)
}

/// Form 3X Summary Page (Lines 6-10), Column A "This Period" and Column B
/// "Calendar Year-to-Date".
fn print_summary_form3x(form: &fec_parser::covers::Form3X) {
    let s = &form.summary;
    let mut b = Builder::with_capacity(3, 0);
    b.push_record(["Summary", "This Period", "Year-to-Date"]);
    let jan_1 = match s.line6a_year {
        Some(year) => format!("6(a) Cash on Hand January 1, {year}"),
        None => "6(a) Cash on Hand January 1".to_string(),
    };
    let rows: Vec<(String, Option<f64>, Option<f64>)> = vec![
        (jan_1, None, Some(s.line6a_cash_on_hand_jan_1)),
        (
            "6(b) Cash on Hand at Beginning of Reporting Period".into(),
            Some(s.line6b_cash_on_hand_beginning_period),
            None,
        ),
        (
            "6(c) Total Receipts".into(),
            Some(s.line6c_total_receipts.column_a),
            Some(s.line6c_total_receipts.column_b),
        ),
        (
            "6(d) Subtotal".into(),
            Some(s.line6d_subtotal.column_a),
            Some(s.line6d_subtotal.column_b),
        ),
        (
            "7. Total Disbursements".into(),
            Some(s.line7_total_disbursements.column_a),
            Some(s.line7_total_disbursements.column_b),
        ),
        (
            "8. Cash on Hand at Close of Reporting Period".into(),
            Some(s.line8_cash_on_hand_close_of_period.column_a),
            Some(s.line8_cash_on_hand_close_of_period.column_b),
        ),
        (
            "9. Debts and Obligations Owed TO the Committee".into(),
            Some(s.line9_debts_owed_to_committee),
            None,
        ),
        (
            "10. Debts and Obligations Owed BY the Committee".into(),
            Some(s.line10_debts_owed_by_committee),
            None,
        ),
    ];
    for (label, a, b_val) in rows {
        b.push_record([
            label,
            a.map(format_usd).unwrap_or_default(),
            b_val.map(format_usd).unwrap_or_default(),
        ]);
    }
    let mut table = b.build();
    table.with(Style::modern());
    table.modify(
        tabled::settings::object::Columns::new(1..),
        tabled::settings::Alignment::right(),
    );
    println!("{}", table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_name_strips_suffix() {
        let f3x = "Report of Receipts and Disbursements for other than an Authorized Committee";
        assert_eq!(form_name("F3XN"), f3x);
        assert_eq!(form_name("F3XA"), f3x);
        assert_eq!(form_name("F3XT"), f3x);
        assert_eq!(form_name("F3T"), form_name("F3N"));
        assert_eq!(form_name("F1MN"), "Notification of Multicandidate Status");
        assert_eq!(form_name("F99"), "Miscellaneous Text");
    }
}
