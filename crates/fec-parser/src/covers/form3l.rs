//! Form 3L: Report of Contributions Bundled by Lobbyists/Registrants and
//! Lobbyist/Registrant PACs.

use crate::covers::fields::{amount, amount_opt, date, flag, text, text_or_empty, Data};
use crate::covers::{Address, PersonName};
use jiff::civil::Date;

/// FEC Form 3L, "Report of Contributions Bundled by Lobbyists/Registrants and
/// Lobbyist/Registrant PACs" — the `F3LN` / `F3LA` cover record.
///
/// **What it is.** Form 3L implements 52 U.S.C. §30104(i) and 11 CFR 104.22
/// ([fecfrm3li.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=1)).
/// It does not report the committee's finances: page 1 identifies the
/// committee, the report type and the covered period(s), and gives on Line 7
/// the total bundled contributions for each covered period; the bundlers
/// themselves are itemized on `SA3L` rows and refunds on `SB3L` rows
/// ([fecfrm3l.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3l.pdf#page=1);
/// FEC format workbook v8.4, sheet `Sch A`, field 1; sheet `Sch B`, field 1).
///
/// **Who files.** "Reporting committees": authorized committees of federal
/// candidates, political party committees and Leadership PACs that receive
/// bundled contributions from lobbyists/registrants and lobbyist/registrant
/// PACs in excess of the reporting threshold during a covered period
/// ([fecfrm3li.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=1)).
///
/// **When.** At the same time as the committee's Form 3, 3P or 3X, except that
/// monthly filers may choose a quarterly schedule; reports filed in July and
/// January also cover the semi-annual periods January 1 – June 30 and
/// July 1 – December 31
/// ([fecfrm3li.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=1),
/// [p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=2)).
/// That is why Line 7 has two amounts, for two *overlapping* periods: do not
/// add them together.
///
/// **Versions.** v6.4 through v8.5 share one layout (Form 3L was introduced in
/// v6.4: FEC e-filing specifications v8.4, p15). The paper-filing layouts
/// (`P2.6`–`P3.4`) name the state-of-election column `election_state` a second
/// time, so on those records [`Form3L::election_state`] holds the Line 5 state
/// of election and [`Form3L::election_held_in_state`] is `None`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Form3L {
    /// Form type as filed, e.g. `F3LN`: the base form plus the
    /// amendment-indicator suffix (see [`crate::covers::base_form_type`]).
    /// Column `form_type` (FEC format workbook v8.4, sheet `F3L`, field 1).
    /// The suffix is Line 3, "Is this report New (N) or Amended (A)"
    /// ([fecfrm3l.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3l.pdf#page=1)).
    pub form_type: String,
    /// Line 1, name of the reporting committee (`committee_name`)
    /// ([fecfrm3l.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3l.pdf#page=1)).
    pub committee_name: String,
    /// Line 2, FEC identification number of the reporting committee
    /// (`filer_committee_id_number`)
    /// ([fecfrm3li.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=4)).
    pub filer_committee_id: String,
    /// Line 1, the committee's mailing address (`street_1`, `street_2`,
    /// `city`, `state`, `zip_code`)
    /// ([fecfrm3l.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3l.pdf#page=1)).
    pub address: Address,
    /// The "Check if different than previously reported" box beside the
    /// address (`change_of_address`; `X` = yes)
    /// (FEC format workbook v8.4, sheet `F3L`, field 4).
    pub change_of_address: bool,
    /// Line 4, the state in which the candidate is running. Authorized
    /// committees of House and Senate candidates only; presidential campaigns,
    /// Leadership PACs and party committees leave it blank (`election_state`)
    /// ([fecfrm3li.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=4)).
    pub election_state: Option<String>,
    /// Line 4, the congressional district, "as necessary" — candidates only
    /// (`election_district`)
    /// ([fecfrm3li.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=4)).
    pub election_district: Option<String>,
    /// Line 5, type of report (`report_code`). See
    /// [`Form3L::report_code_label`] for the codes Form 3L accepts
    /// (FEC e-filing specifications v8.4, p15, "Accepted Report Codes by Type
    /// of Filing").
    pub report_code: Option<String>,
    /// Line 5(c)/(d), date of the election a 12-day pre-election or 30-day
    /// post-election report is for (`election_date`)
    /// ([fecfrm3li.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=4)).
    pub election_date: Option<Date>,
    /// Line 5(c)/(d), the state in which that election is held. The libfec
    /// column is literally named `TODO_UNKNOWN_BLANK`; the workbook calls it
    /// "STATE OF ELECTION" (FEC format workbook v8.4, sheet `F3L`, field 14).
    pub election_held_in_state: Option<String>,
    /// Line 5(c)/(d) box "This report also covers the semi-annual period":
    /// checked on a pre- or post-election report due in July or January when
    /// the July 15 Quarterly or Year-End report has been waived
    /// (`semi_annual_period`)
    /// ([fecfrm3li.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=4)).
    pub also_covers_semi_annual_period: bool,
    /// Line 6(a), first day of the quarterly, monthly, pre- or post-election
    /// covered period (`coverage_from_date`)
    /// ([fecfrm3li.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=4)).
    /// The specifications describe the semi-annual-only report codes (`QSA`,
    /// `QYE`, `MSA`, `MSY`) as having "no coverage dates" (FEC e-filing
    /// specifications v8.4, p15), but real filings with those codes usually
    /// still carry the quarter's dates here.
    pub coverage_from_date: Option<Date>,
    /// Line 6(a), last day of that covered period (`coverage_through_date`).
    pub coverage_through_date: Option<Date>,
    /// Line 6(b) box "January 1 – June 30": the report covers that
    /// semi-annual period (`semi_annual_period_jan_june`)
    /// ([fecfrm3l.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3l.pdf#page=1);
    /// FEC format workbook v8.4, sheet `F3L`, field 18). In practice filers
    /// usually signal the semi-annual period through the report code instead.
    pub semi_annual_january_june: bool,
    /// Line 6(b) box "July 1 – December 31" (`semi_annual_period_jul_dec`)
    /// (FEC format workbook v8.4, sheet `F3L`, field 19).
    pub semi_annual_july_december: bool,
    /// Line 7(a), total reportable bundled contributions for the quarterly,
    /// monthly, pre- or post-election covered period
    /// (`quarterly_monthly_bundled_contributions`)
    /// ([fecfrm3li.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=4)).
    /// This is the sum of the period's `SA3L` rows; refunds on `SB3L` are not
    /// subtracted
    /// ([fecfrm3li.pdf p5](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=5)).
    pub line7a_quarterly_monthly_bundled_contributions: f64,
    /// Line 7(b), total reportable bundled contributions for the January–June
    /// or July–December semi-annual covered period, "if applicable"
    /// (`semi_annual_bundled_contributions`)
    /// ([fecfrm3li.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3li.pdf#page=4)).
    /// `None` when blank, which is the normal case for a report that does not
    /// cover a semi-annual period.
    pub line7b_semi_annual_bundled_contributions: Option<f64>,
    /// Treasurer who signed the report (`treasurer_last_name`,
    /// `treasurer_first_name`, `treasurer_middle_name`, `treasurer_prefix`,
    /// `treasurer_suffix`)
    /// ([fecfrm3l.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3l.pdf#page=1)).
    pub treasurer: PersonName,
    /// Date the treasurer signed (`date_signed`)
    /// ([fecfrm3l.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3l.pdf#page=1)).
    pub date_signed: Option<Date>,
}

