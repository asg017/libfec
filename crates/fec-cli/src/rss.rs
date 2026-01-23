use std::fmt;

use jiff::{Timestamp, Zoned};

use crate::cli::{RssArgs, RssPreset};

pub const FEC_RSS_BASE_URL: &str = "https://efilingapps.fec.gov/rss/generate";

#[derive(Debug, Clone)]
pub struct Feed {
    pub title: String,
    pub link: String,
    pub description: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub title: String,
    pub link: String,
    pub description: String,
    pub pub_date: Option<Timestamp>,
    pub guid: String,
    pub committee_id: Option<String>,
    pub filing_id: Option<String>,
    pub form_type: Option<String>,
    pub coverage_from: Option<String>,
    pub coverage_through: Option<String>,
    pub report_type: Option<String>,
}

#[derive(Debug)]
pub enum Error {
    Http(String),
    Parse(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Http(msg) => write!(f, "HTTP error: {msg}"),
            Error::Parse(msg) => write!(f, "Parse error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

/// Result of fetching the RSS feed, including metadata
#[derive(Debug, Clone)]
pub struct FetchResult {
    pub feed: Feed,
    pub last_modified: Option<Timestamp>,
}

impl Item {
    pub fn time_ago(&self, now: &Zoned) -> Option<String> {
        let ts = self.pub_date?;
        let duration = now.timestamp().duration_since(ts);
        let total_seconds = duration.as_secs();

        if duration.is_negative() {
            return Some("in the future".to_string());
        }

        let minutes = total_seconds / 60;
        let hours = minutes / 60;
        let days = hours / 24;

        Some(if days > 0 {
            format!("{days} day{} ago", if days == 1 { "" } else { "s" })
        } else if hours > 0 {
            format!("{hours} hour{} ago", if hours == 1 { "" } else { "s" })
        } else if minutes > 0 {
            format!("{minutes} minute{} ago", if minutes == 1 { "" } else { "s" })
        } else {
            "just now".to_string()
        })
    }

    pub fn extract_committee_name(&self) -> &str {
        self.title
            .strip_prefix("New filing by ")
            .unwrap_or(&self.title)
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.extract_committee_name())?;
        if let Some(form) = &self.form_type {
            write!(f, " [{form}]")?;
        }
        if let Some(report) = &self.report_type {
            if !report.is_empty() {
                write!(f, " - {report}")?;
            }
        }
        Ok(())
    }
}

/// Description of active filters for display in TUI
#[derive(Debug, Clone, Default)]
pub struct ActiveFilters {
    pub preset: Option<String>,
    pub form_type: Option<String>,
    pub committee: Option<String>,
    pub state: Option<String>,
    pub party: Option<String>,
}

impl ActiveFilters {
    /// Returns a list of active filter descriptions for display
    pub fn to_display_strings(&self) -> Vec<String> {
        let mut filters = Vec::new();
        if let Some(ref p) = self.preset {
            if p != "All" {
                filters.push(format!("Preset: {}", p));
            }
        }
        if let Some(ref f) = self.form_type {
            filters.push(format!("Form: {}", f));
        }
        if let Some(ref c) = self.committee {
            filters.push(format!("Committee: {}", c));
        }
        if let Some(ref s) = self.state {
            filters.push(format!("State: {}", s));
        }
        if let Some(ref p) = self.party {
            filters.push(format!("Party: {}", p));
        }
        filters
    }

    pub fn is_empty(&self) -> bool {
        self.to_display_strings().is_empty()
    }
}

/// Build RSS feed URL with filters from args
pub fn build_feed_url(args: &RssArgs) -> (String, ActiveFilters) {
    let mut filters = ActiveFilters::default();

    // If committee filter is set, use custom URL format
    // Otherwise use predefined filter URL
    let has_custom_filters = args.form_type.is_some()
        || args.committee.is_some()
        || args.state.is_some()
        || args.party.is_some();

    let mut url = String::from(FEC_RSS_BASE_URL);

    if has_custom_filters {
        // Use custom filter format
        url.push('?');
        let mut params = Vec::new();

        if let Some(ref committee) = args.committee {
            params.push(format!("cids={}", committee));
            filters.committee = Some(committee.clone());
        }

        if let Some(ref form_type) = args.form_type {
            params.push(format!("forms={}", form_type.to_uppercase()));
            filters.form_type = Some(form_type.to_uppercase());
        }

        if let Some(ref state) = args.state {
            params.push(format!("states={}", state.to_uppercase()));
            filters.state = Some(state.to_uppercase());
        }

        if let Some(ref party) = args.party {
            params.push(format!("parties={}", party.to_uppercase()));
            filters.party = Some(party.to_uppercase());
        }

        url.push_str(&params.join("&"));
    } else {
        // Use predefined filter
        let preset_code = match args.preset {
            RssPreset::All => "ALL",
            RssPreset::Monthly => "M",
            RssPreset::Quarterly => "Q",
            RssPreset::Presidential => "F3P",
            RssPreset::Congressional => "F3",
            RssPreset::Pac => "F3X",
        };
        url.push_str(&format!("?preDefinedFilingType={}", preset_code));

        filters.preset = Some(match args.preset {
            RssPreset::All => "All".to_string(),
            RssPreset::Monthly => "Monthly".to_string(),
            RssPreset::Quarterly => "Quarterly".to_string(),
            RssPreset::Presidential => "Presidential (F3P)".to_string(),
            RssPreset::Congressional => "Congressional (F3)".to_string(),
            RssPreset::Pac => "PAC/Party (F3X)".to_string(),
        });
    }

    (url, filters)
}

/// Fetch feed with filters from args
pub fn fetch_feed_with_args(args: &RssArgs) -> Result<(FetchResult, ActiveFilters), Error> {
    let (url, filters) = build_feed_url(args);
    let result = fetch_feed_from_url(&url)?;
    Ok((result, filters))
}

pub fn fetch_feed_from_url(url: &str) -> Result<FetchResult, Error> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| Error::Http(e.to_string()))?;

