use crate::{
    api_flags::Trace,
    cli::{ApiFilingsArgs, ApiFormat},
    sourcer::{FilingSourcer, Item, UserArgument},
};
use fec_api::CommitteeId;
use std::io::{self, Write};

pub fn filings(mut sourcer: FilingSourcer, args: &ApiFilingsArgs) -> anyhow::Result<()> {
    let mut api_flags = args.api.clone();
    let mut trace = Trace {
        resolve_candidate_params: vec![],
    };

    for input in &args.inputs {
        match sourcer.resolve_user_argument(input)? {
            UserArgument::Committee(id) => {
                api_flags.committee.get_or_insert_with(Vec::new).push(id);
            }
            UserArgument::Candidate(id) => {
                api_flags.candidate.get_or_insert_with(Vec::new).push(id);
            }
            UserArgument::Contest(contest) => {
                let cycle = api_flags.election.ok_or_else(|| {
                    anyhow::anyhow!("contest '{}' requires --election to be set", input)
                })?;
                let params = contest.resolve_candidate_params(cycle);
                trace.resolve_candidate_params.push(params.clone());
                let committee_strings = sourcer
                    .cache
                    .resolve_candidate_principal_campaign_committees(params)?;
                api_flags.committee.get_or_insert_with(Vec::new).extend(
                    committee_strings
                        .into_iter()
                        .map(|s| CommitteeId::new(&s).unwrap()),
                );
            }
            UserArgument::Filing(Item::FilingId(_)) => {
                return Err(anyhow::anyhow!(
                    "'{}' is a filing ID — the api command queries the filings API by committee/candidate, not by filing ID. Use `libfec info {}` instead.",
                    input, input
                ));
            }
            UserArgument::Filing(_) => {
                return Err(anyhow::anyhow!(
                    "'{}' is a URL or file path — the api command only accepts committee IDs, candidate IDs, and contest shorthand",
                    input
                ));
            }
            UserArgument::InputFile(_) => {
                return Err(anyhow::anyhow!(
                    "'{}' is a file path — the api command only accepts committee IDs, candidate IDs, and contest shorthand",
                    input
                ));
            }
        }
    }

    let items = api_flags.resolve_items(&mut sourcer, None, &mut trace)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if args.filing_ids_only {
        for item in &items {
            writeln!(out, "{}", item.filing_id)?;
        }
    } else {
        match args.format {
            ApiFormat::Json => {
                let values: Vec<&serde_json::Value> =
                    items.iter().map(|item| &item.value).collect();
                serde_json::to_writer_pretty(&mut out, &values)?;
                writeln!(out)?;
            }
            ApiFormat::Jsonl => {
                for item in &items {
                    serde_json::to_writer(&mut out, &item.value)?;
                    writeln!(out)?;
                }
            }
        }
    }

    Ok(())
}
