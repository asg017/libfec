//! Integration tests for the search TUI
//!
//! This module contains snapshot tests for the search interface.

use super::app::ResultsTab;
use super::{app::App, ui::render_search_view};
use crate::cache::bulk::{
    candidates::CandidateSearchResult, committee::CommitteeSearchResult, opexp::OpExpSearchResult,
};
use insta::assert_snapshot;
use ratatui::{backend::TestBackend, Terminal};

fn create_test_app() -> App {
    let mut app = App::new(2024, "biden".to_string());
    // Add some sample results
    app.candidate_results = vec![CandidateSearchResult {
        candidate_id: "P00003392".to_string(),
        name: "BIDEN, JOSEPH R JR".to_string(),
        election_year: 2020,
        office: "P".to_string(),
        state: "".to_string(),
        district: "".to_string(),
        principal_campaign_committee: Some("C00703975".to_string()),
    }];
    app.committee_results = vec![CommitteeSearchResult {
        committee_id: "C00703975".to_string(),
        name: "BIDEN FOR PRESIDENT".to_string(),
        committee_type: "P".to_string(),
        designation: "P".to_string(),
        party_affiliation: "DEM".to_string(),
        connected_org_name: "".to_string(),
        candidate_id: Some("P00003392".to_string()),
    }];
    app.candidate_table_state.select(Some(0));
    app
}

#[test]
fn test_render_search_view() {
    let mut app = create_test_app();
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|f| render_search_view(f, &mut app, f.area()))
        .unwrap();
    assert_snapshot!(terminal.backend());
}

#[test]
fn test_render_search_view_empty() {
    let mut app = App::new(2024, "".to_string());
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|f| render_search_view(f, &mut app, f.area()))
        .unwrap();
    assert_snapshot!(terminal.backend());
}

#[test]
fn test_render_help_text() {
    use super::ui::render_help_text;
    let mut app = App::new(2024, "".to_string());
    let mut terminal = Terminal::new(TestBackend::new(100, 2)).unwrap();
    terminal
        .draw(|f| render_help_text(f, &mut app, f.area()))
        .unwrap();
    assert_snapshot!(terminal.backend());
}

#[test]
fn test_render_search_view_opexp() {
    let mut app = App::new(2024, "consulting".to_string());
    // Set active tab to OpExp
    app.active_tab = ResultsTab::OpExp;
    // Add some sample operating expense results
    app.opexp_results = vec![
        OpExpSearchResult {
            committee_id: "C00703975".to_string(),
            name: "ACME CONSULTING LLC".to_string(),
            city: "WASHINGTON".to_string(),
            state: "DC".to_string(),
            transaction_date: "2024-03-15".to_string(),
            transaction_amount: 5000.00,
            purpose: "STRATEGY CONSULTING".to_string(),
            filing_id: 123456,
        },
        OpExpSearchResult {
            committee_id: "C00703975".to_string(),
            name: "SMITH CONSULTING GROUP".to_string(),
            city: "NEW YORK".to_string(),
            state: "NY".to_string(),
            transaction_date: "2024-02-28".to_string(),
            transaction_amount: 3500.50,
            purpose: "MEDIA CONSULTING".to_string(),
            filing_id: 123457,
        },
    ];
    app.opexp_table_state.select(Some(0));
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal
        .draw(|f| render_search_view(f, &mut app, f.area()))
        .unwrap();
    assert_snapshot!(terminal.backend());
}

#[test]
fn test_render_search_bar_with_spinner() {
    use super::ui::render_search_bar;

    let mut app = App::new(2024, "biden".to_string());
    app.searching = true;
    app.spinner_frame = 2; // Use a specific frame for consistent testing
    let mut terminal = Terminal::new(TestBackend::new(100, 3)).unwrap();
    terminal
        .draw(|f| {
            let area = f.area();
            render_search_bar(f, &mut app, area);
        })
        .unwrap();
    assert_snapshot!(terminal.backend());
}
