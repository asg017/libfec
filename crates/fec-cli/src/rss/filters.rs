use crate::cli::{RssArgs, RssPreset};

use super::types::ActiveFilters;

pub const FEC_RSS_BASE_URL: &str = "https://efilingapps.fec.gov/rss/generate";

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
