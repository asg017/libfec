//! `Row`: one itemization line, mapping-first.
//!
//! By name the value is typed (`float` / `datetime.date` / `str` / `None`), by
//! position it is the raw `str` exactly as it appears in the file.  Column names
//! and per-column kinds are shared by every row of a `(row_type, fec_version)`
//! pair through an interned [`Schema`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use pyo3::exceptions::{PyIndexError, PyKeyError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PySlice, PyString, PyTuple};
use pyo3::IntoPyObjectExt;

use fec_parser::mappings::{column_names_for_field, DATE_COLUMNS, FLOAT_COLUMNS};

/// How a column's raw string is turned into a Python value.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Kind {
    Str,
    Float,
    Date,
}

/// The column names and kinds of one `(row_type, fec_version)` pair.
pub struct Schema {
    /// The row type as it appears in the file (e.g. `"SA11AI"`).
    pub row_type: String,
    /// `header.fec_version`.
    pub version: String,
    /// Interned column names, in column order.
    pub names: Vec<Py<PyString>>,
    pub kinds: Vec<Kind>,
    pub index: HashMap<String, usize>,
}

type SchemaKey = (String, String);

static SCHEMAS: Mutex<Option<HashMap<SchemaKey, Arc<Schema>>>> = Mutex::new(None);

/// The shared schema for `(row_type, version)`, or `None` if the mapping is unknown.
///
/// Misses are not cached: `column_names_for_field` is cheap to retry and the
/// caller turns `None` into `MissingMappingError`.
pub fn schema_for(py: Python<'_>, row_type: &str, version: &str) -> Option<Arc<Schema>> {
    // `column_names_for_field` is case-insensitive on the row type, but the cache is
    // keyed on the spelling in the file so `Row.row_type` round-trips it.
    let key: SchemaKey = (row_type.to_owned(), version.to_owned());

    {
        let cache = SCHEMAS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(schema) = cache.as_ref().and_then(|c| c.get(&key)) {
            return Some(Arc::clone(schema));
        }
    }

    let columns = column_names_for_field(row_type, version).ok()?;
    let schema = Arc::new(Schema {
        row_type: key.0.clone(),
        version: key.1.clone(),
        names: columns
            .iter()
            .map(|name| PyString::intern(py, name).unbind())
            .collect(),
        kinds: columns
            .iter()
            .map(|name| {
                // Same precedence as the CLI's Excel export.
                if DATE_COLUMNS.contains(name) {
                    Kind::Date
                } else if FLOAT_COLUMNS.contains(name) {
                    Kind::Float
                } else {
                    Kind::Str
                }
            })
            .collect(),
        index: columns
            .iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), i))
            .collect(),
    });

    let mut cache = SCHEMAS.lock().unwrap_or_else(|e| e.into_inner());
    Some(Arc::clone(
        cache
            .get_or_insert_with(HashMap::new)
            .entry(key)
            .or_insert(schema),
    ))
}

/// One itemization row.
#[pyclass(module = "libfec_parser.parser", frozen)]
pub struct Row {
    schema: Arc<Schema>,
    /// Raw fields, including field 0 (the row type).
    record: csv::StringRecord,
    /// 1-based physical line of the row in the file.
    line: u64,
}

impl Row {
    pub fn new(schema: Arc<Schema>, record: csv::StringRecord, line: u64) -> Self {
        Row {
            schema,
            record,
            line,
        }
    }

    /// The value rule (Q6): typed if it parses, the raw `str` if it is garbage,
    /// `None` if it is empty.  Text columns keep `""`; a short row reads its
    /// missing columns as `None`.
    fn value<'py>(&self, py: Python<'py>, i: usize) -> PyResult<Bound<'py, PyAny>> {
        let Some(raw) = self.record.get(i) else {
            return Ok(py.None().into_bound(py));
        };
        let kind = self.schema.kinds[i];
        if kind == Kind::Str {
            return PyString::new(py, raw).into_bound_py_any(py);
        }
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(py.None().into_bound(py));
        }
        match kind {
            Kind::Float => match trimmed.parse::<f64>() {
                Ok(v) => v.into_bound_py_any(py),
                Err(_) => PyString::new(py, raw).into_bound_py_any(py),
            },
            Kind::Date => match jiff::civil::Date::strptime("%Y%m%d", trimmed) {
                Ok(d) => d.into_bound_py_any(py),
                Err(_) => PyString::new(py, raw).into_bound_py_any(py),
            },
            Kind::Str => unreachable!("handled above"),
        }
    }

    /// The column index for a name, or `None` if the name is not a column.
    fn column(&self, name: &str) -> Option<usize> {
        self.schema.index.get(name).copied()
    }

    /// The raw field at a (possibly negative) position, over *all* fields.
    fn field_at(&self, index: isize) -> PyResult<&str> {
        let len = self.record.len() as isize;
        let i = if index < 0 { len + index } else { index };
        if i < 0 || i >= len {
            return Err(PyIndexError::new_err("row index out of range"));
        }
        Ok(&self.record[i as usize])
    }

    /// `(row_type, version, fields)` as a Python tuple — the identity used by
    /// `__eq__` and `__hash__`.
    fn identity<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        PyTuple::new(
            py,
            [
                PyString::new(py, &self.schema.row_type).into_any(),
                PyString::new(py, &self.schema.version).into_any(),
                PyTuple::new(py, self.record.iter().collect::<Vec<_>>())?.into_any(),
            ],
        )
    }
}

