//! Typed views of a filing's itemization records (Schedules A–F).
//!
//! Every record after the cover is an *itemization*: one line of a schedule
//! that supports a line of the cover's summary page (`SA11AI` itemizes line
//! 11(a)(i), `SB21B` line 21(b), …). [`crate::FilingRow`] carries the raw
//! fields; this module turns the ones we understand into rich structs, the
//! same way [`crate::covers`] does for the cover record.
//!
//! ```no_run
//! # fn main() -> anyhow::Result<()> {
//! use fec_parser::{itemizations::Itemization, Filing};
//!
//! let mut filing = Filing::<std::fs::File>::from_path("1926068.fec".as_ref())?;
//! let version = filing.header.fec_version.clone();
//! let delimiter = filing.header.name_delimiter.clone();
//! while let Some(row) = filing.next_row() {
//!     let row = row?;
//!     if let Some(Itemization::ScheduleA(sa)) =
//!         Itemization::from_record(&row.record, &version, delimiter.as_deref())
//!     {
//!         println!("{} gave {}", sa.contributor.display_name(), sa.contribution_amount);
//!     }
//! }
//! # Ok(()) }
//! ```
//!
//! # Conventions
//!
//! The cover conventions ([`crate::covers`]) apply, with these additions:
//!
//! - **One module per schedule** (`schedule_a.rs`), top-level struct named
//!   after it (`ScheduleA`), built by `from_data(&impl Fields) -> Option<Self>`.
//! - **One struct per schedule across every version.** Field names are the
//!   v8.x meanings, not the columns: the v8.x column is `transaction_id` on
//!   Schedule A but `transaction_id_number` on B–F, the conduit's street is
//!   `conduit_street1` on A but `conduit_street_1` on B, and v1–5.x give one
//!   combined `contributor_name`; each struct reads whichever its layout has.
//! - **Every struct starts with the same identification fields**: `form_type`
//!   (the row type as filed, `SA11AI`), `filer_committee_id`, `transaction_id`
//!   and, where the schedule has them, the back-reference pair. Helpers common
//!   to every schedule (`line_number()`, …) are on [`Itemization`].
//! - **The amount of the transaction is `f64`**, blank read as `0.0` like the
//!   summary amounts on covers; **aggregates and other secondary amounts are
//!   `Option<f64>`**, since a blank aggregate means "not reported", not zero.
//! - **The other party** — contributor, payee, lender, creditor — is an
//!   [`Entity`] (entity type, organization name, [`PersonName`], [`Address`]);
//!   a candidate referenced by an itemization is a [`CandidateRef`].
//! - **No per-row map.** Records are read through [`RecordFields`], which
//!   borrows the record and a column index shared by every row of a
//!   `(row type, version)` pair, so typing a row costs only its own fields.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::{Arc, OnceLock, RwLock};

use csv::StringRecord;
use serde::Serialize;

use crate::covers::fields::{
    key, person_name, person_name_or_legacy, split_legacy_name, text, Fields,
};
use crate::covers::{Address, PersonName};

mod form13_items;
mod form5_items;
mod form6_items;
mod form7_items;
mod form9_items;
#[cfg(feature = "python")]
pub mod python;
mod schedule_a;
mod schedule_a3l;
mod schedule_b;
mod schedule_c;
mod schedule_c1;
mod schedule_c2;
mod schedule_e;
mod schedule_l;
mod text;

pub use form13_items::{Form13Donation, Form13Refund};
pub use form5_items::{Form5Contribution, Form5Expenditure};
pub use form6_items::Form6Contribution;
pub use form7_items::Form7Communication;
pub use form9_items::{Form9Candidate, Form9ControllingPerson, Form9Disbursement, Form9Donation};
pub use schedule_a::ScheduleA;
pub use schedule_b::ScheduleB;
mod schedule_d;
mod schedule_f;
pub use schedule_d::ScheduleD;
pub use schedule_f::{ScheduleF, ScheduleFCommittee};
mod schedule_h1;
mod schedule_h2;
mod schedule_h3;
mod schedule_h4;
mod schedule_h5;
mod schedule_h6;
pub use schedule_a3l::ScheduleA3L;
pub use schedule_c::{ScheduleC, ScheduleCGuarantor};
pub use schedule_c1::ScheduleC1;
pub use schedule_c2::ScheduleC2;
pub use schedule_e::{category_code_label, support_oppose_label, ScheduleE};
pub use schedule_h1::ScheduleH1;
pub use schedule_h2::ScheduleH2;
pub use schedule_h3::ScheduleH3;
pub use schedule_h4::ScheduleH4;
pub use schedule_h5::ScheduleH5;
pub use schedule_h6::ScheduleH6;
pub use schedule_l::ScheduleL;
pub use text::TextRecord;

