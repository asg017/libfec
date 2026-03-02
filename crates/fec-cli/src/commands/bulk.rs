use crate::{
    cache::bulk::{
        candidate_committee_linkage, candidate_summary, candidate_summary_csv, candidates,
        committee, committee_summary_csv, form1_filers, form2_filers, independent_expenditures,
        individual_contributions, opexp, pac_summary, pas2,
    },
    cli::{BulkArgs, BulkSource, CycleArg},
    sourcer::FilingSourcer,
};
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};

pub fn bulk(sourcer: FilingSourcer, args: &BulkArgs) -> anyhow::Result<()> {
    println!("Running bulk command with args: {:?}", args);
    let offline = sourcer.cache.offline;

    let has_individual_contributions = args.source.contains(&BulkSource::IndividualContributions);
    let non_ic_sources: Vec<&BulkSource> = args
        .source
        .iter()
        .filter(|s| **s != BulkSource::IndividualContributions)
        .collect();

    if !non_ic_sources.is_empty() && args.output.is_none() {
        anyhow::bail!(
            "--output is required when exporting sources other than individual-contributions"
        );
    }

    // Convert cycle argument to a vector of even years
    let cycles: Vec<u16> = match args.cycle {
        CycleArg::Single(year) => vec![year],
        CycleArg::Range(start, end) => (start..=end).filter(|y| y % 2 == 0).collect(),
    };

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

    // Handle non-individual-contributions sources in the output db
    if !non_ic_sources.is_empty() {
        let mut db = rusqlite::Connection::open(args.output.as_ref().unwrap())?;
        let mut tx = db.transaction()?;

        for year in &cycles {
            for source in &args.source {
                if *source == BulkSource::IndividualContributions {
                    continue;
                }
                pb.set_message(format!("{} - {:?}", year, source));
                let result = match source {
                    BulkSource::Opex => opexp::export(&mut tx, *year, Some(&on_progress), offline),
                    BulkSource::Committees => {
                        committee::export(&mut tx, *year, Some(&on_progress), offline)
                    }
                    BulkSource::Candidates => {
                        candidates::export(&mut tx, *year, Some(&on_progress), offline)
                    }
                    BulkSource::ContributionsToCandidates => {
                        pas2::export(&mut tx, *year, Some(&on_progress), offline)
                    }
                    BulkSource::PacSummary => {
                        pac_summary::export(&mut tx, *year, Some(&on_progress), offline)
                    }
                    BulkSource::CandidateSummary => {
                        candidate_summary::export(&mut tx, *year, Some(&on_progress), offline)
                    }
                    BulkSource::IndependentExpenditures => independent_expenditures::export(
                        &mut tx,
                        *year,
                        Some(&on_progress),
                        offline,
                    ),
                    BulkSource::Form2Filers => {
                        form2_filers::export(&mut tx, *year, Some(&on_progress), offline)
                    }
                    BulkSource::CandidateSummaryCsv => {
                        candidate_summary_csv::export(&mut tx, *year, Some(&on_progress), offline)
                    }
                    BulkSource::CommitteeSummaryCsv => {
                        committee_summary_csv::export(&mut tx, *year, Some(&on_progress), offline)
                    }
                    BulkSource::Form1Filers => {
                        form1_filers::export(&mut tx, *year, Some(&on_progress), offline)
                    }
                    BulkSource::IndividualContributions => unreachable!(),
                };
                result.unwrap();
                pb.inc(1);
            }
            if args.source.contains(&BulkSource::Candidates)
                && args.source.contains(&BulkSource::Committees)
            {
                pb.set_message(format!("{} - Candidate-Committee Linkage", *year));
                candidate_committee_linkage::export(&mut tx, *year, Some(&on_progress), offline)?;
                pb.inc(1);
            }
        }

        tx.commit()?;
    }

    // Handle individual contributions: always sync to cache db (tracks freshness),
    // then copy to --output if provided
    if has_individual_contributions {
        let mut cache = sourcer.cache;
        let cache_db_path = cache.individual_contributions_database_path();

        // Sync to cache db, then close it before potentially attaching from output db
        {
            let mut cache_db = cache.open_individual_contributions_database()?;
            let mut ic_tx = cache_db.transaction()?;

            for year in &cycles {
                pb.set_message(format!("{} - IndividualContributions", year));
                individual_contributions::export(&mut ic_tx, *year, Some(&on_progress), offline)?;
                pb.inc(1);
            }

            ic_tx.commit()?;
        }

        if let Some(output) = &args.output {
            let output_canonical = std::fs::canonicalize(output).unwrap_or(output.clone());
            let cache_canonical =
                std::fs::canonicalize(&cache_db_path).unwrap_or(cache_db_path.clone());
            if output_canonical != cache_canonical {
                pb.set_message("Copying individual contributions to output…".to_string());
                let out_db = rusqlite::Connection::open(output)?;
                out_db.execute_batch(individual_contributions::SCHEMA)?;
                let cache_db_str = cache_db_path.to_str().ok_or_else(|| {
                    anyhow::anyhow!("Cache db path is not valid UTF-8: {:?}", cache_db_path)
                })?;
                out_db.execute("ATTACH DATABASE ?1 AS ic_cache", [cache_db_str])?;
                for year in &cycles {
                    out_db.execute(
                        "DELETE FROM individual_contributions WHERE cycle = ?1",
                        [year],
                    )?;
                    out_db.execute(
                        "INSERT INTO individual_contributions SELECT * FROM ic_cache.individual_contributions WHERE cycle = ?1",
                        [year],
                    )?;
                }
                out_db.execute_batch("DETACH DATABASE ic_cache")?;
                eprintln!("Individual contributions written to {}", output.display());
            } else {
                eprintln!(
                    "Individual contributions written to {}",
                    cache_db_path.display()
                );
            }
        } else {
            eprintln!(
                "Individual contributions written to {}",
                cache_db_path.display()
            );
        }
    }

    pb.finish_with_message("Complete!");
    Ok(())
}