impl Form3L {
    /// Build from a cover record's column map. `None` only if the record has
    /// no `committee_name` column at all.
    pub fn from_data(data: &Data) -> Option<Self> {
        data.get("committee_name")?;
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            committee_name: text_or_empty(data, "committee_name"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            address: Address::from_prefixed(data, ""),
            change_of_address: flag(data, "change_of_address"),
            election_state: text(data, "election_state"),
            election_district: text(data, "election_district"),
            report_code: text(data, "report_code"),
            election_date: date(data, "election_date"),
            election_held_in_state: text(data, "TODO_UNKNOWN_BLANK"),
            also_covers_semi_annual_period: flag(data, "semi_annual_period"),
            coverage_from_date: date(data, "coverage_from_date"),
            coverage_through_date: date(data, "coverage_through_date"),
            semi_annual_january_june: flag(data, "semi_annual_period_jan_june"),
            semi_annual_july_december: flag(data, "semi_annual_period_jul_dec"),
            line7a_quarterly_monthly_bundled_contributions: amount(
                data,
                "quarterly_monthly_bundled_contributions",
            ),
            line7b_semi_annual_bundled_contributions: amount_opt(
                data,
                "semi_annual_bundled_contributions",
            ),
            treasurer: PersonName::from_data(data),
            date_signed: date(data, "date_signed"),
        })
    }

    /// True for an amended report (`F3LA`); see [`Form3L::form_type`].
    pub fn is_amendment(&self) -> bool {
        crate::covers::is_amendment_form_type(&self.form_type)
    }

    /// The FEC's description of [`Form3L::report_code`], for the codes Form
    /// 3L accepts: `Q1; [Q2S|QSA]; Q3; [QYS|QYE]; QMS; M2–M12; [M7S|MSA];
    /// [MYS|MSY]; 12G; 12P; 12R; 12S; 12C; 30G; 30R; 30S` (FEC e-filing
    /// specifications v8.4, p15). The semi-annual codes' wording is from the
    /// same page ("Special Report Type Codes used with F3L filings"); the rest
    /// from p14 ("Report Codes").
    pub fn report_code_label(&self) -> Option<&'static str> {
        Some(match self.report_code.as_deref()? {
            "Q1" => "April Quarterly",
            "Q2S" => "July 15 (Q2) report and Semi-annual",
            "QSA" => "Semi-Annual only (no Q2)",
            "Q3" => "October Quarterly",
            "QYS" => "Jan 31 Year End (YE) report and Semi-Annual",
            "QYE" => "Semi-Annual only (no YE)",
            "QMS" => "July 31 Mid-Year (MY) report and Semi-annual",
            "M2" => "February Monthly",
            "M3" => "March Monthly",
            "M4" => "April Monthly",
            "M5" => "May Monthly",
            "M6" => "June Monthly",
            "M7S" => "July 20 (M7) Monthly report and Semi-Annual",
            "MSA" => "Semi-Annual only (no M7)",
            "M8" => "August Monthly",
            "M9" => "September Monthly",
            "M10" => "October Monthly",
            "M11" => "November Monthly",
            "M12" => "December Monthly",
            "MYS" => "Jan 31 Year End (MYE) report and Semi-annual",
            "MSY" => "Semi-Annual only (no MYE)",
            "12G" => "Pre-General",
            "12P" => "Pre-Primary",
            "12R" => "Pre-Runoff",
            "12S" => "Pre-Special",
            "12C" => "Pre-Convention",
            "30G" => "Post-General",
            "30R" => "Post-Runoff",
            "30S" => "Post-Special",
            _ => return None,
        })
    }

    /// True when the report covers a semi-annual period: a semi-annual report
    /// code (`Q2S`, `QSA`, `QYS`, `QYE`, `QMS`, `M7S`, `MSA`, `MYS`, `MSY`;
    /// FEC e-filing specifications v8.4, p15) or any of the Line 5/6(b)
    /// semi-annual boxes checked.
    pub fn covers_semi_annual_period(&self) -> bool {
        matches!(
            self.report_code.as_deref(),
            Some("Q2S" | "QSA" | "QYS" | "QYE" | "QMS" | "M7S" | "MSA" | "MYS" | "MSY")
        ) || self.also_covers_semi_annual_period
            || self.semi_annual_january_june
            || self.semi_annual_july_december
    }
}
