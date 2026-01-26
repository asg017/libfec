use jiff::{Timestamp, Zoned};

use crate::cli::RssArgs;

use super::filters::build_feed_url;
use super::types::{ActiveFilters, Error, Feed, FetchResult, Item};

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

    let feed = parse_feed(&body)?;

    Ok(FetchResult {
        feed,
        last_modified,
    })
}

/// Parse RSS XML into Feed structure
pub fn parse_feed(xml: &str) -> Result<Feed, Error> {
    let mut feed = Feed {
        title: String::new(),
        link: String::new(),
        description: String::new(),
        items: Vec::new(),
    };

    let mut current_item: Option<Item> = None;
    let mut _current_element = String::new();
    let mut current_text = String::new();
    let mut in_channel = false;
    let mut in_item = false;

    for token in xmlparser::Tokenizer::from(xml) {
        let token = token.map_err(|e| Error::Parse(e.to_string()))?;

        match token {
            xmlparser::Token::ElementStart { local, .. } => {
                _current_element = local.as_str().to_string();
                current_text.clear();

                match local.as_str() {
                    "channel" => in_channel = true,
                    "item" => {
                        in_item = true;
                        current_item = Some(Item {
                            title: String::new(),
                            link: String::new(),
                            description: String::new(),
                            pub_date: None,
                            guid: String::new(),
                            committee_id: None,
                            filing_id: None,
                            form_type: None,
                            coverage_from: None,
                            coverage_through: None,
                            report_type: None,
                        });
                    }
                    _ => {}
                }
            }
            xmlparser::Token::Text { text } => {
                current_text.push_str(text.as_str());
            }
            xmlparser::Token::ElementEnd {
                end: xmlparser::ElementEnd::Close(_, local),
                ..
            } => {
                let tag = local.as_str();
                let text = current_text.trim().to_string();

                if in_item {
                    if let Some(ref mut item) = current_item {
                        match tag {
                            "title" => item.title = text,
                            "link" => item.link = text,
                            "description" => {
                                item.description = text.clone();
                                parse_description_metadata(item, &text);
                            }
                            "pubDate" => {
                                item.pub_date = parse_rfc2822(&text);
                            }
                            "guid" => item.guid = text,
                            "item" => {
                                if let Some(item) = current_item.take() {
                                    feed.items.push(item);
                                }
                                in_item = false;
                            }
                            _ => {}
                        }
                    }
                } else if in_channel {
                    match tag {
                        "title" => feed.title = text,
                        "link" => feed.link = text,
                        "description" => feed.description = text,
                        "channel" => in_channel = false,
                        _ => {}
                    }
                }
                current_text.clear();
            }
            _ => {}
        }
    }

    Ok(feed)
}

/// Parse metadata from RSS item description field
fn parse_description_metadata(item: &mut Item, desc: &str) {
    // Format: *********CommitteeId: C00313510 | FilingId: 1934790 | FormType: F99 | ...
    if let Some(start) = desc.find("*********") {
        let metadata = &desc[start + 9..];
        let metadata = metadata.trim_end_matches('*');

        for part in metadata.split(" | ") {
            let mut kv = part.splitn(2, ": ");
            if let (Some(key), Some(value)) = (kv.next(), kv.next()) {
                let value = value.trim();
                if value.is_empty() {
                    continue;
                }
                match key.trim() {
                    "CommitteeId" => item.committee_id = Some(value.to_string()),
                    "FilingId" => item.filing_id = Some(value.to_string()),
                    "FormType" => item.form_type = Some(value.to_string()),
                    "CoverageFrom" => item.coverage_from = Some(value.to_string()),
                    "CoverageThrough" => item.coverage_through = Some(value.to_string()),
                    "ReportType" => item.report_type = Some(value.to_string()),
                    _ => {}
                }
            }
        }
    }
}

/// Parse RFC2822 date format
fn parse_rfc2822(s: &str) -> Option<Timestamp> {
    // Format: "Fri, 23 Jan 2026 17:46:22 GMT"
    jiff::fmt::rfc2822::parse(s)
        .ok()
        .map(|z: Zoned| z.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss xmlns:dc="http://purl.org/dc/elements/1.1/" version="2.0">
  <channel>
    <title>FEC Electronic Filing RSS Feed - ALL</title>
    <link>http://docquery.fec.gov//rss</link>
    <description>New filings within the last 7 days</description>
    <item>
      <title>New filing by TEST COMMITTEE</title>
      <link>http://docquery.fec.gov/dcdev/posted/1234567.fec</link>
      <description>&lt;p&gt;The TEST COMMITTEE successfully filed their F3XN QUATERLY YEAR-END&lt;/p&gt;*********CommitteeId: C00123456 | FilingId: 1234567 | FormType: F3XN | CoverageFrom: 07/01/2025 | CoverageThrough: 12/31/2025 | ReportType: QUATERLY YEAR-END*********</description>
      <pubDate>Fri, 23 Jan 2026 17:46:22 GMT</pubDate>
      <guid>http://docquery.fec.gov/dcdev/posted/1234567.fec</guid>
    </item>
  </channel>
</rss>"#;

    #[test]
    fn test_parse_feed() {
        let feed = parse_feed(SAMPLE_RSS).unwrap();
        assert_eq!(feed.title, "FEC Electronic Filing RSS Feed - ALL");
        assert_eq!(feed.items.len(), 1);

        let item = &feed.items[0];
        assert_eq!(item.title, "New filing by TEST COMMITTEE");
        assert_eq!(item.committee_id, Some("C00123456".to_string()));
        assert_eq!(item.filing_id, Some("1234567".to_string()));
        assert_eq!(item.form_type, Some("F3XN".to_string()));
        assert_eq!(item.report_type, Some("QUATERLY YEAR-END".to_string()));
        assert!(item.pub_date.is_some());
    }
}
