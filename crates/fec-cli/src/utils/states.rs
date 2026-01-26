/// Official FEC state code mappings
///
/// This module provides utilities for working with US state codes and names
/// as used by the Federal Election Commission.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Map of 2-letter state codes to full state names
pub static STATE_CODE_TO_NAME: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        HashMap::from([
            ("AL", "Alabama"),
            ("AK", "Alaska"),
            ("AZ", "Arizona"),
            ("AR", "Arkansas"),
            ("AS", "American Samoa"),
            ("CA", "California"),
            ("CO", "Colorado"),
            ("CT", "Connecticut"),
            ("DE", "Delaware"),
            ("DC", "District of Columbia"),
            ("FL", "Florida"),
            ("GA", "Georgia"),
            ("GU", "Guam"),
            ("HI", "Hawaii"),
            ("ID", "Idaho"),
            ("IL", "Illinois"),
            ("IN", "Indiana"),
            ("IA", "Iowa"),
            ("KS", "Kansas"),
            ("KY", "Kentucky"),
            ("LA", "Louisiana"),
            ("ME", "Maine"),
            ("MD", "Maryland"),
            ("MA", "Massachusetts"),
            ("MI", "Michigan"),
            ("MN", "Minnesota"),
            ("MS", "Mississippi"),
            ("MO", "Missouri"),
            ("MP", "Northern Mariana Islands"),
            ("MT", "Montana"),
            ("NE", "Nebraska"),
            ("NV", "Nevada"),
            ("NH", "New Hampshire"),
            ("NJ", "New Jersey"),
            ("NM", "New Mexico"),
            ("NY", "New York"),
            ("NC", "North Carolina"),
            ("ND", "North Dakota"),
            ("OH", "Ohio"),
            ("OK", "Oklahoma"),
            ("OR", "Oregon"),
            ("PA", "Pennsylvania"),
            ("PR", "Puerto Rico"),
            ("RI", "Rhode Island"),
            ("SC", "South Carolina"),
            ("SD", "South Dakota"),
            ("TN", "Tennessee"),
            ("TX", "Texas"),
            ("UT", "Utah"),
            ("VT", "Vermont"),
            ("VA", "Virginia"),
            ("VI", "Virgin Islands"),
            ("WA", "Washington"),
            ("WV", "West Virginia"),
            ("WI", "Wisconsin"),
            ("WY", "Wyoming"),
        ])
    });

/// Map of full state names to 2-letter state codes
pub static STATE_NAME_TO_CODE: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        STATE_CODE_TO_NAME
            .iter()
            .map(|(code, name)| (*name, *code))
            .collect()
    });

/// Validate a state code (case-insensitive)
pub fn is_valid_state_code(code: &str) -> bool {
    STATE_CODE_TO_NAME.contains_key(code.to_uppercase().as_str())
}

/// Get state name from code (case-insensitive)
pub fn state_name_from_code(code: &str) -> Option<&'static str> {
    STATE_CODE_TO_NAME.get(code.to_uppercase().as_str()).copied()
}

/// Get state code from name (case-insensitive)
pub fn state_code_from_name(name: &str) -> Option<&'static str> {
    // Try exact match first
    if let Some(code) = STATE_NAME_TO_CODE.get(name) {
        return Some(code);
    }

    // Try case-insensitive match
    let name_lower = name.to_lowercase();
    STATE_NAME_TO_CODE
        .iter()
        .find(|(state_name, _)| state_name.to_lowercase() == name_lower)
        .map(|(_, code)| *code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_state_codes() {
        assert!(is_valid_state_code("CA"));
        assert!(is_valid_state_code("ca"));
        assert!(is_valid_state_code("TX"));
        assert!(is_valid_state_code("NY"));
        assert!(!is_valid_state_code("XX"));
        assert!(!is_valid_state_code(""));
    }

    #[test]
    fn test_state_name_from_code() {
        assert_eq!(state_name_from_code("CA"), Some("California"));
        assert_eq!(state_name_from_code("ca"), Some("California"));
        assert_eq!(state_name_from_code("TX"), Some("Texas"));
        assert_eq!(state_name_from_code("NY"), Some("New York"));
        assert_eq!(state_name_from_code("XX"), None);
    }

    #[test]
    fn test_state_code_from_name() {
        assert_eq!(state_code_from_name("California"), Some("CA"));
        assert_eq!(state_code_from_name("california"), Some("CA"));
        assert_eq!(state_code_from_name("Texas"), Some("TX"));
        assert_eq!(state_code_from_name("New York"), Some("NY"));
        assert_eq!(state_code_from_name("Invalid State"), None);
    }

    #[test]
    fn test_all_states_present() {
        // Ensure we have all 50 states + DC + territories
        assert!(STATE_CODE_TO_NAME.len() >= 56);

        // Spot check some states
        assert!(STATE_CODE_TO_NAME.contains_key("CA"));
        assert!(STATE_CODE_TO_NAME.contains_key("TX"));
        assert!(STATE_CODE_TO_NAME.contains_key("NY"));
        assert!(STATE_CODE_TO_NAME.contains_key("DC"));
        assert!(STATE_CODE_TO_NAME.contains_key("PR"));
    }
}
