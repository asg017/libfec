//! FEC Form 1, Statement of Organization (`F1N` / `F1A` cover records).
//!
//! Sources used throughout this module:
//!
//! - the blank form, [fecfrm1.pdf](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=1)
//!   (Revised 03/2022), pages 1–4;
//! - its instructions, [fecfrm1i.pdf](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=1)
//!   (Revised 03/22), pages 1–3;
//! - the FEC e-filing format workbook v8.4, sheet `F1`, whose field numbers
//!   (1–101) are cited as "field N".

use crate::covers::fields::{date, flag, person_name_or_legacy, text, text_or_empty, Data};
use crate::covers::{Address, PersonName};
use jiff::civil::Date;

/// FEC Form 1, **Statement of Organization**: how a political committee
/// registers with the FEC, and how it reports later changes to that
/// registration.
///
/// "All political committees" file it, and Line 5 sorts them into ten types
/// (see [`Form1::committee_type_label`]). New committees file within 10 days
/// of the trigger that applies to them (designation on Form 2, establishment
/// of a separate segregated fund, or crossing a contribution/expenditure
/// threshold), and any change or correction must be reported within 10 days
/// ([fecfrm1i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=1)).
///
/// The paper form has nine lines: committee name and contact details
/// (Line 1), date (2), FEC ID (3), new/amended (4), type of committee (5),
/// connected organization / affiliated committee (6), custodian of records
/// (7), treasurer and designated agent (8) and banks or depositories (9)
/// ([fecfrm1.pdf p1–4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=1)).
/// Line 4 (new or amended) is the `N`/`A` suffix of [`Form1::form_type`]
/// (`F1N`/`F1A`, field 1); see [`Form1::is_amendment`].
///
/// **Amendments.** The paper instructions ask amended statements to include
/// only the changes on Lines 5–9
/// ([fecfrm1i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=1)),
/// but electronic filers resubmit the entire form
/// ([colagui.pdf p22](https://www.fec.gov/resources/cms-content/documents/policy-guidance/colagui.pdf#page=22)),
/// so an `F1A` cover is a complete snapshot. The `change_of_*` flags mark
/// which Line 1 items changed.
///
/// **F1S supplements.** The cover record holds only the *first* connected
/// organization / affiliated committee / joint fundraising representative /
/// leadership PAC sponsor (Line 6), the first designated agent (Line 8) and
/// the first two banks (Line 9). Further instances — and every committee
/// participating in a joint fundraiser — are filed as separate `F1S` records
/// after the cover (FEC format workbook v8.4, sheet `F1`, notes before fields
/// 40, 78 and 90). This struct does not read `F1S` records; iterate the
/// filing's itemizations for those.
///
/// **Not on this record.** The committee "designation" and "filing frequency"
/// shown on fec.gov are not columns of the `F1` record in any format version,
/// so they are not here.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Form1 {
    /// Form type as filed, e.g. `F1N`: the base form plus the
    /// amendment-indicator suffix (see [`crate::covers::base_form_type`]).
    /// Column `form_type` (FEC format workbook v8.4, sheet `F1`, field 1).
    /// The suffix is Line 4, "Is this Statement New (N) or Amended (A)"
    /// ([fecfrm1.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=1)).
    pub form_type: String,

    // ---- Line 1 -------------------------------------------------------------
    /// Line 1, the committee's full name (`committee_name`, field 4).
    /// An authorized committee's name must include the candidate's name; a
    /// separate segregated fund's must include its connected organization's
    /// full name ([fecfrm1i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=1)).
    pub committee_name: String,
    /// Line 1 "(Check if name is changed)" box (`change_of_committee_name`,
    /// field 3, `X` = yes) ([fecfrm1.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=1)).
    pub change_of_committee_name: bool,
    /// Line 1 mailing address (`street_1`, `street_2`, `city`, `state`,
    /// `zip_code`, fields 6–10).
    pub address: Address,
    /// Line 1 "(Check if address is changed)" box next to the mailing address
    /// (`change_of_address`, field 5, `X` = yes).
    pub change_of_address: bool,
    /// Line 1 "Committee's E-mail Address", plus the "Optional Second E-Mail
    /// Address" (`committee_email`, field 12). The workbook allows at most two
    /// addresses "delimited by a semi-colon or a comma"; the raw value is kept
    /// as filed.
    pub committee_email: Option<String>,
    /// "(Check if address is changed)" box next to the e-mail address
    /// (`change_of_committee_email`, field 11, `X` = yes).
    pub change_of_committee_email: bool,
    /// Line 1 "Committee's Web Page Address (URL)" (`committee_url`, field 14):
    /// the committee's official web site, "if such a Web site exists"
    /// ([fecfrm1i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=1)).
    /// Kept as filed; it often lacks a scheme.
    pub committee_url: Option<String>,
    /// "(Check if address is changed)" box next to the URL
    /// (`change_of_committee_url`, field 13, `X` = yes).
    pub change_of_committee_url: bool,

    // ---- Lines 2–3 ----------------------------------------------------------
    /// Line 2 "Date" (`effective_date`, field 15): the date the group became a
    /// political committee or, on an amendment, the date of the change
    /// ([fecfrm1i.pdf p1](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=1)).
    pub effective_date: Option<Date>,
    /// Line 3 FEC identification number (`filer_committee_id_number`, field 2).
    pub filer_committee_id: String,

    // ---- Line 5: type of committee -----------------------------------------
    /// Line 5 type of committee, one letter `A`–`J` for boxes 5(a)–5(j)
    /// (`committee_type`, field 22). See [`Form1::committee_type_label`].
    pub committee_type: Option<String>,
    /// Line 5(a)/(b)/(c) candidate, for candidate committees and committees
    /// supporting or opposing one candidate. `None` when every candidate
    /// column is blank.
    pub candidate: Option<Form1Candidate>,
    /// Line 5 "Party Affiliation" of the candidate (5(a)/(b)) or the party of a
    /// party committee (5(d)), as an abbreviation such as `DEM`, `REP`
    /// (`party_code`, field 32). See [`Form1::party_code_label`].
    pub party_code: Option<String>,
    /// Line 5(d) level of a party committee: `NAT`, `STA` or `SUB`
    /// (`party_type`, field 33). See [`Form1::party_type_label`].
    pub party_type: Option<String>,
    /// Line 5(e) kind of connected organization of a separate segregated fund:
    /// `C`, `T`, `L`, `M`, `V` or `W` (`organization_type`, field 34). See
    /// [`Form1::organization_type_label`].
    pub organization_type: Option<String>,
    /// Lobbyist/Registrant PAC and Leadership PAC check boxes under Line 5.
    pub pac_flags: Form1PacFlags,

    // ---- Line 6 ------------------------------------------------------------
    /// Line 6, the first connected organization, affiliated committee, joint
    /// fundraising representative or leadership PAC sponsor (fields 40–53).
    /// Additional ones are on `F1S` records. `None` when every column is blank.
    /// Filers with nothing to list are told to enter "None" here
    /// ([fecfrm1i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=2)),
    /// so a committee name of `NONE` is common; it is kept as filed.
    pub affiliated: Option<Form1Affiliated>,

    // ---- Lines 7–8 ---------------------------------------------------------
    /// Line 7 custodian of records, "the person in possession of committee
    /// books and records" (`custodian_*`, fields 54–65). If the treasurer is
    /// the custodian, "the term 'treasurer' is sufficient"
    /// ([fecfrm1i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=3)).
    pub custodian: Option<Form1Contact>,
    /// Line 8 treasurer (`treasurer_*`, fields 66–77). "Every political
    /// committee must have a treasurer"
    /// ([fecfrm1i.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=3)).
    pub treasurer: Form1Contact,
    /// Line 8 first designated agent, "e.g., assistant treasurer"
    /// (`agent_*`, fields 78–89); further agents are on `F1S` records.
    pub agent: Option<Form1Contact>,

    // ---- Line 9 ------------------------------------------------------------
    /// Line 9 banks or other depositories "in which the committee deposits
    /// funds, holds accounts, rents safety deposit boxes or maintains funds"
    /// ([fecfrm1.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=4)).
    /// At most two on the cover (`bank_*` fields 90–95, `bank2_*` fields
    /// 96–101); further depositories are on `F1S` records. Blank slots are
    /// omitted.
    pub banks: Vec<Form1Bank>,

    // ---- Certification -----------------------------------------------------
    /// "Type or Print Name of Treasurer" in the certification block on page 1
    /// (`signature_*`, fields 16–20): the person who signed the statement.
    pub signer: PersonName,
    /// Date next to the treasurer's signature (`date_signed`, field 21).
    pub date_signed: Option<Date>,
}

