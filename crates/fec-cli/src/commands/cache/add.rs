use indicatif::MultiProgress;

use crate::{
    cli::CacheAddArgs,
    sourcer::{process_inputs, FilingSourcer},
};

pub fn cache_add(sourcer: &mut FilingSourcer, add_args: &CacheAddArgs) -> anyhow::Result<()> {
    let mb = MultiProgress::new();
    // Resolve filings from API and cache them
    let _result = process_inputs(&add_args.filings, add_args.api.clone(), sourcer, Some(&mb))?;
    Ok(())
}
