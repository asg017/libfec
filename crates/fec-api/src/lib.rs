use std::{str::FromStr, sync::LazyLock};

use anyhow::{Context, Result};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

/// A cached API response entry.
#[derive(Debug, Clone)]
pub struct ApiCacheEntry {
    /// The cached JSON response body.
    pub body: serde_json::Value,
    /// The max-age value from Cache-Control header (in seconds).
    pub max_age_secs: u64,
}

/// Trait for caching API responses. Implement this to provide custom caching behavior.
pub trait ApiCache {
    /// Get a cached response for the given URL, if it exists and is still valid.
    /// Returns None if not cached or expired.
    fn get(&self, url: &Url) -> Option<ApiCacheEntry>;

    /// Store a response in the cache.
    fn set(&mut self, url: &Url, entry: &ApiCacheEntry) -> Result<()>;
}

pub struct Api {
    api_key: String,
    base_url: Url,
}

#[derive(Deserialize, Debug)]
pub struct CandidateSearchItem {
    pub candidate_id: String,
    pub name: String,
    pub office_full: String,
}

#[derive(Deserialize, Serialize, Debug, Default, Builder)]
#[builder(setter(into))]
/// Query parameter arguments for the `/v1/filings/` OpenFEC API endpoint.
/// Source: https://api.open.fec.gov/developers/#/filings/get_v1_filings_
pub struct FilingArgs {
    /// "A unique identifier assigned to each committee or filer registered with the FEC.
    /// In general a committee id begins with the letter C which is followed by eight digits."
    pub committees: Vec<String>,
    pub candidates: Vec<String>,
    pub form_types: Option<Vec<String>>,
    pub report_types: Option<Vec<String>>,
    pub committee_types: Option<Vec<String>>,
    pub cycle: Vec<u16>,

    #[builder(default = "false")]
    pub include_amendments: bool,
    // TODO:
    // candidates: Vec<String>,

    // form_category: Vec<REPORT, NOTICE, STATEMENT, OTHER>
    // q_filer: Option<Vec<String>>,
    // amendment_indicator: Option<Vec<String>>,
    // primary_general_indicator
    // party
    // sort options?
}

pub struct EfilingFilingArgs {
    pub committees: Vec<String>,
    pub form_types: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Office {
    #[serde(rename = "H")]
    House,
    #[serde(rename = "S")]
    Senate,
    #[serde(rename = "P")]
    President,
}

impl std::fmt::Display for Office {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Office::House => write!(f, "house"),
            Office::Senate => write!(f, "senate"),
            Office::President => write!(f, "president"),
        }
    }
}