/// The Line 5 Lobbyist/Registrant PAC and Leadership PAC check boxes.
///
/// The paper form repeats "In addition, this committee is a Lobbyist/Registrant
/// PAC" under boxes 5(e), 5(f), 5(g) and 5(h), and has one "Leadership PAC"
/// box under 5(f) ([fecfrm1.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=2));
/// each box is its own column, `X` = yes (fields 35–39). The 5(g) and 5(h)
/// columns exist from format v8.4 on; earlier records read as `false`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct Form1PacFlags {
    /// 5(e) separate segregated fund is also a Lobbyist/Registrant PAC
    /// (`lobbyist_registrant_pac`, field 35).
    pub lobbyist_registrant_pac_ssf: bool,
    /// 5(f) nonconnected committee is also a Lobbyist/Registrant PAC
    /// (`lobbyist_registrant_pac_2`, field 36).
    pub lobbyist_registrant_pac_nonconnected: bool,
    /// 5(f) nonconnected committee is also a Leadership PAC, whose sponsor is
    /// identified on Line 6 (`leadership_pac`, field 37). Per the instructions
    /// this box is checked when the committee "is directly or indirectly
    /// established, financed, maintained or controlled by a federal candidate
    /// or officeholder, but is not an authorized committee or party"
    /// ([fecfrm1i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=2)).
    pub leadership_pac: bool,
    /// 5(g) Super PAC is also a Lobbyist/Registrant PAC
    /// (`lobbyist_registrant_pac_3`, field 38).
    pub lobbyist_registrant_pac_super_pac: bool,
    /// 5(h) Hybrid PAC is also a Lobbyist/Registrant PAC
    /// (`lobbyist_registrant_pac_4`, field 39).
    pub lobbyist_registrant_pac_hybrid_pac: bool,
}

