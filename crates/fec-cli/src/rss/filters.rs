use fec_rss::{FeedQuery, Preset};

use crate::cli::{RssArgs, RssPreset};

use super::types::ActiveFilters;

/// Build RSS feed URL with filters from args
pub fn build_feed_url(args: &RssArgs) -> (String, ActiveFilters) {
    let query = FeedQuery {
        preset: match args.preset {
            RssPreset::All => Preset::All,
            RssPreset::Monthly => Preset::Monthly,
            RssPreset::Quarterly => Preset::Quarterly,
            RssPreset::Presidential => Preset::Presidential,
            RssPreset::Congressional => Preset::Congressional,
            RssPreset::Pac => Preset::Pac,
        },
        committees: args.committee.clone(),
        forms: args.form_type.clone(),
        states: args.state.clone(),
        parties: args.party.clone(),
    };

    let mut filters = ActiveFilters::default();
    if query.has_custom_filters() {
        filters.committee = args.committee.as_ref().map(|committee| {
            args.committee_label
                .clone()
                .unwrap_or_else(|| committee.clone())
        });
        filters.form_type = args.form_type.as_ref().map(|f| f.to_uppercase());
        filters.state = args.state.as_ref().map(|s| s.to_uppercase());
        filters.party = args.party.as_ref().map(|p| p.to_uppercase());
    } else {
        filters.preset = Some(query.preset.label().to_string());
    }

    (query.url(), filters)
}
