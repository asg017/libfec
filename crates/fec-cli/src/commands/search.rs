use crate::{cli::SearchArgs, sourcer::FilingSourcer};

pub fn search(mut sourcer: FilingSourcer, args: &SearchArgs) -> anyhow::Result<()> {
    Ok(())
    /*
    let results = sourcer.cache.search_candidates(args.cycle, &args.query)?;
    for (candidate_id, name) in results {
      println!("{}: {}", candidate_id, name);
    }
    Ok(())
     */
}
