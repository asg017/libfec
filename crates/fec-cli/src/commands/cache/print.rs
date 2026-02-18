use crate::sourcer::FilingSourcer;

pub fn cache_print(sourcer: &FilingSourcer) {
    println!("{}", sourcer.cache.cache_directory().display());
}
