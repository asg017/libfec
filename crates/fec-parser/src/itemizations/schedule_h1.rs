//! Schedule H1: method of allocation for shared federal/nonfederal costs.

use crate::covers::fields::{amount_opt, flag, text, text_or_empty, Fields};

/// "SCHEDULE H1 - METHOD OF ALLOCATION": the ratio a committee uses to split
/// shared expenses between its federal account and its nonfederal account
/// (or, for party committees' federal election activity, Levin funds). It
/// holds only check-boxes and percentages, no payee or amount (FEC format
/// workbook v8.4, sheet `Sch H1`, fields 1–12); Schedules H4 and H6 apply
/// the ratio it declares
/// ([fecfrm3xi.pdf p25](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=25),
/// [p30](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=30),
/// [p32](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=32)).
/// A schedule of Form 3X
/// ([fecfrm3x.pdf p13](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=13)).
///
/// The record has two mutually exclusive halves (workbook notes after fields
/// 3 and 7): **Part A**, state and local party committees, checks one box for
/// the kind of election year, which fixes the federal percentage (see
/// [`ScheduleH1::party_federal_percent`]); **Part B**, separate segregated
/// funds and nonconnected committees, gives `federal_percent` and
/// `nonfederal_percent` and checks which spending the ratio applies to.
/// Fundraising and direct candidate support ratios are on Schedule H2
/// instead ([fecfrm3xi.pdf p25](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=25)).
///
/// # Versions
///
/// v8.2–8.5 have all 12 fields. v5.2–8.1 add a `flat_minimum_federal_percentage`
/// column, as do paper layouts before P3.4 (paper layouts have no transaction
/// ID but an `image_number`). v5.2–5.3 put the H1 fields after 25 unused
/// columns, read by name like the rest. The pre-BCRA layouts (v1–3, v5.0–5.1)
/// describe a different allocation method (national party percentages, a
/// ballot-composition point count, …); of those only the identification
/// fields, the percentages and the v5.0–5.1 election-year boxes are read.
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
pub struct ScheduleH1 {
    /// The row type as filed, `H1`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `Sch H1`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this record, unique for the life of the report;
    /// electronic-only. Column `transaction_id` (field 3).
    pub transaction_id: Option<String>,
    /// Part A box "Presidential-Only Election Year (28% Federal)". Column
    /// `presidential_only_election_year`, `X` when checked (field 4).
    pub presidential_only_election_year: bool,
    /// Part A box "Presidential and Senate Election Year (36% Federal)".
    /// Column `presidential_senate_election_year` (field 5).
    pub presidential_senate_election_year: bool,
    /// Part A box "Senate-Only Election Year (21% Federal)". Column
    /// `senate_only_election_year` (field 6).
    pub senate_only_election_year: bool,
    /// Part A box "Non-Presidential and Non-Senate Election Year (15%
    /// Federal)". Column `non_presidential_non_senate_election_year`
    /// (field 7).
    pub non_presidential_non_senate_election_year: bool,
    /// Part B federal percentage, as filed: a fraction, `.5000` for 50% (the
    /// workbook's type is `NUM-5`; every non-blank value in the corpus is
    /// between 0 and 1). Blank on Part A rows. Column `federal_percent`
    /// (field 8).
    pub federal_percent: Option<f64>,
    /// Part B nonfederal percentage, a fraction like `federal_percent`.
    /// Column `nonfederal_percent` (field 9).
    pub nonfederal_percent: Option<f64>,
    /// Part B: the ratio applies to administrative costs. Column
    /// `administrative_ratio_applies` (field 10).
    pub administrative_ratio_applies: bool,
    /// Part B: the ratio applies to generic voter drives. Column
    /// `generic_voter_drive_ratio_applies` (field 11).
    pub generic_voter_drive_ratio_applies: bool,
    /// Part B: the ratio applies to public communications referencing a
    /// party only. Column `public_communications_referencing_party_ratio_applies`
    /// (field 12).
    pub public_communications_referencing_party_ratio_applies: bool,
    /// v5.2–8.1 and paper P1–P3.3 only, raw as filed (`X` in every corpus
    /// row that has it). Its meaning is not in our sources. Column
    /// `flat_minimum_federal_percentage`.
    pub flat_minimum_federal_percentage: Option<String>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl ScheduleH1 {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id"),
            presidential_only_election_year: flag(data, "presidential_only_election_year"),
            presidential_senate_election_year: flag(data, "presidential_senate_election_year"),
            senate_only_election_year: flag(data, "senate_only_election_year"),
            non_presidential_non_senate_election_year: flag(
                data,
                "non_presidential_non_senate_election_year",
            ),
            federal_percent: amount_opt(data, "federal_percent"),
            nonfederal_percent: amount_opt(data, "nonfederal_percent"),
            administrative_ratio_applies: flag(data, "administrative_ratio_applies"),
            generic_voter_drive_ratio_applies: flag(data, "generic_voter_drive_ratio_applies"),
            public_communications_referencing_party_ratio_applies: flag(
                data,
                "public_communications_referencing_party_ratio_applies",
            ),
            flat_minimum_federal_percentage: text(data, "flat_minimum_federal_percentage"),
            image_number: text(data, "image_number"),
        })
    }

    /// The fixed federal percentage of a Part A (party committee) row, as a
    /// fraction like `federal_percent`: 0.28 Presidential-only, 0.36
    /// Presidential and Senate, 0.21 Senate-only, 0.15 neither, as printed on
    /// the form revised 05/2016
    /// ([fecfrm3x.pdf p13](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=13)).
    /// `None` unless exactly one Part A box is checked.
    pub fn party_federal_percent(&self) -> Option<f64> {
        let boxes = [
            (self.presidential_only_election_year, 0.28),
            (self.presidential_senate_election_year, 0.36),
            (self.senate_only_election_year, 0.21),
            (self.non_presidential_non_senate_election_year, 0.15),
        ];
        let mut checked = boxes.iter().filter(|(on, _)| *on);
        match (checked.next(), checked.next()) {
            (Some((_, pct)), None) => Some(*pct),
            _ => None,
        }
    }
}
