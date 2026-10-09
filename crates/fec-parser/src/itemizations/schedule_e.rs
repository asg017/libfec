//! Schedule E: itemized independent expenditures.

use jiff::civil::Date;

use crate::covers::fields::{
    amount, amount_opt, date, flag, person_name_or_legacy, split_legacy_name, text, text_or_empty,
    Fields,
};
use crate::covers::{Address, PersonName};
use crate::itemizations::{address_either, text_any, CandidateRef, Entity};

/// "SCHEDULE E - ITEMIZED INDEPENDENT EXPENDITURES": one independent
/// expenditure, i.e. spending on a communication "expressly advocating the
/// election or defeat of a clearly identified candidate" that is not made in
/// cooperation or consultation with, or at the request or suggestion of, a
/// candidate, the candidate's committee or a party committee, or their agents
/// ([fecfrm3xi.pdf p21](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=21)).
/// Each row names the payee, the dates, amount and purpose, and the candidate
/// the expenditure supports or opposes
/// ([fecfrm3xi.pdf p22](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=22)).
///
/// Filed by political committees on Form 3X, and in the 24/48-hour notices
/// of [`crate::covers::Form24`], which "must include all of the information
/// required on Schedule E"
/// ([fecfrm3xi.pdf p21](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=21));
/// persons other than political committees report the same spending on Form 5
/// instead
/// ([fecfrm3xi.pdf p22](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=22)).
/// The same expenditure can therefore appear twice: estimated on a 24/48-hour
/// notice, then reported on the next regular report's Schedule E
/// ([fecfrm3xi.pdf p21](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=21)),
/// so do not add the two together. The row type is plain `SE`, with no line
/// number (FEC format workbook v8.4, sheet `Sch E`, field 1); the schedule's
/// total goes to Form 3X Line 24
/// ([fecfrm3xi.pdf p8](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=8)).
///
/// # Versions
///
/// v8.1–8.5 have all 44 fields. v8.0 and earlier have one date column
/// (see [`ScheduleE::disbursement_date`]). v6.1–7.0 add
/// `expenditure_purpose_code`. v1–5.x have one combined `payee_name` and
/// `candidate_name` (split like legacy names in `Entity::from_prefixed`), a conduit, a
/// combined signer name `ind_name_as_signed` and notary columns; v2 also
/// names a payee candidate; v1 has no entity type. v1–5.2 also carry an
/// `amended_cd` column, which is not read: the v5.3 specification says the
/// schedules' amend codes are "unnecessary and the field is ignored"
/// (`FEC_v530.rtf`, "Amend Code"). Paper layouts have no transaction IDs,
/// entity type or candidate ID but carry an `image_number`.
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
pub struct ScheduleE {
    /// The row type as filed, `SE`. Column `form_type` (FEC format workbook
    /// v8.4, sheet `Sch E`, field 1).
    pub form_type: String,
    /// The filing committee's FEC ID. Column `filer_committee_id_number`
    /// (field 2).
    pub filer_committee_id: String,
    /// The filer's ID for this transaction, unique for the life of the
    /// report; electronic only. Column `transaction_id_number` (field 3).
    pub transaction_id: Option<String>,
    /// The `transaction_id` of a related record, e.g. the Schedule D debt of
    /// an expenditure disseminated before it was paid for (see
    /// [`ScheduleE::memo`]). Column `back_reference_tran_id_number`
    /// (field 4).
    pub back_reference_transaction_id: Option<String>,
    /// The schedule of that related record (`SD10`, …). Column
    /// `back_reference_sched_name` (field 5).
    pub back_reference_schedule_name: Option<String>,
    /// Who was paid: entity type, organization or person, mailing address.
    /// Columns `entity_type`, `payee_organization_name`, `payee_last_name` …
    /// `payee_suffix`, `payee_street_1` … `payee_zip_code` (fields 6–17);
    /// v1–5.x `payee_name`. A payee is itemized once payments to it for
    /// independent expenditures aggregate over $200 in the calendar year
    /// ([fecfrm3xi.pdf p21](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=21)).
    pub payee: Entity,
    /// The payee's FEC committee ID. Column `payee_cmtte_fec_id_number`
    /// (field 26, "PAYEE CMTTE FEC ID NUMBER", not on the paper form); the
    /// v2 format calls it "FEC COMMITTEE ID NUMBER (If Entity-ID = COM, CCM,
    /// PAC or PPO)" (`FEC_v2.rtf`, Schedule E field 19).
    pub payee_committee_fec_id: Option<String>,
    /// The election the expenditure is for, a letter and a year: `G2024`
    /// (see [`ScheduleE::election_code_label`]). Required on Schedule E.
    /// Column `election_code` (field 18).
    pub election_code: Option<String>,
    /// Required when `election_code` is `O…` (Other). Column
    /// `election_other_description` (field 19).
    pub election_other_description: Option<String>,
    /// "Date of Public Distribution/Dissemination"
    /// ([fecfrm3x.pdf p11](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=11)):
    /// when the communication reached the public, which is when the
    /// expenditure is "made" for the 24/48-hour reporting thresholds
    /// ([partygui.pdf p130](https://www.fec.gov/resources/cms-content/documents/policy-guidance/partygui.pdf#page=130)).
    /// Column `dissemination_date` (field 20); the format requires this or
    /// `disbursement_date`, not both (fields 20, 22). Added in v8.1
    /// (`FEC_Format_v8.1.pdf` Appendix D); `None` on earlier layouts.
    pub dissemination_date: Option<Date>,
    /// "Date of Disbursement or Obligation"
    /// ([fecfrm3x.pdf p11](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=11)):
    /// when the committee paid or became obligated to pay. Column
    /// `disbursement_date` (field 22).
    ///
    /// v8.0 and earlier have a single date ("EXPENDITURE DATE", FEC format
    /// workbook v8.0, sheet `Sch E`, field 20; "DATE" in v2–5.x), which the
    /// column mappings name `dissemination_date`. v8.1 added the
    /// dissemination date "in the place of" the old one and kept the old
    /// date, relabelled "Date of Disbursement or Obligation", after the
    /// amount (`FEC_Format_v8.1.pdf` Appendix D, FECPrint changes 11–12), so
    /// that single date is read here. Paper layouts without a
    /// `disbursement_date` column (P1–P3.0) are read the same way.
    pub disbursement_date: Option<Date>,
    /// The amount of the expenditure. Column `expenditure_amount` (field 21).
    pub expenditure_amount: f64,
    /// "The total amount expended in the aggregate during the calendar year,
    /// per election, per office sought"
    /// ([fecfrm3xi.pdf p22](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=22)):
    /// not a per-payee or per-candidate total. Column
    /// `calendar_y_t_d_per_election_office` (field 23; v5.3 and later).
    pub calendar_ytd_per_election_office: Option<f64>,
    /// "Purpose of Expenditure", free text. Column
    /// `expenditure_purpose_descrip` (field 24).
    pub expenditure_purpose_description: Option<String>,
    /// v5.x–7.0 only (dropped in v8.0): `24A` or `24E` ("EXPENDITURE PURPOSE
    /// CODE", FEC format workbook v6.4, sheet `Sch E`, field 23; "TRANS CODE"
    /// in v5.3, field 45), the FEC database's codes for an independent
    /// expenditure opposing (`24A`) or supporting (`24E`) a candidate
    /// (`FEC_v530.rtf`, "Independent Expenditures Support/Oppose"). Column
    /// `expenditure_purpose_code`.
    pub expenditure_purpose_code: Option<String>,
    /// A disbursement category code, `001`–`012` (see
    /// [`ScheduleE::category_code_label`]). Column `category_code`
    /// (field 25). The sources disagree on the allowed values: the workbook
    /// allows 001–010 (FEC format workbook v8.4, sheet `Sch E`, field 25),
    /// the Schedule E instructions print only 004 Advertising Expenses
    /// ([fecfrm3xi.pdf p22](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=22)),
    /// and the format specification defines 001–012 for SE and the other
    /// disbursement schedules (`FEC_Format_v8.4.pdf` p16). Kept as filed;
    /// some filings put a transaction code (`24E`, `24A`) here.
    pub category_code: Option<String>,
    /// `S` support or `O` oppose (FEC format workbook v8.4, sheet `Sch E`,
    /// field 27); legacy filings may use `SUP`/`OPP`, `24E`/`24A` or `UNI`
    /// (see [`ScheduleE::support_oppose_label`]). Column
    /// `support_oppose_code`.
    pub support_oppose_code: Option<String>,
    /// The candidate supported or opposed. Columns `candidate_id_number`,
    /// `candidate_last_name` … `candidate_suffix`, `candidate_office`,
    /// `candidate_district`, `candidate_state` (fields 28–36; v8.0 and
    /// earlier list state before district); v1–5.x `candidate_name`. The
    /// paper form prints no candidate ID box
    /// ([fecfrm3x.pdf p11](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3x.pdf#page=11)).
    pub candidate: CandidateRef,
    /// Who signed the schedule: the treasurer's certification, under penalty
    /// of perjury, that the expenditure was not coordinated
    /// ([fecfrm3xi.pdf p22](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=22)).
    /// Columns `completing_last_name` … `completing_suffix` (fields 37–41);
    /// v1–5.x `ind_name_as_signed` ("IND/NAME (as signed)").
    pub signer_name: PersonName,
    /// Column `date_signed` (field 42).
    pub date_signed: Option<Date>,
    /// True for a memo entry, e.g. an expenditure disseminated before it is
    /// paid for, whose obligation is reported on Schedule D
    /// ([fecfrm3xi.pdf p22](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3xi.pdf#page=22)).
    /// Column `memo_code`, `X` when true (field 43).
    pub memo: bool,
    /// Column `memo_text_description` (field 44).
    pub memo_text: Option<String>,
    /// v2–5.x only: the conduit of the expenditure, "Name of
    /// person/organization serving as transaction conduit" (`FEC_v2.rtf`,
    /// Schedule E field 25). Column `conduit_name`.
    pub conduit_name: Option<String>,
    /// v2–5.x only. Columns `conduit_street_1` … `conduit_zip_code`.
    pub conduit_address: Address,
    /// v2 only: the payee's FEC candidate ID, name, office, state and
    /// district, "If Entity-ID = CCM or CAN" (`FEC_v2.rtf`, Schedule E fields
    /// 20–24). Columns `payee_candidate_id_number`, `payee_candidate_name`, …
    /// `None` when blank.
    pub payee_candidate: Option<CandidateRef>,
    /// v1–5.x only: "DATE (Notarized)" (FEC format workbook v5.3, sheet
    /// `Sch E`, field 33). Column `date_notarized`.
    pub date_notarized: Option<Date>,
    /// v1–5.x only: "DATE (Notary Commission Expires)" (field 34). Column
    /// `date_notary_commission_expires`.
    pub notary_commission_expires: Option<Date>,
    /// v1–5.x only: "IND/NAME (Notary)" (field 35), split like other legacy
    /// names. Column `ind_name_notary`.
    pub notary_name: Option<PersonName>,
    /// The scanned page of a paper filing. Column `image_number` (paper
    /// layouts only).
    pub image_number: Option<String>,
}

