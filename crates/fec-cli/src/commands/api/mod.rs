mod filings;
mod schedule_a;
mod schedule_b;
mod schedule_c;
mod schedule_d;
mod schedule_e;
mod schedule_f;
mod shared;

use crate::cli::{ApiArgs, ApiSubcommand};
use crate::sourcer::FilingSourcer;

pub fn api(sourcer: FilingSourcer, args: &ApiArgs) -> anyhow::Result<()> {
    match &args.command {
        ApiSubcommand::Filings(filings_args) => filings::filings(sourcer, filings_args),
        ApiSubcommand::ScheduleA(sa_args) => schedule_a::schedule_a(sourcer, sa_args),
        ApiSubcommand::ScheduleB(sb_args) => schedule_b::schedule_b(sourcer, sb_args),
        ApiSubcommand::ScheduleC(sc_args) => schedule_c::schedule_c(sourcer, sc_args),
        ApiSubcommand::ScheduleD(sd_args) => schedule_d::schedule_d(sourcer, sd_args),
        ApiSubcommand::ScheduleE(se_args) => schedule_e::schedule_e(sourcer, se_args),
        ApiSubcommand::ScheduleF(sf_args) => schedule_f::schedule_f(sourcer, sf_args),
    }
}

#[cfg(test)]
mod tests {
    use super::shared::redact_url;
    use crate::cli::Cli;
    use clap::Parser;
    use insta::assert_snapshot;

    /// Parse CLI args and extract the ApiSubcommand, then build+snapshot the URL.
    fn url_for(args: &[&str]) -> String {
        let mut full_args = vec!["libfec", "api"];
        full_args.extend_from_slice(args);
        let cli = Cli::try_parse_from(full_args).expect("failed to parse args");
        match *cli.command {
            crate::cli::Commands::Api(ref api_args) => match &api_args.command {
                crate::cli::ApiSubcommand::ScheduleA(a) => {
                    redact_url(&super::schedule_a::build_url(a))
                }
                crate::cli::ApiSubcommand::ScheduleB(a) => {
                    redact_url(&super::schedule_b::build_url(a))
                }
                crate::cli::ApiSubcommand::ScheduleC(a) => {
                    redact_url(&super::schedule_c::build_url(a))
                }
                crate::cli::ApiSubcommand::ScheduleD(a) => {
                    redact_url(&super::schedule_d::build_url(a))
                }
                crate::cli::ApiSubcommand::ScheduleE(a) => {
                    redact_url(&super::schedule_e::build_url(a))
                }
                crate::cli::ApiSubcommand::ScheduleF(a) => {
                    redact_url(&super::schedule_f::build_url(a))
                }
                _ => panic!("expected schedule subcommand"),
            },
            _ => panic!("expected api command"),
        }
    }

    // Schedule A

    #[test]
    fn schedule_a_committee_only() {
        assert_snapshot!(url_for(&["schedule-a", "--committee", "C00401224"]));
    }

    #[test]
    fn schedule_a_multiple_filters() {
        assert_snapshot!(url_for(&[
            "schedule-a",
            "--committee",
            "C00401224",
            "--min-amount",
            "200",
            "--max-date",
            "2025-12-31",
            "--contributor-state",
            "CA",
            "--is-individual",
        ]));
    }

    #[test]
    fn schedule_a_contributor_search() {
        assert_snapshot!(url_for(&[
            "schedule-a",
            "--contributor-employer",
            "Google",
            "--contributor-occupation",
            "Engineer",
            "--two-year-transaction-period",
            "2024",
        ]));
    }

    // Schedule B

    #[test]
    fn schedule_b_committee_only() {
        assert_snapshot!(url_for(&["schedule-b", "--committee", "C00401224"]));
    }

    #[test]
    fn schedule_b_with_filters() {
        assert_snapshot!(url_for(&[
            "schedule-b",
            "--committee",
            "C00401224",
            "--min-amount",
            "500",
            "--min-date",
            "2025-01-01",
            "--recipient-state",
            "NY",
        ]));
    }

    // Schedule C

    #[test]
    fn schedule_c_committee_only() {
        assert_snapshot!(url_for(&["schedule-c", "--committee", "C00401224"]));
    }

    #[test]
    fn schedule_c_with_candidate() {
        assert_snapshot!(url_for(&[
            "schedule-c",
            "--candidate",
            "P80001571",
            "--min-amount",
            "10000",
        ]));
    }

    // Schedule D

    #[test]
    fn schedule_d_committee_only() {
        assert_snapshot!(url_for(&["schedule-d", "--committee", "C00401224"]));
    }

    #[test]
    fn schedule_d_with_filters() {
        assert_snapshot!(url_for(&[
            "schedule-d",
            "--committee",
            "C00401224",
            "--creditor-debtor-name",
            "Acme Corp",
            "--min-amount",
            "1000",
        ]));
    }

    // Schedule E

    #[test]
    fn schedule_e_candidate_only() {
        assert_snapshot!(url_for(&["schedule-e", "--candidate", "P80001571"]));
    }

    #[test]
    fn schedule_e_with_filters() {
        assert_snapshot!(url_for(&[
            "schedule-e",
            "--candidate",
            "P80001571",
            "--support-oppose-indicator",
            "S",
            "--cycle",
            "2024",
            "--min-amount",
            "10000",
            "--most-recent",
            "true",
        ]));
    }

    // Schedule F

    #[test]
    fn schedule_f_committee_only() {
        assert_snapshot!(url_for(&["schedule-f", "--committee", "C00401224"]));
    }

    #[test]
    fn schedule_f_with_filters() {
        assert_snapshot!(url_for(&[
            "schedule-f",
            "--committee",
            "C00401224",
            "--payee-name",
            "Vendor Inc",
            "--cycle",
            "2024",
            "--min-amount",
            "5000",
        ]));
    }

    // Shared flags

    #[test]
    fn schedule_a_custom_per_page() {
        assert_snapshot!(url_for(&[
            "schedule-a",
            "--committee",
            "C00401224",
            "--per-page",
            "20",
        ]));
    }

    #[test]
    fn schedule_a_with_sort() {
        assert_snapshot!(url_for(&[
            "schedule-a",
            "--committee",
            "C00401224",
            "--sort=-contribution_receipt_date",
        ]));
    }

    #[test]
    fn schedule_e_multiple_committees() {
        assert_snapshot!(url_for(&[
            "schedule-e",
            "--committee",
            "C00401224",
            "--committee",
            "C00513531",
        ]));
    }
}
