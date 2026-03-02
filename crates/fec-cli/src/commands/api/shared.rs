use crate::cli::ApiFormat;
use fec_api::{api_request_cached, ApiCache, ApiResponse};
use std::io::{self, Write};
use url::Url;

/// Redact api_key from a URL for display.
pub fn redact_url(url: &Url) -> String {
    let mut redacted = url.clone();
    let pairs: Vec<(String, String)> = redacted.query_pairs().into_owned().collect();
    redacted.query_pairs_mut().clear();
    for (k, v) in &pairs {
        if k == "api_key" {
            redacted.query_pairs_mut().append_pair(k, "DEMO_KEY");
        } else {
            redacted.query_pairs_mut().append_pair(k, v);
        }
    }
    redacted.to_string()
}

/// Print URL with api_key redacted, for --url-only output.
pub fn print_url(url: &Url) {
    println!("{}", redact_url(url));
}

/// Fetch results from an API URL, handling pagination.
/// Outputs results as JSON array or JSONL depending on format.
pub fn fetch_and_output(
    url: Url,
    cache: Option<&mut dyn ApiCache>,
    format: ApiFormat,
    fetch_all: bool,
    offline: bool,
) -> anyhow::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let mut all_results: Vec<serde_json::Value> = Vec::new();
    let first_response = api_request_cached(&url, cache, offline)?;

    match format {
        ApiFormat::Jsonl => {
            output_jsonl_page(&mut out, &first_response)?;
        }
        ApiFormat::Json => {
            all_results.extend(first_response.result_items.clone());
        }
    }

    if fetch_all && !offline {
        let mut next_url = first_response.next_url.clone();
        while let Some(url) = next_url {
            let response = fec_api::api_request(&url)?;

            match format {
                ApiFormat::Jsonl => {
                    output_jsonl_page(&mut out, &response)?;
                }
                ApiFormat::Json => {
                    all_results.extend(response.result_items.clone());
                }
            }

            eprintln!(
                "page {}/{} — {} results so far, {} API calls remaining",
                response.pagination.page,
                response.pagination.pages,
                match format {
                    ApiFormat::Json => all_results.len(),
                    ApiFormat::Jsonl => 0,
                },
                response.rate_limit.remaining
            );

            next_url = response.next_url;
        }
    }

    if matches!(format, ApiFormat::Json) {
        serde_json::to_writer_pretty(&mut out, &all_results)?;
        writeln!(out)?;
    }

    Ok(())
}

fn output_jsonl_page(out: &mut impl Write, response: &ApiResponse) -> anyhow::Result<()> {
    for item in &response.result_items {
        serde_json::to_writer(&mut *out, item)?;
        writeln!(out)?;
    }
    Ok(())
}

pub fn push_opt<'a>(params: &mut Vec<(&'a str, String)>, key: &'a str, value: &Option<String>) {
    if let Some(v) = value {
        params.push((key, v.clone()));
    }
}

pub fn push_opt_f64<'a>(params: &mut Vec<(&'a str, String)>, key: &'a str, value: &Option<f64>) {
    if let Some(v) = value {
        params.push((key, v.to_string()));
    }
}

pub fn push_opt_bool<'a>(params: &mut Vec<(&'a str, String)>, key: &'a str, value: &Option<bool>) {
    if let Some(v) = value {
        params.push((key, v.to_string()));
    }
}

pub fn push_vec<'a>(
    params: &mut Vec<(&'a str, String)>,
    key: &'a str,
    values: &Option<Vec<String>>,
) {
    if let Some(vs) = values {
        for v in vs {
            params.push((key, v.clone()));
        }
    }
}

pub fn push_vec_u16<'a>(
    params: &mut Vec<(&'a str, String)>,
    key: &'a str,
    values: &Option<Vec<u16>>,
) {
    if let Some(vs) = values {
        for v in vs {
            params.push((key, v.to_string()));
        }
    }
}
