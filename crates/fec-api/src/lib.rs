use std::{str::FromStr, sync::LazyLock};

use anyhow::{Context, Result};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

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

impl ToString for Office {
    fn to_string(&self) -> String {
        match self {
            Office::House => "house".to_string(),
            Office::Senate => "senate".to_string(),
            Office::President => "president".to_string(),
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
pub struct FilingsUrl(pub Url);
pub struct ElectionsUrl(pub Url);

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

    pub fn filings_url(&self, args: FilingArgs) -> FilingsUrl {
        let mut url = self.base_url.clone();
        url.set_path("/v1/filings");
        let mut qp = url.query_pairs_mut();
        qp.append_pair("api_key", &self.api_key);

        qp.append_pair("per_page", "100");
        // TODO: parameter
        qp.append_pair("most_recent", "true");
        qp.append_pair("filer_type", "e-file");

        for committee in &args.committees {
            qp.append_pair("committee_id", &committee);
        }
        for candidate in &args.candidates {
            qp.append_pair("candidate_id", &candidate);
        }
        if let Some(form_types) = &args.form_types {
            for form_type in form_types {
                qp.append_pair("form_type", &form_type);
            }
        }
        if let Some(report_types) = &args.report_types {
            for report_type in report_types {
                qp.append_pair("report_type", &report_type);
            }
        }
        if let Some(committee_types) = &args.committee_types {
            for committee_type in committee_types {
                qp.append_pair("committee_type", &committee_type);
            }
        }
        for year in &args.cycle {
            qp.append_pair("cycle", &year.to_string());
        }
        drop(qp);
        FilingsUrl(url)
    }

    pub fn efiling_filings_url(&self, args: EfilingFilingArgs) -> EfilingFilingUrl {
        let mut url = self.base_url.clone();
        url.set_path("/v1/efile/filings");
        let mut qp = url.query_pairs_mut();
        qp.append_pair("api_key", &self.api_key);

        for committee in &args.committees {
            qp.append_pair("committee_id", &committee);
        }
        if let Some(form_types) = &args.form_types {
            for form_type in form_types {
                qp.append_pair("form_type", &form_type);
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
}

static USER_AGENT: &str = concat!("libfec/", env!("CARGO_PKG_VERSION"));

pub fn api_request(url: &Url) -> anyhow::Result<ApiResponse> {
    let mut response = match ureq::get(url.as_str())
        .header("accept", "application/json")
        .header("User-Agent", USER_AGENT)
        .call() {
        Ok(resp) => resp,
          Err(error) => {
            match error {
              ureq::Error::StatusCode(429) => {
                return Err(anyhow::anyhow!("FEC API limit reached for {}", redact_api_key(&url)));
              }
              _ => {
                return Err(anyhow::anyhow!("FEC API request to {} failed: {}", redact_api_key(&url), error));
              }
            }
          }
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
        .iter()
        .map(|v| v.clone())
        .collect();
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
