//! Python bindings for the typed itemizations (`--features python`).
//!
//! Same scheme as [`crate::covers::python`]: every itemization struct is a
//! `#[pyclass]` through a `cfg_attr` on its definition, and `cover_class!`
//! adds `__repr__`, `__eq__`, `to_dict()` and the struct's own helpers.
//! `crates/fec-py` registers the classes in `libfec_parser.itemizations` and
//! turns an [`Itemization`] into the matching class with [`itemization_to_py`].
//! The shared [`crate::covers::PersonName`] and [`crate::covers::Address`]
//! stay classes of `libfec_parser.covers`.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::*;
use crate::covers::python::{cover_class, Label};

cover_class!(
    Entity,
    "is_individual" py_is_individual = is_individual -> bool,
    "display_name" py_display_name = display_name -> String,
    "entity_type_label" py_entity_type_label = entity_type_label -> Label,
);
cover_class!(
    CandidateRef,
    "is_empty" py_is_empty = is_empty -> bool,
    "office_label" py_office_label = office_label -> Label,
);

cover_class!(
    ScheduleA,
    "election_code_label" py_election_code_label = election_code_label -> Label,
    "line_number" py_line_number = line_number -> Option<&str>,
);
cover_class!(
    ScheduleB,
    "election_code_label" py_election_code_label = election_code_label -> Label,
    "category_code_label" py_category_code_label = category_code_label -> Label,
    "line_number" py_line_number = line_number -> Option<&str>,
);

cover_class!(
    ScheduleD,
    "line_number" py_line_number = line_number -> Option<&str>,
);
cover_class!(
    ScheduleFCommittee,
    "is_empty" py_is_empty = is_empty -> bool,
);
cover_class!(ScheduleF);

cover_class!(
    ScheduleH1,
    "party_federal_percent" py_party_federal_percent = party_federal_percent -> Option<f64>,
);
cover_class!(ScheduleH2);
cover_class!(
    ScheduleH3,
    "event_type_label" py_event_type_label = event_type_label -> Label,
);
cover_class!(
    ScheduleH4,
    "activity_label" py_activity_label = activity_label -> Label,
);
cover_class!(ScheduleH5);
cover_class!(
    ScheduleH6,
    "activity_label" py_activity_label = activity_label -> Label,
);

/// An [`Itemization`] as an instance of its schedule's class.
pub fn itemization_to_py(py: Python<'_>, item: Itemization) -> PyResult<Bound<'_, PyAny>> {
    Ok(match item {
        Itemization::ScheduleA(s) => Bound::new(py, *s)?.into_any(),
        Itemization::ScheduleB(s) => Bound::new(py, *s)?.into_any(),
        Itemization::ScheduleD(s) => Bound::new(py, *s)?.into_any(),
        Itemization::ScheduleF(s) => Bound::new(py, *s)?.into_any(),
        Itemization::ScheduleH1(s) => Bound::new(py, *s)?.into_any(),
        Itemization::ScheduleH2(s) => Bound::new(py, *s)?.into_any(),
        Itemization::ScheduleH3(s) => Bound::new(py, *s)?.into_any(),
        Itemization::ScheduleH4(s) => Bound::new(py, *s)?.into_any(),
        Itemization::ScheduleH5(s) => Bound::new(py, *s)?.into_any(),
        Itemization::ScheduleH6(s) => Bound::new(py, *s)?.into_any(),
    })
}

/// Add every itemization class to `module` (`libfec_parser.itemizations`).
pub fn add_classes(module: &Bound<'_, PyModule>) -> PyResult<()> {
    macro_rules! add {
        ($($ty:ty),+ $(,)?) => { $(module.add_class::<$ty>()?;)+ };
    }
    add!(
        Entity,
        CandidateRef,
        ScheduleA,
        ScheduleB,
        ScheduleD,
        ScheduleFCommittee,
        ScheduleF,
        ScheduleH1,
        ScheduleH2,
        ScheduleH3,
        ScheduleH4,
        ScheduleH5,
        ScheduleH6,
    );
    Ok(())
}