impl Form1PacFlags {
    /// True if any of the four Lobbyist/Registrant PAC boxes is checked. A box
    /// is checked when "a lobbyist/registrant established or controls the
    /// committee" ([fecfrm1i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=2)).
    pub fn is_lobbyist_registrant_pac(&self) -> bool {
        self.lobbyist_registrant_pac_ssf
            || self.lobbyist_registrant_pac_nonconnected
            || self.lobbyist_registrant_pac_super_pac
            || self.lobbyist_registrant_pac_hybrid_pac
    }
}

/// The candidate named under Line 5(a), 5(b) or 5(c).
///
/// Principal campaign and other authorized committees (5(a)/(b)) complete
/// name, office, state, district and party; a committee supporting or
/// opposing a single candidate (5(c)) gives the candidate's name
/// ([fecfrm1.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=2)).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Form1Candidate {
    /// The candidate's FEC ID (`candidate_id_number`, field 23). Not printed
    /// on the paper form; often blank for new candidates.
    pub candidate_id: Option<String>,
    /// "Name of Candidate" (`candidate_last_name` … `candidate_suffix`,
    /// fields 24–28; the single caret-delimited `candidate_name` column of
    /// pre-v6 formats is split into parts).
    pub name: PersonName,
    /// "Office Sought": `H`, `S` or `P` (`candidate_office`, field 29). See
    /// [`Form1Candidate::office_label`].
    pub office: Option<String>,
    /// Candidate's state, for House and Senate (`candidate_state`, field 30).
    pub state: Option<String>,
    /// Candidate's district, for House (`candidate_district`, field 31).
    pub district: Option<String>,
}

