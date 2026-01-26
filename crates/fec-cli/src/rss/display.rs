use std::fmt;

use jiff::Zoned;

use super::types::Item;

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
        format!(
            "{minutes} minute{} ago",
            if minutes == 1 { "" } else { "s" }
        )
    } else {
        format!(
            "{seconds} second{} ago",
            if seconds == 1 { "" } else { "s" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
