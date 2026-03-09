use std::{str::FromStr, sync::LazyLock};

use crate::{
    cache::bulk::candidates::{ResolveCandidateParams, ResolveCandidateParamsBuilder},
    sourcer::{resolve_from_path, FecFilingId, FilingSourcer},
};
use anyhow::Context;
use clap::Parser;
use fec_api::{
    Api, ApiCache, ApiResponse, CandidateId, CommitteeId, FilingArgsBuilder, FilingItem, Office,
};
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
    pub committee_ids: Vec<CommitteeId>,
}

#[derive(Parser, Debug, Clone)]
#[command(next_help_heading = "Filing filters")]
pub struct FilingsApiFlags {
    #[arg(long, help = "Filter filings to only these committees")]
    pub committee: Option<Vec<CommitteeId>>,

    #[arg(long, help = "Filter filings to only these candidates")]
    pub candidate: Option<Vec<CandidateId>>,

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

    #[arg(long, help = "Only filings received after this date (YYYY-MM-DD)")]
    pub received_after: Option<Date>,

    #[arg(long, help = "Only filings received before this date (YYYY-MM-DD)")]
    pub received_before: Option<Date>,

    #[arg(long, help = "Only filings related to this election year (e.g., 2024)")]
    pub election: Option<u16>,
    #[arg(long, help = "Only filings related to this state (e.g., 'TX')")]
    pub state: Option<String>,
    #[arg(
        long,
        help = "Only filings related to a specific house district (e.g., '03')"
    )]
    pub district: Option<String>,

    #[arg(
        long,
        help = "Only filings related to a specific office (e.g., 'H' for House, 'S' for Senate, 'P' for President)"
    )]
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
    offline: bool,
) -> anyhow::Result<(Vec<FilingItem>, ApiResponse)> {
    let response = fec_api::api_request_cached(url, cache, offline)?;
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

#[derive(Default)]
struct FetchStats {
    cache_hits: usize,
    cache_misses: usize,
}

impl FetchStats {
    fn record(&mut self, response: &ApiResponse) {
        if response.cache_hit {
            self.cache_hits += 1;
        } else {
            self.cache_misses += 1;
        }
    }
}

fn fetch_all_pages(
    initial_url: Url,
    sourcer: &mut FilingSourcer,
    stats: &mut FetchStats,
    spinner: &Option<ProgressBar>,
    message_prefix: &str,
) -> anyhow::Result<Vec<FilingItem>> {
    let mut results = vec![];
    let mut current = initial_url;
    let offline = sourcer.cache.offline;
    loop {
        let (items, response) = filing_items(
            &current,
            sourcer
                .cache
                .api_cache_mut()
                .map(|c| c as &mut dyn ApiCache),
            offline,
        )?;
        results.extend(items);
        stats.record(&response);
        if let Some(sp) = spinner.as_ref() {
            sp.set_message(format!(
                "{}{}/{} pages — {} cache hits, {} API requests, {} remaining",
                message_prefix,
                response.pagination.page,
                response.pagination.pages,
                stats.cache_hits,
                stats.cache_misses,
                response.rate_limit.remaining
            ))
        }

        if let Some(next_url) = response.next_url {
            current = next_url;
        } else {
            break;
        }
    }
    Ok(results)
}

#[allow(clippy::too_many_arguments)]
fn fetch_efiling_dedup(
    client: &Api,
    committees: &[CommitteeId],
    form_types: &Option<Vec<String>>,
    report_types: &Option<Vec<String>>,
    include_amendments: bool,
    sourcer: &mut FilingSourcer,
    results: &mut Vec<FilingItem>,
    stats: &mut FetchStats,
) -> anyhow::Result<()> {
    let offline = sourcer.cache.offline;
    for chunk in committees.chunks(50) {
        let efiling_filing_args = fec_api::EfilingFilingArgs {
            committees: chunk.to_vec(),
            form_types: form_types.clone(),
            report_types: report_types.clone(),
        };
        let mut current = client.efiling_filings_url(efiling_filing_args).0;
        loop {
            // Note: efile/filings has max-age=0 so caching won't help, but we pass the cache anyway
            let (items, response) = filing_items(
                &current,
                sourcer
                    .cache
                    .api_cache_mut()
                    .map(|c| c as &mut dyn ApiCache),
                offline,
            )?;
            stats.record(&response);
            for item in items {
                // The efile/filings endpoint may not support report_type filtering,
                // so filter client-side as well
                if let Some(rt_filter) = report_types {
                    let item_rt = item
                        .value
                        .get("report_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !rt_filter.iter().any(|rt| rt == item_rt) {
                        continue;
                    }
                }
                // Skip superseded filings when not including amendments.
                // The efile/filings endpoint doesn't support most_recent=true,
                // so we filter client-side using the amended_by field.
                if !include_amendments {
                    let is_superseded = item.value.get("amended_by").is_some_and(|v| match v {
                        serde_json::Value::Number(n) => n.as_u64().is_some_and(|n| n > 0),
                        serde_json::Value::Null => false,
                        _ => false,
                    });
                    if is_superseded {
                        continue;
                    }
                }
                if !results
                    .iter()
                    .any(|existing| existing.filing_id == item.filing_id)
                {
                    results.push(item);
                }
            }
            if let Some(next_url) = response.next_url {
                current = next_url;
            } else {
                break;
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
        let committee_strings = sourcer
            .cache
            .resolve_candidate_principal_campaign_committees(params)?;
        let committees: Vec<CommitteeId> = committee_strings
            .into_iter()
            .map(|s| CommitteeId::new(&s).unwrap())
            .collect();
        if committees.is_empty() {
            panic!("No committees found for election {}", election);
        }

        if let Some(sp) = spinner.as_ref() {
            sp.set_message(format!("Found {} candidate committees…", committees.len()))
        }
        let mut results = vec![];
        let mut stats = FetchStats::default();
        for (idx, chunk) in committees.chunks(50).enumerate() {
            let filing_args = FilingArgsBuilder::default()
                .committees(chunk)
                .candidates(vec![])
                .form_types(self.form_type.clone())
                .report_types(self.report_type.clone())
                .committee_types(self.committee_type.clone())
                .cycle(vec![election])
                .include_amendments(self.include_amendments)
                .min_receipt_date(self.received_after.map(|d| d.to_string()))
                .max_receipt_date(self.received_before.map(|d| d.to_string()))
                .build()
                .with_context(|| {
                    format!("could not build filing args for election {}", election)
                })?;
            let url = client.filings_url(filing_args).0;
            let prefix = format!("chunk={idx} {} ", committees.len());
            let items = fetch_all_pages(url, sourcer, &mut stats, spinner, &prefix)?;
            results.extend(items);
        }
        fetch_efiling_dedup(
            client,
            &committees,
            &self.form_type,
            &self.report_type,
            self.include_amendments,
            sourcer,
            &mut results,
            &mut stats,
        )?;
        Ok(results)
    }

    fn resolve_normal(
        &self,
        client: &Api,
        sourcer: &mut FilingSourcer,
        spinner: &Option<ProgressBar>,
    ) -> anyhow::Result<Vec<FilingItem>> {
        let committees = self.committee.clone().unwrap_or_default();
        let candidates = self.candidate.clone().unwrap_or_default();
        let mut stats = FetchStats::default();
        let mut results = vec![];

        if committees.len() > 50 {
            for (idx, chunk) in committees.chunks(50).enumerate() {
                let args = FilingArgsBuilder::default()
                    .committees(chunk)
                    .candidates(vec![])
                    .form_types(self.form_type.clone())
                    .report_types(self.report_type.clone())
                    .committee_types(self.committee_type.clone())
                    .cycle(self.cycle.clone().unwrap_or_default())
                    .include_amendments(self.include_amendments)
                    .min_receipt_date(self.received_after.map(|d| d.to_string()))
                    .max_receipt_date(self.received_before.map(|d| d.to_string()))
                    .build()
                    .with_context(|| "could not build filing args".to_string())?;
                let url = client.filings_url(args).0;
                let prefix = format!("chunk={idx} {} ", committees.len());
                let items = fetch_all_pages(url, sourcer, &mut stats, spinner, &prefix)?;
                results.extend(items);
            }
            // Fetch candidates separately if any
            if !candidates.is_empty() {
                let args = FilingArgsBuilder::default()
                    .committees(vec![])
                    .candidates(candidates)
                    .form_types(self.form_type.clone())
                    .report_types(self.report_type.clone())
                    .committee_types(self.committee_type.clone())
                    .cycle(self.cycle.clone().unwrap_or_default())
                    .include_amendments(self.include_amendments)
                    .min_receipt_date(self.received_after.map(|d| d.to_string()))
                    .max_receipt_date(self.received_before.map(|d| d.to_string()))
                    .build()
                    .with_context(|| "could not build filing args".to_string())?;
                let url = client.filings_url(args).0;
                let items = fetch_all_pages(url, sourcer, &mut stats, spinner, "")?;
                results.extend(items);
            }
        } else {
            let args = FilingArgsBuilder::default()
                .committees(committees.clone())
                .candidates(candidates)
                .form_types(self.form_type.clone())
                .report_types(self.report_type.clone())
                .committee_types(self.committee_type.clone())
                .cycle(self.cycle.clone().unwrap_or_default())
                .include_amendments(self.include_amendments)
                .min_receipt_date(self.received_after.map(|d| d.to_string()))
                .max_receipt_date(self.received_before.map(|d| d.to_string()))
                .build()
                .with_context(|| "could not build filing args".to_string())?;
            let url = client.filings_url(args).0;
            let items = fetch_all_pages(url, sourcer, &mut stats, spinner, "")?;
            results.extend(items);
        }

        fetch_efiling_dedup(
            client,
            &committees,
            &self.form_type,
            &self.report_type,
            self.include_amendments,
            sourcer,
            &mut results,
            &mut stats,
        )?;

        Ok(results)
    }

    /// Resolve API flags into full FilingItem objects (with JSON values).
    /// Only works for the API path (not bulk-daily-between).
    pub(crate) fn resolve_items(
        &mut self,
        sourcer: &mut FilingSourcer,
        mb: Option<&MultiProgress>,
        trace: &mut Trace,
    ) -> anyhow::Result<Vec<FilingItem>> {
        let client = Api::new(self.api_key.as_deref().unwrap_or("DEMO_KEY"));

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

        Ok(results)
    }

    pub(crate) fn resolve_ids(
        &mut self,
        sourcer: &mut FilingSourcer,
        mb: Option<&MultiProgress>,
        trace: &mut Trace,
    ) -> anyhow::Result<Vec<FecFilingId>> {
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

        let items = self.resolve_items(sourcer, mb, trace)?;
        Ok(items
            .iter()
            .map(|item| FecFilingId::from_str(&item.filing_id).unwrap())
            .collect())
    }

    /// Build all API URLs that would be made for the current flags, without fetching.
    /// Returns (filings_urls, efiling_urls).
    #[cfg(test)]
    fn build_urls(&self) -> (Vec<Url>, Vec<Url>) {
        let client = Api::new("TEST_KEY");
        let committees = self.committee.clone().unwrap_or_default();
        let candidates = self.candidate.clone().unwrap_or_default();
        let cycle = if self.election.is_some() && self.office.is_none() && self.state.is_none() {
            self.election.map(|v| vec![v]).unwrap_or_default()
        } else {
            self.cycle.clone().unwrap_or_default()
        };

        let mut filings_urls = vec![];

        if committees.len() > 50 {
            for chunk in committees.chunks(50) {
                let args = FilingArgsBuilder::default()
                    .committees(chunk)
                    .candidates(vec![])
                    .form_types(self.form_type.clone())
                    .report_types(self.report_type.clone())
                    .committee_types(self.committee_type.clone())
                    .cycle(cycle.clone())
                    .include_amendments(self.include_amendments)
                    .min_receipt_date(self.received_after.map(|d| d.to_string()))
                    .max_receipt_date(self.received_before.map(|d| d.to_string()))
                    .build()
                    .unwrap();
                filings_urls.push(client.filings_url(args).0);
            }
            if !candidates.is_empty() {
                let args = FilingArgsBuilder::default()
                    .committees(vec![])
                    .candidates(candidates)
                    .form_types(self.form_type.clone())
                    .report_types(self.report_type.clone())
                    .committee_types(self.committee_type.clone())
                    .cycle(cycle.clone())
                    .include_amendments(self.include_amendments)
                    .min_receipt_date(self.received_after.map(|d| d.to_string()))
                    .max_receipt_date(self.received_before.map(|d| d.to_string()))
                    .build()
                    .unwrap();
                filings_urls.push(client.filings_url(args).0);
            }
        } else {
            let args = FilingArgsBuilder::default()
                .committees(committees.clone())
                .candidates(candidates)
                .form_types(self.form_type.clone())
                .report_types(self.report_type.clone())
                .committee_types(self.committee_type.clone())
                .cycle(cycle.clone())
                .include_amendments(self.include_amendments)
                .min_receipt_date(self.received_after.map(|d| d.to_string()))
                .max_receipt_date(self.received_before.map(|d| d.to_string()))
                .build()
                .unwrap();
            filings_urls.push(client.filings_url(args).0);
        }

        let mut efiling_urls = vec![];
        for chunk in committees.chunks(50) {
            let efiling_args = fec_api::EfilingFilingArgs {
                committees: chunk.to_vec(),
                form_types: self.form_type.clone(),
                report_types: self.report_type.clone(),
            };
            efiling_urls.push(client.efiling_filings_url(efiling_args).0);
        }

        (filings_urls, efiling_urls)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn parse_flags(args: &str) -> FilingsApiFlags {
        let argv: Vec<&str> = std::iter::once("test")
            .chain(args.split_whitespace())
            .collect();
        FilingsApiFlags::parse_from(argv)
    }

    fn format_url_params(url: &Url) -> String {
        let redacted = fec_api::redact_api_key(url);
        let parsed = Url::parse(&redacted).unwrap();
        let mut lines = vec![];
        for (key, value) in parsed.query_pairs() {
            lines.push(format!("  {}: {}", key, value));
        }
        lines.join("\n")
    }

    fn snapshot_urls(flags_str: &str) -> String {
        let flags = parse_flags(flags_str);
        let (filings_urls, efiling_urls) = flags.build_urls();

        let mut out = format!("flags: {}\n", flags_str);

        for (i, url) in filings_urls.iter().enumerate() {
            if filings_urls.len() > 1 {
                out.push_str(&format!("\n/v1/filings [{}]\n", i));
            } else {
                out.push_str("\n/v1/filings\n");
            }
            out.push_str(&format_url_params(url));
            out.push('\n');
        }

        for (i, url) in efiling_urls.iter().enumerate() {
            if efiling_urls.len() > 1 {
                out.push_str(&format!("\n/v1/efile/filings [{}]\n", i));
            } else {
                out.push_str("\n/v1/efile/filings\n");
            }
            out.push_str(&format_url_params(url));
            out.push('\n');
        }

        out
    }

    #[test]
    fn test_urls_single_committee_f3_ye() {
        assert_snapshot!(snapshot_urls(
            "--committee C00401224 --form-type F3 --report-type YE --cycle 2026"
        ));
    }

    #[test]
    fn test_urls_multiple_committees() {
        assert_snapshot!(snapshot_urls(
            "--committee C00401224 --committee C00558437 --committee C00796649 --form-type F3 --report-type YE --cycle 2026"
        ));
    }

    #[test]
    fn test_urls_candidate_only() {
        assert_snapshot!(snapshot_urls("--candidate P80000722 --cycle 2024"));
    }

    #[test]
    fn test_urls_with_amendments() {
        assert_snapshot!(snapshot_urls(
            "--committee C00401224 --cycle 2024 --include-amendments"
        ));
    }

    #[test]
    fn test_urls_with_date_filters() {
        assert_snapshot!(snapshot_urls(
            "--committee C00401224 --form-type F3 --received-after 2025-01-01 --received-before 2025-12-31"
        ));
    }

    #[test]
    fn test_urls_multiple_form_and_report_types() {
        assert_snapshot!(snapshot_urls(
            "--committee C00401224 --form-type F3 --form-type F3X --report-type YE --report-type Q3 --cycle 2024 --cycle 2026"
        ));
    }

    #[test]
    fn test_urls_election_without_office() {
        assert_snapshot!(snapshot_urls(
            "--committee C00401224 --form-type F3 --election 2026"
        ));
    }
}