impl Form1Candidate {
    fn from_data(data: &Data) -> Option<Self> {
        let candidate = Self {
            candidate_id: text(data, "candidate_id_number"),
            name: person_name_or_legacy(data, "candidate_", "candidate_name"),
            office: text(data, "candidate_office"),
            state: text(data, "candidate_state"),
            district: text(data, "candidate_district"),
        };
        let blank = candidate.candidate_id.is_none()
            && candidate.name.is_empty()
            && candidate.office.is_none()
            && candidate.state.is_none()
            && candidate.district.is_none();
        (!blank).then_some(candidate)
    }

    /// The candidate's name as one string (`"Pete Barlow"`).
    pub fn full_name(&self) -> String {
        self.name.to_string()
    }

    /// `H` House, `S` Senate, `P` President: the workbook lists the codes
    /// `H,S,P` (FEC format workbook v8.4, sheet `F1`, field 29) and the form's
    /// "Office Sought" boxes read "House", "Senate", "President"
    /// ([fecfrm1.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=2)).
    pub fn office_label(&self) -> Option<&'static str> {
        self.office.as_deref().and_then(crate::covers::office_label)
    }
}

/// Line 6: a connected organization, affiliated committee, joint fundraising
/// representative/participant or leadership PAC sponsor.
///
/// Which one is given by [`Form1Affiliated::relationship_code`]. Committees
/// and organizations are named in `committee_name` (with `committee_id`); an
/// individual, such as a leadership PAC's sponsoring candidate or
/// officeholder, is named in `name` (with `candidate_id`) (FEC format
/// workbook v8.4, sheet `F1`, fields 40–53).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Form1Affiliated {
    /// FEC ID of the affiliated committee (`affiliated_committee_id_number`,
    /// field 40). Not printed on the paper form.
    pub committee_id: Option<String>,
    /// "Name of Any Connected Organization, Affiliated Committee, Joint
    /// Fundraising Representative, or Leadership PAC Sponsor"
    /// (`affiliated_committee_name`, field 41).
    pub committee_name: Option<String>,
    /// FEC candidate ID of an individual named here
    /// (`affiliated_candidate_id_number`, field 42).
    pub candidate_id: Option<String>,
    /// Individual's name (`affiliated_last_name` … `affiliated_suffix`,
    /// fields 43–47).
    pub name: PersonName,
    /// Mailing address (`affiliated_street_1` … `affiliated_zip_code`,
    /// fields 48–52).
    pub address: Address,
    /// "Relationship": `ORG`, `AFF`, `JFR` or `LPS`
    /// (`affiliated_relationship_code`, field 53). See
    /// [`Form1Affiliated::relationship_label`].
    pub relationship_code: Option<String>,
}

impl Form1Affiliated {
    fn from_data(data: &Data) -> Option<Self> {
        let a = Self {
            committee_id: text(data, "affiliated_committee_id_number"),
            committee_name: text(data, "affiliated_committee_name"),
            candidate_id: text(data, "affiliated_candidate_id_number"),
            name: PersonName::from_prefixed(data, "affiliated_"),
            address: Address::from_prefixed(data, "affiliated_"),
            relationship_code: text(data, "affiliated_relationship_code"),
        };
        let blank = a.committee_id.is_none()
            && a.committee_name.is_none()
            && a.candidate_id.is_none()
            && a.name.is_empty()
            && a.address.is_empty()
            && a.relationship_code.is_none();
        (!blank).then_some(a)
    }

    /// The name to display: the committee/organization name, else the
    /// individual's name.
    pub fn display_name(&self) -> String {
        match &self.committee_name {
            Some(name) => name.clone(),
            None => self.name.to_string(),
        }
    }