impl ScheduleE {
    pub fn from_data<F: Fields + ?Sized>(data: &F) -> Option<Self> {
        // v8.0 and earlier: one date column, mapped as `dissemination_date`,
        // that is the disbursement date (see the field docs).
        let split_dates = data.raw("disbursement_date").is_some();
        let (dissemination_date, disbursement_date) = if split_dates {
            (
                date(data, "dissemination_date"),
                date(data, "disbursement_date"),
            )
        } else {
            (None, date(data, "dissemination_date"))
        };
        let payee_candidate =
            CandidateRef::from_prefixed(data, "payee_candidate_id_number", "payee_candidate_");
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            transaction_id: text_any(data, &["transaction_id_number", "transaction_id"]),
            back_reference_transaction_id: text(data, "back_reference_tran_id_number"),
            back_reference_schedule_name: text(data, "back_reference_sched_name"),
            payee: Entity::from_prefixed(data, "payee_", "payee_name"),
            payee_committee_fec_id: text(data, "payee_cmtte_fec_id_number"),
            election_code: text(data, "election_code"),
            election_other_description: text(data, "election_other_description"),
            dissemination_date,
            disbursement_date,
            expenditure_amount: amount(data, "expenditure_amount"),
            calendar_ytd_per_election_office: amount_opt(
                data,
                "calendar_y_t_d_per_election_office",
            ),
            expenditure_purpose_description: text(data, "expenditure_purpose_descrip"),
            expenditure_purpose_code: text(data, "expenditure_purpose_code"),
            category_code: text(data, "category_code"),
            support_oppose_code: text(data, "support_oppose_code"),
            candidate: CandidateRef::from_prefixed(data, "candidate_id_number", "candidate_"),
            signer_name: person_name_or_legacy(data, "completing_", "ind_name_as_signed"),
            date_signed: date(data, "date_signed"),
            memo: flag(data, "memo_code"),
            memo_text: text(data, "memo_text_description"),
            conduit_name: text(data, "conduit_name"),
            conduit_address: address_either(data, "conduit_"),
            payee_candidate: (!payee_candidate.is_empty()).then_some(payee_candidate),
            date_notarized: date(data, "date_notarized"),
            notary_commission_expires: date(data, "date_notary_commission_expires"),
            notary_name: text(data, "ind_name_notary")
                .map(|raw| split_legacy_name(&raw, data.name_delimiter())),
            image_number: text(data, "image_number"),
        })
    }

    /// "Support", "Oppose" or "Unitemized" for `support_oppose_code`; see
    /// [`support_oppose_label`].
    pub fn support_oppose_label(&self) -> Option<&'static str> {
        support_oppose_label(self.support_oppose_code.as_deref()?)
    }

    /// The disbursement category of `category_code`; see
    /// [`category_code_label`].
    pub fn category_code_label(&self) -> Option<&'static str> {
        category_code_label(self.category_code.as_deref()?)
    }

    /// "Primary", "General", … for `election_code`; see
    /// [`crate::covers::election_code_label`].
    pub fn election_code_label(&self) -> Option<&'static str> {
        crate::covers::election_code_label(self.election_code.as_deref()?)
    }
}