impl FromStr for Office {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "h" | "house" => Ok(Office::House),
            "s" | "senate" => Ok(Office::Senate),
            "p" | "president" | "prez" => Ok(Office::President),
            _ => Err(anyhow::anyhow!("Invalid office type")),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ElectionsArgs {
    pub cycle: u16,
    pub office: Office,
    pub state: Option<String>,
    pub district: Option<u16>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct CalendarDatesArgs {
    /// Category IDs to filter by (e.g., 21 for Reporting Deadlines, 36 for Elections)
    pub calendar_category_id: Option<Vec<u32>>,
    /// Minimum start date (YYYY-MM-DD format)
    pub min_start_date: Option<String>,
    /// Maximum start date (YYYY-MM-DD format)
    pub max_start_date: Option<String>,
    /// Sort field (default: -start_date for descending)
    pub sort: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ElectionItem {
    pub candidate_election_year: i64,
    pub candidate_id: String,
    pub candidate_name: String,
    pub candidate_pcc_id: String,
    pub candidate_pcc_name: String,
    pub cash_on_hand_end_period: f64,
    pub committee_ids: Vec<String>,
    pub coverage_end_date: Option<String>,
    pub incumbent_challenge_full: Option<String>,
    pub party_full: String,
    pub total_disbursements: f64,
    pub total_receipts: f64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FilingItem {
    pub filing_id: String,
    pub value: Value,
}

static FEC_API_BASE_URL: LazyLock<Url> = LazyLock::new(|| {
    Url::parse("https://api.open.fec.gov").expect("Failed to parse FEC API base URL")
});

#[derive(Debug, Clone)]
pub struct FilingsUrl(pub Url);
pub struct ElectionsUrl(pub Url);
pub struct CalendarDatesUrl(pub Url);

pub struct EfilingFilingUrl(pub Url);

fn redact_api_key(url: &Url) -> String {
    let mut redacted = url.clone();
    let mut qp = redacted.query_pairs_mut();
    qp.clear().append_pair("api_key", "REDACTED");
    drop(qp);
    redacted.to_string()
}

impl Api {
    pub fn new<'a, S: Into<&'a str>>(api_key: S) -> Self {
        Self {
            api_key: api_key.into().to_string(),
            base_url: FEC_API_BASE_URL.clone(),
        }
    }

    pub fn search_candidates(&self, q: &str) -> Result<Vec<CandidateSearchItem>> {
        let mut url = self.base_url.clone();
        url.set_path("/v1/candidates/search/");
        url.query_pairs_mut()
            .append_pair("api_key", &self.api_key)
            .append_pair("q", q);
        println!("{url}");
        let mut response = ureq::get(url.as_str())
            .header("accept", "application/json")
            .header(
                "User-Agent",
                format!("libfec/{}", env!("CARGO_PKG_VERSION")),
            )
            .call()?;
        let x: serde_json::Value = response.body_mut().read_json().with_context(|| {
            format!(
                "Could not read API response as JSON at {}",
                redact_api_key(&url)
            )
        })?;

        let results = x
            .get("results")
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Missing 'results' field in API JSON response for {}",
                    redact_api_key(&url)
                )
            })?
            .as_array()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Expected 'results' field in API JSON response to be an array, at {}",
                    redact_api_key(&url)
                )
            })?
            .iter()
            .map(|v| {
                serde_json::from_value::<CandidateSearchItem>(v.clone()).map_err(|_| {
                    anyhow::anyhow!(
                        "Error deserializing candidate search item at {}",
                        redact_api_key(&url)
                    )
                })
            })
            .collect::<Result<Vec<CandidateSearchItem>, _>>()?;
        Ok(results)
    }

    pub fn elections_url(&self, args: ElectionsArgs) -> ElectionsUrl {
        let mut url = self.base_url.clone();
        url.set_path("/v1/elections");
        let mut qp = url.query_pairs_mut();
        qp.append_pair("api_key", &self.api_key);

        qp.append_pair("cycle", &args.cycle.to_string());
        qp.append_pair("office", &args.office.to_string());
        if let Some(state) = &args.state {
            qp.append_pair("state", state);
        }
        if let Some(district) = &args.district {
            qp.append_pair("district", format!("{:02}", district).as_str());
        }
        drop(qp);
        ElectionsUrl(url)
    }

    pub fn calendar_dates_url(&self, args: CalendarDatesArgs) -> CalendarDatesUrl {
        let mut url = self.base_url.clone();
        url.set_path("/v1/calendar-dates/");
        let mut qp = url.query_pairs_mut();
        qp.append_pair("api_key", &self.api_key);

        if let Some(ref categories) = args.calendar_category_id {
            for cat_id in categories {
                qp.append_pair("calendar_category_id", &cat_id.to_string());
            }
        }
        if let Some(ref min_date) = args.min_start_date {
            qp.append_pair("min_start_date", min_date);
        }
        if let Some(ref max_date) = args.max_start_date {
            qp.append_pair("max_start_date", max_date);
        }

        if let Some(ref sort) = args.sort {
            qp.append_pair("sort", sort);
        } else {
            qp.append_pair("sort", "start_date");
        }
        // calendar dates API max per_page is 500, so just max it out
        qp.append_pair("per_page", "500");

        drop(qp);
        CalendarDatesUrl(url)
    }

    pub fn filings_url(&self, args: FilingArgs) -> FilingsUrl {
        let mut url = self.base_url.clone();
        url.set_path("/v1/filings");
        let mut qp = url.query_pairs_mut();
        qp.append_pair("api_key", &self.api_key);

        qp.append_pair("per_page", "100");

        if !args.include_amendments {
            qp.append_pair("most_recent", "true");
        }

        qp.append_pair("filer_type", "e-file");

        for committee in &args.committees {
            qp.append_pair("committee_id", committee);
        }
        for candidate in &args.candidates {
            qp.append_pair("candidate_id", candidate);
        }
        if let Some(form_types) = &args.form_types {
            for form_type in form_types {
                qp.append_pair("form_type", form_type);
            }
        }
        if let Some(report_types) = &args.report_types {
            for report_type in report_types {
                qp.append_pair("report_type", report_type);
            }
        }
        if let Some(committee_types) = &args.committee_types {
            for committee_type in committee_types {
                qp.append_pair("committee_type", committee_type);
            }
        }
        for year in &args.cycle {
            qp.append_pair("cycle", &year.to_string());
        }
        qp.append_pair("sort", "committee_id");
        drop(qp);
        FilingsUrl(url)
    }

    pub fn efiling_filings_url(&self, args: EfilingFilingArgs) -> EfilingFilingUrl {
        let mut url = self.base_url.clone();
        url.set_path("/v1/efile/filings");
        let mut qp = url.query_pairs_mut();
        qp.append_pair("api_key", &self.api_key);

        for committee in &args.committees {
            qp.append_pair("committee_id", committee);
        }
        if let Some(form_types) = &args.form_types {
            for form_type in form_types {
                qp.append_pair("form_type", form_type);
            }
        }

        qp.append_pair("per_page", "100");

        drop(qp);
        EfilingFilingUrl(url)
    }
}

