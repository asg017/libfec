use std::borrow::Cow;

use jiff::{Timestamp, Zoned};

use crate::{Feed, Item, ParseError};

/// Parse RSS XML into Feed structure
pub fn parse_feed(xml: &str) -> Result<Feed, ParseError> {
    let mut feed = Feed::default();

    let mut current_item: Option<Item> = None;
    let mut current_text = String::new();
    let mut in_channel = false;

    for token in xmlparser::Tokenizer::from(xml) {
        let token = token.map_err(|e| ParseError(e.to_string()))?;

        match token {
            xmlparser::Token::ElementStart { local, .. } => {
                current_text.clear();

                match local.as_str() {
                    "channel" => in_channel = true,
                    "item" => current_item = Some(Item::default()),
                    _ => {}
                }
            }
            xmlparser::Token::Text { text } => {
                current_text.push_str(&unescape(text.as_str()));
            }
            xmlparser::Token::Cdata { text, .. } => {
                current_text.push_str(text.as_str());
            }
            xmlparser::Token::ElementEnd {
                end: xmlparser::ElementEnd::Close(_, local),
                ..
            } => {
                let tag = local.as_str();
                let text = current_text.trim().to_string();

                if let Some(ref mut item) = current_item {
                    match tag {
                        "title" => item.title = text,
                        "link" => item.link = text,
                        "description" => {
                            parse_description_metadata(item, &text);
                            item.description = text;
                        }
                        "pubDate" => {
                            item.pub_date = parse_rfc2822(&text);
                        }
                        "guid" => item.guid = text,
                        "item" => {
                            if let Some(item) = current_item.take() {
                                feed.items.push(item);
                            }
                        }
                        _ => {}
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

/// Decode the five predefined XML entities and numeric character references.
/// Unknown or malformed references are kept verbatim.
fn unescape(s: &str) -> Cow<'_, str> {
    if !s.contains('&') {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest.find(';').and_then(|semi| {
            let ch = match &rest[1..semi] {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                name => name
                    .strip_prefix("#x")
                    .or_else(|| name.strip_prefix("#X"))
                    .map(|hex| u32::from_str_radix(hex, 16))
                    .or_else(|| name.strip_prefix('#').map(str::parse))
                    .and_then(Result::ok)
                    .and_then(char::from_u32),
            }?;
            Some((ch, semi + 1))
        });
        match decoded {
            Some((ch, len)) => {
                out.push(ch);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
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

/// Parse RFC2822 date format (the feed's `pubDate` and the `Last-Modified` header)
pub fn parse_rfc2822(s: &str) -> Option<Timestamp> {
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
        assert!(item.description.starts_with("<p>The TEST COMMITTEE"));
        assert!(item.pub_date.is_some());
    }

    #[test]
    fn test_unescape() {
        assert_eq!(unescape("plain"), "plain");
        assert_eq!(unescape("A &amp; B"), "A & B");
        assert_eq!(unescape("&lt;p&gt;&quot;x&apos;"), "<p>\"x'");
        assert_eq!(unescape("&#233;&#xE9;"), "éé");
        assert_eq!(unescape("AT&T &bogus; &"), "AT&T &bogus; &");
    }
}
