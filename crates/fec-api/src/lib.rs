use std::str::FromStr;

use anyhow::{Context, Result};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

pub struct Api {
    api_key: String,
    base_url: String,
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
    pub form_types: Option<Vec<String>>,
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

pub struct FilingsUrl(pub Url);
pub struct ElectionsUrl(pub Url);

impl Api {
    pub fn new<'a, S: Into<&'a str>>(api_key: S) -> Self {
        Self {
            api_key: api_key.into().to_string(),
            base_url: "https://api.open.fec.gov".to_string(),
        }
    }

    pub fn search_candidates(&self, q: &str) -> Result<Vec<CandidateSearchItem>> {
        let mut url = Url::parse(&format!("{}/v1/candidates/search/", self.base_url))?;
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
        let x: serde_json::Value = response.body_mut().read_json().unwrap();

        let results = x
            .get("results")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| serde_json::from_value(v.clone()).unwrap())
            .collect();
        Ok(results)
    }

    pub fn elections_url(&self, args: ElectionsArgs) -> Result<ElectionsUrl> {
        let mut url = Url::parse(&format!("{}/v1/elections", self.base_url))?;
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
        Ok(ElectionsUrl(url))
    }

    pub fn filings_url(&self, args: FilingArgs) -> Result<FilingsUrl> {
        let mut url = Url::parse(&format!("{}/v1/filings", self.base_url))?;
        let mut qp = url.query_pairs_mut();
        qp.append_pair("api_key", &self.api_key);

        // TODO: parameter
        qp.append_pair("most_recent", "true");

        qp.append_pair("per_page", "100");

        for committee in &args.committees {
            qp.append_pair("committee_id", &committee);
        }
        if let Some(form_types) = &args.form_types {
            for form_type in form_types {
                qp.append_pair("form_type", &form_type);
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
        Ok(FilingsUrl(url))
    }

    pub fn filings(&self, url: FilingsUrl) -> Result<Vec<FilingItem>> {
        Ok(paginate_all_results(&url.0)?
            .iter()
            .filter_map(|v| match v.get("fec_file_id").and_then(|id| id.as_str()) {
                Some(id) => Some(FilingItem {
                    filing_id: id.to_string(),
                    value: v.clone(),
                }),
                None => None,
            })
            .collect::<Vec<FilingItem>>())
    }
    pub fn elections(&self, url: ElectionsUrl) -> Result<Vec<ElectionItem>> {
        let mut response = ureq::get(url.0.as_str())
            .header("accept", "application/json")
            .header(
                "User-Agent",
                format!("libfec/{}", env!("CARGO_PKG_VERSION")),
            )
            .call()
            .context("Elections URL hit error")?;
        let body: serde_json::Value = response.body_mut().read_json().unwrap();
        let results = body.get("results").unwrap().as_array().unwrap();
        Ok(results
            .iter()
            .map(|v| serde_json::from_value(v.clone()))
            .collect::<Result<Vec<ElectionItem>, _>>()
            .context("Error collecting election items")?)
    }
}

fn paginate_all_results(url: &Url) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut results = vec![];
    let mut current_url = url.clone();
    loop {
        let mut response = ureq::get(current_url.as_str())
            .header("accept", "application/json")
            .header(
                "User-Agent",
                format!("libfec/{}", env!("CARGO_PKG_VERSION")),
            )
            .call()
            .context("Error making request to FEC API endpoint")?;
        let body: serde_json::Value = response.body_mut().read_json().unwrap();
        let pagination: FecApiPaginationObject =
            serde_json::from_value(body.get("pagination").unwrap().clone())?;
        results.extend(body.get("results").unwrap().as_array().unwrap().to_owned());
        if pagination.page >= pagination.pages {
            break;
        }
        let mut next_url = current_url.clone();
        next_url.query_pairs_mut().clear();
        for (key, value) in current_url.query_pairs() {
            if key != "page" {
                next_url.query_pairs_mut().append_pair(&key, &value);
            }
        }
        next_url.query_pairs_mut().append_pair("page", &(pagination.page + 1).to_string());
        /*next_url
            .query_pairs_mut()
            .append_pair("page", &(pagination.page + 1).to_string());*/
        current_url = next_url;
    }

    Ok(results)
}

#[derive(Deserialize, Debug)]
struct FecApiPaginationObject {
    count: usize,
    is_count_exact: bool,
    page: usize,
    pages: usize,
    per_page: usize,
}
