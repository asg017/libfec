use crate::cli::RssArgs;
use crate::rss::{self, format_duration_ago};
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
        crate::commands::export::sqlite::init_schema(&mut db)?;
        let existing_ids =
            crate::commands::export::sqlite::get_existing_filing_ids(&db).unwrap_or_default();

        // Export ALL filtered items from feed, not just the displayed limit
        let mut export_count = 0;
        for item in filtered_items.iter() {
            if let Some(ref filing_id) = item.filing_id {
                if !existing_ids.contains(filing_id) {
                    match export_filing_by_id(sourcer, &mut db, filing_id, args.cover_only) {
                        Ok(_) => {
                            export_count += 1;
                            println!("Exported filing {}", filing_id);
                        }
                        Err(e) => {
                            eprintln!("Error exporting filing {}: {}", filing_id, e);
                        }
                    }
                }
            }
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

    println!("{}", result.feed.title);
    if let Some(last_mod) = result.last_modified {
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
