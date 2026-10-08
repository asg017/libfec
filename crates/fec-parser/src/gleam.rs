//! Gleam descriptions and Erlang encoders for the typed covers and
//! itemizations (feature `gleam`).
//!
//! `#[derive(GleamType)]` (from `fec-parser-macros`) is put on every struct
//! reachable from [`Cover`](crate::covers::Cover) and
//! [`Itemization`](crate::itemizations::Itemization), and on those two enums.
//! For each type it implements:
//!
//! - [`GleamType`]: a runtime description ([`GleamDef`]) of the Gleam custom
//!   type to generate: its fields in declaration order, their Gleam types and
//!   the Rust doc comments. The Gleam bindings' `gen` binary walks it from
//!   `Cover::gleam_def()` and `Itemization::gleam_def()` and writes the
//!   `.gleam` modules.
//! - [`GleamValue`] and [`rustler::Encoder`]: the Erlang term Gleam reads as
//!   that type. A struct `X { a, b }` is the tuple `{x, A, B}` (the
//!   constructor atom is [`gleam_snake_case`] of the type name, fields in
//!   declaration order); an enum variant `V(payload)` is `{v, Payload}`.
//!
//! Both halves come from the same struct definition, so the field order
//! on the Gleam side and the Rust side cannot drift.
//!
//! Field types and their Gleam encodings ([`GleamValue`] impls):
//!
//! | Rust | Gleam | Erlang term |
//! |---|---|---|
//! | `String`, `&'static str` | `String` | binary |
//! | `f64` | `Float` | float; NaN/±inf (no BEAM equivalent) → `0.0`, and `Some` of one → `none` |
//! | `bool` | `Bool` | `true` / `false` |
//! | `i16`, `u16` | `Int` | integer |
//! | `jiff::civil::Date` | `calendar.Date` (gleam_time) | `{date, Year, MonthAtom, Day}`, `MonthAtom` = `january` … `december` |
//! | `Option<T>` | `Option(T)` | `none` / `{some, T}` (**not** rustler's `nil` / bare value) |
//! | `Vec<T>` | `List(T)` | list |
//! | `Box<T>` | `T` | `T` |
//! | a struct deriving `GleamType` | its record | `{atom, …}` |
//!
//! Any other field type is a compile error from the derive. Field names that
//! are Gleam keywords would be a compile error too (none exists today; see
//! [`GLEAM_KEYWORDS`]).

use jiff::civil::Date;

/// The Gleam type of one field (or of an enum variant's payload).
#[derive(Debug, Clone)]
pub enum GleamTy {
    /// `String`
    String,
    /// `Float`
    Float,
    /// `Int`
    Int,
    /// `Bool`
    Bool,
    /// gleam_time's `calendar.Date`
    Date,
    /// `Option(T)` from `gleam/option`
    Option(Box<GleamTy>),
    /// `List(T)`
    List(Box<GleamTy>),
    /// Another generated type.
    Named(GleamRef),
}

impl GleamTy {
    /// Every [`GleamTy::Named`] inside this type (`Option(List(X))` → `X`).
    pub fn named(&self) -> Option<&GleamRef> {
        match self {
            GleamTy::Named(r) => Some(r),
            GleamTy::Option(t) | GleamTy::List(t) => t.named(),
            _ => None,
        }
    }
}

/// A reference to another type deriving [`GleamType`].
#[derive(Debug, Clone, Copy)]
pub struct GleamRef {
    /// Rust (and Gleam) type name, e.g. `"Address"`.
    pub rust_name: &'static str,
    /// `module_path!()` where the type is defined, e.g. `"fec_parser::covers"`.
    pub rust_module: &'static str,
    /// The referenced type's own description.
    pub def: fn() -> GleamDef,
}

impl PartialEq for GleamRef {
    fn eq(&self, other: &Self) -> bool {
        self.rust_name == other.rust_name && self.rust_module == other.rust_module
    }
}
impl Eq for GleamRef {}

