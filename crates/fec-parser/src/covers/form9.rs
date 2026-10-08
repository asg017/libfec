//! Form 9: 24-hour notice of disbursements/obligations for electioneering
//! communications.

use crate::covers::fields::{amount, date, flag, person_name, text, text_or_empty, Data};
use crate::covers::{Address, PersonName};
use jiff::civil::Date;

/// "FEC FORM 9 - 24 HOUR NOTICE OF DISBURSEMENTS/OBLIGATIONS FOR
/// ELECTIONEERING COMMUNICATIONS" (form REV. 01/2018,
/// [fecfrm9.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9.pdf#page=1)).
/// Form types `F9N` (new) and `F9A` (amendment).
///
/// **What it reports.** An electioneering communication is "any broadcast,
/// cable or satellite communication that (1) refers to a clearly identified
/// candidate; (2) is publicly distributed; (3) is distributed within 60 days
/// prior to a general election or 30 days prior to a primary election; and
/// (4) can be received by 50,000 or more people" in the candidate's district
/// or state. "Disbursements" "includes actual disbursements and the execution
/// of contracts creating an obligation to make disbursements" (instructions
/// revised 01/2018,
/// [fecfrm9i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=1)).
///
/// **Who files, when.** "Every person that makes disbursements for
/// electioneering communications aggregating in excess of $10,000 during a
/// calendar year", by 11:59 p.m. Eastern of the day after the communication is
/// first publicly distributed, and again each time later disbursements
/// aggregate in excess of $10,000
/// ([fecfrm9i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=1)).
/// A "person" may be "an individual, unincorporated organization, corporation
/// or labor organization"; political committees report on Form 3X instead
/// ([fecfrm9i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=2)).
///
/// **Line numbers.** The 01/2018 paper form prints the totals as Lines 10
/// and 11; the v8.4 workbook labels the same fields "9." and "10." (FEC
/// format workbook v8.4, sheet F9, fields 38-39). Docs here use the paper
/// numbering.
///
/// # Versions
///
/// v8.3–8.5 is the layout documented here. v6.2–8.2 lack
/// `original_amendment_date`. v6.1 and v5.x have `qualified_non_profit` in
/// place of `filer_code`/`filer_code_description`; v5.x has no individual name
/// columns, and gives the custodian and person completing the form as single
/// caret-delimited names in the `custodian_last_name` and
/// `person_completing_last_name` columns, which are split into parts (FEC
/// format workbook v5.2, sheet F9).
///
/// The test fixture is a real `F9A` (FEC-2015422, v8.5); no `F9N` was
/// available locally.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "columnar", derive(fec_parser_macros::Columnar))]
pub struct Form9 {
    /// Form type as filed, e.g. `F9N`: the base form plus the
    /// amendment-indicator suffix (see [`crate::covers::base_form_type`]).
    /// Column `form_type` (FEC format workbook v8.4, sheet `F9`, field 1).
    pub form_type: String,
    /// Line 3, the filer's FEC identification number ("First time
    /// filers—leave this line blank",
    /// [fecfrm9i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=2)).
    /// Column `filer_committee_id_number` (FEC format workbook v8.4, sheet F9,
    /// field 2).
    pub filer_committee_id: String,
    /// Electronic-format entity code, one of `COM, IND, ORG, PAC, PTY` (no
    /// box on the paper form; the workbook gives no descriptions). Column
    /// `entity_type` (FEC format workbook v8.4, sheet F9, field 3).
    pub entity_type: Option<String>,
    /// Line 1(a), name of the organization or corporation making the
    /// disbursements. Column `organization_name` (FEC format workbook v8.4,
    /// sheet F9, field 4).
    pub organization_name: Option<String>,
    /// Line 1(a), name of the individual making the disbursements. Columns
    /// `individual_last_name`, `individual_first_name`,
    /// `individual_middle_name`, `individual_prefix`, `individual_suffix` (FEC
    /// format workbook v8.4, sheet F9, fields 5-9).
    pub individual: PersonName,
    /// Line 1(b) "check if different than previously reported" (`X` = yes).
    /// Column `change_of_address` (FEC format workbook v8.4, sheet F9,
    /// field 10).
    pub change_of_address: bool,
    /// Lines 1(b)-(c), the filer's address. Columns `street_1`, `street_2`,
    /// `city`, `state`, `zip_code` (FEC format workbook v8.4, sheet F9,
    /// fields 11-15).
    pub address: Address,
    /// Line 2, employer, "If Entity is an Individual". Column
    /// `individual_employer` (FEC format workbook v8.4, sheet F9, field 16).
    pub individual_employer: Option<String>,
    /// Line 2, occupation, "If Entity is an Individual". Column
    /// `individual_occupation` (FEC format workbook v8.4, sheet F9, field 17).
    pub individual_occupation: Option<String>,
    /// Line 5, on an amendment the date of the report it amends ("Use date of
    /// original report or of most recent amendment"). Column
    /// `original_amendment_date` (FEC format workbook v8.4, sheet F9,
    /// field 18). v8.3+ only.
    pub original_amendment_date: Option<Date>,
    /// Line 4, covered period start. These dates "should begin with the date
    /// of the first related disbursement and end with the date of public
    /// distribution"
    /// ([fecfrm9i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=2)).
    /// Column `coverage_from_date` (FEC format workbook v8.4, sheet F9,
    /// field 19).
    pub coverage_from_date: Option<Date>,
    /// Line 4, covered period end. Column `coverage_through_date` (FEC format
    /// workbook v8.4, sheet F9, field 20).
    pub coverage_through_date: Option<Date>,
    /// Line 6(a), date of public distribution: the date the communication
    /// was first publicly distributed, or for a later notice about the same
    /// communication the date it was distributed again
    /// ([fecfrm9i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=2)).
    /// Column `date_public_distribution` (FEC format workbook v8.4, sheet F9,
    /// field 21).
    pub date_public_distribution: Option<Date>,
    /// Line 6(b), the communication's title "as named by the media vendor or
    /// producer"
    /// ([fecfrm9i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=2)).
    /// Column `communication_title` (FEC format workbook v8.4, sheet F9,
    /// field 22).
    pub communication_title: Option<String>,
    /// Line 7, "THE FILER IS": one of `IND`, `UNO`, `QNC`, `CLQ`, `OTH`.
    /// Column `filer_code` (FEC format workbook v8.4, sheet F9, field 23). See
    /// [`Form9::filer_code_label`]. v6.2+ only.
    pub filer_code: Option<String>,
    /// Line 7(d) "Other, specify:", required when the filer code is `OTH`.
    /// Column `filer_code_description` (FEC format workbook v8.4, sheet F9,
    /// field 24).
    pub filer_code_description: Option<String>,
    /// v6.1/v5.x only: "QUALIFIED NON-PROFIT", `Y` or `N` (FEC format
    /// workbook v5.2, sheet F9, field 16). Column `qualified_non_profit`.
    pub qualified_nonprofit: Option<String>,
    /// Line 8, "WERE THE DISBURSEMENTS MADE EXCLUSIVELY FROM DONATIONS TO A
    /// SEGREGATED BANK ACCOUNT?": `Y` or `N`. The answer changes which donors
    /// must be itemized on Schedule 9-A
    /// ([fecfrm9i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=2)).
    /// Column `segregated_bank_account` (FEC format workbook v8.4, sheet F9,
    /// field 25, `Y=YES, N=NO`). See [`Form9::used_segregated_bank_account`].
    pub segregated_bank_account: Option<String>,
    /// Line 9, custodian of records: "the individual who controls the books
    /// and records that support this filing"
    /// ([fecfrm9i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=2)).
    pub custodian: Form9Custodian,
    /// Line 10, total donations this statement: "the sum total of donations
    /// itemized on Schedule 9A", or zero if none required itemization
    /// ([fecfrm9i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=2)).
    /// Workbook rule "= Sum of F92 Donations". Column `total_donations` (FEC
    /// format workbook v8.4, sheet F9, field 38, labelled line 9 there).
    pub total_donations: f64,
    /// Line 11, total disbursements/obligations this statement: "the sum
    /// total of disbursements itemized on Schedule 9B"
    /// ([fecfrm9i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9i.pdf#page=2)).
    /// Workbook rule "= Sum of F93 Disbursements". Column
    /// `total_disbursements` (FEC format workbook v8.4, sheet F9, field 39,
    /// labelled line 10 there).
    pub total_disbursements: f64,
    /// "TYPE OR PRINT NAME OF PERSON COMPLETING FORM", certifying "under
    /// penalty of perjury" that the statement is true, correct and complete
    /// ([fecfrm9.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9.pdf#page=1)).
    /// Columns `person_completing_last_name`, `_first_name`, `_middle_name`,
    /// `_prefix`, `_suffix` (FEC format workbook v8.4, sheet F9, fields
    /// 40-44).
    pub person_completing: PersonName,
    /// Date signed. Column `date_signed` (FEC format workbook v8.4, sheet F9,
    /// field 45).
    pub date_signed: Option<Date>,
}