pub struct FecApiRateLimit {
    pub limit: usize,
    pub remaining: usize,
}
pub struct ApiResponse {
    pub json: serde_json::Value,
    pub result_items: Vec<serde_json::Value>,
    pub rate_limit: FecApiRateLimit,
    pub pagination: FecApiPaginationObject,
    pub next_url: Option<Url>,
    /// Whether this response was served from cache.
    pub cache_hit: bool,
}

static USER_AGENT: &str = concat!("libfec/", env!("CARGO_PKG_VERSION"));

pub fn api_request(url: &Url) -> anyhow::Result<ApiResponse> {
    let mut response = match ureq::get(url.as_str())
        .header("accept", "application/json")
        .header("User-Agent", USER_AGENT)
        .call()
    {
        Ok(resp) => resp,
        Err(error) => match error {
            ureq::Error::StatusCode(429) => {
                return Err(anyhow::anyhow!(
                    "FEC API limit reached for {}",
                    redact_api_key(url)
                ));
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "FEC API request to {} failed: {}",
                    redact_api_key(url),
                    error
                ));
            }
        },
    };

    let headers = response.headers();

    let rate_limit = FecApiRateLimit {
        limit: headers
            .get("X-RateLimit-Limit")
            .ok_or_else(|| anyhow::anyhow!("missing X-RateLimit-Limit header"))?
            .to_str()?
            .parse::<usize>()?,
        remaining: headers
            .get("X-RateLimit-Remaining")
            .ok_or_else(|| anyhow::anyhow!("missing X-RateLimit-Remaining header"))?
            .to_str()?
            .parse::<usize>()?,
    };
    let body: serde_json::Value = response.body_mut().read_json()?;

    let pagination: FecApiPaginationObject = serde_json::from_value(
        body.get("pagination")
            .ok_or_else(|| anyhow::anyhow!("missing pagination field"))?
            .clone(),
    )?;

    let result_items = body["results"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("missing results array"))?
        .to_vec();
    let next_url = if pagination.page >= pagination.pages {
        None
    } else {
        let mut next_url = url.clone();
        next_url.query_pairs_mut().clear();
        for (key, value) in url.query_pairs() {
            if key != "page" {
                next_url.query_pairs_mut().append_pair(&key, &value);
            }
        }
        next_url
            .query_pairs_mut()
            .append_pair("page", &(pagination.page + 1).to_string());
        Some(next_url)
    };

    Ok(ApiResponse {
        json: body,
        rate_limit,
        result_items,
        pagination,
        next_url,
        cache_hit: false,
    })
}

/// Parse the max-age value from a Cache-Control header.
/// Returns None if the header is missing or doesn't contain max-age.
fn parse_cache_control_max_age(header_value: &str) -> Option<u64> {
    for directive in header_value.split(',') {
        let directive = directive.trim();
        if let Some(value) = directive.strip_prefix("max-age=") {
            if let Ok(secs) = value.trim().parse::<u64>() {
                return Some(secs);
            }
        }
    }
    None
}

