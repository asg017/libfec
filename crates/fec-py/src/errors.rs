use pyo3::create_exception;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyType;

create_exception!(
    libfec_parser.parser,
    FecError,
    PyValueError,
    "Base class for libfec_parser errors."
);
create_exception!(
    libfec_parser.parser,
    FecParseError,
    FecError,
    "The input is not a parseable .fec filing."
);

/// Wrap any displayable parser error (`anyhow::Error`, `csv::Error`, …) as a `FecParseError`.
pub fn parse_error(e: impl std::fmt::Display) -> PyErr {
    FecParseError::new_err(e.to_string())
}

// `MissingMappingError` is defined in Python (`python/libfec_parser/parser.py`), not via
// `create_exception!`, so it can carry real `.row_type`/`.version`/`.line` attributes (see
// todos/python/12's "Recommended alternative"). The class object is cached after the first
// lookup; importing `libfec_parser.parser` here is safe because `_native` is only ever loaded
// from `libfec_parser/__init__.py`, so the package is already importable by the time a row is
// read.
static MISSING_MAPPING_ERROR: PyOnceLock<Py<PyType>> = PyOnceLock::new();

fn missing_mapping_class(py: Python<'_>) -> PyResult<&Py<PyType>> {
    MISSING_MAPPING_ERROR.get_or_try_init(py, || -> PyResult<Py<PyType>> {
        Ok(py
            .import("libfec_parser.parser")?
            .getattr("MissingMappingError")?
            .cast_into::<PyType>()?
            .unbind())
    })
}

/// Raise `libfec_parser.parser.MissingMappingError(row_type, version, line)`.
pub fn missing_mapping(py: Python<'_>, row_type: &str, version: &str, line: u64) -> PyErr {
    let class = match missing_mapping_class(py) {
        Ok(class) => class,
        Err(e) => return e,
    };
    match class.bind(py).call1((row_type, version, line)) {
        Ok(instance) => PyErr::from_value(instance),
        Err(e) => e,
    }
}

/// Wrap an `io::Error` from opening/stat-ing a filing path so `FileNotFoundError` (and friends)
/// carry `errno`/`filename`, matching stdlib `open()`.
pub fn io_error(e: std::io::Error, path: &std::path::Path) -> PyErr {
    let errno = e.raw_os_error().unwrap_or(0);
    let strerror = e.to_string();
    let filename = path.display().to_string();
    match e.kind() {
        std::io::ErrorKind::NotFound => {
            pyo3::exceptions::PyFileNotFoundError::new_err((errno, strerror, filename))
        }
        std::io::ErrorKind::PermissionDenied => {
            pyo3::exceptions::PyPermissionError::new_err((errno, strerror, filename))
        }
        _ => pyo3::exceptions::PyOSError::new_err((errno, strerror, filename)),
    }
}
