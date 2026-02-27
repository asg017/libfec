use crate::cli::ApiScheduleFArgs;
use crate::sourcer::FilingSourcer;
use fec_api::Api;
use url::Url;

use super::shared::{fetch_and_output, print_url, push_opt, push_opt_f64, push_vec, push_vec_u16};

pub fn build_url(args: &ApiScheduleFArgs) -> Url {
    let api_key = args.api_key.as_deref().unwrap_or("DEMO_KEY");
    let client = Api::new(api_key);

    let mut params: Vec<(&str, String)> = Vec::new();
    params.push(("per_page", args.per_page.to_string()));

    push_vec(&mut params, "committee_id", &args.committee);
    push_vec(&mut params, "candidate_id", &args.candidate);
    push_vec(&mut params, "payee_name", &args.payee_name);
    push_opt_f64(&mut params, "min_amount", &args.min_amount);
    push_opt_f64(&mut params, "max_amount", &args.max_amount);
    push_opt(&mut params, "min_date", &args.min_date);
    push_opt(&mut params, "max_date", &args.max_date);
    push_vec_u16(&mut params, "cycle", &args.cycle);
    push_opt(&mut params, "sort", &args.sort);

    let param_refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
    client
        .endpoint_url("/v1/schedules/schedule_f/", &param_refs)
        .0
}

pub fn schedule_f(mut sourcer: FilingSourcer, args: &ApiScheduleFArgs) -> anyhow::Result<()> {
    let url = build_url(args);

    if args.url_only {
        print_url(&url);
        return Ok(());
    }

    let cache = sourcer
        .cache
        .api_cache_mut()
        .map(|c| c as &mut dyn fec_api::ApiCache);
    fetch_and_output(url, cache, args.format, args.all)
}
