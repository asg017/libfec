//! Schedule A of Form 3L: contributions bundled by lobbyists/registrants.

use jiff::civil::Date;

use crate::covers::fields::{amount_opt, date, flag, text, text_or_empty, Fields};
use crate::itemizations::{text_any, Entity};

/// "SCHEDULE A (FEC Form 3L) REPORTABLE BUNDLED CONTRIBUTIONS FORWARDED BY OR
/// CREDITED TO LOBBYISTS/REGISTRANTS AND LOBBYIST/REGISTRANT PACs"
/// ([fecfrm3l.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3l.pdf#page=2)):
/// one bundler — a lobbyist/registrant or lobbyist/registrant PAC that
/// forwarded, or is credited with raising, bundled contributions over the
/// reporting threshold in the covered period — with its name, address and the
/// aggregate amount bundled
/// ([fecfrm3li.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=5)).
///
/// Each row is a bundler, not a contributor, and its amount is not a receipt
/// of the filing committee's own report: Form 3L reports no finances, only
/// the bundling disclosures of 52 U.S.C. §30104(i)
/// ([fecfrm3li.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=1)).
/// The schedule's totals go to Form 3L Line 7, without subtracting the
/// refunds reported on Schedule B (`SB3L`)
/// ([fecfrm3li.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=5)).
///
/// In the electronic format these rows are Schedule A records whose form type
/// "must" be `SA3L` (FEC format workbook v8.4, sheet `Sch A`, field 1), with
/// some columns changing meaning; the column mappings give the 8.x layout its
/// own names (`lobbyist_registrant_*`, `bundled_amount_*`), and field numbers
/// below are the workbook's `Sch A` ones. The layout also carries Schedule
/// A's donor-candidate (fields 28–36), conduit (37–42) and Schedule I/L
/// reference (45) columns, which Schedule A (Form 3L) has no box for
/// ([fecfrm3l.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3l.pdf#page=2));
/// they are not read.
///
/// # Versions
///
/// v8.0–8.5 have 45 fields; v6.4–7.0 add `contribution_purpose_code`. Paper
/// layouts (P2.6–P3.4) use the ordinary Schedule A names
/// (`contributor_*`, `contribution_amount`, `contribution_aggregate`), have no
/// transaction IDs or entity type, carry an `image_number`, and from P3.2 a
/// `memo_code`.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        module = "libfec_parser.itemizations",
        frozen,
        get_all,
        skip_from_py_object
    )
)]
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "gleam", derive(fec_parser_macros::GleamType))]
pub struct ScheduleA3L {
    /// The row type as filed, `SA3L`. Column `form_type` (FEC format
    /// workbook v8.4, sheet `Sch A`, field 1).
    pub form_type: String,
    /// The reporting committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this row; electronic only. Column `transaction_id`
    /// (field 3).
    pub transaction_id: Option<String>,
    /// Column `back_reference_tran_id_number` (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// Column `back_reference_sched_name` (field 5).
    pub back_reference_schedule_name: Option<String>,
    /// The bundler: "Full Name of Lobbyist/Registrant (Last, First, Middle
    /// Initial) or Lobbyist/Registrant PAC", mailing address
    /// ([fecfrm3l.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3l.pdf#page=2)),
    /// and entity type (`IND` or `PAC` in practice). Columns `entity_type`,
    /// `lobbyist_registrant_organization_name`, `lobbyist_registrant_last_name`
    /// … `lobbyist_registrant_zip_code` (fields 6–17); paper layouts
    /// `contributor_*`.
    pub lobbyist_registrant: Entity,
    /// "Name of Employer", required when the lobbyist/registrant is an
    /// individual
    /// ([fecfrm3li.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=5)).
    /// Column `lobbyist_registrant_employer` (field 24); paper
    /// `contributor_employer`.
    pub lobbyist_registrant_employer: Option<String>,
    /// Column `lobbyist_registrant_occupation` (field 25); not printed on the
    /// paper schedule. Paper `contributor_occupation`.
    pub lobbyist_registrant_occupation: Option<String>,
    /// "FEC ID number of Lobbyist/Registrant PAC, if applicable"
    /// ([fecfrm3l.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3l.pdf#page=2)),
    /// which the committee should enter for a bundler that is a
    /// lobbyist/registrant PAC
    /// ([fecfrm3li.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=5)).
    /// Column `donor_committee_fec_id` (field 26).
    pub lobbyist_registrant_pac_fec_id: Option<String>,
    /// That PAC's name. Column `donor_committee_name` (field 27).
    pub lobbyist_registrant_pac_name: Option<String>,
    /// Schedule A's election code (`P2024`, see
    /// [`ScheduleA3L::election_code_label`]); Schedule A (Form 3L) has no
    /// election box. Column `election_code` (field 18).
    pub election_code: Option<String>,
    /// Column `election_other_description` (field 19).
    pub election_other_description: Option<String>,
    /// Schedule A's receipt date; Schedule A (Form 3L) has no date box, since
    /// a row is an aggregate over the covered period
    /// ([fecfrm3l.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3l.pdf#page=2)).
    /// Column `contribution_date` (field 20).
    pub contribution_date: Option<Date>,
    /// "Reportable Bundled Contributions during: Quarterly / Monthly /
    /// Pre-Election or Post-Election Covered Period"
    /// ([fecfrm3l.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3l.pdf#page=2)):
    /// the aggregate bundled by this bundler during the report's covered
    /// period ("CONTRIBUTION AMOUNT {F3L Bundled}", FEC format workbook v8.4,
    /// sheet `Sch A`, field 21). A July or January report whose bundler is
    /// over the threshold only for the semi-annual period may report `0` or
    /// leave it blank
    /// ([fecfrm3li.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=6)).
    /// Column `bundled_amount_period`; paper `contribution_amount`.
    pub bundled_amount: f64,
    /// "Reportable Bundled Contributions during: Semi-annual Covered Period"
    /// ([fecfrm3l.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3l.pdf#page=2)),
    /// used on reports filed in July and January for
    /// January 1–June 30 or July 1–December 31
    /// ([fecfrm3li.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=5));
    /// "CONTRIBUTION AGGREGATE {F3L Semi-annual Bundled}" (field 22). The two
    /// periods overlap: do not add this to `bundled_amount`. Column
    /// `bundled_amount_semi_annual`; paper `contribution_aggregate`.
    pub bundled_amount_semi_annual: Option<f64>,
    /// A coded purpose, v6.4–7.0 only (dropped in v8.0). Column
    /// `contribution_purpose_code`.
    pub contribution_purpose_code: Option<String>,
    /// Column `contribution_purpose_descrip` (field 23).
    pub contribution_purpose_description: Option<String>,
    /// True for a memo entry. Column `memo_code`, `X` when true (field 43);
    /// the 8.x column mapping names that position `associated_text_record`.
    pub memo: bool,
    /// Optional explanation of the entries
    /// ([fecfrm3li.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=5)).
    /// Column `memo_text` (field 44); paper `memo_text_description`.
    pub memo_text: Option<String>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl ScheduleA3L {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        // Paper layouts use the ordinary Schedule A names.
        let prefix = if data.raw("lobbyist_registrant_last_name").is_some() {
            "lobbyist_registrant_"
        } else {
            "contributor_"
        };
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text_any(data, &["transaction_id", "transaction_id_number"]),
            back_reference_transaction_id: text(data, "back_reference_tran_id_number"),
            back_reference_schedule_name: text(data, "back_reference_sched_name"),
            lobbyist_registrant: Entity::from_prefixed(data, prefix, "contributor_name"),
            lobbyist_registrant_employer: text_any(
                data,
                &["lobbyist_registrant_employer", "contributor_employer"],
            ),
            lobbyist_registrant_occupation: text_any(
                data,
                &["lobbyist_registrant_occupation", "contributor_occupation"],
            ),
            lobbyist_registrant_pac_fec_id: text(data, "donor_committee_fec_id"),
            lobbyist_registrant_pac_name: text(data, "donor_committee_name"),
            election_code: text(data, "election_code"),
            election_other_description: text(data, "election_other_description"),
            contribution_date: date(data, "contribution_date"),
            bundled_amount: amount_opt(data, "bundled_amount_period")
                .or_else(|| amount_opt(data, "contribution_amount"))
                .unwrap_or(0.0),
            bundled_amount_semi_annual: amount_opt(data, "bundled_amount_semi_annual")
                .or_else(|| amount_opt(data, "contribution_aggregate")),
            contribution_purpose_code: text(data, "contribution_purpose_code"),
            contribution_purpose_description: text(data, "contribution_purpose_descrip"),
            memo: flag(data, "memo_code") || flag(data, "associated_text_record"),
            memo_text: text_any(data, &["memo_text", "memo_text_description"]),
            image_number: text(data, "image_number"),
        })
    }

    /// "Primary", "General", … for `election_code`; see
    /// [`crate::covers::election_code_label`].
    pub fn election_code_label(&self) -> Option<&'static str> {
        crate::covers::election_code_label(self.election_code.as_deref()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::covers::fields::Data;

    /// Paper layouts read the ordinary Schedule A columns.
    #[test]
    fn paper_layout() {
        let d: Data = [
            ("form_type", "SA3L"),
            ("filer_committee_id_number", "C00123456"),
            ("contributor_last_name", "Lobby"),
            ("contributor_first_name", "Lou"),
            ("contributor_employer", "Firm LLP"),
            ("contribution_amount", "20000.00"),
            ("contribution_aggregate", "35000.00"),
            ("memo_code", "X"),
            ("image_number", "29934567890"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
        let s = ScheduleA3L::from_data(&d).expect("typed");
        assert_eq!(s.lobbyist_registrant.display_name(), "Lou Lobby");
        assert_eq!(s.lobbyist_registrant_employer.as_deref(), Some("Firm LLP"));
        assert_eq!(s.bundled_amount, 20000.0);
        assert_eq!(s.bundled_amount_semi_annual, Some(35000.0));
        assert!(s.memo);
    }
}