    /// The workbook's description of [`Form1Affiliated::relationship_code`]
    /// (FEC format workbook v8.4, sheet `F1`, field 53): `ORG` "Connected
    /// Organization", `AFF` "Affiliated Committee", `JFR` "Joint Fundraising
    /// Participant", `LPS` "Leadership PAC Sponsor".
    ///
    /// The paper form's check boxes read "Connected Organization", "Affiliated
    /// Organization", "Joint Fundraising Representative" and "Leadership PAC
    /// Sponsor" ([fecfrm1.pdf p3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=3));
    /// this method follows the workbook, which defines the electronic codes.
    pub fn relationship_label(&self) -> Option<&'static str> {
        match self
            .relationship_code
            .as_deref()?
            .to_ascii_uppercase()
            .as_str()
        {
            "ORG" => Some("Connected Organization"),
            "AFF" => Some("Affiliated Committee"),
            "JFR" => Some("Joint Fundraising Participant"),
            "LPS" => Some("Leadership PAC Sponsor"),
            _ => None,
        }
    }
}

/// A person listed on Lines 7–8: custodian of records, treasurer or
/// designated agent. Each has a name, mailing address, "Title or Position"
/// and optional "Telephone number"
/// ([fecfrm1.pdf p3–4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=3)).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Form1Contact {
    /// Full name (`{role}_last_name` … `{role}_suffix`; the single
    /// caret-delimited `{role}_name` column of pre-v6 formats is split into
    /// parts).
    pub name: PersonName,
    /// Mailing address (`{role}_street_1` … `{role}_zip_code`).
    pub address: Address,
    /// "Title or Position" (`{role}_title`).
    pub title: Option<String>,
    /// "Telephone number" (`{role}_telephone`, ten digits, kept as filed).
    pub telephone: Option<String>,
}

impl Form1Contact {
    fn from_prefixed(data: &Data, prefix: &str) -> Self {
        Self {
            name: person_name_or_legacy(data, prefix, &format!("{prefix}name")),
            address: Address::from_prefixed(data, prefix),
            title: text(data, &format!("{prefix}title")),
            telephone: text(data, &format!("{prefix}telephone")),
        }
    }

    fn from_prefixed_opt(data: &Data, prefix: &str) -> Option<Self> {
        let c = Self::from_prefixed(data, prefix);
        (!c.is_empty()).then_some(c)
    }

    /// True when every part is blank.
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
            && self.address.is_empty()
            && self.title.is_none()
            && self.telephone.is_none()
    }
}

/// Line 9: a bank, depository, etc. — "Name of Bank, Depository, etc." and
/// its mailing address ([fecfrm1.pdf p4](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=4)).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Form1Bank {
    /// `bank_name` / `bank2_name` (fields 90 and 96).
    pub name: Option<String>,
    /// `bank_street_1` … `bank_zip_code` (fields 91–95) or `bank2_*` (97–101).
    pub address: Address,
}

impl Form1Bank {
    fn from_prefixed(data: &Data, prefix: &str) -> Option<Self> {
        let b = Self {
            name: text(data, &format!("{prefix}name")),
            address: Address::from_prefixed(data, prefix),
        };
        (b.name.is_some() || !b.address.is_empty()).then_some(b)
    }
}