/// Form 9 Line 9, custodian of records.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "libfec_parser.covers", frozen, get_all, skip_from_py_object)
)]
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "columnar", derive(fec_parser_macros::Columnar))]
pub struct Form9Custodian {
    /// Line 9(a), name. Columns `custodian_last_name`, `custodian_first_name`,
    /// `custodian_middle_name`, `custodian_prefix`, `custodian_suffix` (FEC
    /// format workbook v8.4, sheet F9, fields 26-30).
    pub name: PersonName,
    /// Lines 9(b)-(c), address. Columns `custodian_street_1`,
    /// `custodian_street_2`, `custodian_city`, `custodian_state`,
    /// `custodian_zip_code` (FEC format workbook v8.4, sheet F9, fields
    /// 31-35).
    pub address: Address,
    /// Line 9(d), "Name of Employer or Principal Place of Business". Column
    /// `custodian_employer` (FEC format workbook v8.4, sheet F9, field 36).
    pub employer: Option<String>,
    /// Line 9(e), occupation. Column `custodian_occupation` (FEC format
    /// workbook v8.4, sheet F9, field 37).
    pub occupation: Option<String>,
}

impl Form9 {
    pub fn from_data(data: &Data) -> Option<Self> {
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            entity_type: text(data, "entity_type"),
            organization_name: text(data, "organization_name"),
            individual: PersonName::from_prefixed(data, "individual_"),
            change_of_address: flag(data, "change_of_address"),
            address: Address::from_prefixed(data, ""),
            individual_employer: text(data, "individual_employer"),
            individual_occupation: text(data, "individual_occupation"),
            original_amendment_date: date(data, "original_amendment_date"),
            coverage_from_date: date(data, "coverage_from_date"),
            coverage_through_date: date(data, "coverage_through_date"),
            date_public_distribution: date(data, "date_public_distribution"),
            communication_title: text(data, "communication_title"),
            filer_code: text(data, "filer_code"),
            filer_code_description: text(data, "filer_code_description"),
            qualified_nonprofit: text(data, "qualified_non_profit"),
            segregated_bank_account: text(data, "segregated_bank_account"),
            custodian: Form9Custodian {
                name: person_name(data, "custodian_"),
                address: Address::from_prefixed(data, "custodian_"),
                employer: text(data, "custodian_employer"),
                occupation: text(data, "custodian_occupation"),
            },
            total_donations: amount(data, "total_donations"),
            total_disbursements: amount(data, "total_disbursements"),
            person_completing: person_name(data, "person_completing_"),
            date_signed: date(data, "date_signed"),
        })
    }

    /// True for an amended notice (`F9A`); see [`Form9::form_type`].
    pub fn is_amendment(&self) -> bool {
        crate::covers::is_amendment_form_type(&self.form_type)
    }

    /// The filer's name as on Line 1(a): the organization name, or the
    /// individual's name when there is none.
    pub fn filer_name(&self) -> String {
        match &self.organization_name {
            Some(name) => name.clone(),
            None => self.individual.to_string(),
        }
    }

    /// The workbook's description of [`Form9::filer_code`]: "IND -
    /// Individual", "UNO - Unincorporated Org.", "QNC - Qualified Nonprofit
    /// Corp. Under CFR 114.10", "CLQ - Corp, Labor or QNC under CFR 114.15",
    /// "OTH - Other, specify" (FEC format workbook v8.4, sheet F9, field 23).
    /// The 01/2018 paper form has only four boxes (an Individual; a
    /// Corporation or Labor Organization making communications under 11 CFR
    /// 114.10; an Unincorporated Organization; Other)
    /// ([fecfrm9.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm9.pdf#page=1)).
    pub fn filer_code_label(&self) -> Option<&'static str> {
        match self
            .filer_code
            .as_deref()?
            .trim()
            .to_ascii_uppercase()
            .as_str()
        {
            "IND" => Some("Individual"),
            "UNO" => Some("Unincorporated Org."),
            "QNC" => Some("Qualified Nonprofit Corp. Under CFR 114.10"),
            "CLQ" => Some("Corp, Labor or QNC under CFR 114.15"),
            "OTH" => Some("Other"),
            _ => None,
        }
    }

    /// Line 8 as a boolean: `Y` → `Some(true)`, `N` → `Some(false)` (FEC
    /// format workbook v8.4, sheet F9, field 25, `Y=YES, N=NO`); blank or any
    /// other value → `None`.
    pub fn used_segregated_bank_account(&self) -> Option<bool> {
        match self.segregated_bank_account.as_deref()?.trim() {
            "Y" | "y" => Some(true),
            "N" | "n" => Some(false),
            _ => None,
        }
    }
}