/// One typed itemization record. `None` from [`Itemization::from_record`]
/// means the row is not an itemization (the cover, `F3PS`, …) or its record
/// type has no struct yet.
///
/// Serialized with its [`record_family`] as a `"family"` tag (`"SA"`), then
/// the struct's fields.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "family")]
pub enum Itemization {
    #[serde(rename = "SA")]
    ScheduleA(Box<ScheduleA>),
    #[serde(rename = "SB")]
    ScheduleB(Box<ScheduleB>),
    #[serde(rename = "SD")]
    ScheduleD(Box<ScheduleD>),
    #[serde(rename = "SF")]
    ScheduleF(Box<ScheduleF>),
    #[serde(rename = "H1")]
    ScheduleH1(Box<ScheduleH1>),
    #[serde(rename = "H2")]
    ScheduleH2(Box<ScheduleH2>),
    #[serde(rename = "H3")]
    ScheduleH3(Box<ScheduleH3>),
    #[serde(rename = "H4")]
    ScheduleH4(Box<ScheduleH4>),
    #[serde(rename = "H5")]
    ScheduleH5(Box<ScheduleH5>),
    #[serde(rename = "H6")]
    ScheduleH6(Box<ScheduleH6>),
    #[serde(rename = "F56")]
    Form5Contribution(Box<Form5Contribution>),
    #[serde(rename = "F57")]
    Form5Expenditure(Box<Form5Expenditure>),
    #[serde(rename = "F65")]
    Form6Contribution(Box<Form6Contribution>),
    #[serde(rename = "F76")]
    Form7Communication(Box<Form7Communication>),
    #[serde(rename = "F91")]
    Form9ControllingPerson(Box<Form9ControllingPerson>),
    #[serde(rename = "F92")]
    Form9Donation(Box<Form9Donation>),
    #[serde(rename = "F93")]
    Form9Disbursement(Box<Form9Disbursement>),
    #[serde(rename = "F94")]
    Form9Candidate(Box<Form9Candidate>),
    #[serde(rename = "F132")]
    Form13Donation(Box<Form13Donation>),
    #[serde(rename = "F133")]
    Form13Refund(Box<Form13Refund>),
    #[serde(rename = "SL")]
    ScheduleL(Box<ScheduleL>),
    #[serde(rename = "TEXT")]
    Text(Box<TextRecord>),
    #[serde(rename = "SA3L")]
    ScheduleA3L(Box<ScheduleA3L>),
    #[serde(rename = "SE")]
    ScheduleE(Box<ScheduleE>),
    #[serde(rename = "SC")]
    ScheduleC(Box<ScheduleC>),
    #[serde(rename = "SC1")]
    ScheduleC1(Box<ScheduleC1>),
    #[serde(rename = "SC2")]
    ScheduleC2(Box<ScheduleC2>),
}

/// The record family of a row type: `SA11AI` → `SA`, `SC1/10` → `SC1`,
/// `SA3L` → `SA3L`, `H4` → `H4`, `TEXT` → `TEXT`, `F57` → `F57`. Longest
/// prefix first, so `SC1` is not read as `SC`. `None` for anything else
/// (covers, summary records).
pub fn record_family(row_type: &str) -> Option<&'static str> {
    const FAMILIES: &[&str] = &[
        "SA3L", "SC1", "SC2", "SA", "SB", "SC", "SD", "SE", "SF", "SI", "SL", "H1", "H2", "H3",
        "H4", "H5", "H6", "TEXT", "F56", "F57", "F65", "F76", "F91", "F92", "F93", "F94", "F132",
        "F133",
    ];
    let row_type = row_type.trim();
    FAMILIES.iter().copied().find(|family| {
        row_type
            .get(..family.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(family))
    })
}