/// Make an API request with optional caching support.
///
/// If a cache is provided, it will:
/// 1. Check for a valid cached response first
/// 2. On cache hit, return the cached response (with placeholder rate limits)
/// 3. On cache miss, make the request and store the response if max-age > 0
pub fn api_request_cached(
    url: &Url,
    cache: Option<&mut dyn ApiCache>,
) -> anyhow::Result<ApiResponse> {
    // Check cache first
    if let Some(ref cache) = cache {
        if let Some(entry) = cache.get(url) {
            // Cache hit - reconstruct ApiResponse from cached body
            let pagination: FecApiPaginationObject = serde_json::from_value(
                entry
                    .body
                    .get("pagination")
                    .ok_or_else(|| anyhow::anyhow!("missing pagination field in cached response"))?
                    .clone(),
            )?;

            let result_items = entry.body["results"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("missing results array in cached response"))?
                .to_vec();

            let next_url = if pagination.page >= pagination.pages {
                None
            } else {
                let mut next_url = url.clone();
                next_url.query_pairs_mut().clear();
                for (key, value) in url.query_pairs() {
                    if key != "page" {
                        next_url.query_pairs_mut().append_pair(&key, &value);
                    }
                }
                next_url
                    .query_pairs_mut()
                    .append_pair("page", &(pagination.page + 1).to_string());
                Some(next_url)
            };

            return Ok(ApiResponse {
                json: entry.body,
                // Placeholder values for cached responses - we don't have the real limits
                rate_limit: FecApiRateLimit {
                    limit: 0,
                    remaining: 0,
                },
                result_items,
                pagination,
                next_url,
                cache_hit: true,
            });
        }
    }

    // Cache miss - make the actual request
    let mut response = match ureq::get(url.as_str())
        .header("accept", "application/json")
        .header("User-Agent", USER_AGENT)
        .call()
    {
        Ok(resp) => resp,
        Err(error) => match error {
            ureq::Error::StatusCode(429) => {
                return Err(anyhow::anyhow!(
                    "FEC API limit reached for {}",
                    redact_api_key(url)
                ));
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "FEC API request to {} failed: {}",
                    redact_api_key(url),
                    error
                ));
            }
        },
    };

    let headers = response.headers();

    let rate_limit = FecApiRateLimit {
        limit: headers
            .get("X-RateLimit-Limit")
            .ok_or_else(|| anyhow::anyhow!("missing X-RateLimit-Limit header"))?
            .to_str()?
            .parse::<usize>()?,
        remaining: headers
            .get("X-RateLimit-Remaining")
            .ok_or_else(|| anyhow::anyhow!("missing X-RateLimit-Remaining header"))?
            .to_str()?
            .parse::<usize>()?,
    };

    // Parse Cache-Control header for max-age
    let max_age_secs = headers
        .get("Cache-Control")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_cache_control_max_age)
        .unwrap_or(0);

    let body: serde_json::Value = response.body_mut().read_json()?;

    // Store in cache if max-age > 0
    if let Some(cache) = cache {
        if max_age_secs > 0 {
            let entry = ApiCacheEntry {
                body: body.clone(),
                max_age_secs,
            };
            // Ignore cache write errors - caching is best-effort
            let _ = cache.set(url, &entry);
        }
    }

    let pagination: FecApiPaginationObject = serde_json::from_value(
        body.get("pagination")
            .ok_or_else(|| anyhow::anyhow!("missing pagination field"))?
            .clone(),
    )?;

    let result_items = body["results"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("missing results array"))?
        .to_vec();

    let next_url = if pagination.page >= pagination.pages {
        None
    } else {
        let mut next_url = url.clone();
        next_url.query_pairs_mut().clear();
        for (key, value) in url.query_pairs() {
            if key != "page" {
                next_url.query_pairs_mut().append_pair(&key, &value);
            }
        }
        next_url
            .query_pairs_mut()
            .append_pair("page", &(pagination.page + 1).to_string());
        Some(next_url)
    };

    Ok(ApiResponse {
        json: body,
        rate_limit,
        result_items,
        pagination,
        next_url,
        cache_hit: false,
    })
}

#[derive(Deserialize, Debug)]
pub struct FecApiPaginationObject {
    pub count: usize,
    pub is_count_exact: bool,
    pub page: usize,
    pub pages: usize,
    pub per_page: usize,
}