impl Form1 {
    /// Build from an `F1N`/`F1A` cover record. Returns `None` only when the
    /// record has no `committee_name` column at all.
    pub fn from_data(data: &Data) -> Option<Self> {
        if !data.contains_key("committee_name") {
            return None;
        }
        Some(Self {
            form_type: text_or_empty(data, "form_type"),
            committee_name: text_or_empty(data, "committee_name"),
            change_of_committee_name: flag(data, "change_of_committee_name"),
            address: Address::from_prefixed(data, ""),
            change_of_address: flag(data, "change_of_address"),
            committee_email: text(data, "committee_email"),
            change_of_committee_email: flag(data, "change_of_committee_email"),
            committee_url: text(data, "committee_url"),
            change_of_committee_url: flag(data, "change_of_committee_url"),
            effective_date: date(data, "effective_date"),
            filer_committee_id: text_or_empty(data, "filer_committee_id_number"),
            committee_type: text(data, "committee_type"),
            candidate: Form1Candidate::from_data(data),
            party_code: text(data, "party_code"),
            party_type: text(data, "party_type"),
            organization_type: text(data, "organization_type"),
            pac_flags: Form1PacFlags {
                lobbyist_registrant_pac_ssf: flag(data, "lobbyist_registrant_pac"),
                lobbyist_registrant_pac_nonconnected: flag(data, "lobbyist_registrant_pac_2"),
                leadership_pac: flag(data, "leadership_pac"),
                lobbyist_registrant_pac_super_pac: flag(data, "lobbyist_registrant_pac_3"),
                lobbyist_registrant_pac_hybrid_pac: flag(data, "lobbyist_registrant_pac_4"),
            },
            affiliated: Form1Affiliated::from_data(data),
            custodian: Form1Contact::from_prefixed_opt(data, "custodian_"),
            treasurer: Form1Contact::from_prefixed(data, "treasurer_"),
            agent: Form1Contact::from_prefixed_opt(data, "agent_"),
            banks: ["bank_", "bank2_"]
                .into_iter()
                .filter_map(|p| Form1Bank::from_prefixed(data, p))
                .collect(),
            signer: person_name_or_legacy(data, "signature_", "signature_name"),
            date_signed: date(data, "date_signed"),
        })
    }

    /// True for an amended statement (`F1A`); see [`Form1::form_type`].
    pub fn is_amendment(&self) -> bool {
        crate::covers::is_amendment_form_type(&self.form_type)
    }

    /// The workbook's description of [`Form1::committee_type`] (FEC format
    /// workbook v8.4, sheet `F1`, field 22), one per Line 5 box
    /// ([fecfrm1.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1.pdf#page=2)):
    ///
    /// | Code | Box | Description |
    /// |---|---|---|
    /// | `A` | 5(a) | Principal Campaign Committee |
    /// | `B` | 5(b) | Authorized Committee |
    /// | `C` | 5(c) | Support/Oppose One Candidate (Not Authorized Committee) |
    /// | `D` | 5(d) | National, State, or Subordinate Party Committee |
    /// | `E` | 5(e) | Separate Segregated Fund |
    /// | `F` | 5(f) | Support/Oppose more than One Federal Cand & Not Segregated Fund/Party |
    /// | `G` | 5(g) | Independent expenditure-only political committee (Super PAC) |
    /// | `H` | 5(h) | Political committee with both contribution and non-contribution accounts (Hybrid PAC) |
    /// | `I` | 5(i) | Joint Fundraising Representative {At least one is authorized} |
    /// | `J` | 5(j) | Joint Fundraising Representative {None are authorized} |
    ///
    /// The instructions call a 5(f) committee "a nonconnected committee"
    /// ([fecfrm1i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=2)).
    pub fn committee_type_label(&self) -> Option<&'static str> {
        committee_type_label(self.committee_type.as_deref()?)
    }

    /// The Line 5 box (`"5(a)"` … `"5(j)"`) for [`Form1::committee_type`]
    /// (FEC format workbook v8.4, sheet `F1`, field 22: "A through J"
    /// matching 5(a)–(j)).
    pub fn committee_type_line(&self) -> Option<&'static str> {
        let code = self.committee_type.as_deref()?.to_ascii_uppercase();
        Some(match code.as_str() {
            "A" => "5(a)",
            "B" => "5(b)",
            "C" => "5(c)",
            "D" => "5(d)",
            "E" => "5(e)",
            "F" => "5(f)",
            "G" => "5(g)",
            "H" => "5(h)",
            "I" => "5(i)",
            "J" => "5(j)",
            _ => return None,
        })
    }

    /// The workbook's description of [`Form1::organization_type`] (FEC format
    /// workbook v8.4, sheet `F1`, field 34): `C` Corporation, `T` Trade
    /// Association, `L` Labor Organization, `M` Membership Organization, `V`
    /// Cooperative, `W` Corporation w/o capital stock.
    pub fn organization_type_label(&self) -> Option<&'static str> {
        match self
            .organization_type
            .as_deref()?
            .to_ascii_uppercase()
            .as_str()
        {
            "C" => Some("Corporation"),
            "T" => Some("Trade Association"),
            "L" => Some("Labor Organization"),
            "M" => Some("Membership Organization"),
            "V" => Some("Cooperative"),
            "W" => Some("Corporation w/o capital stock"),
            _ => None,
        }
    }

    /// Description of [`Form1::party_type`]. The Line 5(d) instructions say to
    /// fill in "whether the party is the national party (use code NAT), state
    /// party (use code STA) or subordinate committee (use code SUB)"
    /// ([fecfrm1i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=2)).
    pub fn party_type_label(&self) -> Option<&'static str> {
        match self.party_type.as_deref()?.to_ascii_uppercase().as_str() {
            "NAT" => Some("National party"),
            "STA" => Some("State party"),
            "SUB" => Some("Subordinate committee"),
            _ => None,
        }
    }

    /// Party name for [`Form1::party_code`], only for the five abbreviations
    /// the Line 5 instructions spell out: `DEM` Democratic Party, `REP`
    /// Republican Party, `REF` Reform Party, `GRE` Green Party, `IND`
    /// Independent
    /// ([fecfrm1i.pdf p2](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm1i.pdf#page=2)).
    /// Other codes return `None`: the workbook only says `AIC,AIP,...` /
    /// `Edit: PTY` (field 32) without listing descriptions. See
    /// [`crate::covers::party_label`].
    pub fn party_code_label(&self) -> Option<&'static str> {
        crate::covers::party_label(self.party_code.as_deref()?)
    }
}

