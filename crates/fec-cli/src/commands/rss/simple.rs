use crate::cli::RssArgs;
use crate::commands::export::sqlite;
use crate::rss::{self, format_duration_ago, Item};
use crate::sourcer::FilingSourcer;
use anyhow::Result;
use jiff::{Timestamp, Zoned};
use tabled::{builder::Builder as TableBuilder, settings::Style as TableStyle};

use super::export::{export_filing_by_id, open_or_create_export_db};

/// Simple mode: fetch once and display a table, then exit
pub fn run_simple_mode(
    sourcer: &FilingSourcer,
    args: &RssArgs,
    since_ts: Option<Timestamp>,
) -> Result<()> {
    let (url, _) = rss::build_feed_url(args);
    let (result, filters) =
        rss::fetch_feed_with_args(args).map_err(|e| anyhow::anyhow!("{}", e))?;
    let now = Zoned::now();

    // Capture feed metadata before moving items
    let total_feed_items = result.feed.items.len();
    let feed_title = match args.committee_label {
        Some(ref label) => super::app::replace_committee_ids_in_title(&result.feed.title, label),
        None => result.feed.title.clone(),
    };
    let feed_last_modified = result.last_modified;

    // Filter items by --since if provided
    let filtered_items: Vec<_> = if let Some(since) = since_ts {
        result
            .feed
            .items
            .into_iter()
            .filter(|item| {
                item.pub_date
                    .map(|pub_date| pub_date >= since)
                    .unwrap_or(false)
            })
            .collect()
    } else {
        result.feed.items
    };

    // Handle export if -x flag is provided
    if let Some(ref export_path) = args.export {
        let mut db = open_or_create_export_db(export_path)?;
        sqlite::init_schema(&mut db)?;
        let existing_ids = sqlite::get_existing_filing_ids(&db).unwrap_or_default();

        // Initialize metadata if enabled
        let metadata_sync_id = if args.write_metadata {
            sqlite::init_rss_metadata_schema(&db)?;
            let sync_uuid = format!("sync-{}", uuid::Uuid::new_v4());
            let preset_name = format!("{:?}", args.preset);
            let metadata_params = sqlite::RssSyncParams {
                feed_url: Some(url.clone()),
                feed_last_modified: feed_last_modified.map(|ts| ts.to_string()),
                feed_title: Some(feed_title.clone()),
                since_filter: args.since.clone(),
                preset_filter: Some(preset_name),
                form_type_filter: args.form_type.clone(),
                committee_filter: args.committee.clone(),
                state_filter: args.state.clone(),
                party_filter: args.party.clone(),
                cover_only: args.cover_only,
            };
            let metadata = sqlite::create_rss_sync(&db, &sync_uuid, &metadata_params)?;
            Some(metadata.sync_id)
        } else {
            None
        };

        // Build a map from filing_id to item for metadata recording
        let item_map: std::collections::HashMap<&str, &Item> = filtered_items
            .iter()
            .filter_map(|item| item.filing_id.as_deref().map(|id| (id, item)))
            .collect();

        // Export ALL filtered items from feed, not just the displayed limit
        let mut export_count = 0;
        let mut new_filings_count = 0;
        for item in filtered_items.iter() {
            if let Some(ref filing_id) = item.filing_id {
                if !existing_ids.contains(filing_id) {
                    new_filings_count += 1;
                    let (success, message) =
                        match export_filing_by_id(sourcer, &mut db, filing_id, args.cover_only) {
                            Ok(_) => {
                                export_count += 1;
                                println!("Exported filing {}", filing_id);
                                (true, None)
                            }
                            Err(e) => {
                                let msg = e.to_string();
                                eprintln!("Error exporting filing {}: {}", filing_id, msg);
                                (false, Some(msg))
                            }
                        };

                    // Record metadata if enabled
                    if let Some(sync_id) = metadata_sync_id {
                        let rss_params = if let Some(rss_item) = item_map.get(filing_id.as_str()) {
                            sqlite::RssFilingParams {
                                rss_pub_date: rss_item.pub_date.map(|ts| ts.to_string()),
                                rss_guid: Some(rss_item.guid.clone()),
                                rss_title: Some(rss_item.title.clone()),
                                committee_id: rss_item.committee_id.clone(),
                                form_type: rss_item.form_type.clone(),
                                coverage_from: rss_item.coverage_from.clone(),
                                coverage_through: rss_item.coverage_through.clone(),
                                report_type: rss_item.report_type.clone(),
                            }
                        } else {
                            sqlite::RssFilingParams::default()
                        };
                        let _ = sqlite::record_rss_filing(
                            &db,
                            sync_id,
                            filing_id,
                            &rss_params,
                            success,
                            message.as_deref(),
                        );
                    }
                }
            }
        }

        // Finalize metadata
        if let Some(sync_id) = metadata_sync_id {
            let _ = sqlite::finalize_rss_sync(
                &db,
                sync_id,
                "complete",
                Some(total_feed_items),
                Some(filtered_items.len()),
                new_filings_count,
                export_count,
                None,
            );
        }

        if export_count > 0 {
            println!(
                "Exported {} new filing(s) to {}",
                export_count,
                export_path.display()
            );
        } else {
            println!("No new filings to export");
        }
        println!();
    }

    let mut builder = TableBuilder::new();
    builder.push_record(["Committee", "Form", "Report", "Filing ID", "Age"]);

    for item in filtered_items.iter().take(args.limit) {
        let committee = item.extract_committee_name();
        let form = item.form_type.as_deref().unwrap_or("-");
        let report = item.report_type.as_deref().unwrap_or("-");
        let filing_id = item.filing_id.as_deref().unwrap_or("-");
        let age = item.time_ago(&now).unwrap_or_else(|| "-".to_string());

        builder.push_record([committee, form, report, filing_id, &age]);
    }

    let table = builder.build().with(TableStyle::rounded()).to_string();

    println!("{}", feed_title);
    if let Some(last_mod) = feed_last_modified {
        let age = now.timestamp().duration_since(last_mod).as_secs();
        println!("Data freshness: {}", format_duration_ago(age));
    }

    // Display active filters
    let filter_strs = filters.to_display_strings();
    if !filter_strs.is_empty() {
        println!("Filters: {}", filter_strs.join(" | "));
    }

    println!("URL: {}", url);
    println!();
    println!("{}", table);

    if let Some(since) = since_ts {
        let since_zoned = Zoned::new(since, jiff::tz::TimeZone::UTC);
        println!("Since: {}", since_zoned);
    }

    println!(
        "\nShowing {} of {} items",
        args.limit.min(filtered_items.len()),
        filtered_items.len()
    );

    Ok(())
}
