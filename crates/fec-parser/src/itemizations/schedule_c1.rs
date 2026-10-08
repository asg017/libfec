//! Schedule C-1: loans and lines of credit from lending institutions.

use jiff::civil::Date;

use crate::covers::fields::{
    amount, amount_opt, date, person_name_or_legacy, text, text_or_empty, Fields,
};
use crate::covers::{Address, PersonName};
use crate::itemizations::schedule_c::{terms_date, yes_no};
use crate::itemizations::{text_any, Entity};

/// "SCHEDULE C-1 - LOANS AND LINES OF CREDIT FROM LENDING INSTITUTIONS": the
/// terms of a loan or line of credit from a bank or other lending
/// institution, filed so the Commission can verify the loan is not a
/// prohibited contribution from the lender
/// ([fecfrm3xi.pdf p18](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=18)).
/// An authorized committee also files one when the candidate takes such a
/// loan or line of credit for the campaign
/// ([fecfrm3i.pdf p13](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3i.pdf#page=13)).
///
/// Unlike Schedule C it is not repeated on every report: it is filed when the
/// loan is obtained and each time it is restructured, and for a line of
/// credit also with each draw
/// ([fecfrm3xi.pdf p18](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=18)).
/// Each row is a child of a [`crate::itemizations::ScheduleC`] row:
/// `back_reference_transaction_id` is that row's `transaction_id` (FEC
/// format workbook v8.4, sheet `Sch C1`, field 4).
///
/// The fields follow the form's lettered sections: A restructured loan, B
/// draws on a line of credit, C others liable, D traditional collateral, E
/// future receipts as collateral, F other basis for the loan, G the
/// treasurer's signature, and the lender's certification signed by an
/// authorized representative (workbook fields 15–48;
/// [fecfrm3xi.pdf p18–19](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=18)).
/// The quoted questions below are printed on the form
/// ([fecfrm3x.pdf p9](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=9)).
///
/// # Amounts
///
/// `loan_amount` is the record's amount (`f64`, blank `0.0`; required,
/// workbook field 11). The others answer one section each and are
/// `Option<f64>`: blank means the section does not apply.
///
/// # Terms
///
/// The incurred date is `YYYYMMDD` (workbook field 13) and typed. The due
/// date and interest rate are 15-character text (fields 12 and 14) that
/// filers fill with `NONE`, `Monthly Payment`, `3/31/2025`, `.07`,
/// `WSJ Prime`, …; they are kept as written, and
/// [`ScheduleC1::loan_due_date`] reads the due date when it is one.
///
/// # Versions
///
/// v6.1–8.5 have all 48 fields. v2–5.x have no transaction ID, an
/// `entity_type`, and one combined `treasurer_name` and `authorized_name`
/// (split per [`crate::covers::split_legacy_name`]); v2 has no
/// `authorized_date`. v1 links to its Schedule C row with a
/// `reference_id_number` "Same as Ref ID on Schedule C" (FEC format v1,
/// `Fec_v1.rtf`, Schedule C1 field 3), read as
/// `back_reference_transaction_id`. Paper layouts have neither ID but carry
/// an `image_number`.
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
pub struct ScheduleC1 {
    /// The row type as filed, `SC1/` plus the line number: `SC1/10`. Column
    /// `form_type` (FEC format workbook v8.4, sheet `Sch C1`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this record; "may be a combination of Parent SC/
    /// TranID + a unique ID for this Child SC1/ record". Column
    /// `transaction_id_number` (field 3; v6.1+).
    pub transaction_id: Option<String>,
    /// The `transaction_id` of the Schedule C loan this record details.
    /// Column `back_reference_tran_id_number` (field 4); v1
    /// `reference_id_number`.
    pub back_reference_transaction_id: Option<String>,
    /// The lending institution: name and mailing address. Columns
    /// `lender_organization_name`, `lender_street_1` … `lender_zip_code`
    /// (fields 5–10), and `entity_type` in v2–5.x.
    pub lender: Entity,
    /// "Amount of Loan"; for a restructured loan, the amount under the new
    /// terms
    /// ([fecfrm3xi.pdf p18](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=18)).
    /// Column `loan_amount` (field 11).
    pub loan_amount: f64,
    /// "Interest Rate (APR)", as written (see the struct's *Terms*). Column
    /// `loan_interest_rate` (field 12, `A/N-15`).
    pub loan_interest_rate_terms: Option<String>,
    /// "Date Incurred or Established". Column `loan_incurred_date` (field 13,
    /// `YYYYMMDD`).
    pub loan_incurred_date: Option<Date>,
    /// "Date Due", as written (see the struct's *Terms*). Column
    /// `loan_due_date` (field 14, `A/N-15`).
    pub loan_due_date_terms: Option<String>,
    /// A. "Has loan been restructured?" Column `loan_restructured` (field 15,
    /// `Y,N`; `Y` → `Some(true)`, `N` → `Some(false)`, else `None`).
    pub loan_restructured: Option<bool>,
    /// A. "If yes, date originally incurred". Column
    /// `loan_incurred_date_original` (field 16).
    pub loan_incurred_date_original: Option<Date>,
    /// B.1 "Amount of this Draw" on a line of credit. Column
    /// `credit_amount_this_draw` (field 17).
    pub credit_amount_this_draw: Option<f64>,
    /// B.2 "Total outstanding balance" of a line of credit: cumulative draws
    /// less repayments
    /// ([fecfrm3xi.pdf p18](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=18)).
    /// Column `total_balance` (field 18).
    pub total_balance: Option<f64>,
    /// C. "Are other parties secondarily liable for the debt incurred?"
    /// Column `others_liable` (field 19, `Y,N`).
    pub others_liable: Option<bool>,
    /// D. "Are any of the following pledged as collateral for the loan: real
    /// estate, personal property, goods, negotiable instruments,
    /// certificates of deposit, chattel papers, stocks, accounts
    /// receivable, cash on deposit, or other similar traditional
    /// collateral?" Column `collateral` (field 20, `Y,N`).
    pub collateral: Option<bool>,
    /// D.1 "If yes, specify". Column `description` (field 21).
    pub collateral_description: Option<String>,
    /// D.2 "What is the value of this collateral?" Column
    /// `collateral_value_amount` (field 22).
    pub collateral_value: Option<f64>,
    /// D.3 "Does the lender have a perfected security interest in it?"
    /// Column `perfected_interest` (field 23, `Y,N`).
    pub perfected_interest: Option<bool>,
    /// E.1 "Are any future contributions or future receipts of interest
    /// income, pledged as collateral for the loan?" Column `future_income`
    /// (field 24, `Y,N`).
    pub future_income: Option<bool>,
    /// E.2 "If yes, specify". Column `description_TODO_DUP` (field 25; the
    /// parser's name for the layout's second `description` column).
    pub future_income_description: Option<String>,
    /// E.3 "What is the estimated value?" Column `estimated_value` (field
    /// 26).
    pub future_income_estimated_value: Option<f64>,
    /// E.4 "Date account established": the depository account that "must be
    /// established" when future receipts are pledged. Column
    /// `established_date` (field 27).
    pub depository_account_established_date: Option<Date>,
    /// E.5 "Location of account". Column `account_location_name` (field 28).
    pub depository_account_location: Option<String>,
    /// E.6–E.10 the account's address. Columns `street_1`, `street_2`,
    /// `city`, `state`, `zip_code` (fields 29–33).
    pub depository_account_address: Address,
    /// E.11 "Date debtor authorized the Secretary of the U.S. Treasury to
    /// make direct deposits of public financing payments to the depository
    /// account", printed only on Schedule C-P-1
    /// ([fecfrm3p.pdf p12](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3p.pdf#page=12)).
    /// Column `deposit_acct_auth_date_presidential` (field 34).
    pub depository_account_authorization_date: Option<Date>,
    /// F. "If neither of the types of collateral described above was pledged
    /// for this loan, or if the amount pledged does not equal or exceed the
    /// loan amount, state the basis upon which this loan was made and the
    /// basis on which it assures repayment." Column
    /// `f_basis_of_loan_description` (field 35).
    pub basis_of_loan_description: Option<String>,
    /// G. "Type or Print Name of Committee Treasurer". Columns
    /// `treasurer_last_name` … `treasurer_suffix` (fields 36–40); v1–5.x
    /// `treasurer_name`.
    pub treasurer: PersonName,
    /// G. The treasurer's signature date. Column `date_signed` (field 41).
    pub date_signed: Option<Date>,
    /// The lender's "Authorized Representative" who signs its
    /// certification. Columns `authorized_last_name` … `authorized_suffix`
    /// (fields 42–46); v1–5.x `authorized_name`.
    pub authorized_representative: PersonName,
    /// The representative's title. Column `authorized_title` (field 47).
    pub authorized_representative_title: Option<String>,
    /// The representative's signature date. Column `authorized_date` (field
    /// 48).
    pub authorized_date: Option<Date>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl ScheduleC1 {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text(data, "transaction_id_number"),
            back_reference_transaction_id: text_any(
                data,
                &["back_reference_tran_id_number", "reference_id_number"],
            ),
            lender: Entity::from_prefixed(data, "lender_", "lender_name"),
            loan_amount: amount(data, "loan_amount"),
            loan_interest_rate_terms: text(data, "loan_interest_rate"),
            loan_incurred_date: date(data, "loan_incurred_date"),
            loan_due_date_terms: text(data, "loan_due_date"),
            loan_restructured: yes_no(data, "loan_restructured"),
            loan_incurred_date_original: date(data, "loan_incurred_date_original"),
            credit_amount_this_draw: amount_opt(data, "credit_amount_this_draw"),
            total_balance: amount_opt(data, "total_balance"),
            others_liable: yes_no(data, "others_liable"),
            collateral: yes_no(data, "collateral"),
            collateral_description: text(data, "description"),
            collateral_value: amount_opt(data, "collateral_value_amount"),
            perfected_interest: yes_no(data, "perfected_interest"),
            future_income: yes_no(data, "future_income"),
            future_income_description: text(data, "description_TODO_DUP"),
            future_income_estimated_value: amount_opt(data, "estimated_value"),
            depository_account_established_date: date(data, "established_date"),
            depository_account_location: text(data, "account_location_name"),
            depository_account_address: Address::from_prefixed(data, ""),
            depository_account_authorization_date: date(
                data,
                "deposit_acct_auth_date_presidential",
            ),
            basis_of_loan_description: text(data, "f_basis_of_loan_description"),
            treasurer: person_name_or_legacy(data, "treasurer_", "treasurer_name"),
            date_signed: date(data, "date_signed"),
            authorized_representative: person_name_or_legacy(
                data,
                "authorized_",
                "authorized_name",
            ),
            authorized_representative_title: text(data, "authorized_title"),
            authorized_date: date(data, "authorized_date"),
            image_number: text(data, "image_number"),
        })
    }

    /// `loan_due_date_terms` as a date when it is one: `YYYYMMDD` or
    /// `MM/DD/YYYY` (one-digit month and day accepted). `None` for `NONE`,
    /// `Monthly Payment`, two-digit years and other text.
    pub fn loan_due_date(&self) -> Option<Date> {
        terms_date(self.loan_due_date_terms.as_deref()?)
    }

    /// The summary-page line, `10`; see [`crate::itemizations::line_number`].
    pub fn line_number(&self) -> Option<&str> {
        crate::itemizations::line_number(&self.form_type)
    }
}
