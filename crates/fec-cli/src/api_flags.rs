use std::{str::FromStr, sync::LazyLock};

use crate::{
    cache::bulk_candidates::{ResolveCandidateParams, ResolveCandidateParamsBuilder},
    sourcer::{FecFilingId, FilingSourcer},
};
use anyhow::Context;
use clap::Parser;
use fec_api::{Api, ApiResponse, EfilingFilingUrl, FilingArgsBuilder, FilingItem, FilingsUrl, Office};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use jiff::civil::Date;
use url::Url;

static DAILY_ZIP_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::default_bar()
        .template("{msg} {bar:40.gray/white} {pos}/{len} ({elapsed_precise})")
        .expect("valid progress bar template")
});

#[derive(Debug, Clone)]
pub struct Trace {
    pub resolve_candidate_params: Vec<ResolveCandidateParams>,
}

#[derive(Parser, Debug, Clone)]
#[command(next_help_heading = "FEC API options")]
pub struct FilingsApiFlags {
    #[arg(long, help = "Filter filings to only these committees")]
    pub committee: Option<Vec<String>>,

    #[arg(long, help = "Filter filings to only these candidates")]
    pub candidate: Option<Vec<String>>,

    #[arg(
        long,
        help = "Form type to filter filings by (e.g. 'F3', 'F3X', 'F1', 'F2', etc)"
    )]
    pub form_type: Option<Vec<String>>,

    #[arg(long, help = "one-letter type code of the organization")]
    pub committee_type: Option<Vec<String>>,

    #[arg(long, help = "Election cycle")]
    pub cycle: Option<Vec<u16>>,

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
    pub election: Option<u16>,
    #[arg(long)]
    pub state: Option<String>,
    #[arg(long)]
    pub district: Option<String>,

    #[arg(long)]
    pub office: Option<String>,

    #[arg(long, num_args = 2)]
    pub bulk_daily_between: Option<Vec<Date>>,

    #[arg(
        long,
        help = "API key to use for OpenFEC API requests. If not provided, the DEMO_KEY will be used.",
        env = "LIBFEC_API_KEY",
        hide_env = true
    )]
    pub api_key: Option<String>,
}

fn filing_items(url: &Url) -> anyhow::Result<(Vec<FilingItem>, ApiResponse)> {
    let response = fec_api::api_request(url)?;
    let results = response
        .result_items
        .iter()
        // TODO this filter out RFAI, as they appear in this endpoint yet have no fec_file_id
        .filter_map(|v| match v.get("fec_file_id").and_then(|id| id.as_str()) {
            Some(id) => Some(FilingItem {
                filing_id: id.to_string(),
                value: v.clone(),
            }),
            None => None,
        })
        .collect::<Vec<FilingItem>>();
    Ok((results, response))
}


impl FilingsApiFlags {
    pub(crate) fn any_provided(&self) -> bool {
        self.candidate.is_some()
            || self.committee.is_some()
            || self.form_type.is_some()
            || self.election.is_some()
            || self.bulk_daily_between.is_some()
    }

    /**
     * We should support:
     *   - [ ] candidates themselves, no committee (H8VA01233, F2's, etc.)
     *   - [x] principal campaign commiteees
     *   - [ ] JFC/Leadership PACs connected to candidates
     *   - [ ] Independent expenditures (trace) and
     *
     */
    fn resolve_election(
        &self,
        client: &Api,
        sourcer: &mut FilingSourcer,
        spinner: &Option<ProgressBar>,
        election: u16,
        trace: &mut Trace,
    ) -> anyhow::Result<Vec<FilingItem>> {
        let params = ResolveCandidateParamsBuilder::default()
            .cycle(election)
            .state(self.state.clone())
            .district(self.district.clone())
            .office(Some(Office::from_str(
                self.office.as_deref().unwrap_or("H"),
            )?))
            .build()?;
        trace.resolve_candidate_params.push(params.clone());
        spinner
            .as_ref()
            .map(|sp| sp.set_message(format!("Resolving committees for election {election}…")));
        let committees = sourcer
            .cache
            .resolve_candidate_principal_campaign_committees(params)?;
        if committees.is_empty() {
            panic!("No committees found for election {}", election);
        }

        spinner
            .as_ref()
            .map(|sp| sp.set_message(format!("Found {} candidate committees…", committees.len())));
        dbg!(&committees);
        let mut results = vec![];
        for (idx, chunk) in committees.chunks(50).enumerate() {
            let filing_args = FilingArgsBuilder::default()
                .committees(chunk)
                .candidates(vec![])
                .form_types(self.form_type.clone())
                .committee_types(self.committee_type.clone())
                .cycle(vec![election])
                .build()
                .with_context(|| {
                    format!("could not build filing args for election {}", election)
                })?;
            let mut current = client.filings_url(filing_args);
            loop {
                let (items, response) = filing_items(&current.0)?;
                results.extend(items);
                spinner.as_ref().map(|sp| {
                    sp.set_message(format!(
                        "chunk={} {} {}/{} pages — {} API requests made, {} remaining",
                        idx,
                        committees.len(),
                        response.pagination.page,
                        response.pagination.pages,
                        1,
                        response.rate_limit.remaining
                    ))
                });

                if let Some(next_url) = response.next_url {
                    current.0 = next_url;
                } else {
                    break;
                }
            }
        }
        for (idx, chunk) in committees.chunks(50).enumerate() {
          let efiling_filing_args = fec_api::EfilingFilingArgs {
              committees: chunk.to_vec(),
              form_types: self.form_type.clone(),
          };
          let mut current = client.efiling_filings_url(efiling_filing_args);
          loop {
              let (items, response) = filing_items(&current.0)?;
              for item in items {
                  if !results.iter().any(|existing| existing.filing_id == item.filing_id) {
                      results.push(item);
                  }
              }
              if let Some(next_url) = response.next_url {
                    current.0 = next_url;
                } else {
                    break;
                }
               
          }
        }
        Ok(results)
    }

