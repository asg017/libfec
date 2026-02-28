use std::{fmt, str::FromStr, sync::LazyLock};

use anyhow::{Context, Result};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

// ---------------------------------------------------------------------------
// ID newtypes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidIdError(String);

impl fmt::Display for InvalidIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for InvalidIdError {}

/// A validated FEC committee ID (e.g. `C00401224`).
///
/// Committee IDs are 9 characters starting with 'C' followed by 8 digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommitteeId(String);

impl CommitteeId {
    pub fn new(s: &str) -> std::result::Result<Self, InvalidIdError> {
        if s.len() != 9 {
            return Err(InvalidIdError(format!(
                "committee ID must be 9 characters, got {} ('{}')",
                s.len(),
                s
            )));
        }
        if !s.starts_with('C') {
            return Err(InvalidIdError(format!(
                "committee ID must start with 'C', got '{}'",
                s
            )));
        }
        Ok(CommitteeId(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn fec_url(&self) -> String {
        format!("https://www.fec.gov/data/committee/{}/", self.0)
    }
}

impl fmt::Display for CommitteeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for CommitteeId {
    type Err = InvalidIdError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        CommitteeId::new(s)
    }
}

impl Serialize for CommitteeId {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CommitteeId {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        CommitteeId::new(&s).map_err(serde::de::Error::custom)
    }
}

/// A validated FEC candidate ID (e.g. `H0CA12345`, `S6CA00123`, `P80000722`).
///
/// Candidate IDs are 9 characters starting with H (House), S (Senate), or P (President).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CandidateId(String);

impl CandidateId {
    pub fn new(s: &str) -> std::result::Result<Self, InvalidIdError> {
        if s.len() != 9 {
            return Err(InvalidIdError(format!(
                "candidate ID must be 9 characters, got {} ('{}')",
                s.len(),
                s
            )));
        }
        match s.as_bytes()[0] {
            b'H' | b'S' | b'P' => {}
            _ => {
                return Err(InvalidIdError(format!(
                    "candidate ID must start with H, S, or P, got '{}'",
                    s
                )));
            }
        }
        Ok(CandidateId(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The office this candidate is running for, derived from the first character.
    pub fn office(&self) -> Office {
        match self.0.as_bytes()[0] {
            b'H' => Office::House,
            b'S' => Office::Senate,
            b'P' => Office::President,
            _ => unreachable!("validated in new()"),
        }
    }

    /// The two-letter state code for House/Senate candidates (chars 2-4), or None for President.
    pub fn state(&self) -> Option<&str> {
        match self.0.as_bytes()[0] {
            b'H' | b'S' => Some(&self.0[2..4]),
            _ => None,
        }
    }

    pub fn fec_url(&self) -> String {
        format!("https://www.fec.gov/data/candidate/{}/", self.0)
    }
}

impl fmt::Display for CandidateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for CandidateId {
    type Err = InvalidIdError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        CandidateId::new(s)
    }
}

impl Serialize for CandidateId {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CandidateId {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        CandidateId::new(&s).map_err(serde::de::Error::custom)
    }
}

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
    pub committees: Vec<CommitteeId>,
    pub candidates: Vec<CandidateId>,
    pub form_types: Option<Vec<String>>,
    pub report_types: Option<Vec<String>>,
    pub committee_types: Option<Vec<String>>,
    pub cycle: Vec<u16>,

    #[builder(default = "false")]
    pub include_amendments: bool,

    pub min_receipt_date: Option<String>,
    pub max_receipt_date: Option<String>,
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
    pub committees: Vec<CommitteeId>,
    pub form_types: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
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

/// A URL for any arbitrary API endpoint, built via `Api::endpoint_url()`.
#[derive(Debug, Clone)]
pub struct GenericApiUrl(pub Url);

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
            qp.append_pair("committee_id", committee.as_str());
        }
        for candidate in &args.candidates {
            qp.append_pair("candidate_id", candidate.as_str());
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
        if let Some(min_receipt_date) = &args.min_receipt_date {
            qp.append_pair("min_receipt_date", min_receipt_date);
        }
        if let Some(max_receipt_date) = &args.max_receipt_date {
            qp.append_pair("max_receipt_date", max_receipt_date);
        }
        qp.append_pair("sort", "committee_id");
        drop(qp);
        FilingsUrl(url)
    }

    /// Build a URL for any endpoint with arbitrary query parameters.
    pub fn endpoint_url(&self, path: &str, params: &[(&str, &str)]) -> GenericApiUrl {
        let mut url = self.base_url.clone();
        url.set_path(path);
        let mut qp = url.query_pairs_mut();
        qp.append_pair("api_key", &self.api_key);
        for (key, value) in params {
            qp.append_pair(key, value);
        }
        drop(qp);
        GenericApiUrl(url)
    }

    pub fn efiling_filings_url(&self, args: EfilingFilingArgs) -> EfilingFilingUrl {
        let mut url = self.base_url.clone();
        url.set_path("/v1/efile/filings");
        let mut qp = url.query_pairs_mut();
        qp.append_pair("api_key", &self.api_key);

        for committee in &args.committees {
            qp.append_pair("committee_id", committee.as_str());
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

/// Compute the next URL for pagination. Handles both page-based and cursor-based pagination.
///
/// Cursor-based (schedule A/B/E): `last_indexes` contains cursor values; no `page` field.
/// Page-based (filings, schedule D/F): uses `page`/`pages` fields.
/// Hybrid (schedule C): has both `page`+`last_index`.
fn compute_next_url(url: &Url, pagination: &FecApiPaginationObject) -> Option<Url> {
    // Cursor-based pagination: use last_indexes fields as query params.
    // If last_indexes is present with non-null values, there are more results.
    if let Some(ref last_indexes) = pagination.last_indexes {
        if let Some(obj) = last_indexes.as_object() {
            // Check that at least one cursor value is non-null (null values = no more pages)
            let has_cursor = obj.values().any(|v| !v.is_null());
            if has_cursor {
                let mut next_url = url.clone();
                next_url.query_pairs_mut().clear();
                for (key, value) in url.query_pairs() {
                    // Strip existing cursor params that will be replaced
                    if key.starts_with("last_") {
                        continue;
                    }
                    next_url.query_pairs_mut().append_pair(&key, &value);
                }
                for (key, value) in obj {
                    if let Some(s) = value.as_str() {
                        next_url.query_pairs_mut().append_pair(key, s);
                    } else if !value.is_null() {
                        next_url
                            .query_pairs_mut()
                            .append_pair(key, &value.to_string());
                    }
                }
                return Some(next_url);
            }
            // All cursor values are null — no more pages
            return None;
        }
    }

    // Page-based pagination
    if pagination.page >= pagination.pages {
        return None;
    }
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
}

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
    let next_url = compute_next_url(url, &pagination);

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

            let next_url = compute_next_url(url, &pagination);

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

    let next_url = compute_next_url(url, &pagination);

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
    #[serde(default)]
    pub count: usize,
    #[serde(default = "default_true")]
    pub is_count_exact: bool,
    /// Page number. Absent on cursor-based endpoints (schedule A/B/E).
    #[serde(default = "default_page")]
    pub page: usize,
    /// Total pages. May be present even on cursor-based endpoints.
    #[serde(default)]
    pub pages: usize,
    #[serde(default)]
    pub per_page: usize,
    /// For cursor-based pagination (schedule endpoints).
    /// Contains fields like `last_index`, `last_contribution_receipt_date`, etc.
    #[serde(default)]
    pub last_indexes: Option<serde_json::Value>,
}

fn default_true() -> bool {
    true
}

fn default_page() -> usize {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_committee_id() {
        let id = CommitteeId::new("C00401224").unwrap();
        assert_eq!(id.as_str(), "C00401224");
        assert_eq!(id.to_string(), "C00401224");
        assert_eq!(
            id.fec_url(),
            "https://www.fec.gov/data/committee/C00401224/"
        );
    }

    #[test]
    fn test_committee_id_from_str() {
        let id: CommitteeId = "C00401224".parse().unwrap();
        assert_eq!(id.as_str(), "C00401224");
    }

    #[test]
    fn test_invalid_committee_id_wrong_prefix() {
        let err = CommitteeId::new("H00401224").unwrap_err();
        assert_eq!(
            err.to_string(),
            "committee ID must start with 'C', got 'H00401224'"
        );
    }

    #[test]
    fn test_invalid_committee_id_wrong_length() {
        let err = CommitteeId::new("C004").unwrap_err();
        assert_eq!(
            err.to_string(),
            "committee ID must be 9 characters, got 4 ('C004')"
        );
    }

    #[test]
    fn test_valid_candidate_id_house() {
        let id = CandidateId::new("H0CA12345").unwrap();
        assert_eq!(id.as_str(), "H0CA12345");
        assert_eq!(id.office(), Office::House);
        assert_eq!(id.state(), Some("CA"));
        assert_eq!(
            id.fec_url(),
            "https://www.fec.gov/data/candidate/H0CA12345/"
        );
    }

    #[test]
    fn test_valid_candidate_id_senate() {
        let id = CandidateId::new("S6CA00123").unwrap();
        assert_eq!(id.office(), Office::Senate);
        assert_eq!(id.state(), Some("CA"));
    }

    #[test]
    fn test_valid_candidate_id_president() {
        let id = CandidateId::new("P80000722").unwrap();
        assert_eq!(id.office(), Office::President);
        assert_eq!(id.state(), None);
    }

    #[test]
    fn test_candidate_id_from_str() {
        let id: CandidateId = "P80000722".parse().unwrap();
        assert_eq!(id.as_str(), "P80000722");
    }

    #[test]
    fn test_invalid_candidate_id_wrong_prefix() {
        let err = CandidateId::new("C00401224").unwrap_err();
        assert_eq!(
            err.to_string(),
            "candidate ID must start with H, S, or P, got 'C00401224'"
        );
    }

    #[test]
    fn test_invalid_candidate_id_wrong_length() {
        let err = CandidateId::new("H0CA").unwrap_err();
        assert_eq!(
            err.to_string(),
            "candidate ID must be 9 characters, got 4 ('H0CA')"
        );
    }

    #[test]
    fn test_committee_id_serde_roundtrip() {
        let id = CommitteeId::new("C00401224").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"C00401224\"");
        let deserialized: CommitteeId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }

    #[test]
    fn test_candidate_id_serde_roundtrip() {
        let id = CandidateId::new("P80000722").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"P80000722\"");
        let deserialized: CandidateId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }
}
