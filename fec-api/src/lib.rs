use anyhow::Result;
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

#[derive(Deserialize, Serialize, Debug)]
pub struct FilingArgs {
    pub committees: Vec<String>,
    pub form_types: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FilingItem {
    pub filing_id: String,
    pub value: Value,
}


pub struct FilingsUrl(pub Url);


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

    
    pub fn filings_url(&self, args: FilingArgs) -> Result<FilingsUrl> {
      let mut url = Url::parse(&format!("{}/v1/filings", self.base_url))?;
      let mut qp = url.query_pairs_mut();
      qp.append_pair("api_key", &self.api_key);
      
      // TODO: parameter
      qp.append_pair("most_recent", "true");

      for committee in &args.committees {
          qp.append_pair("committee_id", &committee);
      }
      if let Some(form_types) = &args.form_types {
          for form_type in form_types {
              qp.append_pair("form_type", &form_type);
          }
      }
      drop(qp);
      Ok(FilingsUrl(url))
    }

    pub fn filings(
        &self,
        url: FilingsUrl,
    ) -> Result<Vec<FilingItem>> {
        
        let mut response = ureq::get(url.0.as_str())
            .header("accept", "application/json")
            .header(
                "User-Agent",
                format!("libfec/{}", env!("CARGO_PKG_VERSION")),
            )
            .call()?;
        let x: serde_json::Value = response.body_mut().read_json().unwrap();
        let results = x.get("results").unwrap().as_array().unwrap();
        let ids = results
            .iter()
            .filter_map(|v| match v.get("fec_file_id").unwrap().as_str() {
                Some(id) => Some(FilingItem {
                    // TODO why some null?
                    filing_id: id.to_string(),
                    value: v.clone(),
                }),
                None => None,
            })
            .collect::<Vec<FilingItem>>();

        Ok(ids)
    }
}