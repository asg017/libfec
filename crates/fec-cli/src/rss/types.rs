use std::fmt;

use jiff::Timestamp;

/// RSS feed structure
#[derive(Debug, Clone)]
pub struct Feed {
    pub title: String,
    pub link: String,
    pub description: String,
    pub items: Vec<Item>,
}

/// RSS feed item
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

/// RSS feed errors
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

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.to_display_strings().is_empty()
    }
}