#[pymethods]
impl Row {
    /// The row type, as written in the file.
    #[getter]
    fn row_type(&self) -> &str {
        self.record.get(0).unwrap_or("")
    }

    /// The 1-based physical line of this row in the file.
    #[getter]
    fn line(&self) -> u64 {
        self.line
    }

    /// Fields past the last mapped column, but only if at least one is non-empty.
    ///
    /// A single trailing empty field is a stray delimiter, not data (Q16).
    #[getter]
    fn extra_fields(&self) -> Vec<String> {
        let mapped = self.schema.names.len();
        if self.record.len() <= mapped {
            return Vec::new();
        }
        let extras: Vec<String> = self
            .record
            .iter()
            .skip(mapped)
            .map(|s| s.to_owned())
            .collect();
        if extras.iter().all(|s| s.is_empty()) {
            Vec::new()
        } else {
            extras
        }
    }

    /// The number of *columns* in the mapping (`len(row.fields())` is the raw count).
    fn __len__(&self) -> usize {
        self.schema.names.len()
    }

    /// `row[name]` → typed value, `row[i]` → raw `str`, `row[a:b]` → `list[str]`.
    fn __getitem__<'py>(&self, key: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = key.py();
        if let Ok(name) = key.cast::<PyString>() {
            let name = name.to_cow()?;
            return match self.column(&name) {
                Some(i) => self.value(py, i),
                None => Err(PyKeyError::new_err(name.into_owned())),
            };
        }
        if let Ok(slice) = key.cast::<PySlice>() {
            let indices = slice.indices(self.record.len() as isize)?;
            let mut out: Vec<&str> = Vec::new();
            let mut i = indices.start;
            let mut n = indices.slicelength;
            while n > 0 {
                out.push(&self.record[i as usize]);
                i += indices.step;
                n -= 1;
            }
            return out.into_bound_py_any(py);
        }
        match key.extract::<isize>() {
            Ok(index) => self.field_at(index)?.into_bound_py_any(py),
            Err(_) => Err(PyTypeError::new_err(format!(
                "row indices must be str, int or slice, not {}",
                key.get_type().name()?
            ))),
        }
    }

    /// Iterating a row yields its column names (mapping semantics).
    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(PyList::new(py, self.keys(py))?.try_iter()?.into_any())
    }

    /// Column-name membership; a non-`str` key is simply not a column.
    fn __contains__(&self, key: &Bound<'_, PyAny>) -> PyResult<bool> {
        match key.cast::<PyString>() {
            Ok(name) => Ok(self.column(&name.to_cow()?).is_some()),
            Err(_) => Ok(false),
        }
    }

    /// The column names, in column order.
    fn keys(&self, py: Python<'_>) -> Vec<Py<PyString>> {
        self.schema
            .names
            .iter()
            .map(|name| name.clone_ref(py))
            .collect()
    }

    /// The typed values, in column order.
    fn values<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyAny>>> {
        (0..self.schema.names.len())
            .map(|i| self.value(py, i))
            .collect()
    }

    /// `(name, value)` pairs, in column order.
    fn items<'py>(&self, py: Python<'py>) -> PyResult<Vec<(Py<PyString>, Bound<'py, PyAny>)>> {
        self.schema
            .names
            .iter()
            .enumerate()
            .map(|(i, name)| Ok((name.clone_ref(py), self.value(py, i)?)))
            .collect()
    }

    /// Like `dict.get`: the typed value, or `default` if `key` is not a column.
    #[pyo3(signature = (key, default=None, /))]
    fn get<'py>(
        &self,
        py: Python<'py>,
        key: &str,
        default: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        match self.column(key) {
            Some(i) => self.value(py, i),
            None => Ok(default.unwrap_or_else(|| py.None().into_bound(py))),
        }
    }

    /// Every raw field, in file order, including field 0 and any extras.
    fn fields(&self) -> Vec<String> {
        self.record.iter().map(|s| s.to_owned()).collect()
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let py = other.py();
        let Ok(other) = other.cast::<Row>() else {
            return Ok(py.NotImplemented());
        };
        let other = other.get();
        let equal = self.schema.row_type == other.schema.row_type
            && self.schema.version == other.schema.version
            && self.record == other.record;
        equal.into_py_any(py)
    }

    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        self.identity(py)?.hash()
    }

    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        // `libfec_parser.parser`, not `_native.parser`: the latter is an attribute of
        // the extension module, not importable, so pickle cannot resolve it.
        let module = py.import("libfec_parser.parser")?;
        let rebuild = module.getattr("_row_from_parts")?;
        let args = (
            &self.schema.row_type,
            &self.schema.version,
            self.fields(),
            self.line,
        );
        PyTuple::new(py, [rebuild, args.into_bound_py_any(py)?])
    }

    fn __repr__(&self) -> String {
        format!(
            "Row(row_type='{}', line={}, {} fields)",
            self.row_type(),
            self.line,
            self.record.len()
        )
    }
}

/// Rebuild a `Row` from its pickled parts.  Private; exists for `Row.__reduce__`.
#[pyfunction]
#[pyo3(name = "_row_from_parts", signature = (row_type, version, fields, line, /))]
pub fn row_from_parts(
    py: Python<'_>,
    row_type: &str,
    version: &str,
    fields: Vec<String>,
    line: u64,
) -> PyResult<Row> {
    let schema = schema_for(py, row_type, version).ok_or_else(|| {
        PyValueError::new_err(format!(
            "no column mapping for row type '{row_type}' in FEC version '{version}'"
        ))
    })?;
    Ok(Row::new(schema, csv::StringRecord::from(fields), line))
}
