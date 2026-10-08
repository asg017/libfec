//! Schedule C: loans.

use jiff::civil::Date;

use crate::covers::fields::{amount, date, flag, key, text, text_or_empty, Fields};
use crate::itemizations::{text_any, CandidateRef, Entity};

/// "SCHEDULE C - LOANS": one loan a committee has received or made, with the
/// lender (or debtor), the original amount, the cumulative payment to date,
/// the balance outstanding at the close of the period, and the loan's terms
/// ([fecfrm3xi.pdf p16](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=16)).
/// Filed with Forms 3, 3X, 3P (as Schedule C-P) and 4
/// ([fecfrm3.pdf p7](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf#page=7),
/// [fecfrm3p.pdf p11](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=11)).
///
/// The row type carries the summary-page line: `SC/10` is line 10, debts
/// owed *by* the committee, and `SC/9` line 9, debts owed *to* it, on Forms 3
/// and 3X ([fecfrm3xi.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=5);
/// Form 3P uses `SC/11` and `SC/12`, see fec-docs
/// `wiki/Itemization-Form-Types.md`). A loan "must continue to be reported
/// on each subsequent report until repaid"
/// ([fecfrm3xi.pdf p16](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=16)),
/// so the same loan reappears in every report: summing
/// `loan_amount_original` across a committee's filings counts one loan many
/// times. The receipt of a loan is itemized separately on Schedule A (a loan
/// made, on Schedule B); a loan from a lending institution also has a
/// [`crate::itemizations::ScheduleC1`], and its endorsers and guarantors are
/// [`crate::itemizations::ScheduleC2`] rows, both pointing back at this
/// row's `transaction_id`.
///
/// # Amounts
///
/// The three amounts are the schedule's three printed money boxes and are
/// all `f64`, blank read as `0.0`: each is required (FEC format workbook
/// v8.4, sheet `Sch C`, fields 19–21) and a blank box means nothing, the
/// way it does on the summary pages. None is "the" amount of the record;
/// `loan_balance` is the one carried forward, to Schedule D line 3 or the
/// summary page
/// ([fecfrm3xi.pdf p16](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=16)).
///
/// # Terms
///
/// The date incurred is a `YYYYMMDD` date in every version (workbook field
/// 22, `NUM-8`) and is typed. The due date and interest rate are 15-character
/// free text (fields 23–24): the instructions ask for "None" when there is no
/// due date and "0" for no interest
/// ([fecfrm3xi.pdf p16](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=16)),
/// and filers write `On Demand`, `12/31/2026`, `0.0500`, `5.00000` or
/// `Prime+2%`. They are kept as written; [`ScheduleC::loan_due_date`] reads
/// the due date when it is one.
///
/// # Versions
///
/// v6.2–8.5 have all 38 fields (v6.1 lacks `personal_funds` and the memo).
/// v2–5.x have one combined `lender_name` and `lender_candidate_name`, split
/// per [`Entity::from_prefixed`], and `transaction_id_number` near the end.
/// v1 has a `reference_id_number` in its place ("used to relate Schedule C-1
/// records ... to the appropriate Schedule C record", FEC format v1,
/// `Fec_v1.rtf`, Schedule C field 3) and lists up to three endorsers or
/// guarantors inline, read into `guarantors`. Paper layouts have no
/// transaction ID or entity type but carry an `image_number`.
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
#[cfg_attr(feature = "columnar", derive(fec_parser_macros::Columnar))]
pub struct ScheduleC {
    /// The row type as filed, `SC/` plus the line number: `SC/10`. Column
    /// `form_type` (FEC format workbook v8.4, sheet `Sch C`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this loan, unique for the life of the report; the
    /// `back_reference_transaction_id` of its Schedule C-1 and C-2 rows.
    /// Column `transaction_id_number` (field 3); v1 `reference_id_number`.
    pub transaction_id: Option<String>,
    /// The Detailed Summary line the loan was received on: `13A` / `13B`
    /// (Form 3 line 13(a) / 13(b)), `13` (Form 3X), `19A` / `19B` (Form 3P).
    /// Required on Form 3 / 3X `SC/10` and Form 3P `SC/12` rows. Column
    /// `receipt_line_number` (field 4).
    pub receipt_line_number: Option<String>,
    /// The loan source (or, for a loan the committee made, the debtor):
    /// entity type, organization or person, mailing address. Columns
    /// `entity_type`, `lender_organization_name`, `lender_last_name` …
    /// `lender_suffix`, `lender_street_1` … `lender_zip_code` (fields 5–16);
    /// v1–5.x `lender_name`.
    pub lender: Entity,
    /// The election the loan is for: a letter and a year, `P2024` (see
    /// [`ScheduleC::election_code_label`]). Column `election_code` (field
    /// 17).
    pub election_code: Option<String>,
    /// Required when `election_code` is `O…` (Other). Column
    /// `election_other_description` (field 18).
    pub election_other_description: Option<String>,
    /// "Original Amount of Loan". Column `loan_amount_original` (field 19).
    pub loan_amount_original: f64,
    /// "Cumulative Payment To Date": every repayment so far, not only this
    /// period's ([fecfrm3xi.pdf p16](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=16)).
    /// Column `loan_payment_to_date` (field 20).
    pub loan_payment_to_date: f64,
    /// "Balance Outstanding at Close of This Period". Column `loan_balance`
    /// (field 21).
    pub loan_balance: f64,
    /// "Date Incurred". Column `loan_incurred_date_terms` (field 22,
    /// `YYYYMMDD`).
    pub loan_incurred_date: Option<Date>,
    /// "Date Due", as written: a date in any format, `None`, `On Demand`, …
    /// (see the struct's *Terms*). Column `loan_due_date_terms` (field 23,
    /// `A/N-15`).
    pub loan_due_date_terms: Option<String>,
    /// "Interest Rate (if none, enter 0)", as written: `0.0500`, `5.00000`,
    /// `Prime+2%`, … (see the struct's *Terms*). Column
    /// `loan_interest_rate_terms` (field 24, `A/N-15`).
    pub loan_interest_rate_terms: Option<String>,
    /// "Secured": `Y` → `Some(true)`, `N` → `Some(false)`, anything else
    /// `None`. Column `secured` (field 25, `Y,N`).
    pub secured: Option<bool>,
    /// "Personal Funds of the Candidate", printed on Forms 3 and 3P: `Y` →
    /// `Some(true)`, `N` → `Some(false)`, anything else `None`. Column
    /// `personal_funds` (field 26, `Y,N`; v6.2+).
    pub personal_funds: Option<bool>,
    /// FEC ID of a lending committee (entity `CCM`, `COM`, `PAC`, `PTY`).
    /// Column `lender_committee_id_number` (field 27).
    pub lender_committee_id: Option<String>,
    /// A lending candidate (entity `CAN`, `CCM`). Columns
    /// `lender_candidate_id_number`, `lender_candidate_last_name` …
    /// `lender_candidate_district` (fields 28–36; the middle name's column is
    /// `lender_candidate_middle_nm`); v2–5.x `lender_candidate_name`.
    pub lender_candidate: CandidateRef,
    /// True for a memo entry. Column `memo_code`, `X` when true (field 37).
    pub memo: bool,
    /// Column `memo_text_description` (field 38).
    pub memo_text: Option<String>,
    /// The endorsers or guarantors listed on the loan row itself, v1 only
    /// (FEC format v1, `Fec_v1.rtf`, Schedule C fields 19–45: three blocks
    /// of name, address, employer, occupation and "AMT GUARANTEED BAL").
    /// Blocks left blank are skipped. From v2 on each one is a separate
    /// [`crate::itemizations::ScheduleC2`] row, and this is empty.
    pub guarantors: Vec<ScheduleCGuarantor>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

/// One endorser or guarantor listed inline on a v1 Schedule C row
/// (FEC format v1, `Fec_v1.rtf`, Schedule C fields 19–27, 28–36, 37–45).
/// The same facts as a [`crate::itemizations::ScheduleC2`] row.
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
#[cfg_attr(feature = "columnar", derive(fec_parser_macros::Columnar))]
pub struct ScheduleCGuarantor {
    /// Name and address. Columns `guarantor_N_name` ("IND/NAME
    /// (ENDORSER/GUARANTOR)", split per [`Entity::from_prefixed`]),
    /// `guarantor_N_street_1` … `guarantor_N_zip_code`.
    pub guarantor: Entity,
    /// Column `guarantor_N_employer` ("INDEMP").
    pub employer: Option<String>,
    /// Column `guarantor_N_occupation` ("INDOCC").
    pub occupation: Option<String>,
    /// The amount guaranteed outstanding. Column
    /// `guarantor_N_guaranteed_amount` ("AMT GUARANTEED BAL AMOUNT").
    pub guaranteed_amount: f64,
}

impl ScheduleC {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        let mut lender_candidate =
            CandidateRef::from_prefixed(data, "lender_candidate_id_number", "lender_candidate_");
        if lender_candidate.name.middle_name.is_none() {
            lender_candidate.name.middle_name = text(data, "lender_candidate_middle_nm");
        }
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text_any(data, &["transaction_id_number", "reference_id_number"]),
            receipt_line_number: text(data, "receipt_line_number"),
            lender: Entity::from_prefixed(data, "lender_", "lender_name"),
            election_code: text(data, "election_code"),
            election_other_description: text(data, "election_other_description"),
            loan_amount_original: amount(data, "loan_amount_original"),
            loan_payment_to_date: amount(data, "loan_payment_to_date"),
            loan_balance: amount(data, "loan_balance"),
            loan_incurred_date: date(data, "loan_incurred_date_terms"),
            loan_due_date_terms: text(data, "loan_due_date_terms"),
            loan_interest_rate_terms: text(data, "loan_interest_rate_terms"),
            secured: yes_no(data, "secured"),
            personal_funds: yes_no(data, "personal_funds"),
            lender_committee_id: text(data, "lender_committee_id_number"),
            lender_candidate,
            memo: flag(data, "memo_code"),
            memo_text: text(data, "memo_text_description"),
            guarantors: ["guarantor_1_", "guarantor_2_", "guarantor_3_"]
                .into_iter()
                .filter_map(|prefix| ScheduleCGuarantor::from_prefixed(data, prefix))
                .collect(),
            image_number: text(data, "image_number"),
        })
    }

    /// `loan_due_date_terms` as a date when it is one: `YYYYMMDD` or
    /// `MM/DD/YYYY` (one-digit month and day accepted). `None` for `None`,
    /// `On Demand`, two-digit years and other text.
    pub fn loan_due_date(&self) -> Option<Date> {
        terms_date(self.loan_due_date_terms.as_deref()?)
    }

    /// "Primary", "General", … for `election_code`; see
    /// [`crate::covers::election_code_label`].
    pub fn election_code_label(&self) -> Option<&'static str> {
        crate::covers::election_code_label(self.election_code.as_deref()?)
    }

    /// The summary-page line, `10`; see [`crate::itemizations::line_number`].
    pub fn line_number(&self) -> Option<&str> {
        crate::itemizations::line_number(&self.form_type)
    }
}

