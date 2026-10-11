//! Python bindings for the typed covers (`--features python`).
//!
//! Every cover struct is a `#[pyclass]` (`frozen`, `get_all`) through a
//! `cfg_attr` on its definition, so Python sees the very same structs and
//! fields, with the same names, as Rust and `libfec info -f json`. This module
//! adds the methods: the ones every cover class shares (`__repr__`, `__eq__`,
//! `to_dict()`), and each struct's own helpers (`is_amendment()`, the
//! `*_label()` lookups, …), which keep their Rust names.
//!
//! `crates/fec-py` registers the classes in `libfec.covers` and turns a
//! [`Cover`] into the matching class with [`cover_to_py`].

use pyo3::prelude::*;
use pyo3::pyclass::boolean_struct::True;
use pyo3::types::{PyDict, PyList};
use pyo3::PyClass;
use serde::Serialize;
use serde_json::Value;

use super::*;

/// What the shared methods need of a cover class: `frozen` (so `Bound::get`
/// reads it without a borrow flag) and `Serialize` (for its field list).
trait Covered: Serialize + PyClass<Frozen = True> + Sync {}
impl<T: Serialize + PyClass<Frozen = True> + Sync> Covered for T {}

/// The field names of `value` in declaration order, each with its JSON shape.
///
/// `serde` already walks the struct in declaration order with the public field
/// names, so it is the one place the field list lives; nothing here repeats it.
/// The JSON is read back entry by entry rather than into a `Value::Object`,
/// whose map sorts its keys unless serde_json's `preserve_order` is on.
fn fields<T: Serialize>(value: &T) -> Vec<(String, Value)> {
    struct Entries;

    impl<'de> serde::de::Visitor<'de> for Entries {
        type Value = Vec<(String, Value)>;

        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a struct")
        }

        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut entries = Vec::new();
            while let Some(entry) = map.next_entry()? {
                entries.push(entry);
            }
            Ok(entries)
        }
    }

    serde_json::to_string(value)
        .ok()
        .and_then(|json| {
            serde::Deserializer::deserialize_map(
                &mut serde_json::Deserializer::from_str(&json),
                Entries,
            )
            .ok()
        })
        .unwrap_or_default()
}

/// `Form3X(form_type='F3XN', filer_committee_id='C00016899', ...)`: the scalar
/// fields' reprs, and nested structs as `ClassName(...)` so a Form 3X does not
/// print its whole 100-line detailed summary.
fn repr<T: Covered>(slf: &Bound<'_, T>) -> PyResult<String> {
    let name = slf.as_any().get_type().qualname()?;
    let mut parts = Vec::new();
    for (key, json) in fields(slf.get()) {
        let value = slf.as_any().getattr(key.as_str())?;
        let shown = match json {
            Value::Object(_) => format!("{}(...)", value.get_type().qualname()?),
            Value::Array(items) => format!("[...{} items]", items.len()),
            _ => value.repr()?.to_string(),
        };
        parts.push(format!("{key}={shown}"));
    }
    Ok(format!("{name}({})", parts.join(", ")))
}

/// The fields as a `dict`, nested covers as nested dicts, values typed as the
/// attributes are (`datetime.date`, `float`, `bool`, `str`, `None`).
fn to_dict<'py, T: Covered>(slf: &Bound<'py, T>) -> PyResult<Bound<'py, PyDict>> {
    let py = slf.py();
    let dict = PyDict::new(py);
    for (key, json) in fields(slf.get()) {
        let value = slf.as_any().getattr(key.as_str())?;
        let value = match json {
            Value::Object(_) => value.call_method0("to_dict")?,
            Value::Array(items) if items.iter().any(Value::is_object) => {
                let list = PyList::empty(py);
                for item in value.try_iter()? {
                    list.append(item?.call_method0("to_dict")?)?;
                }
                list.into_any()
            }
            _ => value,
        };
        dict.set_item(key, value)?;
    }
    Ok(dict)
}