impl Itemization {
    /// Type a raw record: field 0 is the row type, `fec_version` the
    /// header's, `name_delimiter` the header's (for legacy combined names;
    /// `None` means `^`).
    pub fn from_record(
        record: &StringRecord,
        fec_version: &str,
        name_delimiter: Option<&str>,
    ) -> Option<Self> {
        let row_type = record.get(0)?;
        // Cheap reject before the layout lookup: most callers type every row.
        record_family(row_type)?;
        let layout = Layout::get(row_type, fec_version)?;
        let fields = RecordFields {
            layout: &layout,
            record,
            name_delimiter: name_delimiter.filter(|d| !d.is_empty()).unwrap_or("^"),
        };
        Self::from_fields(row_type, &fields)
    }

    /// Type a record already read into [`Fields`] (a cover-style `Data` map
    /// works too), given its row type.
    pub fn from_fields<F: Fields + ?Sized>(row_type: &str, data: &F) -> Option<Self> {
        Some(match record_family(row_type)? {
            "SA" => Itemization::ScheduleA(Box::new(ScheduleA::from_data(data)?)),
            "SB" => Itemization::ScheduleB(Box::new(ScheduleB::from_data(data)?)),
            "SD" => Itemization::ScheduleD(Box::new(ScheduleD::from_data(data)?)),
            "SF" => Itemization::ScheduleF(Box::new(ScheduleF::from_data(data)?)),
            "H1" => Itemization::ScheduleH1(Box::new(ScheduleH1::from_data(data)?)),
            "H2" => Itemization::ScheduleH2(Box::new(ScheduleH2::from_data(data)?)),
            "H3" => Itemization::ScheduleH3(Box::new(ScheduleH3::from_data(data)?)),
            "H4" => Itemization::ScheduleH4(Box::new(ScheduleH4::from_data(data)?)),
            "H5" => Itemization::ScheduleH5(Box::new(ScheduleH5::from_data(data)?)),
            "H6" => Itemization::ScheduleH6(Box::new(ScheduleH6::from_data(data)?)),
            "F56" => Itemization::Form5Contribution(Box::new(Form5Contribution::from_data(data)?)),
            "F57" => Itemization::Form5Expenditure(Box::new(Form5Expenditure::from_data(data)?)),
            "F65" => Itemization::Form6Contribution(Box::new(Form6Contribution::from_data(data)?)),
            "F76" => {
                Itemization::Form7Communication(Box::new(Form7Communication::from_data(data)?))
            }
            "F91" => Itemization::Form9ControllingPerson(Box::new(
                Form9ControllingPerson::from_data(data)?,
            )),
            "F92" => Itemization::Form9Donation(Box::new(Form9Donation::from_data(data)?)),
            "F93" => Itemization::Form9Disbursement(Box::new(Form9Disbursement::from_data(data)?)),
            "F94" => Itemization::Form9Candidate(Box::new(Form9Candidate::from_data(data)?)),
            "F132" => Itemization::Form13Donation(Box::new(Form13Donation::from_data(data)?)),
            "F133" => Itemization::Form13Refund(Box::new(Form13Refund::from_data(data)?)),
            "SL" => Itemization::ScheduleL(Box::new(ScheduleL::from_data(data)?)),
            "TEXT" => Itemization::Text(Box::new(TextRecord::from_data(data)?)),
            "SA3L" => Itemization::ScheduleA3L(Box::new(ScheduleA3L::from_data(data)?)),
            "SE" => Itemization::ScheduleE(Box::new(ScheduleE::from_data(data)?)),
            "SC" => Itemization::ScheduleC(Box::new(ScheduleC::from_data(data)?)),
            "SC1" => Itemization::ScheduleC1(Box::new(ScheduleC1::from_data(data)?)),
            "SC2" => Itemization::ScheduleC2(Box::new(ScheduleC2::from_data(data)?)),
            _ => return None,
        })
    }

