use std::{str::FromStr, sync::LazyLock};

use crate::{
    cache::bulk_candidates::{ResolveCandidateParams, ResolveCandidateParamsBuilder},
    sourcer::{resolve_from_path, FecFilingId, FilingSourcer},
};
use anyhow::Context;
use clap::Parser;
use fec_api::{Api, ApiCache, ApiResponse, FilingArgsBuilder, FilingItem, Office};
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

    #[arg(long, help = "Report type to filter filings by (e.g 'M1', 'Q1', etc)")]
    pub report_type: Option<Vec<String>>,

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

    #[arg(long, help = "Include amendments in results", default_value_t = false)]
    pub include_amendments: bool,
}

fn filing_items(
    url: &Url,
    cache: Option<&mut dyn ApiCache>,
) -> anyhow::Result<(Vec<FilingItem>, ApiResponse)> {
    let response = fec_api::api_request_cached(url, cache)?;
    let results = response
        .result_items
        .iter()
        // TODO this filter out RFAI, as they appear in this endpoint yet have no fec_file_id
        .filter_map(|v| {
            v.get("fec_file_id")
                .and_then(|id| id.as_str())
                .map(|id| FilingItem {
                    filing_id: id.to_string(),
                    value: v.clone(),
                })
        })
        .collect::<Vec<FilingItem>>();
    Ok((results, response))
}

