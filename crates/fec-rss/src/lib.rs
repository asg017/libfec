//! Parser for the FEC e-filing RSS feed (`https://efilingapps.fec.gov/rss/generate`).
//!
//! This crate does no I/O: fetch the XML however you like (ureq, `fetch()` in WASM,
//! `urllib` in Python), then hand the body to [`parse_feed`]. [`FeedQuery::url`]
//! builds the request URL for the feed's filter parameters.

mod parse;
mod query;

use std::fmt;

use jiff::{Timestamp, Zoned};

pub use parse::{parse_feed, parse_rfc2822};
pub use query::{FeedQuery, Preset, FEC_RSS_BASE_URL};

/// RSS feed structure
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Feed {
    pub title: String,
    pub link: String,
    pub description: String,
    pub items: Vec<Item>,
}

/// RSS feed item: one new filing.
///
/// The `committee_id` .. `report_type` fields come from the
/// `*********CommitteeId: C00313510 | FilingId: 1934790 | ...*********` trailer the
/// FEC appends to each item's description.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// Errors from [`parse_feed`].
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

impl Item {
    /// Calculate how long ago this item was published
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
            format!(
                "{minutes} minute{} ago",
                if minutes == 1 { "" } else { "s" }
            )
        } else {
            "just now".to_string()
        })
    }

    /// Extract committee name from title (strips "New filing by " prefix)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_display() {
        let item = Item {
            title: "New filing by TEST COMMITTEE".to_string(),
            committee_id: Some("C00123456".to_string()),
            filing_id: Some("1234567".to_string()),
            form_type: Some("F3XN".to_string()),
            report_type: Some("QUARTERLY".to_string()),
            ..Default::default()
        };
        assert_eq!(format!("{item}"), "TEST COMMITTEE [F3XN] - QUARTERLY");
    }
}