    /// The row type as filed, e.g. `SA11AI`.
    pub fn form_type(&self) -> &str {
        match self {
            Itemization::ScheduleA(s) => &s.form_type,
            Itemization::ScheduleB(s) => &s.form_type,
            Itemization::ScheduleD(s) => &s.form_type,
            Itemization::ScheduleF(s) => &s.form_type,
            Itemization::ScheduleH1(s) => &s.form_type,
            Itemization::ScheduleH2(s) => &s.form_type,
            Itemization::ScheduleH3(s) => &s.form_type,
            Itemization::ScheduleH4(s) => &s.form_type,
            Itemization::ScheduleH5(s) => &s.form_type,
            Itemization::ScheduleH6(s) => &s.form_type,
            Itemization::Form5Contribution(s) => &s.form_type,
            Itemization::Form5Expenditure(s) => &s.form_type,
            Itemization::Form6Contribution(s) => &s.form_type,
            Itemization::Form7Communication(s) => &s.form_type,
            Itemization::Form9ControllingPerson(s) => &s.form_type,
            Itemization::Form9Donation(s) => &s.form_type,
            Itemization::Form9Disbursement(s) => &s.form_type,
            Itemization::Form9Candidate(s) => &s.form_type,
            Itemization::Form13Donation(s) => &s.form_type,
            Itemization::Form13Refund(s) => &s.form_type,
            Itemization::ScheduleL(s) => &s.form_type,
            Itemization::Text(s) => &s.form_type,
            Itemization::ScheduleA3L(s) => &s.form_type,
            Itemization::ScheduleE(s) => &s.form_type,
            Itemization::ScheduleC(s) => &s.form_type,
            Itemization::ScheduleC1(s) => &s.form_type,
            Itemization::ScheduleC2(s) => &s.form_type,
        }
    }