impl FilingsApiFlags {
    pub(crate) fn any_provided(&self) -> bool {
        self.candidate.is_some()
            || self.committee.is_some()
            || self.form_type.is_some()
            || self.report_type.is_some()
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
        if let Some(sp) = spinner.as_ref() {
            sp.set_message(format!("Resolving committees for election {election}…"))
        }
        let committees = sourcer
            .cache
            .resolve_candidate_principal_campaign_committees(params)?;
        if committees.is_empty() {
            panic!("No committees found for election {}", election);
        }

        if let Some(sp) = spinner.as_ref() {
            sp.set_message(format!("Found {} candidate committees…", committees.len()))
        }
        let mut results = vec![];
        let mut cache_hits = 0usize;
        let mut cache_misses = 0usize;
        for (idx, chunk) in committees.chunks(50).enumerate() {
            let filing_args = FilingArgsBuilder::default()
                .committees(chunk)
                .candidates(vec![])
                .form_types(self.form_type.clone())
                .report_types(self.report_type.clone())
                .committee_types(self.committee_type.clone())
                .cycle(vec![election])
                .include_amendments(self.include_amendments)
                .build()
                .with_context(|| {
                    format!("could not build filing args for election {}", election)
                })?;
            let mut current = client.filings_url(filing_args);
            loop {
                let (items, response) = filing_items(
                    &current.0,
                    sourcer
                        .cache
                        .api_cache_mut()
                        .map(|c| c as &mut dyn ApiCache),
                )?;
                results.extend(items);
                if response.cache_hit {
                    cache_hits += 1;
                } else {
                    cache_misses += 1;
                }
                if let Some(sp) = spinner.as_ref() {
                    sp.set_message(format!(
                        "chunk={} {} {}/{} pages — {} cache hits, {} API requests, {} remaining",
                        idx,
                        committees.len(),
                        response.pagination.page,
                        response.pagination.pages,
                        cache_hits,
                        cache_misses,
                        response.rate_limit.remaining
                    ))
                }

                if let Some(next_url) = response.next_url {
                    current.0 = next_url;
                } else {
                    break;
                }
            }
        }
        for chunk in committees.chunks(50) {
            let efiling_filing_args = fec_api::EfilingFilingArgs {
                committees: chunk.to_vec(),
                form_types: self.form_type.clone(),
            };
            let mut current = client.efiling_filings_url(efiling_filing_args);
            loop {
                // Note: efile/filings has max-age=0 so caching won't help, but we pass the cache anyway
                let (items, response) = filing_items(
                    &current.0,
                    sourcer
                        .cache
                        .api_cache_mut()
                        .map(|c| c as &mut dyn ApiCache),
                )?;
                if response.cache_hit {
                    cache_hits += 1;
                } else {
                    cache_misses += 1;
                }
                for item in items {
                    if !results
                        .iter()
                        .any(|existing| existing.filing_id == item.filing_id)
                    {
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
        sourcer: &mut FilingSourcer,
        spinner: &Option<ProgressBar>,
    ) -> anyhow::Result<Vec<FilingItem>> {
        let args = FilingArgsBuilder::default()
            .committees(self.committee.clone().unwrap_or_default())
            .candidates(self.candidate.clone().unwrap_or_default())
            .form_types(self.form_type.clone())
            .report_types(self.report_type.clone())
            .committee_types(self.committee_type.clone())
            .cycle(self.cycle.clone().unwrap_or_default())
            .include_amendments(self.include_amendments)
            .build()
            .with_context(|| "could not build filing args".to_string())?;
        let filing_url = client.filings_url(args);

        let mut results = vec![];
        let mut cache_hits = 0usize;
        let mut cache_misses = 0usize;
        let mut current = filing_url;
        loop {
            let (items, response) = filing_items(
                &current.0,
                sourcer
                    .cache
                    .api_cache_mut()
                    .map(|c| c as &mut dyn ApiCache),
            )?;
            results.extend(items);
            if response.cache_hit {
                cache_hits += 1;
            } else {
                cache_misses += 1;
            }
            if let Some(sp) = spinner.as_ref() {
                sp.set_message(format!(
                    "{}/{} pages — {} cache hits, {} API requests, {} remaining",
                    response.pagination.page,
                    response.pagination.pages,
                    cache_hits,
                    cache_misses,
                    response.rate_limit.remaining
                ))
            }

            if let Some(next_url) = response.next_url {
                current = fec_api::FilingsUrl(next_url);
            } else {
                break;
            }
        }

        let committees = self.committee.clone().unwrap_or_default();
        for chunk in committees.chunks(50) {
            let efiling_filing_args = fec_api::EfilingFilingArgs {
                committees: chunk.to_vec(),
                form_types: self.form_type.clone(),
            };
            let mut current = client.efiling_filings_url(efiling_filing_args);
            loop {
                // Note: efile/filings has max-age=0 so caching won't help, but we pass the cache anyway
                let (items, response) = filing_items(
                    &current.0,
                    sourcer
                        .cache
                        .api_cache_mut()
                        .map(|c| c as &mut dyn ApiCache),
                )?;
                if response.cache_hit {
                    cache_hits += 1;
                } else {
                    cache_misses += 1;
                }
                for item in items {
                    if !results
                        .iter()
                        .any(|existing| existing.filing_id == item.filing_id)
                    {
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
                let items: Vec<crate::cache::CacheBulkDailyZipResultItem> =
                    sourcer.cache.cache_bulk_daily_zip(day, mb)?;

                // if the --form-types filter flag is provided, then we need to manually filter
                // by parsing the filing header. Not great performance-wise
                if let Some(form_types) = &self.form_type {
                    for item in items {
                        let f = resolve_from_path(item.output_path.clone())?;
                        let filing_form_type =
                            crate::commands::export::sqlite::form_type_parse(&f.cover.form_type).0;
                        if !form_types.iter().any(|ft| ft == filing_form_type) {
                            continue;
                        }

                        // also filter by coverage dates, if provided
                        // TODO does this only work if --form-types is provided?
                        if let Some(coverage_after) = self.coverage_after {
                            if let Some(through_date) = f.cover.coverage_through_date {
                                if through_date < coverage_after {
                                    continue;
                                }
                            }
                        }
                        if let Some(coverage_before) = self.coverage_before {
                            if let Some(from_date) = f.cover.coverage_from_date {
                                if from_date > coverage_before {
                                    continue;
                                }
                            }
                        }
                        all_filing_ids.push(item.filing_id);
                    }
                } else {
                    all_filing_ids.extend(
                        items
                            .iter()
                            .map(|item| item.filing_id.clone())
                            .collect::<Vec<_>>(),
                    );
                }
                if let Some(sp) = pb_days.as_ref() {
                    sp.inc(1)
                }
            }
            if let Some(sp) = pb_days.as_ref() {
                sp.finish_with_message(format!(
                    "Resolved {} filings from between {} and {}",
                    all_filing_ids.len(),
                    before,
                    after
                ))
            }
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
                self.cycle = self.election.map(|v| vec![v]);
                self.resolve_normal(&client, sourcer, &spinner)?
            }
        } else {
            self.resolve_normal(&client, sourcer, &spinner)?
        };

        // post-filter coverage dates, if provided
        if let Some(coverage_before) = self.coverage_before {
            results.retain(|item| {
                item.value
                    .get("coverage_start_date")
                    .and_then(|date_value| date_value.as_str())
                    .and_then(|date_str| date_str.parse::<Date>().ok())
                    .is_some_and(|coverage_start_date| coverage_start_date <= coverage_before)
            });
        }
        if let Some(coverage_after) = self.coverage_after {
            results.retain(|item| {
                item.value
                    .get("coverage_end_date")
                    .and_then(|date_value| date_value.as_str())
                    .and_then(|date_str| date_str.parse::<Date>().ok())
                    .is_some_and(|coverage_end_date| coverage_end_date >= coverage_after)
            });
        }
        if let Some(sp) = spinner.as_ref() {
            sp.finish_and_clear()
        }

        Ok(results
            .iter()
            .map(|item| FecFilingId::from_str(&item.filing_id).unwrap())
            .collect())
    }
}
