//! Schedule D: debts and obligations.

use crate::covers::fields::{amount, text, text_or_empty, Fields};
use crate::covers::Address;
use crate::itemizations::{address_either, CandidateRef, Entity};

/// "SCHEDULE D - DEBTS AND OBLIGATIONS": one debt or obligation, other than
/// a loan, owed by or to the committee, with the creditor or debtor, the
/// nature of the debt, and its running account for the period: the balance
/// at the beginning, the amount incurred, the payments and the balance at
/// the close
/// ([fecfrm3xi.pdf p20](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=20)).
/// Filed with Forms 3, 3X, 3P (as Schedule D-P) and 4
/// ([fecfrm3.pdf p9](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=9),
/// [fecfrm3x.pdf p10](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=10),
/// [fecfrm3p.pdf p14](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=14),
/// [fecfrm4.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm4.pdf#page=7)).
///
/// The row type carries the summary-page line (see
/// [`crate::itemizations::line_number`]): on Forms 3, 3X and 4 `SD9` is a
/// debt owed *to* the committee and `SD10` one owed *by* it (Summary Page
/// Lines 9 and 10,
/// [fecfrm3xi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=5));
/// Form 3P uses `SD11` and `SD12` (FEC format specification v8.4,
/// `FEC_Format_v8.4.pdf` Appendix A p18–19; fec-docs
/// `wiki/Itemization-Form-Types.md`).
///
/// A debt is reported on every report until it is extinguished or settled
/// ([fecfrm3xi.pdf p20](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=20)),
/// so the same debt reappears filing after filing: do not sum balances
/// across filings. The rows are obligations, not cash movements; a payment
/// on a debt is itemized on Schedule B (or, received, Schedule A) and
/// included in the period's payment here (same page). The layout has no
/// memo code (FEC format workbook v8.4, sheet `Sch D`, fields 1–20).
///
/// # Amounts
///
/// The four balance columns are all `f64`, blank read as `0.0`: every one is
/// a box on the paper schedule (fields 17–20, "Outstanding Balance Beginning
/// This Period", "Amount Incurred This Period", "Payment This Period",
/// "Outstanding Balance at Close of This Period"), where an empty money box
/// means no activity, as on the covers' summary pages. Together they are one
/// period's account, so none of them is "the" amount of the record.
///
/// # Versions
///
/// v6.1–8.5 have all 20 fields. v3–5.x have one combined `creditor_name`,
/// split per [`Entity::from_prefixed`], and add a committee ID, a candidate
/// and a conduit (read into [`ScheduleD::committee_fec_id`],
/// [`ScheduleD::candidate`], [`ScheduleD::conduit_name`] and
/// [`ScheduleD::conduit_address`]); v1 has neither entity type nor
/// transaction ID. Paper layouts have no transaction ID or entity type but
/// carry an `image_number`.
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
pub struct ScheduleD {
    /// The row type as filed, `SD` plus the line number: `SD10`. Column
    /// `form_type` (FEC format workbook v8.4, sheet `Sch D`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this debt, unique for the life of the report
    /// (original and all amendments); electronic-only. Column
    /// `transaction_id_number` (field 3).
    pub transaction_id: Option<String>,
    /// The creditor (or, on `SD9`, the debtor): entity type, organization or
    /// person, mailing address. Columns `entity_type`,
    /// `creditor_organization_name`, `creditor_last_name` …
    /// `creditor_suffix`, `creditor_street_1` … `creditor_zip_code` (fields
    /// 4–15); v1–5.x `creditor_name`.
    pub creditor: Entity,
    /// "Nature of Debt (Purpose)". Column `purpose_of_debt_or_obligation`
    /// (field 16).
    pub purpose_of_debt_or_obligation: Option<String>,
    /// "Outstanding Balance Beginning This Period". Column
    /// `beginning_balance_this_period` (field 17); blank is `0.0`.
    pub beginning_balance: f64,
    /// "Amount Incurred This Period". Column `incurred_amount_this_period`
    /// (field 18); blank is `0.0`.
    pub incurred_amount: f64,
    /// "Payment This Period", the payments itemized on Schedule B (or A)
    /// ([fecfrm3xi.pdf p20](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=20)).
    /// Column `payment_amount_this_period` (field 19); blank is `0.0`.
    pub payment_amount: f64,
    /// "Outstanding Balance at Close of This Period", the amount carried to
    /// the summary page. Column `balance_at_close_this_period` (field 20);
    /// blank is `0.0`.
    pub balance_at_close: f64,
    /// A committee FEC ID, v3–5.x only (dropped in v6.1). Column
    /// `fec_committee_id_number`. Its role is not in the sources read for
    /// this crate (the v8.4 workbook has no such field); in the rows seen it is
    /// the creditor's ID when the creditor is a committee.
    pub committee_fec_id: Option<String>,
    /// A candidate, v3–5.x only (dropped in v6.1). Columns
    /// `fec_candidate_id_number`, `candidate_name`, `candidate_office`,
    /// `candidate_state`, `candidate_district`. Unsourced like
    /// `committee_fec_id`; in the rows seen it names the filer's own candidate.
    pub candidate: CandidateRef,
    /// A conduit, v3–5.x only (dropped in v6.1). Column `conduit_name`.
    /// Unsourced like `committee_fec_id`; in the data it is the card issuer
    /// of a debt charged to a credit card, the debt's `purpose` often
    /// prefixed `(MEMO)`.
    pub conduit_name: Option<String>,
    /// Columns `conduit_street_1` … `conduit_zip_code`, v3–5.x only.
    pub conduit_address: Address,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl ScheduleD {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id_number"),
            creditor: Entity::from_prefixed(data, "creditor_", "creditor_name"),
            purpose_of_debt_or_obligation: text(data, "purpose_of_debt_or_obligation"),
            beginning_balance: amount(data, "beginning_balance_this_period"),
            incurred_amount: amount(data, "incurred_amount_this_period"),
            payment_amount: amount(data, "payment_amount_this_period"),
            balance_at_close: amount(data, "balance_at_close_this_period"),
            committee_fec_id: text(data, "fec_committee_id_number"),
            candidate: CandidateRef::from_prefixed(data, "fec_candidate_id_number", "candidate_"),
            conduit_name: text(data, "conduit_name"),
            conduit_address: address_either(data, "conduit_"),
            image_number: text(data, "image_number"),
        })
    }

    /// The summary-page line, `10`; see [`crate::itemizations::line_number`].
    pub fn line_number(&self) -> Option<&str> {
        crate::itemizations::line_number(&self.form_type)
    }
}