impl ScheduleCGuarantor {
    /// The `{prefix}name`, … block, or `None` if every column is blank.
    fn from_prefixed<F: Fields + ?Sized>(data: &F, prefix: &str) -> Option<Self> {
        let guarantor = Entity::from_prefixed(data, prefix, &key(prefix, "name"));
        let employer = text(data, &key(prefix, "employer"));
        let occupation = text(data, &key(prefix, "occupation"));
        let guaranteed_amount = text(data, &key(prefix, "guaranteed_amount"));
        let blank = guarantor.organization_name.is_none()
            && guarantor.name.is_empty()
            && guarantor.address.is_empty()
            && employer.is_none()
            && occupation.is_none()
            && guaranteed_amount.is_none();
        (!blank).then(|| Self {
            guarantor,
            employer,
            occupation,
            guaranteed_amount: amount(data, &key(prefix, "guaranteed_amount")),
        })
    }
}

/// A `Y,N` column ("Edit: Yes/No" in the FEC format workbook): `Y` →
/// `Some(true)`, `N` → `Some(false)`, blank or anything else `None`.
pub(crate) fn yes_no<F: Fields + ?Sized>(data: &F, key: &str) -> Option<bool> {
    match data.raw(key)?.trim() {
        "Y" | "y" => Some(true),
        "N" | "n" => Some(false),
        _ => None,
    }
}

/// A free-text loan date ("terms") read as a date when it is one:
/// `YYYYMMDD` or `M/D/YYYY`.
pub(crate) fn terms_date(raw: &str) -> Option<Date> {
    let raw = raw.trim();
    if let Ok(d) = Date::strptime("%Y%m%d", raw) {
        return Some(d);
    }
    let mut parts = raw.split('/');
    let (m, d, y) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || y.len() != 4 || m.is_empty() || d.is_empty() {
        return None;
    }
    Date::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_dates() {
        assert_eq!(
            terms_date("20261231"),
            Some(jiff::civil::date(2026, 12, 31))
        );
        assert_eq!(
            terms_date("12/31/2026"),
            Some(jiff::civil::date(2026, 12, 31))
        );
        assert_eq!(terms_date("3/1/2025"), Some(jiff::civil::date(2025, 3, 1)));
        assert_eq!(terms_date("1/30/05"), None);
        assert_eq!(terms_date("On Demand"), None);
        assert_eq!(terms_date("None"), None);
        assert_eq!(terms_date("13/40/2025"), None);
    }
}
