use crate::cli::FilingsArgs;
use fec_api::{Api, FilingArgs};
use jiff::civil::Date;
use std::error::Error;

pub fn cmd_filings(flags: FilingsArgs) -> Result<(), Box<dyn Error>> {
    let client = Api::new("DEMO_KEY");
    let url = client.filings_url(FilingArgs {
        committees: flags.api.committee.unwrap_or_default(),
        form_types: flags.api.form_type,
    })?;
    if flags.print_url {
        println!("{}", url.0);
        if flags.url_only {
            return Ok(());
        }
    }
    let mut results = client.filings(url)?;

    // post-filter coverage dates, if provided
    if let Some(coverage_before) = flags.api.coverage_before {
        results.retain(|item| {
            let coverage_start_date: Date = item.value["coverage_start_date"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            coverage_start_date <= coverage_before
        });
    }
    if let Some(coverage_after) = flags.api.coverage_after {
        results.retain(|item| {
            let coverage_end_date: Date = item.value["coverage_end_date"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            coverage_end_date >= coverage_after
        });
    }

    if flags.ids_only {
        let ids: Vec<String> = results.iter().map(|item| item.filing_id.clone()).collect();
        if flags.json {
            println!("{}", serde_json::to_string(&ids)?);
        } else {
            println!("{}", ids.join(","));
        }
        return Ok(());
    }
    if flags.filing_urls_only {
        let urls: Vec<String> = results
            .iter()
            .map(|item| {
                format!(
                    "https://docquery.fec.gov/dcdev/posted/{}.fec",
                    item.filing_id
                        .strip_prefix("FEC-")
                        .unwrap_or(&item.filing_id)
                )
            })
            .collect();
        if flags.json {
            println!("{}", serde_json::to_string(&urls)?);
        } else {
            println!("{}", urls.join("\n"));
        }
        return Ok(());
    }
    if flags.json {
        println!(
            "{}",
            serde_json::to_string(
                &results
                    .iter()
                    .map(|v| v.value.clone())
                    .collect::<Vec<serde_json::Value>>()
            )?
        );
        return Ok(());
    }

    for item in results {
        println!("{:?}", item.filing_id);
    }

    Ok(())
}
