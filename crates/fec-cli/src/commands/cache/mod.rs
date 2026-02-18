mod add;
mod info;
mod print;

use crate::{
    cli::{CacheArgs, CacheSubcommand},
    sourcer::FilingSourcer,
};

pub fn cache(mut sourcer: FilingSourcer, args: &CacheArgs) -> anyhow::Result<()> {
    match &args.command {
        CacheSubcommand::Print => print::cache_print(&sourcer),
        CacheSubcommand::Info => info::cache_info(&sourcer),
        CacheSubcommand::Add(add_args) => return add::cache_add(&mut sourcer, add_args),
    }

    Ok(())
}
