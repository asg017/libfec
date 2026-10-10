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