/// One field of a record, in declaration order (the order is the contract
/// with the Erlang tuple).
#[derive(Debug, Clone)]
pub struct GleamField {
    /// The Gleam label: the Rust field name (no field name needs mapping,
    /// see [`GLEAM_KEYWORDS`]).
    pub name: &'static str,
    pub ty: GleamTy,
    /// The field's Rust doc comment, one leading space per line stripped.
    /// Empty if undocumented.
    pub doc: &'static str,
}

/// One constructor of a Gleam type generated from a newtype-variant enum.
#[derive(Debug, Clone)]
pub struct GleamVariant {
    /// Rust variant name, which is also the Gleam constructor name
    /// (`"Form3X"`). Its Erlang atom is [`gleam_snake_case`] of it.
    pub name: &'static str,
    /// The payload type (`Box<T>` is described as `T`).
    pub payload: GleamTy,
    /// The variant's Rust doc comment. Empty if undocumented.
    pub doc: &'static str,
}

/// What a generated Gleam type looks like.
#[derive(Debug, Clone)]
pub enum GleamShape {
    /// A struct: a Gleam type with one constructor of the same name.
    Record { fields: Vec<GleamField> },
    /// An enum of single-field tuple variants: one constructor per variant.
    Variants { variants: Vec<GleamVariant> },
}

/// The description of one Gleam type, from [`GleamType::gleam_def`].
#[derive(Debug, Clone)]
pub struct GleamDef {
    /// Rust type name, also the Gleam type (and, for a record, constructor)
    /// name: `"ScheduleA"`.
    pub rust_name: &'static str,
    /// `module_path!()` where the type is defined:
    /// `"fec_parser::itemizations::schedule_a"`. The Gleam module layout is
    /// decided from this by the bindings' `gen` binary.
    pub rust_module: &'static str,
    /// The type's Rust doc comment, one leading space per line stripped.
    pub doc: &'static str,
    pub shape: GleamShape,
}

impl GleamDef {
    /// The types this one refers to directly, in field/variant order, with
    /// duplicates removed.
    pub fn deps(&self) -> Vec<GleamRef> {
        let tys: Vec<&GleamTy> = match &self.shape {
            GleamShape::Record { fields } => fields.iter().map(|f| &f.ty).collect(),
            GleamShape::Variants { variants } => variants.iter().map(|v| &v.payload).collect(),
        };
        let mut out: Vec<GleamRef> = Vec::new();
        for r in tys.into_iter().filter_map(GleamTy::named) {
            if !out.contains(r) {
                out.push(*r);
            }
        }
        out
    }

    /// This type and every type reachable from it, each once, depth-first
    /// in field order (this type first).
    pub fn walk(&self) -> Vec<GleamDef> {
        fn go(
            def: GleamDef,
            seen: &mut Vec<(&'static str, &'static str)>,
            out: &mut Vec<GleamDef>,
        ) {
            let key = (def.rust_module, def.rust_name);
            if seen.contains(&key) {
                return;
            }
            seen.push(key);
            let deps = def.deps();
            out.push(def);
            for d in deps {
                go((d.def)(), seen, out);
            }
        }
        let mut seen = Vec::new();
        let mut out = Vec::new();
        go(self.clone(), &mut seen, &mut out);
        out
    }

    /// The Erlang atom of this type's record constructor
    /// (`gleam_snake_case(rust_name)`).
    pub fn atom(&self) -> String {
        gleam_snake_case(self.rust_name)
    }
}

/// A type with a generated Gleam counterpart. Implemented by
/// `#[derive(GleamType)]`; see the [module docs](self).
pub trait GleamType {
    /// The Rust type name.
    const RUST_NAME: &'static str;
    /// `module_path!()` at the type's definition.
    const RUST_MODULE: &'static str;
    /// The description of the Gleam type.
    fn gleam_def() -> GleamDef;
}

