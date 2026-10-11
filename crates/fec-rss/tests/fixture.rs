//! Parses a trimmed copy of the live feed (`?preDefinedFilingType=ALL`, 2026-10-10).

use fec_rss::parse_feed;

const ALL: &str = include_str!("fixtures/all.xml");

#[test]
fn parses_live_feed_sample() {
    let feed = parse_feed(ALL).unwrap();
    assert_eq!(feed.title, "FEC Electronic Filing RSS Feed - ALL");
    assert_eq!(feed.items.len(), 30);

    for item in &feed.items {
        assert!(item.title.starts_with("New filing by "), "{}", item.title);
        assert!(item.pub_date.is_some(), "{}", item.title);
        assert!(item.committee_id.as_deref().unwrap().starts_with('C'));
        let filing_id = item.filing_id.as_deref().unwrap();
        assert!(item.link.ends_with(&format!("/{filing_id}.fec")));
        assert!(!item.title.contains("&amp;"), "{}", item.title);
    }

    let first = &feed.items[0];
    assert_eq!(first.extract_committee_name(), "ALLIED UNIVERSAL PAC");
    assert_eq!(first.filing_id.as_deref(), Some("2019253"));
    assert_eq!(first.form_type.as_deref(), Some("F3XN"));
    assert_eq!(first.coverage_from.as_deref(), Some("07/01/2026"));
    assert_eq!(first.report_type.as_deref(), Some("OCTOBER QUARTERLY"));
    assert_eq!(first.pub_date.unwrap().to_string(), "2026-10-10T17:50:22Z");

    assert!(feed
        .items
        .iter()
        .any(|i| i.title == "New filing by Marshall & Blue Origin Investments"));
}
