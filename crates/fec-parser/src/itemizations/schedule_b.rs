//! Schedule B: itemized disbursements.

use jiff::civil::Date;

use crate::covers::fields::{amount, amount_opt, date, flag, text, text_or_empty, Fields};
use crate::covers::Address;
use crate::itemizations::{address_either, category_code_label, text_any, CandidateRef, Entity};

/// "SCHEDULE B - ITEMIZED DISBURSEMENTS": one disbursement a committee
/// itemizes, with the payee's name and mailing address and the date, amount
/// and purpose of the disbursement
/// ([fecfrm3i.pdf p9](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=9),
/// [fecfrm3xi.pdf p13](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=13)).
/// Filed with Forms 3, 3X, 3P and 3L (FEC format workbook v8.4, sheet `Sch B`,
/// field 1); on Form 3L the row type is `SB3L` and the row discloses a refund
/// of bundled contributions
/// ([fecfrm3li.pdf p6](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=6)).
///
/// The summary-page line the disbursement supports is carried in the row
/// type: `SB17` is line 17 (see [`crate::itemizations::line_number`]), and
/// the line belongs to the parent form (line 17 is operating expenditures on
/// Form 3 but "Other Federal Receipts" on Form 3X;
/// [fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5),
/// [fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)).
/// A committee itemizes payments to a payee once they aggregate over $200 in
/// the election cycle (Forms 3, 3P) or calendar year (Form 3X)
/// ([fecfrm3i.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=5),
/// [fecfrm3xi.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=7)),
/// so summing a filing's Schedule B rows does not give its total
/// disbursements. Unlike Schedule A there is no aggregate column.
///
/// # Versions
///
/// v8.0–8.5 have all 44 fields. v6.4–7.0 add `expenditure_purpose_code`;
/// v6.1–6.3 also have `refund_or_disposal_of_excess` and
/// `communication_date` but not `semi_annual_refunded_bundled_amt` (and v6.1
/// has no `beneficiary_committee_name`). v1–5.x have one combined
/// `payee_name` and `beneficiary_candidate_name`, split per
/// [`Entity::from_prefixed`] (v5.1–5.3 also carry the split payee columns at
/// the end of the row, which win when filled). Paper layouts have no
/// transaction IDs or entity type but carry an `image_number`.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        module = "libfec.itemizations",
        frozen,
        get_all,
        skip_from_py_object
    )
)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScheduleB {
    /// The row type as filed, `SB` plus the line number: `SB17`, `SB21B`,
    /// `SB3L`. Column `form_type` (FEC format workbook v8.4, sheet `Sch B`,
    /// field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transaction, required, electronic-only, and
    /// unique for the life of the report (original and all amendments).
    /// Column `transaction_id_number` (field 3).
    pub transaction_id: Option<String>,
    /// The `transaction_id` of a related record this one is a memo or child
    /// of. Column `back_reference_tran_id_number` (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// The schedule of that related record (`SB21`, …). Column
    /// `back_reference_sched_name` (field 5).
    pub back_reference_schedule_name: Option<String>,
    /// Who was paid: entity type, organization or person, mailing address.
    /// Columns `entity_type`, `payee_organization_name`, `payee_last_name` …
    /// `payee_suffix`, `payee_street_1` … `payee_zip_code` (fields 6–17);
    /// v1–5.x `payee_name`.
    pub payee: Entity,
    /// The election a contribution to a candidate is for: a letter and a
    /// year, `P2024` (see [`ScheduleB::election_code_label`]). The election
    /// boxes "should not be used when itemizing operating expenditures"
    /// ([fecfrm3xi.pdf p14](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=14)),
    /// though many filers fill them anyway. Column `election_code` (field 18).
    pub election_code: Option<String>,
    /// Required when `election_code` is `O…` (Other). Column
    /// `election_other_description` (field 19).
    pub election_other_description: Option<String>,
    /// Date of disbursement. Column `expenditure_date` (field 20).
    pub expenditure_date: Option<Date>,
    /// "Amount of Each Disbursement this Period"; on Form 3L the bundled
    /// refund amount (FEC format workbook v8.4, sheet `Sch B`, field 21).
    /// Negative for a refund received by return of an uncashed check
    /// ([fecfrm3i.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=8))
    /// or a prepaid independent expenditure moved to Schedule E
    /// ([fecfrm3xi.pdf p22](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=22)).
    /// Column `expenditure_amount`.
    pub expenditure_amount: f64,
    /// "Used for F3L only. Semi-annual Bundled Refund." (FEC format workbook
    /// v8.4, sheet `Sch B`, field 22). On rows that are not `SB3L` filers
    /// mostly leave it blank or write `0.00` (every v6.4–7.0 row in the test
    /// corpus has `0.00`); a few put what looks like an aggregate here, so
    /// read it only on `SB3L`. Column `semi_annual_refunded_bundled_amt`
    /// (v6.4 on).
    pub semi_annual_refunded_bundled_amount: Option<f64>,
    /// A coded purpose, v1–7.0 only (dropped in v8.0). No source in the
    /// format documentation defines its codes; values seen are category-like
    /// (`001`, `003`) in v3–5.0 and line-like (`24U`, `24K`, `20C`) later.
    /// Column `expenditure_purpose_code`.
    pub expenditure_purpose_code: Option<String>,
    /// "Purpose of Disbursement": a brief description of why the
    /// disbursement was made, such as "media", "salary", "polling" or
    /// "travel"
    /// ([fecfrm3xi.pdf p13](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=13)).
    /// Column `expenditure_purpose_descrip` (field 23).
    pub expenditure_purpose_description: Option<String>,
    /// A coarse category of the disbursement, `001`–`012` for
    /// non-Presidential committees and `101`–`107` for Presidential ones
    /// (FEC format workbook v8.4, sheet `Sch B`, field 24); see
    /// [`ScheduleB::category_code_label`]. Not a substitute for the purpose.
    /// Column `category_code`.
    pub category_code: Option<String>,
    /// FEC ID of the committee receiving the disbursement, used when the
    /// payee is a `CCM`, `PAC` or `PTY`. Column `beneficiary_committee_fec_id`
    /// (field 25).
    pub beneficiary_committee_fec_id: Option<String>,
    /// Column `beneficiary_committee_name` (field 26; v6.2 on).
    pub beneficiary_committee_name: Option<String>,
    /// The candidate a contribution is to, used when the payee is a `CAN` or
    /// `CCM`. Columns `beneficiary_candidate_fec_id`,
    /// `beneficiary_candidate_last_name` … `beneficiary_candidate_district`
    /// (fields 27–35); v1–5.x `beneficiary_candidate_name`.
    pub beneficiary_candidate: CandidateRef,
    /// The conduit an earmarked disbursement went through. Column
    /// `conduit_name` (field 36).
    pub conduit_name: Option<String>,
    /// Columns `conduit_street_1` … `conduit_zip_code` (fields 37–41).
    pub conduit_address: Address,
    /// True for a memo entry: an informational entry that does not affect
    /// cash on hand ("contributor attribution, conduit information, ultimate
    /// payee information, etc.";
    /// [fecfrm3i.pdf p9](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=9))
    /// and does not count toward the schedule's or summary page's totals
    /// ([partygui.pdf p213](https://www.fec.gov/resources/cms-content/documents/policy-guidance/partygui.pdf#page=213)).
    /// Column `memo_code`, `X` when true (field 42).
    pub memo: bool,
    /// Column `memo_text_description` (field 43).
    pub memo_text: Option<String>,
    /// A Schedule I or L system code tying the disbursement to an account.
    /// Column `reference_to_si_or_sl_system_code_that_identifies_the_account`
    /// (field 44).
    pub reference_code: Option<String>,
    /// A checkbox, `X` when set, in v1–6.3 and early paper layouts; the
    /// format documentation does not describe it beyond its column name.
    /// Column `refund_or_disposal_of_excess`.
    pub refund_or_disposal_of_excess: bool,
    /// A date in v5.0–6.3 layouts, undocumented (and blank in every filing of
    /// the test corpus). Column `communication_date`.
    pub communication_date: Option<Date>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl ScheduleB {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text_any(data, &["transaction_id_number", "transaction_id"]),
            back_reference_transaction_id: text(data, "back_reference_tran_id_number"),
            back_reference_schedule_name: text(data, "back_reference_sched_name"),
            payee: Entity::from_prefixed(data, "payee_", "payee_name"),
            election_code: text(data, "election_code"),
            election_other_description: text(data, "election_other_description"),
            expenditure_date: date(data, "expenditure_date"),
            expenditure_amount: amount(data, "expenditure_amount"),
            semi_annual_refunded_bundled_amount: amount_opt(
                data,
                "semi_annual_refunded_bundled_amt",
            ),
            expenditure_purpose_code: text(data, "expenditure_purpose_code"),
            expenditure_purpose_description: text(data, "expenditure_purpose_descrip"),
            category_code: text(data, "category_code"),
            beneficiary_committee_fec_id: text(data, "beneficiary_committee_fec_id"),
            beneficiary_committee_name: text(data, "beneficiary_committee_name"),
            beneficiary_candidate: CandidateRef::from_prefixed(
                data,
                "beneficiary_candidate_fec_id",
                "beneficiary_candidate_",
            ),
            conduit_name: text(data, "conduit_name"),
            conduit_address: address_either(data, "conduit_"),
            memo: flag(data, "memo_code"),
            memo_text: text(data, "memo_text_description"),
            reference_code: text_any(
                data,
                &[
                    "reference_to_si_or_sl_system_code_that_identifies_the_account",
                    "reference_code",
                ],
            ),
            refund_or_disposal_of_excess: flag(data, "refund_or_disposal_of_excess"),
            communication_date: date(data, "communication_date"),
            image_number: text(data, "image_number"),
        })
    }

    /// "Primary", "General", … for `election_code`; see
    /// [`crate::covers::election_code_label`].
    pub fn election_code_label(&self) -> Option<&'static str> {
        crate::covers::election_code_label(self.election_code.as_deref()?)
    }

    /// See [`crate::itemizations::category_code_label`].
    pub fn category_code_label(&self) -> Option<&'static str> {
        category_code_label(self.category_code.as_deref()?)
    }

    /// The summary-page line, `17`; see [`crate::itemizations::line_number`].
    pub fn line_number(&self) -> Option<&str> {
        crate::itemizations::line_number(&self.form_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_codes() {
        assert_eq!(
            category_code_label("001"),
            Some("Administrative/Salary/Overhead Expenses")
        );
        assert_eq!(
            category_code_label("1"),
            Some("Administrative/Salary/Overhead Expenses")
        );
        assert_eq!(category_code_label(" 012 "), Some("Donations"));
        assert_eq!(
            category_code_label("101"),
            Some("Expenses that are not Allocable")
        );
        assert_eq!(category_code_label("24U"), None);
        assert_eq!(category_code_label(""), None);
    }
}