/// A Rust type that can be a field of a type deriving `GleamType`: knows its
/// Gleam type and (with a live NIF environment) encodes itself as the term
/// Gleam reads as that type.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no Gleam encoding",
    note = "supported field types: String, &'static str, f64, bool, i16, u16, jiff::civil::Date, Option<T>, Vec<T>, Box<T>, and structs deriving GleamType"
)]
pub trait GleamValue {
    fn gleam_ty() -> GleamTy;
    fn encode_gleam<'a>(&self, env: rustler::Env<'a>) -> rustler::Term<'a>;
    /// Whether `Some(self)` is encoded as `None`: true only for a
    /// non-finite `f64` (see the `f64` impl).
    fn gleam_absent(&self) -> bool {
        false
    }
}

// Re-exported so the derive's output names `crate::gleam::rustler`.
#[doc(hidden)]
pub use rustler;

mod atoms {
    rustler::atoms! { none, some, date, january, february, march, april, may, june, july, august, september, october, november, december }
}

use rustler::Encoder;

impl GleamValue for String {
    fn gleam_ty() -> GleamTy {
        GleamTy::String
    }
    fn encode_gleam<'a>(&self, env: rustler::Env<'a>) -> rustler::Term<'a> {
        self.as_str().encode(env)
    }
}

/// A string from a static table (e.g. `Form3PStateAllocation::state`).
impl GleamValue for &'static str {
    fn gleam_ty() -> GleamTy {
        GleamTy::String
    }
    fn encode_gleam<'a>(&self, env: rustler::Env<'a>) -> rustler::Term<'a> {
        (*self).encode(env)
    }
}

/// The BEAM has no NaN or infinity: `enif_make_double` raises `badarg` for
/// them, which would crash the NIF call. `amount`/`amount_opt`
/// (`covers::fields`) parse with `f64::from_str`, which accepts `nan`,
/// `inf` and `1e999`, so a filing can produce them. They are treated like
/// any other unparsable amount: a non-finite `f64` is encoded as `0.0`
/// (what `amount` gives for garbage) and `Some(non-finite)` as `None` (what
/// `amount_opt` gives), see [`GleamValue::gleam_absent`].
impl GleamValue for f64 {
    fn gleam_ty() -> GleamTy {
        GleamTy::Float
    }
    fn encode_gleam<'a>(&self, env: rustler::Env<'a>) -> rustler::Term<'a> {
        if self.is_finite() { *self } else { 0.0 }.encode(env)
    }
    fn gleam_absent(&self) -> bool {
        !self.is_finite()
    }
}

impl GleamValue for bool {
    fn gleam_ty() -> GleamTy {
        GleamTy::Bool
    }
    fn encode_gleam<'a>(&self, env: rustler::Env<'a>) -> rustler::Term<'a> {
        self.encode(env)
    }
}

impl GleamValue for i16 {
    fn gleam_ty() -> GleamTy {
        GleamTy::Int
    }
    fn encode_gleam<'a>(&self, env: rustler::Env<'a>) -> rustler::Term<'a> {
        i64::from(*self).encode(env)
    }
}

impl GleamValue for u16 {
    fn gleam_ty() -> GleamTy {
        GleamTy::Int
    }
    fn encode_gleam<'a>(&self, env: rustler::Env<'a>) -> rustler::Term<'a> {
        i64::from(*self).encode(env)
    }
}

/// gleam_time's `calendar.Date(year:, month:, day:)`: `{date, Y, MonthAtom, D}`.
impl GleamValue for Date {
    fn gleam_ty() -> GleamTy {
        GleamTy::Date
    }
    fn encode_gleam<'a>(&self, env: rustler::Env<'a>) -> rustler::Term<'a> {
        let month = match self.month() {
            1 => atoms::january(),
            2 => atoms::february(),
            3 => atoms::march(),
            4 => atoms::april(),
            5 => atoms::may(),
            6 => atoms::june(),
            7 => atoms::july(),
            8 => atoms::august(),
            9 => atoms::september(),
            10 => atoms::october(),
            11 => atoms::november(),
            _ => atoms::december(),
        };
        (
            atoms::date(),
            i64::from(self.year()),
            month,
            i64::from(self.day()),
        )
            .encode(env)
    }
}

