use colored::Colorize;
use indicatif::HumanBytes;
use jiff::Timestamp;

use crate::{
    cli::CacheArgs,
    sourcer::{FecFilingId, FilingSourcer},
};

pub fn cache(sourcer: FilingSourcer, args: &CacheArgs) -> Result<(), ()> {
    let t0 = jiff::Timestamp::now();

    let stats = sourcer
        .cache
        .cache_all(
            &sourcer,
            match args.filings.clone() {
                Some(filings) => filings
                    .iter()
                    .map(|x| FecFilingId::from_str(x).unwrap())
                    .collect(),
                None => vec![],
            },
            &args.api,
        )
        .unwrap();

    let duration = Timestamp::now() - t0;
    println!(
        "{} Cached {} filings in {:#}",
        "✓".green(),
        stats.number_downloaded + stats.number_preexisting,
        duration,
    );
    println!(
        "  {} filings downloaded ({})",
        stats.number_downloaded,
        HumanBytes(stats.downloaded_bytes as u64)
    );
    if stats.number_preexisting > 0 {
        println!("  {} pre-existing", stats.number_preexisting);
    }

    Ok(())
}
