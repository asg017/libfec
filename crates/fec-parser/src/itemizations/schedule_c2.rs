//! Schedule C-2: endorsers and guarantors of a loan.

use crate::covers::fields::{amount, person_name_or_legacy, text, text_or_empty, Fields};
use crate::itemizations::{address_either, Entity};

/// "SCHEDULE C2 - LOAN GUARANTOR NAME & ADDRESS INFORMATION (SUPPLEMENTARY
/// FOR INFORMATION FOUND ON SCHEDULE C)": one endorser or guarantor of a loan
/// reported on Schedule C, with the amount of the guarantee outstanding at
/// the close of the period (FEC format workbook v8.4, sheet `Sch C2`;
/// [fecfrm3xi.pdf p16](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=16)).
/// On paper these are the "List All Endorsers or Guarantors (if any) to
/// Loan Source" blocks of Schedule C itself
/// ([fecfrm3x.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=8)).
///
/// Each row is a child of a [`crate::itemizations::ScheduleC`] row:
/// `back_reference_transaction_id` is that row's `transaction_id`
/// (workbook field 4). "A loan is a contribution by each endorser or
/// guarantor" for the portion each agreed to be liable for
/// ([fecfrm3xi.pdf p17](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=17)).
/// The first guarantor of a loan reported through an intermediary may be
/// the loan's original source instead
/// ([fecfrm3xi.pdf p16](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=16)).
///
/// # Versions
///
/// v6.1–8.4 have 17 fields, a guarantor identified by a person's name only.
/// v8.5 adds three columns, `guarantor_entity`, `guarantor_organization_name`
/// and `guarantor_committee_fec_id`, read into `guarantor.entity_type`,
/// `guarantor.organization_name` and `guarantor_committee_fec_id`; no v8.5
/// format specification is in fec-docs, so their meaning is the columns'
/// names (filers use the entity codes `IND`, `CAN`, `CCM`, `ORG`). v2–5.x
/// have no transaction ID and one combined `guarantor_name`. v1 has no
/// Schedule C-2: its guarantors are on the Schedule C row
/// ([`crate::itemizations::ScheduleC::guarantors`]). Paper layouts carry an
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
pub struct ScheduleC2 {
    /// The row type as filed, `SC2/` plus the line number: `SC2/10`. Column
    /// `form_type` (FEC format workbook v8.4, sheet `Sch C2`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this record. Column `transaction_id_number` (field
    /// 3; v6.1+).
    pub transaction_id: Option<String>,
    /// The `transaction_id` of the Schedule C loan guaranteed. Column
    /// `back_reference_tran_id_number` (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// The endorser or guarantor: name and mailing address. Columns
    /// `guarantor_last_name` … `guarantor_suffix`, `guarantor_street_1` …
    /// `guarantor_zip_code` (fields 5–14); v2–5.x `guarantor_name`; v8.5
    /// also `guarantor_entity` and `guarantor_organization_name`.
    pub guarantor: Entity,
    /// An FEC committee ID for the guarantor, v8.5 only. Column
    /// `guarantor_committee_fec_id`.
    pub guarantor_committee_fec_id: Option<String>,
    /// "Name of Employer", for an individual; flagged by the workbook when
    /// the amount is over $200. Column `guarantor_employer` (field 15).
    pub guarantor_employer: Option<String>,
    /// "Occupation". Column `guarantor_occupation` (field 16).
    pub guarantor_occupation: Option<String>,
    /// "Amount Guaranteed Outstanding" at the close of the period, not a
    /// payment
    /// ([fecfrm3xi.pdf p16](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=16));
    /// the record's amount, blank read as `0.0`. Column `guaranteed_amount`
    /// (field 17).
    pub guaranteed_amount: f64,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl ScheduleC2 {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id_number"),
            back_reference_transaction_id: text(data, "back_reference_tran_id_number"),
            guarantor: Entity {
                entity_type: text(data, "guarantor_entity"),
                organization_name: text(data, "guarantor_organization_name"),
                name: person_name_or_legacy(data, "guarantor_", "guarantor_name"),
                address: address_either(data, "guarantor_"),
            },
            guarantor_committee_fec_id: text(data, "guarantor_committee_fec_id"),
            guarantor_employer: text(data, "guarantor_employer"),
            guarantor_occupation: text(data, "guarantor_occupation"),
            guaranteed_amount: amount(data, "guaranteed_amount"),
            image_number: text(data, "image_number"),
        })
    }

    /// The summary-page line, `10`; see [`crate::itemizations::line_number`].
    pub fn line_number(&self) -> Option<&str> {
        crate::itemizations::line_number(&self.form_type)
    }
}