/// Gleam's `Option(a)`: the atom `none`, or `{some, A}`. Rustler's own
/// `Option` encoder gives Elixir's `nil` / bare value, which Gleam would
/// misread, so it is never used for these types.
impl<T: GleamValue> GleamValue for Option<T> {
    fn gleam_ty() -> GleamTy {
        GleamTy::Option(Box::new(T::gleam_ty()))
    }
    fn encode_gleam<'a>(&self, env: rustler::Env<'a>) -> rustler::Term<'a> {
        match self {
            Some(v) if !v.gleam_absent() => rustler::types::tuple::make_tuple(
                env,
                &[atoms::some().encode(env), v.encode_gleam(env)],
            ),
            _ => atoms::none().encode(env),
        }
    }
}

impl<T: GleamValue> GleamValue for Vec<T> {
    fn gleam_ty() -> GleamTy {
        GleamTy::List(Box::new(T::gleam_ty()))
    }
    fn encode_gleam<'a>(&self, env: rustler::Env<'a>) -> rustler::Term<'a> {
        let terms: Vec<rustler::Term<'a>> = self.iter().map(|v| v.encode_gleam(env)).collect();
        terms.encode(env)
    }
}

impl<T: GleamValue> GleamValue for Box<T> {
    fn gleam_ty() -> GleamTy {
        T::gleam_ty()
    }
    fn encode_gleam<'a>(&self, env: rustler::Env<'a>) -> rustler::Term<'a> {
        (**self).encode_gleam(env)
    }
    fn gleam_absent(&self) -> bool {
        (**self).gleam_absent()
    }
}

/// Used by the derive: the cached constructor atom for `name`.
#[doc(hidden)]
pub fn __atom<'a>(
    env: rustler::Env<'a>,
    cell: &'static std::sync::OnceLock<rustler::Atom>,
    name: &str,
) -> rustler::Term<'a> {
    // Atoms are never garbage collected, so one created once stays valid
    // for every later env (rustler's `atoms!` caches them the same way).
    cell.get_or_init(|| {
        rustler::Atom::from_str(env, &gleam_snake_case(name))
            .unwrap_or_else(|_| panic!("invalid atom for {name}"))
    })
    .encode(env)
}

/// Gleam reserved words, which cannot be record labels. The derive rejects a
/// field with one of these names (none exists today); if one is ever needed,
/// map it to e.g. `type_` in the derive and document it here.
pub const GLEAM_KEYWORDS: &[&str] = &[
    "as",
    "assert",
    "auto",
    "case",
    "const",
    "delegate",
    "derive",
    "echo",
    "else",
    "fn",
    "if",
    "implement",
    "import",
    "let",
    "macro",
    "opaque",
    "panic",
    "pub",
    "test",
    "todo",
    "type",
    "use",
];