    fn resolve_normal(
        &self,
        client: &Api,
        spinner: &Option<ProgressBar>,
    ) -> anyhow::Result<Vec<FilingItem>> {
        let args = FilingArgsBuilder::default()
            .committees(self.committee.clone().unwrap_or_default())
            .candidates(self.candidate.clone().unwrap_or_default())
            .form_types(self.form_type.clone())
            .committee_types(self.committee_type.clone())
            .cycle(self.cycle.clone().unwrap_or_default())
            .build()
            .with_context(|| format!("could not build filing args"))?;
        let filing_url = client.filings_url(args);

        let mut results = vec![];
        let mut n = 0;
        let mut current = filing_url;
        loop {
            let (items, response) = filing_items(&current.0)?;
            results.extend(items);
            n += 1;
            spinner.as_ref().map(|sp| {
                sp.set_message(format!(
                    "{}/{} pages — {} API requests made, {} remaining",
                    response.pagination.page,
                    response.pagination.pages,
                    n,
                    response.rate_limit.remaining
                ))
            });

            if let Some(next_url) = response.next_url {
                current = fec_api::FilingsUrl(next_url);
            } else {
                break;
            }
        }

        let committees = self.committee.clone().unwrap_or_default();
        for (idx, chunk) in committees.chunks(50).enumerate() {
          let efiling_filing_args = fec_api::EfilingFilingArgs {
              committees: chunk.to_vec(),
              form_types: self.form_type.clone(),
          };
          let mut current = client.efiling_filings_url(efiling_filing_args);
          loop {
              let (items, response) = filing_items(&current.0)?;
              for item in items {
                  if !results.iter().any(|existing| existing.filing_id == item.filing_id) {
                      results.push(item);
                  }
              }
              if let Some(next_url) = response.next_url {
                    current.0 = next_url;
                } else {
                    break;
                }
               
          }
        }


        Ok(results)
    }

    pub(crate) fn resolve_ids(
        &mut self,
        sourcer: &mut FilingSourcer,
        mb: Option<&MultiProgress>,
        trace: &mut Trace,
    ) -> anyhow::Result<Vec<FecFilingId>> {
        let client = Api::new(self.api_key.as_deref().unwrap_or("DEMO_KEY"));

        if let Some(bulk_daily_between) = &self.bulk_daily_between {
            if bulk_daily_between.len() != 2 {
                return Err(anyhow::anyhow!(
                    "bulk-daily-between requires exactly two dates"
                ));
            }
            let before = bulk_daily_between[0];
            let after = bulk_daily_between[1];
            assert!(before <= after, "bulk-daily-between dates must be in order");
            let days: Vec<Date> = before
                .series(jiff::Span::new().days(1))
                .take_while(|&d| d <= after)
                .collect();
            let mut all_filing_ids = vec![];
            let pb_days = mb.as_ref().map(|mb| {
                let pb = mb.add(ProgressBar::new(days.len() as u64));
                pb.set_style(DAILY_ZIP_STYLE.clone());
                pb.set_message(format!("Daily zips {} to {}…", before, after));
                pb
            });

            for day in days {
                let filing_ids = sourcer.cache.cache_bulk_daily_zip(day, mb)?;
                all_filing_ids.extend(filing_ids);
                pb_days.as_ref().map(|sp| sp.inc(1));
            }
            pb_days.as_ref().map(|sp| {
                sp.finish_with_message(format!(
                    "Resolved {} filings from between {} and {}",
                    all_filing_ids.len(),
                    before,
                    after
                ))
            });
            return Ok(all_filing_ids);
        }

        let spinner = mb.as_ref().map(|mb| {
            let sp = mb.add(indicatif::ProgressBar::new_spinner());
            sp.enable_steady_tick(std::time::Duration::from_millis(16));
            sp.set_message("Resolving filings from API…");
            sp
        });

        let mut results = if let Some(election) = &self.election {
            if self.office.is_some() || self.state.is_some() {
                self.resolve_election(&client, sourcer, &spinner, *election, trace)?
            } else {
                self.cycle = self.election.clone().map(|v| vec![v]);
                self.resolve_normal(&client, &spinner)?
            }
        } else {
            self.resolve_normal(&client, &spinner)?
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
        spinner.as_ref().map(|sp| sp.finish_and_clear());

        Ok(results
            .iter()
            .map(|item| FecFilingId::from_str(&item.filing_id).unwrap())
            .collect())
    }
}
