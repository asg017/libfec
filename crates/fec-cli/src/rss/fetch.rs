use fec_rss::{parse_feed, parse_rfc2822};

use crate::cli::RssArgs;

use super::filters::build_feed_url;
use super::types::{ActiveFilters, Error, FetchResult};

/// Fetch feed with filters from args
pub fn fetch_feed_with_args(args: &RssArgs) -> Result<(FetchResult, ActiveFilters), Error> {
    let (url, filters) = build_feed_url(args);
    let result = fetch_feed_from_url(&url)?;
    Ok((result, filters))
}

/// Fetch RSS feed from URL
pub fn fetch_feed_from_url(url: &str) -> Result<FetchResult, Error> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| Error::Http(e.to_string()))?;

    // Extract Last-Modified header
    let last_modified = response
        .headers()
        .get("Last-Modified")
        .and_then(|h| h.to_str().ok())
        .and_then(parse_rfc2822);

    let body = response
        .into_body()
        .read_to_string()
        .map_err(|e| Error::Http(e.to_string()))?;

    let feed = parse_feed(&body).map_err(|e| Error::Parse(e.to_string()))?;

    Ok(FetchResult {
        feed,
        last_modified,
    })
}
