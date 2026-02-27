use crate::{cli::SearchArgs, sourcer::FilingSourcer};
use anyhow::Result;

use super::app;

pub fn print_table(sourcer: &mut FilingSourcer, args: &SearchArgs) -> Result<()> {
    let mut app = app::App::new(args.cycle, args.query.clone());
    app.search(sourcer)?;

    if !app.candidate_results.is_empty() {
        println!("Candidates ({}):", app.candidate_results.len());
        println!(
            "{:<13} {:<40} {:>4} {:>6} {:>10} {:<11}",
            "ID", "Name", "Year", "Office", "State/Dist", "Committee"
        );
        println!("{}", "-".repeat(90));
        for c in &app.candidate_results {
            let state_dist = if c.office == "H" && !c.state.is_empty() && !c.district.is_empty() {
                format!("{}-{:02}", c.state, c.district.parse::<u8>().unwrap_or(0))
            } else if !c.state.is_empty() {
                c.state.clone()
            } else {
                String::new()
            };
            println!(
                "{:<13} {:<40} {:>4} {:>6} {:>10} {:<11}",
                c.candidate_id,
                truncate(&c.name, 40),
                c.election_year,
                c.office,
                state_dist,
                c.principal_campaign_committee.as_deref().unwrap_or(""),
            );
        }
    }

    if !app.committee_results.is_empty() {
        if !app.candidate_results.is_empty() {
            println!();
        }
        println!("Committees ({}):", app.committee_results.len());
        println!(
            "{:<11} {:<45} {:>4} {:>5} {:>5} {:<11}",
            "ID", "Name", "Type", "Desig", "Party", "Candidate"
        );
        println!("{}", "-".repeat(85));
        for c in &app.committee_results {
            println!(
                "{:<11} {:<45} {:>4} {:>5} {:>5} {:<11}",
                c.committee_id,
                truncate(&c.name, 45),
                c.committee_type,
                c.designation,
                c.party_affiliation,
                c.candidate_id.as_deref().unwrap_or(""),
            );
        }
    }

    if app.candidate_results.is_empty() && app.committee_results.is_empty() {
        println!("No results for \"{}\" (cycle {})", args.query, args.cycle);
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
