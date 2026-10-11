//! The `fec-rss` feed parser and URL builder, as in `crates/fec-node/src/rss.rs`.

use jiff::civil::Date;
use wasm_bindgen::prelude::*;

use crate::js::{opt_str, parse_error, Obj};

/// `MM/DD/YYYY` (the feed's coverage dates) as `YYYY-MM-DD`; `None` if invalid.
fn iso_date(s: &Option<String>) -> Option<String> {
    let mut parts = s.as_deref()?.splitn(3, '/');
    let month = parts.next()?.trim().parse().ok()?;
    let day = parts.next()?.trim().parse().ok()?;
    let year = parts.next()?.trim().parse().ok()?;
    Date::new(year, month, day).ok().map(|d| d.to_string())
}

fn item(item: fec_rss::Item) -> JsValue {
    Obj::new()
        .set("title", item.title.as_str())
        .set("link", item.link.as_str())
        .set("description", item.description.as_str())
        .set(
            "pubDate",
            item.pub_date.map_or(JsValue::NULL, |t| {
                JsValue::from_f64(t.as_millisecond() as f64)
            }),
        )
        .set("guid", item.guid.as_str())
        .set("committeeId", opt_str(item.committee_id.as_deref()))
        .set("filingId", opt_str(item.filing_id.as_deref()))
        .set("formType", opt_str(item.form_type.as_deref()))
        .set(
            "coverageFrom",
            opt_str(iso_date(&item.coverage_from).as_deref()),
        )
        .set(
            "coverageThrough",
            opt_str(iso_date(&item.coverage_through).as_deref()),
        )
        .set("reportType", opt_str(item.report_type.as_deref()))
        .set("committeeName", item.extract_committee_name())
        .build()
}

fn parse(xml: &str) -> Result<JsValue, JsValue> {
    let feed = fec_rss::parse_feed(xml).map_err(|e| parse_error(&e))?;
    let items: js_sys::Array = feed.items.into_iter().map(item).collect();
    Ok(Obj::new()
        .set("title", feed.title)
        .set("link", feed.link)
        .set("description", feed.description)
        .set("items", items)
        .build())
}

#[wasm_bindgen(js_name = parseRssText, unchecked_return_type = "NativeRssFeed")]
pub fn parse_rss_text(xml: String) -> Result<JsValue, JsValue> {
    parse(&xml)
}

#[wasm_bindgen(js_name = parseRssBytes, unchecked_return_type = "NativeRssFeed")]
pub fn parse_rss_bytes(xml: &[u8]) -> Result<JsValue, JsValue> {
    parse(std::str::from_utf8(xml).map_err(|e| parse_error(&e))?)
}

/// The feed URL (`fec_rss::FeedQuery::url`). `preset` is validated in JS;
/// each filter is comma-separated.
#[wasm_bindgen(js_name = rssFeedUrl)]
pub fn rss_feed_url(
    preset: String,
    committees: Option<String>,
    forms: Option<String>,
    states: Option<String>,
    parties: Option<String>,
) -> Result<String, JsValue> {
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
