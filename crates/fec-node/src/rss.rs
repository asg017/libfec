//! The `fec-rss` crate's feed parser and URL builder. No HTTP here:
//! `js/rss.ts` fetches with the runtime's own `fetch()`.

use jiff::civil::Date;
use napi_derive::napi;

use crate::errors::{parse_error, Result};

/// One item of the feed (`fec_rss::Item`, camelCased by napi-rs).
#[napi(object, use_nullable = true, object_from_js = false)]
pub struct NativeRssItem {
    pub title: String,
    pub link: String,
    pub description: String,
    /// `pubDate` as milliseconds since the Unix epoch.
    pub pub_date: Option<f64>,
    pub guid: String,
    pub committee_id: Option<String>,
    pub filing_id: Option<String>,
    pub form_type: Option<String>,
    /// `"YYYY-MM-DD"`.
    pub coverage_from: Option<String>,
    /// `"YYYY-MM-DD"`.
    pub coverage_through: Option<String>,
    pub report_type: Option<String>,
    pub committee_name: String,
}

/// A parsed feed (`fec_rss::Feed`).
#[napi(object, object_from_js = false)]
pub struct NativeRssFeed {
    pub title: String,
    pub link: String,
    pub description: String,
    pub items: Vec<NativeRssItem>,
}

/// `MM/DD/YYYY` (the feed's coverage dates) as `YYYY-MM-DD`; `None` if invalid.
fn iso_date(s: &Option<String>) -> Option<String> {
    let mut parts = s.as_deref()?.splitn(3, '/');
    let month = parts.next()?.trim().parse().ok()?;
    let day = parts.next()?.trim().parse().ok()?;
    let year = parts.next()?.trim().parse().ok()?;
    Date::new(year, month, day).ok().map(|d| d.to_string())
}

impl From<fec_rss::Item> for NativeRssItem {
    fn from(item: fec_rss::Item) -> Self {
        NativeRssItem {
            committee_name: item.extract_committee_name().to_owned(),
            pub_date: item.pub_date.map(|t| t.as_millisecond() as f64),
            coverage_from: iso_date(&item.coverage_from),
            coverage_through: iso_date(&item.coverage_through),
            title: item.title,
            link: item.link,
            description: item.description,
            guid: item.guid,
            committee_id: item.committee_id,
            filing_id: item.filing_id,
            form_type: item.form_type,
            report_type: item.report_type,
        }
    }
}

fn parse(xml: &str) -> Result<NativeRssFeed> {
    let feed = fec_rss::parse_feed(xml).map_err(|e| parse_error(&e))?;
    Ok(NativeRssFeed {
        title: feed.title,
        link: feed.link,
        description: feed.description,
        items: feed.items.into_iter().map(Into::into).collect(),
    })
}

#[napi]
pub fn parse_rss_text(xml: String) -> Result<NativeRssFeed> {
    parse(&xml)
}

#[napi]
pub fn parse_rss_bytes(xml: &[u8]) -> Result<NativeRssFeed> {
    parse(std::str::from_utf8(xml).map_err(|e| parse_error(&e))?)
}

/// The feed URL (`fec_rss::FeedQuery::url`). `preset` is validated in JS;
/// each filter is comma-separated.
#[napi]
pub fn rss_feed_url(
    preset: String,
    committees: Option<String>,
    forms: Option<String>,
    states: Option<String>,
    parties: Option<String>,
) -> Result<String> {
    use fec_rss::Preset;
    let preset = match preset.as_str() {
        "all" => Preset::All,
        "monthly" => Preset::Monthly,
        "quarterly" => Preset::Quarterly,
        "presidential" => Preset::Presidential,
        "congressional" => Preset::Congressional,
        "pac" => Preset::Pac,
        other => return Err(parse_error(&format!("unknown RSS preset {other:?}"))),
    };
    Ok(fec_rss::FeedQuery {
        preset,
        committees,
        forms,
        states,
        parties,
    }
    .url())
}