/// The Erlang atom Gleam compiles a constructor named `name` to: Gleam's
/// `to_snake_case` (compiler-core). A new word starts at each uppercase
/// letter (and after `_`); digits continue the current word. So
/// `ScheduleA` → `schedule_a`, `Form5Contribution` → `form5_contribution`,
/// `F3XN` → `f3_x_n`, `SA11AI` → `s_a11_a_i`. Verified against Gleam 1.15.2's
/// compiled output (see the tests).
pub fn gleam_snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    let mut boundary = true;
    for c in name.chars() {
        if c == '_' || c == ' ' {
            boundary = true;
            continue;
        }
        if c.is_uppercase() {
            boundary = true;
        }
        if boundary {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            out.extend(c.to_lowercase());
            boundary = false;
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::covers::Cover;
    use crate::itemizations::Itemization;

    /// Constructor name → atom, read from the `.erl` that Gleam 1.15.2
    /// compiled a scratch module of `pub type T { Name(x: Int) … }` to
    /// (`all() -> [{address, 1}, …]`), 2026-10-05. Every type and root-enum
    /// variant name (see `every_constructor_is_verified`), plus `F3XN` and
    /// `SA11AI`. The same build gave `{{date, 2023, july, 1}, {some, 1}, none}`
    /// for `#(calendar.Date(2023, calendar.July, 1), Some(1), None)`.
    const VERIFIED: &[(&str, &str)] = &[
        ("Address", "address"),
        ("CandidateRef", "candidate_ref"),
        ("Cover", "cover"),
        ("DetailedSummaryRow", "detailed_summary_row"),
        ("Entity", "entity"),
        ("F3XN", "f3_x_n"),
        ("Form1", "form1"),
        ("Form13", "form13"),
        ("Form13Donation", "form13_donation"),
        ("Form13Refund", "form13_refund"),
        ("Form1Affiliated", "form1_affiliated"),
        ("Form1Bank", "form1_bank"),
        ("Form1Candidate", "form1_candidate"),
        ("Form1Contact", "form1_contact"),
        ("Form1M", "form1_m"),
        ("Form1MAffiliation", "form1_m_affiliation"),
        ("Form1MCandidate", "form1_m_candidate"),
        ("Form1MQualification", "form1_m_qualification"),
        ("Form1PacFlags", "form1_pac_flags"),
        ("Form2", "form2"),
        ("Form24", "form24"),
        ("Form2Committee", "form2_committee"),
        (
            "Form2PersonalFundsDeclaration",
            "form2_personal_funds_declaration",
        ),
        ("Form3", "form3"),
        ("Form3CashSummary", "form3_cash_summary"),
        ("Form3DetailedSummary", "form3_detailed_summary"),
        (
            "Form3DetailedSummaryDisbursements",
            "form3_detailed_summary_disbursements",
        ),
        (
            "Form3DetailedSummaryReceipts",
            "form3_detailed_summary_receipts",
        ),
        ("Form3L", "form3_l"),
        ("Form3P", "form3_p"),
        ("Form3PDetailedSummary", "form3_p_detailed_summary"),
        (
            "Form3PDetailedSummaryDisbursements",
            "form3_p_detailed_summary_disbursements",
        ),
        (
            "Form3PDetailedSummaryReceipts",
            "form3_p_detailed_summary_receipts",
        ),
        ("Form3PStateAllocation", "form3_p_state_allocation"),
        ("Form3PStateAllocations", "form3_p_state_allocations"),
        ("Form3PSummary", "form3_p_summary"),
        ("Form3Summary", "form3_summary"),
        ("Form3X", "form3_x"),
        ("Form3XDetailedSummary", "form3_x_detailed_summary"),
        ("Form3XDisbursements", "form3_x_disbursements"),
        (
            "Form3XNetContributionsAndOperatingExpenditures",
            "form3_x_net_contributions_and_operating_expenditures",
        ),
        ("Form3XReceipts", "form3_x_receipts"),
        ("Form3XSummary", "form3_x_summary"),
        ("Form4", "form4"),
        ("Form4DetailedSummary", "form4_detailed_summary"),
        ("Form4Disbursements", "form4_disbursements"),
        ("Form4ItemizedLine", "form4_itemized_line"),
        ("Form4LoanLine", "form4_loan_line"),
        ("Form4Receipts", "form4_receipts"),
        ("Form4Summary", "form4_summary"),
        ("Form5", "form5"),
        ("Form5Contribution", "form5_contribution"),
        ("Form5Expenditure", "form5_expenditure"),
        ("Form6", "form6"),
        ("Form6Candidate", "form6_candidate"),
        ("Form6Contribution", "form6_contribution"),
        ("Form7", "form7"),
        ("Form7Communication", "form7_communication"),
        ("Form9", "form9"),
        ("Form99", "form99"),
        ("Form9Candidate", "form9_candidate"),
        ("Form9ControllingPerson", "form9_controlling_person"),
        ("Form9Custodian", "form9_custodian"),
        ("Form9Disbursement", "form9_disbursement"),
        ("Form9Donation", "form9_donation"),
        ("Itemization", "itemization"),
        ("PersonName", "person_name"),
        ("SA11AI", "s_a11_a_i"),
        ("ScheduleA", "schedule_a"),
        ("ScheduleA3L", "schedule_a3_l"),
        ("ScheduleB", "schedule_b"),
        ("ScheduleC", "schedule_c"),
        ("ScheduleC1", "schedule_c1"),
        ("ScheduleC2", "schedule_c2"),
        ("ScheduleCGuarantor", "schedule_c_guarantor"),
        ("ScheduleD", "schedule_d"),
        ("ScheduleE", "schedule_e"),
        ("ScheduleF", "schedule_f"),
        ("ScheduleFCommittee", "schedule_f_committee"),
        ("ScheduleH1", "schedule_h1"),
        ("ScheduleH2", "schedule_h2"),
        ("ScheduleH3", "schedule_h3"),
        ("ScheduleH4", "schedule_h4"),
        ("ScheduleH5", "schedule_h5"),
        ("ScheduleH6", "schedule_h6"),
        ("ScheduleL", "schedule_l"),
        ("Text", "text"),
        ("TextRecord", "text_record"),
    ];

    /// Non-finite floats have no Erlang term; see the `f64` impl.
    #[test]
    fn non_finite_f64_is_absent() {
        for x in ["nan", "inf", "-inf", "1e999"] {
            let x: f64 = x.parse().expect("float literal");
            assert!(x.gleam_absent(), "{x}");
            assert!(Box::new(x).gleam_absent(), "{x}");
        }
        assert!(!1.5f64.gleam_absent());
        assert!(!0.0f64.gleam_absent());
    }

    #[test]
    fn snake_case_matches_compiled_gleam() {
        for (name, atom) in VERIFIED {
            assert_eq!(gleam_snake_case(name), *atom, "{name}");
        }
    }

    fn all_defs() -> Vec<GleamDef> {
        let mut defs = Cover::gleam_def().walk();
        for d in Itemization::gleam_def().walk() {
            if !defs
                .iter()
                .any(|e| (e.rust_module, e.rust_name) == (d.rust_module, d.rust_name))
            {
                defs.push(d);
            }
        }
        defs
    }

    /// Every constructor name the generated Gleam has: type names and the
    /// root enums' variant names.
    fn constructor_names() -> Vec<&'static str> {
        let mut names = Vec::new();
        for d in all_defs() {
            names.push(d.rust_name);
            if let GleamShape::Variants { variants } = &d.shape {
                names.extend(variants.iter().map(|v| v.name));
            }
        }
        names.sort();
        names.dedup();
        names
    }

    #[test]
    fn every_constructor_is_verified() {
        let verified: Vec<&str> = VERIFIED.iter().map(|(n, _)| *n).collect();
        let missing: Vec<&str> = constructor_names()
            .into_iter()
            .filter(|n| !verified.contains(n))
            .collect();
        assert!(missing.is_empty(), "not in VERIFIED: {missing:?}");
    }

    #[test]
    fn walk_reaches_every_type() {
        let defs = all_defs();
        // 15 covers + their nested structs + 27 itemizations + shared
        // structs, plus the two root enums.
        assert_eq!(defs.len(), 85);
        let names: Vec<&str> = defs.iter().map(|d| d.rust_name).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "a type name is defined twice");
        for d in &defs {
            for r in d.deps() {
                assert!(
                    defs.iter()
                        .any(|e| (e.rust_module, e.rust_name) == (r.rust_module, r.rust_name)),
                    "{} references unresolved {}",
                    d.rust_name,
                    r.rust_name
                );
                assert_eq!((r.def)().rust_name, r.rust_name);
            }
        }
        match Cover::gleam_def().shape {
            GleamShape::Variants { variants } => assert_eq!(variants.len(), 15),
            _ => panic!("Cover is not Variants"),
        }
        match Itemization::gleam_def().shape {
            GleamShape::Variants { variants } => assert_eq!(variants.len(), 27),
            _ => panic!("Itemization is not Variants"),
        }
    }

    #[test]
    fn no_field_is_a_gleam_keyword() {
        for d in all_defs() {
            if let GleamShape::Record { fields } = &d.shape {
                for f in fields {
                    assert!(
                        !GLEAM_KEYWORDS.contains(&f.name),
                        "{}.{}",
                        d.rust_name,
                        f.name
                    );
                }
            }
        }
    }

    #[test]
    fn descriptions() {
        let d = crate::covers::Address::gleam_def();
        assert_eq!(d.rust_module, "fec_parser::covers");
        assert_eq!(d.atom(), "address");
        let GleamShape::Record { fields } = &d.shape else {
            panic!()
        };
        let names: Vec<&str> = fields.iter().map(|f| f.name).collect();
        assert_eq!(names, ["street_1", "street_2", "city", "state", "zip_code"]);
        assert!(matches!(&fields[0].ty, GleamTy::Option(t) if matches!(**t, GleamTy::String)));

        let d = crate::itemizations::ScheduleA::gleam_def();
        assert_eq!(d.rust_module, "fec_parser::itemizations::schedule_a");
        assert!(
            d.doc
                .starts_with("\"SCHEDULE A - ITEMIZED RECEIPTS\": one receipt"),
            "{:?}",
            d.doc
        );
        assert!(d.doc.contains("\n"), "doc keeps its line breaks");
        let GleamShape::Record { fields } = &d.shape else {
            panic!()
        };
        let f = |n: &str| fields.iter().find(|f| f.name == n).expect(n);
        assert!(matches!(f("contribution_amount").ty, GleamTy::Float));
        assert!(
            matches!(&f("contribution_date").ty, GleamTy::Option(t) if matches!(**t, GleamTy::Date))
        );
        assert!(
            matches!(&f("contributor").ty, GleamTy::Named(r) if r.rust_name == "Entity"
            && r.rust_module == "fec_parser::itemizations")
        );
        assert!(!f("contribution_amount").doc.is_empty());

        // `Box<Form3X>` is described as `Form3X`; `Vec<T>` as `List(T)`.
        let GleamShape::Variants { variants } = Cover::gleam_def().shape else {
            panic!()
        };
        let v = variants
            .iter()
            .find(|v| v.name == "Form3X")
            .expect("Form3X");
        assert!(
            matches!(&v.payload, GleamTy::Named(r) if r.rust_name == "Form3X"
            && r.rust_module == "fec_parser::covers::form3x")
        );
        let d = crate::covers::Form3P::gleam_def();
        let GleamShape::Record { fields } = &d.shape else {
            panic!()
        };
        assert!(fields.iter().any(
            |f| matches!(&f.ty, GleamTy::Named(r) if r.rust_name == "Form3PStateAllocations")
        ));
        let d = crate::covers::Form3PStateAllocations::gleam_def();
        let GleamShape::Record { fields } = &d.shape else {
            panic!()
        };
        assert!(
            matches!(&fields[0].ty, GleamTy::List(t) if matches!(&**t, GleamTy::Named(r) if r.rust_name == "Form3PStateAllocation"))
        );
    }

    /// `cargo test -p fec-parser --features gleam -- --ignored --nocapture dump_constructor_names`
    /// prints a Gleam module with every constructor, for re-verifying
    /// [`VERIFIED`] against a Gleam compiler.
    #[test]
    #[ignore]
    fn dump_constructor_names() {
        let mut names = constructor_names();
        names.extend(["F3XN", "SA11AI", "ScheduleA", "Form5Contribution"]);
        names.sort();
        names.dedup();
        println!("pub type T {{");
        for n in names {
            println!("  {n}");
        }
        println!("}}");
    }
}