/// The meaning of a Schedule E support/oppose code. v8.x allows `S` and `O`
/// (FEC format workbook v8.4, sheet `Sch E`, field 27); the v5.3
/// specification's "Support/Oppose Code (Edit: SOP)" table lists `S` Support
/// "(also: SUP; 24E)", `O` Oppose "(also: OPP; 24A)" and `UNI` Unitemized
/// "(Schedule E only)" (`FEC_v530.rtf`), and in v2 a `UNI` row carries the
/// total of all unitemized independent expenditures (`FEC_v2.rtf`, Schedule
/// E rules). Case-insensitive; other values return `None`.
pub fn support_oppose_label(code: &str) -> Option<&'static str> {
    Some(match code.trim().to_ascii_uppercase().as_str() {
        "S" | "SUP" | "24E" => "Support",
        "O" | "OPP" | "24A" => "Oppose",
        "UNI" => "Unitemized",
        _ => return None,
    })
}

/// The heading of a disbursement category code, from the format
/// specification's "Category {of disbursement} Codes (Sched F57; SB; SE; SF;
/// H4 & H6)" (`FEC_Format_v8.4.pdf` p16): 001–012 "for use by any
/// non-Presidential filing committee", 101–107 "for use ONLY by Presidential
/// filing committees". Labels are the headings without their examples.
/// Leading zeros are optional (`4`, `04` and `004` are all Advertising
/// Expenses); any other value returns `None`. See
/// [`ScheduleE::category_code`] for how the sources disagree.
pub fn category_code_label(code: &str) -> Option<&'static str> {
    let code = code.trim();
    if code.is_empty() || !code.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(match code.parse::<u16>().ok()? {
        1 => "Administrative/Salary/Overhead Expenses",
        2 => "Travel Expenses",
        3 => "Solicitation and Fundraising Expenses",
        4 => "Advertising Expenses",
        5 => "Polling Expenses",
        6 => "Campaign Materials",
        7 => "Campaign Event Expenses",
        8 => "Transfers",
        9 => "Loan Repayments",
        10 => "Refunds of Contributions",
        11 => "Political Contributions",
        12 => "Donations",
        101 => "Expenses that are not Allocable",
        102 => "Media Expenditures",
        103 => "Expenditures for Mass Mailings and other Campaign Materials",
        104 => "Overhead Expenditures of State Offices and their Facilities",
        105 => "Expenditures for Special Telephone Programs",
        106 => "Public Opinion Poll Expenditures",
        107 => "Fundraising Expenditures",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::covers::fields::Data;

    fn data(pairs: &[(&str, &str)]) -> Data {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn labels() {
        assert_eq!(support_oppose_label("s"), Some("Support"));
        assert_eq!(support_oppose_label("24A"), Some("Oppose"));
        assert_eq!(support_oppose_label("X"), None);
        assert_eq!(category_code_label("004"), Some("Advertising Expenses"));
        assert_eq!(category_code_label("04"), Some("Advertising Expenses"));
        assert_eq!(category_code_label("012"), Some("Donations"));
        assert_eq!(category_code_label("24E"), None);
        assert_eq!(category_code_label("013"), None);
    }

    /// A v5.3 row: combined names, one date column, notary and conduit.
    #[test]
    fn legacy_v5_row() {
        let d = data(&[
            ("form_type", "SE"),
            ("filer_committee_id_number", "C00123456"),
            ("entity_type", "IND"),
            ("payee_name", "Doe^Jane^Ms.^"),
            ("payee_city", "Springfield"),
            ("dissemination_date", "20040915"),
            ("expenditure_amount", "1500.00"),
            ("support_oppose_code", "O"),
            ("candidate_id_number", "H4MA01234"),
            ("candidate_name", "Smith^Pat T.^Mr.^Jr."),
            ("candidate_office", "H"),
            ("conduit_name", "Conduit Co"),
            ("conduit_street_1", "1 Main St"),
            ("ind_name_as_signed", "Roe^Rick"),
            ("date_signed", "20040916"),
            ("date_notarized", ""),
            ("ind_name_notary", "Notary^Nina"),
            ("expenditure_purpose_code", "24A"),
        ]);
        let se = ScheduleE::from_data(&d).expect("typed");
        assert_eq!(se.payee.name.last_name, "Doe");
        assert_eq!(se.payee.display_name(), "Ms. Jane Doe");
        assert_eq!(se.dissemination_date, None);
        assert_eq!(se.disbursement_date, Date::new(2004, 9, 15).ok());
        assert_eq!(se.candidate.name.last_name, "Smith");
        assert_eq!(se.candidate.fec_id.as_deref(), Some("H4MA01234"));
        assert_eq!(se.signer_name.first_name, "Rick");
        assert_eq!(
            se.notary_name.as_ref().map(|n| n.last_name.as_str()),
            Some("Notary")
        );
        assert_eq!(se.conduit_address.street_1.as_deref(), Some("1 Main St"));
        assert!(se.payee_candidate.is_none());
        assert_eq!(se.support_oppose_label(), Some("Oppose"));
    }

    /// A v2 row names the payee as a candidate.
    #[test]
    fn legacy_v2_payee_candidate() {
        let d = data(&[
            ("form_type", "SE"),
            ("payee_name", "Committee to Elect"),
            ("payee_candidate_id_number", "S2NY00001"),
            ("payee_candidate_name", "Cand^Carl"),
        ]);
        let se = ScheduleE::from_data(&d).expect("typed");
        let pc = se.payee_candidate.expect("payee candidate");
        assert_eq!(pc.fec_id.as_deref(), Some("S2NY00001"));
        assert_eq!(pc.name.first_name, "Carl");
        assert_eq!(
            se.payee.organization_name.as_deref(),
            Some("Committee to Elect")
        );
    }

    /// v8.1+ has both dates; a blank dissemination date stays `None`.
    #[test]
    fn split_dates() {
        let d = data(&[
            ("form_type", "SE"),
            ("dissemination_date", ""),
            ("disbursement_date", "20221010"),
        ]);
        let se = ScheduleE::from_data(&d).expect("typed");
        assert_eq!(se.dissemination_date, None);
        assert_eq!(se.disbursement_date, Date::new(2022, 10, 10).ok());
    }
}
