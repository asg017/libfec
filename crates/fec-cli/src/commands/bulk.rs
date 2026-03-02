use crate::{
    cache::bulk::{
        candidate_committee_linkage, candidate_summary, candidate_summary_csv, candidates,
        committee, committee_summary_csv, form1_filers, form2_filers, independent_expenditures,
        opexp, pac_summary, pas2,
    },
    cli::{BulkArgs, BulkSource, CycleArg},
    sourcer::FilingSourcer,
};
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};

pub fn bulk(_sourcer: FilingSourcer, args: &BulkArgs) -> anyhow::Result<()> {
    println!("Running bulk command with args: {:?}", args);
    let mut db = rusqlite::Connection::open(&args.output)?;

    // Convert cycle argument to a vector of even years
    let cycles: Vec<u16> = match args.cycle {
        CycleArg::Single(year) => vec![year],
        CycleArg::Range(start, end) => (start..=end).filter(|y| y % 2 == 0).collect(),
    };

    let mut tx = db.transaction()?;

    // Calculate total steps for progress bar
    let mut total_steps = cycles.len() * args.source.len();
    if args.source.contains(&BulkSource::Candidates)
        && args.source.contains(&BulkSource::Committees)
    {
        total_steps += cycles.len(); // Add linkage steps
    }

    let pb = ProgressBar::new(total_steps as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .expect("Progress bar template is valid")
            .progress_chars("#>-"),
    );

    let on_progress = |bytes: u64, total: Option<u64>| {
        let msg = match total {
            Some(t) => format!("downloading {} / {}", HumanBytes(bytes), HumanBytes(t)),
            None => format!("downloading {}", HumanBytes(bytes)),
        };
        pb.set_message(msg);
    };

    for year in cycles {
        for source in &args.source {
            pb.set_message(format!("{} - {:?}", year, source));
            let result = match source {
                BulkSource::Opex => opexp::export(&mut tx, year, Some(&on_progress)),
                BulkSource::Committees => committee::export(&mut tx, year, Some(&on_progress)),
                BulkSource::Candidates => candidates::export(&mut tx, year, Some(&on_progress)),
                BulkSource::ContributionsToCandidates => {
                    pas2::export(&mut tx, year, Some(&on_progress))
                }
                BulkSource::PacSummary => pac_summary::export(&mut tx, year, Some(&on_progress)),
                BulkSource::CandidateSummary => {
                    candidate_summary::export(&mut tx, year, Some(&on_progress))
                }
                BulkSource::IndependentExpenditures => {
                    independent_expenditures::export(&mut tx, year, Some(&on_progress))
                }
                BulkSource::Form2Filers => form2_filers::export(&mut tx, year, Some(&on_progress)),
                BulkSource::CandidateSummaryCsv => {
                    candidate_summary_csv::export(&mut tx, year, Some(&on_progress))
                }
                BulkSource::CommitteeSummaryCsv => {
                    committee_summary_csv::export(&mut tx, year, Some(&on_progress))
                }
                BulkSource::Form1Filers => form1_filers::export(&mut tx, year, Some(&on_progress)),
            };
            result.unwrap();
            pb.inc(1);
        }
        if args.source.contains(&BulkSource::Candidates)
            && args.source.contains(&BulkSource::Committees)
        {
            pb.set_message(format!("{} - Candidate-Committee Linkage", year));
            candidate_committee_linkage::export(&mut tx, year, Some(&on_progress))?;
            pb.inc(1);
        }
    }

    pb.finish_with_message("Complete!");
    tx.commit()?;
    Ok(())
}
