use crate::cli::RssArgs;
use crate::rss::{self, format_duration_ago};
use crate::sourcer::FilingSourcer;
use anyhow::Result;
use jiff::Zoned;
use tabled::{builder::Builder as TableBuilder, settings::Style as TableStyle};

use super::export::{export_filing_by_id, open_or_create_export_db};

/// Simple mode: fetch once and display a table, then exit
pub fn run_simple_mode(sourcer: &FilingSourcer, args: &RssArgs) -> Result<()> {
    let (url, _) = rss::build_feed_url(args);
    let (result, filters) =
        rss::fetch_feed_with_args(args).map_err(|e| anyhow::anyhow!("{}", e))?;
    let now = Zoned::now();

    // Handle export if -x flag is provided
    if let Some(ref export_path) = args.export {
        let mut db = open_or_create_export_db(export_path)?;
        crate::commands::export::sqlite::init_schema(&mut db)?;
        let existing_ids =
            crate::commands::export::sqlite::get_existing_filing_ids(&db).unwrap_or_default();

        let mut export_count = 0;
        for item in result.feed.items.iter().take(args.limit) {
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

    for item in result.feed.items.iter().take(args.limit) {
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
    println!(
        "\nShowing {} of {} items",
        args.limit.min(result.feed.items.len()),
        result.feed.items.len()
    );

    Ok(())
}
