//! Schedule H2: allocation ratios for fundraising and candidate support.

use crate::covers::fields::{amount_opt, flag, text, text_or_empty, Fields};

/// "SCHEDULE H2 - ALLOCATION RATIOS": the federal/nonfederal ratio of one
/// fundraising event or direct candidate support activity whose cost a
/// committee splits between its accounts
/// ([fecfrm3xi.pdf p27](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=27)).
/// One row is one named activity or event (FEC format workbook v8.4, sheet
/// `Sch H2`, fields 4–9), a schedule of Form 3X
/// ([fecfrm3x.pdf p14](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=14)).
///
/// The activity's name "must be assigned a unique identifying title or code"
/// used "consistently throughout a committee's reports"
/// ([fecfrm3xi.pdf p27](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=27)):
/// it reappears as the event of Schedule H3 transfers and Schedule H4
/// disbursements ([fecfrm3xi.pdf p28](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=28),
/// [p30](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=30)),
/// free text matched only as typed. Administrative and generic voter drive
/// ratios are on Schedule H1.
///
/// # Versions
///
/// v6.1–8.5 have all 9 fields. v1–5.x add an `exempt_activity` box and put
/// the transaction ID last. Paper layouts have no transaction ID but an
/// `image_number`.
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ScheduleH2 {
    /// The row type as filed, `H2`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `Sch H2`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this record, unique for the life of the report;
    /// electronic-only. Column `transaction_id` (field 3).
    pub transaction_id: Option<String>,
    /// "Activity or Event Identifier": the name H3 and H4 rows refer to.
    /// Column `activity_event_name` (field 4).
    pub activity_event_name: Option<String>,
    /// The activity is direct fundraising. Mutually exclusive with
    /// `direct_candidate_support`; an event in both is listed twice
    /// ([fecfrm3xi.pdf p27](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=27)).
    /// Column `direct_fundraising`, `X` when true (field 5).
    pub direct_fundraising: bool,
    /// The activity is direct candidate support. Column
    /// `direct_candidate_support` (field 6).
    pub direct_candidate_support: bool,
    /// v1–5.x only: the activity is an exempt activity. Column
    /// `exempt_activity`.
    pub exempt_activity: bool,
    /// `N`, `R` or `S` (FEC format workbook v8.4, sheet `Sch H2`, field 7),
    /// kept raw: the paper form's three boxes are "New", "Revised" and "Same
    /// as Previously Reported"
    /// ([fecfrm3x.pdf p14](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=14)),
    /// but no source states which code is which. Column `ratio_code`.
    pub ratio_code: Option<String>,
    /// The federal percentage, as filed: a fraction, `.5000` for 50% (the
    /// workbook's sample is `0.5`, field 8; every non-blank value in the
    /// corpus is between 0 and 1). Column `federal_percentage`.
    pub federal_percent: Option<f64>,
    /// The nonfederal percentage, a fraction like `federal_percent`. Column
    /// `nonfederal_percentage` (field 9).
    pub nonfederal_percent: Option<f64>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl ScheduleH2 {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id"),
            activity_event_name: text(data, "activity_event_name"),
            direct_fundraising: flag(data, "direct_fundraising"),
            direct_candidate_support: flag(data, "direct_candidate_support"),
            exempt_activity: flag(data, "exempt_activity"),
            ratio_code: text(data, "ratio_code"),
            federal_percent: amount_opt(data, "federal_percentage"),
            nonfederal_percent: amount_opt(data, "nonfederal_percentage"),
            image_number: text(data, "image_number"),
        })
    }
}
