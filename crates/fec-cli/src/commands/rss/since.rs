use anyhow::{bail, Context, Result};
use jiff::{Span, Timestamp, Zoned};
use std::str::FromStr;

/// Parse a "since" argument which can be either:
/// - An absolute timestamp (ISO 8601)
/// - A relative span like "1 day ago", "2 hours ago"
pub fn parse_since(input: &str) -> Result<Timestamp> {
    let now = Zoned::now();

    // Try parsing as absolute timestamp first
    // Jiff's Timestamp::parse handles ISO 8601 and other formats
    if let Ok(ts) = Timestamp::from_str(input) {
        if ts > now.timestamp() {
            bail!("Timestamp cannot be in the future: {}", input);
        }
        return Ok(ts);
    }

    // Also try parsing as a Zoned datetime (handles timezone info)
    if let Ok(zoned) = Zoned::from_str(input) {
        let ts = zoned.timestamp();
        if ts > now.timestamp() {
            bail!("Timestamp cannot be in the future: {}", input);
        }
        return Ok(ts);
    }

    // Try parsing as relative span like "1 day ago" or "2 hours ago"
    if let Some(span_str) = input.strip_suffix(" ago") {
        let span = span_str
            .trim()
            .parse::<Span>()
            .with_context(|| format!("Invalid time span: {}", span_str))?;

        // Subtract the span from now to get the target timestamp
        let target = now
            .checked_sub(span)
            .with_context(|| format!("Failed to calculate time from span: {}", span_str))?;

        return Ok(target.timestamp());
    }

    // If neither worked, try parsing as a span without "ago" suffix
    if let Ok(span) = input.parse::<Span>() {
        let target = now
            .checked_sub(span)
            .with_context(|| format!("Failed to calculate time from span: {}", input))?;
        return Ok(target.timestamp());
    }

    bail!(
        "Invalid timestamp format: '{}'. Expected ISO 8601 timestamp or relative time like '1 day ago'",
        input
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_since_absolute() {
        let result = parse_since("2025-01-20T00:00:00Z");
        if let Err(ref e) = result {
            eprintln!("Error: {}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_since_future_fails() {
        let result = parse_since("2030-01-01T00:00:00Z");
        if let Err(ref e) = result {
            eprintln!("Error message: {}", e);
        }
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("cannot be in the future") || err_msg.contains("Invalid timestamp"),
            "Expected error about future timestamp, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_parse_since_relative_with_ago() {
        let result = parse_since("1 day ago");
        assert!(result.is_ok());

        let result = parse_since("2 hours ago");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_since_relative_span() {
        let result = parse_since("1d");
        assert!(result.is_ok());

        let result = parse_since("2h");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_since_invalid() {
        let result = parse_since("invalid timestamp");
        assert!(result.is_err());
    }
}