    // Extract Last-Modified header
    let last_modified = response
        .headers()
        .get("Last-Modified")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| parse_rfc2822(s));

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

fn parse_rfc2822(s: &str) -> Option<Timestamp> {
    // Format: "Fri, 23 Jan 2026 17:46:22 GMT"
    jiff::fmt::rfc2822::parse(s)
        .ok()
        .map(|z: Zoned| z.timestamp())
}

/// Format seconds as "X:XX" minutes:seconds display
pub fn format_countdown(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{mins}:{secs:02}")
}

/// Format a duration in seconds as a human-readable string
pub fn format_duration_ago(seconds: i64) -> String {
    if seconds < 0 {
        return "in the future".to_string();
    }
    let seconds = seconds as u64;
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;

    if days > 0 {
        format!("{days} day{} ago", if days == 1 { "" } else { "s" })
    } else if hours > 0 {
        format!("{hours} hour{} ago", if hours == 1 { "" } else { "s" })
    } else if minutes > 0 {
        format!("{minutes} minute{} ago", if minutes == 1 { "" } else { "s" })
    } else {
        format!("{seconds} second{} ago", if seconds == 1 { "" } else { "s" })
    }
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

    #[test]
    fn test_item_display() {
        let item = Item {
            title: "New filing by TEST COMMITTEE".to_string(),
            link: String::new(),
            description: String::new(),
            pub_date: None,
            guid: String::new(),
            committee_id: Some("C00123456".to_string()),
            filing_id: Some("1234567".to_string()),
            form_type: Some("F3XN".to_string()),
            coverage_from: None,
            coverage_through: None,
            report_type: Some("QUARTERLY".to_string()),
        };
        assert_eq!(format!("{item}"), "TEST COMMITTEE [F3XN] - QUARTERLY");
    }

    #[test]
    fn test_format_countdown() {
        assert_eq!(format_countdown(0), "0:00");
        assert_eq!(format_countdown(59), "0:59");
        assert_eq!(format_countdown(60), "1:00");
        assert_eq!(format_countdown(125), "2:05");
        assert_eq!(format_countdown(300), "5:00");
    }

    #[test]
    fn test_format_duration_ago() {
        assert_eq!(format_duration_ago(0), "0 seconds ago");
        assert_eq!(format_duration_ago(1), "1 second ago");
        assert_eq!(format_duration_ago(45), "45 seconds ago");
        assert_eq!(format_duration_ago(60), "1 minute ago");
        assert_eq!(format_duration_ago(120), "2 minutes ago");
        assert_eq!(format_duration_ago(3600), "1 hour ago");
        assert_eq!(format_duration_ago(86400), "1 day ago");
    }
}
