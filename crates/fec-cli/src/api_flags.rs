use std::{io::Cursor, path::PathBuf, str::FromStr};

use crate::{
    cache::bulk_candidates::ResolveCandidateParamsBuilder,
    sourcer::{FecFilingId, FilingSourcer},
};
use anyhow::Context;
use clap::Parser;
use fec_api::{Api, FilingArgs, FilingArgsBuilder, Office};
use jiff::civil::Date;
use std::io::Read;

#[derive(Parser, Debug, Clone)]
#[command(next_help_heading = "FEC API options")]
pub struct FilingsApiFlags {
    #[arg(long, help = "Committee ID to filter filings by")]
    pub committee: Option<Vec<String>>,

    #[arg(long, help = "Committee ID to filter filings by")]
    pub candidate: Option<Vec<String>>,

    #[arg(long, help = "Form type to filter filings by")]
    pub form_type: Option<Vec<String>>,

    #[arg(long, help = "one-letter type code of the organization")]
    pub committee_type: Option<Vec<String>>,

    // coverage-before
    #[arg(long, help = "Only filings that cover dates before this date")]
    pub coverage_before: Option<Date>,

    // coverage-after
    #[arg(
        long,
        help = "Only filings that cover dates after this date",
        conflicts_with = "coverage_between"
    )]
    pub coverage_after: Option<Date>,

    #[arg(
        long,
        help = "Only filings container coverage between these dates",
        num_args = 2
    )]
    pub coverage_between: Vec<Date>,

    #[arg(long)]
    pub election: Option<String>,
    #[arg(long)]
    pub state: Option<String>,
    #[arg(long)]
    pub district: Option<String>,

    #[arg(long, requires = "election")]
    pub office: Option<String>,

    #[arg(long, num_args = 2)]
    pub bulk_daily_between: Option<Vec<Date>>,

    #[arg(
        long,
        help = "API key to use for OpenFEC API requests. If not provided, the DEMO_KEY will be used.",
        env = "LIBFEC_API_KEY"
    )]
    pub api_key: Option<String>,

    #[arg(long)]
    pub cache: bool,
}

static BASE_URL: &str =
    "https://cg-519a459a-0ea3-42c2-b7bc-fa1143481f74.s3-us-gov-west-1.amazonaws.com";

fn cache_day(date: Date, cache_directory: &PathBuf) -> anyhow::Result<()> {
    let _info_path = cache_directory
        .join(date.to_string())
        .with_extension("info");
    let zip_url = format!(
        "{BASE_URL}/bulk-downloads/electronic/{}.zip",
        date.strftime("%Y%m%d")
    );
    let response = ureq::get(zip_url)
        .call()
        .unwrap()
        .into_parts()
        .1
        .into_reader();
    let mut buffer = Vec::new();
    response.take(100_000_000).read_to_end(&mut buffer)?;

    let reader = Cursor::new(buffer);
    let mut archive = zip::ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.name().ends_with(".fec") {
            let output_path = cache_directory.join(file.name());
            if !output_path.exists() {
                std::fs::create_dir_all(output_path.parent().unwrap())?;
                let mut out_file = std::fs::File::create(&output_path)?;
                std::io::copy(&mut file, &mut out_file)?;
            }
        }
    }

    Ok(())
}

impl FilingsApiFlags {
    pub(crate) fn any_provided(&self) -> bool {
        self.candidate.is_some()
            || self.committee.is_some()
            || self.form_type.is_some()
            || self.election.is_some()
            || self.bulk_daily_between.is_some()
    }

    pub(crate) fn resolve_ids(&self, sourcer: &FilingSourcer) -> anyhow::Result<Vec<FecFilingId>> {
        let client = Api::new(self.api_key.as_deref().unwrap_or("DEMO_KEY"));

        let mut results = if let Some(election) = &self.election {
            let params = ResolveCandidateParamsBuilder::default()
                .cycle(election.parse::<u16>()?)
                .state(self.state.clone())
                .district(self.district.clone())
                .office(Some(Office::from_str(
                    self.office.as_deref().unwrap_or("H"),
                )?))
                .build()?;
            let committees = sourcer.cache.resolve_candidate_committees(params)?;
            if committees.is_empty() {
                panic!("No committees found for election {}", election);
            }
            dbg!(&committees);

            let filing_args = FilingArgsBuilder::default()
                .committees(committees)
                .form_types(self.form_type.clone())
                .committee_types(self.committee_type.clone())
                .cycle(vec![election.parse::<u16>()?])
                .build()
                .unwrap();
            let url = client.filings_url(filing_args)?;
            dbg!(&url.0.to_string());

            client
                .filings(url)
                .context("Filings API Call from election committees")?
        } else if let Some(bulk_daily_between) = &self.bulk_daily_between {
            if bulk_daily_between.len() != 2 {
                return Err(anyhow::anyhow!(
                    "bulk-daily-between requires exactly two dates"
                ));
            }
            let before = bulk_daily_between[0];
            let after = bulk_daily_between[1];
            println!("{}-{}", before, after);
            assert!(before <= after, "bulk-daily-between dates must be in order");
            let days: Vec<Date> = before
                .series(jiff::Span::new().days(1))
                .take_while(|&d| d <= after)
                .collect();
            println!("{:?} days", days);
            for day in days {
                cache_day(day, &PathBuf::from("tmpA"))?;
            }
            vec![]
        } else if self.candidate.is_some() {
            let url = client.filings_url(FilingArgs {
                committees: self.candidate.clone().unwrap_or_default(),
                form_types: self.form_type.clone(),
                committee_types: self.committee_type.clone(),
                cycle: vec![],
            })?;

            client.filings(url)?
        } else if self.committee.is_some() {
            let url = client.filings_url(FilingArgs {
                committees: self.committee.clone().unwrap_or_default(),
                form_types: self.form_type.clone(),
                committee_types: self.committee_type.clone(),
                cycle: vec![],
            })?;
            client.filings(url)?
        } else {
            let url = client.filings_url(FilingArgs {
                committees: self.committee.clone().unwrap_or_default(),
                form_types: self.form_type.clone(),
                committee_types: self.committee_type.clone(),
                cycle: vec![],
            })?;

            client.filings(url)?
        };

        // post-filter coverage dates, if provided
        if let Some(coverage_before) = self.coverage_before {
            results.retain(|item| {
                item.value
                    .get("coverage_start_date")
                    .and_then(|date_value| date_value.as_str())
                    .and_then(|date_str| date_str.parse::<Date>().ok())
                    .map_or(false, |coverage_start_date| {
                        coverage_start_date <= coverage_before
                    })
            });
        }
        if let Some(coverage_after) = self.coverage_after {
            results.retain(|item| {
                item.value
                    .get("coverage_end_date")
                    .and_then(|date_value| date_value.as_str())
                    .and_then(|date_str| date_str.parse::<Date>().ok())
                    .map_or(false, |coverage_end_date| {
                        coverage_end_date >= coverage_after
                    })
            });
        }

        Ok(results
            .iter()
            .map(|item| FecFilingId::from_str(&item.filing_id).unwrap())
            .collect())
    }
}
