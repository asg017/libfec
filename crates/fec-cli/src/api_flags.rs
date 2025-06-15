use std::str::FromStr;

use anyhow::Context;
use clap::{Parser};
use fec_api::{Api, ElectionsArgs, FilingArgs, Office};
use jiff::civil::Date;

#[derive(Parser, Debug, Clone)]
#[command(next_help_heading = "FEC API options")]
pub struct FilingsApiFlags {
    #[arg(long, help = "Committee ID to filter filings by")]
    pub committee: Option<Vec<String>>,

    #[arg(long, help = "Committee ID to filter filings by")]
    pub candidate: Option<Vec<String>>,

    #[arg(long, help = "Form type to filter filings by")]
    pub form_type: Option<Vec<String>>,

    // coverage-before
    #[arg(long, help = "Only filings that cover dates before this date")]
    pub coverage_before: Option<Date>,

    // coverage-after
    #[arg(long, help = "Only filings that cover dates after this date", conflicts_with = "coverage_between")]
    pub coverage_after: Option<Date>,

    #[arg(long, help = "Only filings container coverage between these dates", num_args = 2)]
    pub coverage_between: Vec<Date>,

    #[arg(long)]
    pub election: Option<String>,
    #[arg(long)]
    pub state: Option<String>,
    #[arg(long)]
    pub district: Option<u16>,

    #[arg(long, requires = "election")]
    pub office: Option<String>,


}

impl FilingsApiFlags {
  pub(crate) fn any_provided(&self)->bool {
    self.candidate.is_some() || self.committee.is_some() || self.election.is_some()
  }
    pub(crate)  fn resolve_ids(&self) -> anyhow::Result<Vec<String>> {
        let client = Api::new("DEMO_KEY");
      
      let mut results  = if let Some(election) = &self.election {
          let url = client.elections_url(ElectionsArgs {
              cycle: election.parse::<u16>()?,

              // TODO required
              office: Office::from_str(self.office.as_deref().unwrap_or("H"))?,
              state: self.state.clone(),
              district: self.district,
          })?;
          dbg!(&url.0.to_string());

          let items = client.elections(url).context("Elections API Call")?;
          let committees = items.iter().map(|item| {
              item.committee_ids.clone()
          }).flatten().collect::<Vec<String>>();
          let url = client.filings_url(FilingArgs {
              committees: committees,
              form_types: self.form_type.clone(),
          })?;
          dbg!(&url.0.to_string());
          client.filings(url).context("Filings API Call from election committees")?
      }else {
        let url = client.filings_url(FilingArgs {
            committees: self.committee.clone().unwrap_or_default(),
            form_types: self.form_type.clone(),
        })?;
      
        client.filings(url)?
      };
      

      // post-filter coverage dates, if provided
      if let Some(coverage_before) = self.coverage_before {
          results.retain(|item| {
            item.value.get("coverage_start_date")
              .and_then(|date_value| date_value.as_str())
              .and_then(|date_str| date_str.parse::<Date>().ok())
              .map_or(false, |coverage_start_date| coverage_start_date <= coverage_before)
          });
      }
      if let Some(coverage_after) = self.coverage_after {
          results.retain(|item| {
          item.value.get("coverage_end_date")
            .and_then(|date_value| date_value.as_str())
            .and_then(|date_str| date_str.parse::<Date>().ok())
            .map_or(false, |coverage_end_date| coverage_end_date >= coverage_after)
          });
      }
      
      Ok(results.iter().map(|item| item.filing_id.clone()).collect())

    }
}