/// Equal when `other` is the same class and every field compares equal with
/// Python's `==`, as a dataclass's `__eq__` does: nested covers recurse, and a
/// NaN amount is unequal to everything, itself included. (Comparing the serde
/// JSON instead would be wrong: it writes NaN and both infinities as `null`.)
fn eq<T: Covered>(slf: &Bound<'_, T>, other: &Bound<'_, PyAny>) -> PyResult<bool> {
    if !other.is_instance_of::<T>() {
        return Ok(false);
    }
    for (key, _) in fields(slf.get()) {
        let key = key.as_str();
        if !slf.as_any().getattr(key)?.eq(other.getattr(key)?)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// `#[pymethods]` for one cover class: `__repr__`, `__eq__`, `to_dict()`, then
/// the struct's own helpers, each listed as `"name" wrapper = method -> Ret`.
///
/// The wrapper (`py_is_amendment`) calls the inherent method (`is_amendment`)
/// and is exposed under its name; it cannot reuse the name itself, and pyo3's
/// `name` wants a string literal, hence all three. One `#[pymethods]` block per
/// class, as pyo3 allows without `multiple-pymethods`.
macro_rules! cover_class {
    ($ty:ty $(, $name:literal $wrapper:ident = $method:ident -> $ret:ty)* $(,)?) => {
        #[pymethods]
        impl $ty {
            fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
                repr(slf)
            }

            fn __eq__(slf: &Bound<'_, Self>, other: &Bound<'_, PyAny>) -> PyResult<bool> {
                eq(slf, other)
            }

            /// The fields as a `dict` (nested covers as nested dicts), keyed and
            /// ordered like the attributes and `libfec info -f json`'s
            /// `cover_data`.
            fn to_dict<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyDict>> {
                to_dict(slf)
            }

            $(
                #[pyo3(name = $name)]
                fn $wrapper(&self) -> $ret {
                    self.$method()
                }
            )*
        }
    };
}

type Label = Option<&'static str>;

cover_class!(PersonName, "is_empty" py_is_empty = is_empty -> bool);
cover_class!(
    Address,
    "is_empty" py_is_empty = is_empty -> bool,
    "one_line" py_one_line = one_line -> String,
);
cover_class!(DetailedSummaryRow);

cover_class!(
    Form1,
    "is_amendment" py_is_amendment = is_amendment -> bool,
    "committee_type_label" py_committee_type_label = committee_type_label -> Label,
    "committee_type_line" py_committee_type_line = committee_type_line -> Label,
    "organization_type_label" py_organization_type_label = organization_type_label -> Label,
    "party_type_label" py_party_type_label = party_type_label -> Label,
    "party_code_label" py_party_code_label = party_code_label -> Label,
);
cover_class!(
    Form1PacFlags,
    "is_lobbyist_registrant_pac" py_is_lobbyist_registrant_pac = is_lobbyist_registrant_pac -> bool,
);
cover_class!(
    Form1Candidate,
    "full_name" py_full_name = full_name -> String,
    "office_label" py_office_label = office_label -> Label,
);
cover_class!(
    Form1Affiliated,
    "display_name" py_display_name = display_name -> String,
    "relationship_label" py_relationship_label = relationship_label -> Label,
);
cover_class!(Form1Contact, "is_empty" py_is_empty = is_empty -> bool);
cover_class!(Form1Bank);

cover_class!(
    Form1M,
    "is_amendment" py_is_amendment = is_amendment -> bool,
    "committee_type_label" py_committee_type_label = committee_type_label -> Label,
);
cover_class!(Form1MAffiliation);
cover_class!(Form1MQualification);
cover_class!(Form1MCandidate, "office_label" py_office_label = office_label -> Label);

cover_class!(
    Form2,
    "is_amendment" py_is_amendment = is_amendment -> bool,
    "office_label" py_office_label = office_label -> Label,
    "party_code_label" py_party_code_label = party_code_label -> Label,
);
cover_class!(Form2Committee, "is_empty" py_is_empty = is_empty -> bool);
cover_class!(Form2PersonalFundsDeclaration);

cover_class!(
    Form3,
    "is_amendment" py_is_amendment = is_amendment -> bool,
    "report_code_label" py_report_code_label = report_code_label -> Label,
    "election_code_label" py_election_code_label = election_code_label -> Label,
);
cover_class!(Form3Summary);
cover_class!(Form3DetailedSummary);
cover_class!(Form3DetailedSummaryReceipts);
cover_class!(Form3DetailedSummaryDisbursements);
cover_class!(Form3CashSummary);

cover_class!(
    Form3L,
    "is_amendment" py_is_amendment = is_amendment -> bool,
    "report_code_label" py_report_code_label = report_code_label -> Label,
    "covers_semi_annual_period" py_covers_semi_annual_period = covers_semi_annual_period -> bool,
);

cover_class!(
    Form3P,
    "is_amendment" py_is_amendment = is_amendment -> bool,
    "report_code_label" py_report_code_label = report_code_label -> Label,
    "election_code_label" py_election_code_label = election_code_label -> Label,
);
cover_class!(Form3PSummary);
cover_class!(Form3PDetailedSummary);
cover_class!(Form3PDetailedSummaryReceipts);
cover_class!(Form3PDetailedSummaryDisbursements);
cover_class!(Form3PStateAllocations, "is_empty" py_is_empty = is_empty -> bool);
cover_class!(Form3PStateAllocation);

cover_class!(
    Form3X,
    "is_amendment" py_is_amendment = is_amendment -> bool,
    "report_code_label" py_report_code_label = report_code_label -> Label,
    "election_code_label" py_election_code_label = election_code_label -> Label,
);
cover_class!(Form3XSummary);
cover_class!(Form3XDetailedSummary);
cover_class!(Form3XReceipts);
cover_class!(Form3XDisbursements);
cover_class!(Form3XNetContributionsAndOperatingExpenditures);

cover_class!(
    Form4,
    "is_amendment" py_is_amendment = is_amendment -> bool,
    "committee_type_label" py_committee_type_label = committee_type_label -> Label,
    "report_code_label" py_report_code_label = report_code_label -> Label,
);
cover_class!(Form4Summary);
cover_class!(Form4DetailedSummary);
cover_class!(Form4Receipts);
cover_class!(Form4Disbursements);
cover_class!(Form4ItemizedLine);
cover_class!(Form4LoanLine);

cover_class!(
    Form5,
    "is_amendment" py_is_amendment = is_amendment -> bool,
    "is_individual" py_is_individual = is_individual -> bool,
    "filer_name" py_filer_name = filer_name -> String,
    "report_code_label" py_report_code_label = report_code_label -> Label,
    "report_type_label" py_report_type_label = report_type_label -> Label,
);
cover_class!(Form6, "is_amendment" py_is_amendment = is_amendment -> bool);
cover_class!(Form6Candidate, "office_label" py_office_label = office_label -> Label);
cover_class!(
    Form7,
    "is_amendment" py_is_amendment = is_amendment -> bool,
    "organization_type_label" py_organization_type_label = organization_type_label -> Label,
    "report_code_label" py_report_code_label = report_code_label -> Label,
);
cover_class!(
    Form9,
    "is_amendment" py_is_amendment = is_amendment -> bool,
    "filer_name" py_filer_name = filer_name -> String,
    "filer_code_label" py_filer_code_label = filer_code_label -> Label,
    "used_segregated_bank_account" py_used_segregated_bank_account = used_segregated_bank_account -> Option<bool>,
);
cover_class!(Form9Custodian);
cover_class!(
    Form13,
    "is_amendment" py_is_amendment = is_amendment -> bool,
    "report_code_label" py_report_code_label = report_code_label -> Label,
);
cover_class!(
    Form24,
    "is_amendment" py_is_amendment = is_amendment -> bool,
    "report_type_label" py_report_type_label = report_type_label -> Label,
);
cover_class!(
    Form99,
    "is_amendment" py_is_amendment = is_amendment -> bool,
    "text_code_label" py_text_code_label = text_code_label -> Label,
);

/// A [`Cover`] as an instance of its form's class (`Form3X`, `Form1`, …).
pub fn cover_to_py<'py>(py: Python<'py>, cover: &Cover) -> PyResult<Bound<'py, PyAny>> {
    Ok(match cover {
        Cover::Form1(f) => Bound::new(py, f.clone())?.into_any(),
        Cover::Form3(f) => Bound::new(py, f.clone())?.into_any(),
        Cover::Form3P(f) => Bound::new(py, f.clone())?.into_any(),
        Cover::Form1M(f) => Bound::new(py, f.clone())?.into_any(),
        Cover::Form3X(f) => Bound::new(py, (**f).clone())?.into_any(),
        Cover::Form3L(f) => Bound::new(py, f.clone())?.into_any(),
        Cover::Form4(f) => Bound::new(py, (**f).clone())?.into_any(),
        Cover::Form7(f) => Bound::new(py, f.clone())?.into_any(),
        Cover::Form13(f) => Bound::new(py, f.clone())?.into_any(),
        Cover::Form24(f) => Bound::new(py, f.clone())?.into_any(),
        Cover::Form5(f) => Bound::new(py, (**f).clone())?.into_any(),
        Cover::Form6(f) => Bound::new(py, f.clone())?.into_any(),
        Cover::Form9(f) => Bound::new(py, (**f).clone())?.into_any(),
        Cover::Form2(f) => Bound::new(py, (**f).clone())?.into_any(),
        Cover::Form99(f) => Bound::new(py, f.clone())?.into_any(),
    })
}