/// See [`Form1::committee_type_label`].
pub(crate) fn committee_type_label(code: &str) -> Option<&'static str> {
    Some(match code.trim().to_ascii_uppercase().as_str() {
        "A" => "Principal Campaign Committee",
        "B" => "Authorized Committee",
        "C" => "Support/Oppose One Candidate (Not Authorized Committee)",
        "D" => "National, State, or Subordinate Party Committee",
        "E" => "Separate Segregated Fund",
        "F" => "Support/Oppose more than One Federal Cand & Not Segregated Fund/Party",
        "G" => "Independent expenditure-only political committee (Super PAC)",
        "H" => {
            "Political committee with both contribution and non-contribution accounts (Hybrid PAC)"
        }
        "I" => "Joint Fundraising Representative {At least one is authorized}",
        "J" => "Joint Fundraising Representative {None are authorized}",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(pairs: &[(&str, &str)]) -> Data {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn candidate_without_id_is_kept() {
        let d = data(&[
            ("committee_name", "X for Congress"),
            ("candidate_id_number", ""),
            ("candidate_last_name", "Barlow"),
            ("candidate_first_name", "Pete"),
            ("candidate_office", "H"),
        ]);
        let f = Form1::from_data(&d).unwrap();
        assert!(f.affiliated.is_none());
        assert!(f.banks.is_empty());
        let c = f.candidate.unwrap();
        assert_eq!(c.full_name(), "Pete Barlow");
        assert_eq!(c.office_label(), Some("House"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn legacy_single_name_columns() {
        let d = data(&[
            ("committee_name", "Old PAC"),
            ("treasurer_name", "DOE^JANE"),
            ("candidate_name", "SMITH^JOHN"),
        ]);
        let f = Form1::from_data(&d).unwrap();
        assert_eq!(f.treasurer.name.last_name, "DOE");
        assert_eq!(f.treasurer.name.first_name, "JANE");
        assert_eq!(f.candidate.unwrap().name.to_string(), "JOHN SMITH");
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn labels() {
        assert_eq!(
            committee_type_label("g"),
            Some("Independent expenditure-only political committee (Super PAC)")
        );
        assert_eq!(committee_type_label("Z"), None);
    }
}
