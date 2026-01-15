use crate::{
    cache::{bulk_candidate_committee_linkage, bulk_candidates, bulk_committee, bulk_opexp},
    cli::{BulkArgs, BulkSource, CycleArg},
    sourcer::FilingSourcer,
};
use indicatif::{ProgressBar, ProgressStyle};

pub fn bulk(sourcer: FilingSourcer, args: &BulkArgs) -> anyhow::Result<()> {
    println!("Running bulk command with args: {:?}", args);
    let mut db = rusqlite::Connection::open(&args.output)?;
    
    // Convert cycle argument to a vector of even years
    let cycles: Vec<u16> = match args.cycle {
        CycleArg::Single(year) => vec![year],
        CycleArg::Range(start, end) => {
            (start..=end)
                .filter(|y| y % 2 == 0)
                .collect()
        }
    };
    
    let mut tx = db.transaction()?;
    
    // Calculate total steps for progress bar
    let mut total_steps = cycles.len() * args.source.len();
    if args.source.contains(&BulkSource::Candidates) && args.source.contains(&BulkSource::Committees) {
        total_steps += cycles.len(); // Add linkage steps
    }
    
    let pb = ProgressBar::new(total_steps as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .expect("Progress bar template is valid")
            .progress_chars("#>-")
    );
    
    for year in cycles {
        for source in &args.source {
            pb.set_message(format!("{} - {:?}", year, source));
            let result = match source {
                BulkSource::Opex => bulk_opexp::export(&mut tx, year),
                BulkSource::Committees => bulk_committee::export(&mut tx, year),
                BulkSource::Candidates => bulk_candidates::export(&mut tx, year),
            };
            result.unwrap();
            pb.inc(1);
        }
        if args.source.contains(&BulkSource::Candidates) && args.source.contains(&BulkSource::Committees) {
            pb.set_message(format!("{} - Candidate-Committee Linkage", year));
            bulk_candidate_committee_linkage::export(&mut tx, year)?;
            pb.inc(1);
        }
    }
    
    pb.finish_with_message("Complete!");
    tx.commit()?;
    Ok(())
}