/// Add every cover class to `module` (`libfec.covers`).
pub fn add_classes(module: &Bound<'_, PyModule>) -> PyResult<()> {
    macro_rules! add {
        ($($ty:ty),+ $(,)?) => { $(module.add_class::<$ty>()?;)+ };
    }
    add!(
        PersonName,
        Address,
        DetailedSummaryRow,
        Form1,
        Form1PacFlags,
        Form1Candidate,
        Form1Affiliated,
        Form1Contact,
        Form1Bank,
        Form1M,
        Form1MAffiliation,
        Form1MQualification,
        Form1MCandidate,
        Form2,
        Form2Committee,
        Form2PersonalFundsDeclaration,
        Form3,
        Form3Summary,
        Form3DetailedSummary,
        Form3DetailedSummaryReceipts,
        Form3DetailedSummaryDisbursements,
        Form3CashSummary,
        Form3L,
        Form3P,
        Form3PSummary,
        Form3PDetailedSummary,
        Form3PDetailedSummaryReceipts,
        Form3PDetailedSummaryDisbursements,
        Form3PStateAllocations,
        Form3PStateAllocation,
        Form3X,
        Form3XSummary,
        Form3XDetailedSummary,
        Form3XReceipts,
        Form3XDisbursements,
        Form3XNetContributionsAndOperatingExpenditures,
        Form4,
        Form4Summary,
        Form4DetailedSummary,
        Form4Receipts,
        Form4Disbursements,
        Form4ItemizedLine,
        Form4LoanLine,
        Form5,
        Form6,
        Form6Candidate,
        Form7,
        Form9,
        Form9Custodian,
        Form13,
        Form24,
        Form99,
    );
    Ok(())
}