    /// The filer's ID for the transaction, unique within the report; see each
    /// schedule's `transaction_id`. `None` for records without one.
    pub fn transaction_id(&self) -> Option<&str> {
        match self {
            Itemization::ScheduleA(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleB(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleD(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleF(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleH1(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleH2(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleH3(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleH4(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleH5(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleH6(s) => s.transaction_id.as_deref(),
            Itemization::Form5Contribution(s) => s.transaction_id.as_deref(),
            Itemization::Form5Expenditure(s) => s.transaction_id.as_deref(),
            Itemization::Form6Contribution(s) => s.transaction_id.as_deref(),
            Itemization::Form7Communication(s) => s.transaction_id.as_deref(),
            Itemization::Form9ControllingPerson(s) => s.transaction_id.as_deref(),
            Itemization::Form9Donation(s) => s.transaction_id.as_deref(),
            Itemization::Form9Disbursement(s) => s.transaction_id.as_deref(),
            Itemization::Form9Candidate(s) => s.transaction_id.as_deref(),
            Itemization::Form13Donation(s) => s.transaction_id.as_deref(),
            Itemization::Form13Refund(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleL(s) => s.transaction_id.as_deref(),
            Itemization::Text(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleA3L(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleE(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleC(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleC1(s) => s.transaction_id.as_deref(),
            Itemization::ScheduleC2(s) => s.transaction_id.as_deref(),
        }
    }

    /// The summary-page line this record itemizes; see [`line_number`].
    pub fn line_number(&self) -> Option<&str> {
        line_number(self.form_type())
    }
}

/// The summary-page line an itemization's row type carries: `SA11AI` →
/// `11AI`, `SB21B` → `21B`, `SC/10` and `SC1/10` → `10`, `SD9` → `9`.
/// `None` for row types without one (`SE`, `SF`, Form 3L's `SA3L`/`SB3L`). The line belongs to the
/// parent form: `11AI` is line 11(a)(i) on Forms 3 and 3X but itemized
/// individual contributions are line 17(a)(i) on Form 3P (FEC format
/// specification v8.4, `FEC_Format_v8.4.pdf` Appendix A p18–19; see
/// fec-docs `wiki/Itemization-Form-Types.md`).
pub fn line_number(form_type: &str) -> Option<&str> {
    let form_type = form_type.trim();
    let prefix = ["SC1", "SC2", "SA", "SB", "SC", "SD", "SE", "SF"]
        .into_iter()
        .find(|p| {
            form_type
                .get(..p.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(p))
        })?;
    let line = form_type[prefix.len()..].trim_start_matches('/');
    // `SA3L`/`SB3L` name Form 3L's schedules, not a summary-page line.
    (!line.is_empty() && !line.eq_ignore_ascii_case("3L")).then_some(line)
}

/// The FEC's name for an `entity_type` code: `CAN` Candidate, `CCM`
/// Candidate Committee, `COM` Committee, `IND` Individual (a person), `ORG`
/// Organization (not a committee and not a person), `PAC` Political Action
/// Committee, `PTY` Party Organization (FEC format specification v8.4,
/// `FEC_Format_v8.4.pdf` p10). `None` for any other value.
pub fn entity_type_label(code: &str) -> Option<&'static str> {
    Some(match code.trim().to_ascii_uppercase().as_str() {
        "CAN" => "Candidate",
        "CCM" => "Candidate Committee",
        "COM" => "Committee",
        "IND" => "Individual",
        "ORG" => "Organization",
        "PAC" => "Political Action Committee",
        "PTY" => "Party Organization",
        _ => return None,
    })
}

/// The other party to an itemized transaction: a contributor, payee, lender
/// or creditor. The FEC format gives each one an `entity_type` code, an
/// organization name (required unless the entity is an individual or a
/// candidate) and a person's name (required for those two), and an address
/// (e.g. FEC format workbook v8.4, sheet `Sch A`, fields 6–17).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        module = "libfec_parser.itemizations",
        frozen,
        get_all,
        skip_from_py_object
    )
)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Entity {
    /// `IND`, `ORG`, `COM`, … — see [`Entity::entity_type_label`]. Absent
    /// before v5.0 and on paper filings.
    pub entity_type: Option<String>,
    /// The organization's name, for anything but a person.
    pub organization_name: Option<String>,
    /// The person's name, for an individual or a candidate.
    pub name: PersonName,
    /// The entity's mailing address.
    pub address: Address,
}

impl Entity {
    /// Read `{prefix}organization_name`, `{prefix}last_name`, …,
    /// `{prefix}street_1`, … and `entity_type`. `legacy_name_key` is the one
    /// combined-name column of v1–5.x layouts (`contributor_name`): split as a
    /// person's name for individuals and candidates (or any value containing
    /// the filing's name delimiter), otherwise kept as the organization name.
    pub(crate) fn from_prefixed<F: Fields + ?Sized>(
        data: &F,
        prefix: &str,
        legacy_name_key: &str,
    ) -> Self {
        let entity_type = text(data, "entity_type");
        let mut organization_name = text(data, &key(prefix, "organization_name"));
        let mut name = person_name(data, prefix);
        if organization_name.is_none() && name.is_empty() {
            if let Some(raw) = text(data, legacy_name_key) {
                let person = matches!(entity_type.as_deref(), Some("IND" | "CAN"))
                    || raw.contains(data.name_delimiter());
                if person {
                    name = split_legacy_name(&raw, data.name_delimiter());
                } else {
                    organization_name = Some(raw);
                }
            }
        }
        Self {
            entity_type,
            organization_name,
            name,
            address: Address::from_prefixed(data, prefix),
        }
    }

    /// True for an individual (`IND`) or a candidate (`CAN`), the entity
    /// types identified by a person's name.
    pub fn is_individual(&self) -> bool {
        matches!(self.entity_type.as_deref(), Some("IND" | "CAN"))
            || (self.entity_type.is_none()
                && self.organization_name.is_none()
                && !self.name.is_empty())
    }

    /// The name to show: the person's for individuals and candidates, else
    /// the organization's (falling back to whichever is present).
    pub fn display_name(&self) -> String {
        let person = self.name.to_string();
        match (&self.organization_name, self.is_individual()) {
            (Some(org), false) => org.clone(),
            (_, true) if !person.is_empty() => person,
            (Some(org), _) => org.clone(),
            (None, _) => person,
        }
    }

    /// See [`entity_type_label`].
    pub fn entity_type_label(&self) -> Option<&'static str> {
        entity_type_label(self.entity_type.as_deref()?)
    }
}

/// A federal candidate an itemization names: the donor candidate of a
/// receipt, the beneficiary of a disbursement, the candidate an independent
/// expenditure supports or opposes. Office is `H`, `S` or `P`; state and
/// district identify the seat (e.g. FEC format workbook v8.4, sheet `Sch A`,
/// fields 28–36).
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        module = "libfec_parser.itemizations",
        frozen,
        get_all,
        skip_from_py_object
    )
)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CandidateRef {
    /// The candidate's FEC ID (`H0VA01234`).
    pub fec_id: Option<String>,
    /// The candidate's name.
    pub name: PersonName,
    /// `H` House, `S` Senate, `P` President; see [`CandidateRef::office_label`].
    pub office: Option<String>,
    /// The state of the seat sought (two-letter code); for a presidential
    /// candidate, usually blank or `US`.
    pub state: Option<String>,
    /// The congressional district of a House seat (`01`); blank otherwise.
    pub district: Option<String>,
}

impl CandidateRef {
    /// Read `id_key`, then `{prefix}last_name`, … (or the legacy combined
    /// `{prefix}name`), `{prefix}office`, `{prefix}state`, `{prefix}district`.
    pub(crate) fn from_prefixed<F: Fields + ?Sized>(data: &F, id_key: &str, prefix: &str) -> Self {
        Self {
            fec_id: text(data, id_key),
            name: person_name_or_legacy(data, prefix, &key(prefix, "name")),
            office: text(data, &key(prefix, "office")),
            state: text(data, &key(prefix, "state")),
            district: text(data, &key(prefix, "district")),
        }
    }

    /// True when no part of the reference is filled in.
    pub fn is_empty(&self) -> bool {
        self.fec_id.is_none()
            && self.name.is_empty()
            && self.office.is_none()
            && self.state.is_none()
            && self.district.is_none()
    }

    /// "House", "Senate" or "President"; see [`crate::covers::office_label`].
    pub fn office_label(&self) -> Option<&'static str> {
        crate::covers::office_label(self.office.as_deref()?)
    }
}

/// The column index of one `(row type, FEC version)` layout, shared by every
/// row of that pair.
#[derive(Debug)]
pub struct Layout {
    index: HashMap<String, usize, BuildHasherDefault<Fnv>>,
}

/// FNV-1a: column names are short and trusted, and SipHash showed up in
/// profiles of typing itemizations (a row reads ~40 columns by name).
#[derive(Default)]
struct Fnv(u64);

impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut hash = if self.0 == 0 {
            0xcbf2_9ce4_8422_2325
        } else {
            self.0
        };
        for b in bytes {
            hash = (hash ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
        }
        self.0 = hash;
    }
}

type LayoutKey = (String, String);

static LAYOUTS: OnceLock<RwLock<HashMap<LayoutKey, Option<Arc<Layout>>>>> = OnceLock::new();

impl Layout {
    /// The layout of `row_type` in `fec_version` from `mappings2.json`, or
    /// `None` if there is none. Cached, misses included.
    pub fn get(row_type: &str, fec_version: &str) -> Option<Arc<Layout>> {
        // Rows come in runs of one row type, so remember the last lookup per
        // thread: typing a row then neither allocates a key nor takes a lock.
        type LastLookup = Option<(String, String, Option<Arc<Layout>>)>;
        thread_local! {
            static LAST: std::cell::RefCell<LastLookup> = const { std::cell::RefCell::new(None) };
        }
        let hit = LAST.with_borrow(|last| match last {
            Some((r, v, layout)) if r == row_type && v == fec_version => Some(layout.clone()),
            _ => None,
        });
        if let Some(layout) = hit {
            return layout;
        }
        let layout = Self::lookup(row_type, fec_version);
        LAST.set(Some((
            row_type.to_owned(),
            fec_version.to_owned(),
            layout.clone(),
        )));
        layout
    }

    fn lookup(row_type: &str, fec_version: &str) -> Option<Arc<Layout>> {
        let key = (row_type.trim().to_ascii_uppercase(), fec_version.to_owned());
        let layouts = LAYOUTS.get_or_init(Default::default);
        if let Some(hit) = layouts.read().unwrap_or_else(|e| e.into_inner()).get(&key) {
            return hit.clone();
        }
        let layout = crate::mappings::column_names_for_field(&key.0, fec_version)
            .ok()
            .map(|columns| {
                let mut index =
                    HashMap::with_capacity_and_hasher(columns.len(), Default::default());
                for (i, name) in columns.iter().enumerate() {
                    // As on covers, a repeated name keeps its first column.
                    index.entry(name.clone()).or_insert(i);
                }
                Arc::new(Layout { index })
            });
        layouts
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .entry(key)
            .or_insert(layout)
            .clone()
    }
}

/// A raw record read by column name through its [`Layout`], without copying.
pub struct RecordFields<'a> {
    pub layout: &'a Layout,
    pub record: &'a StringRecord,
    /// The header's name delimiter, `^` when unset.
    pub name_delimiter: &'a str,
}

impl Fields for RecordFields<'_> {
    fn raw(&self, key: &str) -> Option<&str> {
        self.record.get(*self.layout.index.get(key)?)
    }

    fn name_delimiter(&self) -> &str {
        self.name_delimiter
    }
}

/// The address of a prefixed party, whichever spelling the layout uses for
/// its streets (`conduit_street1` on Schedule A, `conduit_street_1` on B).
pub(crate) fn address_either<F: Fields + ?Sized>(data: &F, prefix: &str) -> Address {
    let mut address = Address::from_prefixed(data, prefix);
    if address.street_1.is_none() {
        address.street_1 = text(data, &key(prefix, "street1"));
    }
    if address.street_2.is_none() {
        address.street_2 = text(data, &key(prefix, "street2"));
    }
    address
}

/// The first of `keys` with a non-blank value.
pub(crate) fn text_any<F: Fields + ?Sized>(data: &F, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| text(data, k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_numbers() {
        assert_eq!(line_number("SA11AI"), Some("11AI"));
        assert_eq!(line_number("sb21b"), Some("21b"));
        assert_eq!(line_number("SC/10"), Some("10"));
        assert_eq!(line_number("SC1/9"), Some("9"));
        assert_eq!(line_number("SC2/10"), Some("10"));
        assert_eq!(line_number("SD10"), Some("10"));
        assert_eq!(line_number("SE"), None);
        assert_eq!(line_number("SA3L"), None);
        assert_eq!(line_number("SB3L"), None);
        assert_eq!(line_number("F3XN"), None);
    }

    #[test]
    fn families() {
        assert_eq!(record_family("SA11AI"), Some("SA"));
        assert_eq!(record_family("sa3l"), Some("SA3L"));
        assert_eq!(record_family("SC1/10"), Some("SC1"));
        assert_eq!(record_family("SC/10"), Some("SC"));
        assert_eq!(record_family("H4"), Some("H4"));
        assert_eq!(record_family("F3XN"), None);
        assert_eq!(record_family("F3PS"), None);
    }

    #[test]
    fn legacy_entity_names() {
        let data: crate::covers::fields::Data = [
            ("entity_type", "IND"),
            ("contributor_name", "Smith^Jane^Ms.^"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
        let e = Entity::from_prefixed(&data, "contributor_", "contributor_name");
        assert_eq!(e.name.last_name, "Smith");
        assert_eq!(e.display_name(), "Ms. Jane Smith");

        let data: crate::covers::fields::Data =
            [("entity_type", "ORG"), ("contributor_name", "ACME Corp")]
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect();
        let e = Entity::from_prefixed(&data, "contributor_", "contributor_name");
        assert_eq!(e.organization_name.as_deref(), Some("ACME Corp"));
        assert_eq!(e.display_name(), "ACME Corp");
    }
}
