//! `open(src)` → [`FilingReader`]: a streaming, single-pass reader over a filing.
//!
//! The HDR and cover records are parsed eagerly so `.header`/`.cover` are
//! available before iteration; every itemization row after them is pulled lazily,
//! [`BATCH`] rows at a time, with the GIL released for the pull.

use std::collections::VecDeque;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedBytes;
use pyo3::types::{PyBytes, PyDict};

use crate::errors::{io_error, missing_mapping, parse_error};
use crate::row::{schema_for, Row};

/// The filing's `HDR` record.
#[pyclass(module = "libfec_parser.parser", frozen, get_all)]
pub struct Header {
    pub record_type: String,
    pub ef_type: String,
    pub fec_version: String,
    pub software_name: String,
    pub software_version: String,
    pub report_id: Option<String>,
    pub report_number: Option<String>,
    pub comment: Option<String>,
}

#[pymethods]
impl Header {
    fn __repr__(&self) -> String {
        format!(
            "Header(fec_version='{}', software_name='{}', software_version='{}')",
            self.fec_version, self.software_name, self.software_version
        )
    }
}

/// The six normalized cover attributes, shared by every form type (Q19).
///
/// The full cover line is `FilingReader.cover_row`.
#[pyclass(module = "libfec_parser.parser", frozen, get_all)]
pub struct Cover {
    pub form_type: String,
    pub filer_id: String,
    pub filer_name: String,
    pub report_code: Option<String>,
    pub coverage_from_date: Option<jiff::civil::Date>,
    pub coverage_through_date: Option<jiff::civil::Date>,
}

#[pymethods]
impl Cover {
    fn __repr__(&self) -> String {
        format!(
            "Cover(form_type='{}', filer_id='{}', filer_name='{}')",
            self.form_type, self.filer_id, self.filer_name
        )
    }

    /// The six cover attributes above as a dict (same keys as before, typed dates).
    fn fields<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("form_type", &self.form_type)?;
        dict.set_item("filer_id", &self.filer_id)?;
        dict.set_item("filer_name", &self.filer_name)?;
        dict.set_item("report_code", &self.report_code)?;
        dict.set_item("coverage_from_date", self.coverage_from_date)?;
        dict.set_item("coverage_through_date", self.coverage_through_date)?;
        Ok(dict)
    }
}

/// The parser's filing, over a type-erased reader.
///
/// `Box<dyn Read + Send>` (not just `Read`) so the whole `Filing` is `Send` and can
/// be pulled inside `Python::detach`.
type Source = fec_parser::Filing<Box<dyn Read + Send>>;

type PendingRow = Result<fec_parser::FilingRow, fec_parser::FilingRowReadError>;

/// How many rows one `detach` pull queues up before handing the GIL back.
const BATCH: usize = 256;

/// A streaming reader over one filing.
///
/// `frozen` — the ticket's sketch says "not frozen", but `Bound::get()` (which the
/// same sketch uses) is `T: PyClass<Frozen = True> + Sync`, and every field here is
/// already behind a `Mutex` or a `Py<T>`, so `frozen` is both legal and cheaper than
/// a runtime borrow flag.
#[pyclass(module = "libfec_parser.parser", frozen)]
pub struct FilingReader {
    /// `None` once closed.
    inner: Mutex<Option<Source>>,
    /// Rows pulled but not yet handed to Python, still as raw `FilingRow`s.
    pending: Mutex<VecDeque<PendingRow>>,
    /// The `rows(*prefixes)` filter; `None` means "every row".
    prefixes: Mutex<Option<Vec<String>>>,
    header: Py<Header>,
    cover: Py<Cover>,
    cover_row: Py<Row>,
    id: Option<String>,
    /// `header.fec_version`, needed for every `schema_for` lookup.
    version: String,
    source_length: usize,
}

/// `row_type` matches if it starts with any of `prefixes`, case-insensitively.
///
/// Compared over bytes: slicing a `str` at `p.len()` would panic on a multi-byte
/// boundary, and FEC row types are ASCII anyway.
fn matches_prefix(prefixes: Option<&Vec<String>>, row_type: &str) -> bool {
    match prefixes {
        None => true,
        Some(prefixes) => {
            let row_type = row_type.as_bytes();
            prefixes.iter().any(|p| {
                let p = p.as_bytes();
                row_type.len() >= p.len() && row_type[..p.len()].eq_ignore_ascii_case(p)
            })
        }
    }
}

fn closed_error() -> PyErr {
    PyValueError::new_err("I/O operation on closed filing")
}

/// A `Mutex` here is only ever held by this module's own short critical sections,
/// so a poisoned lock means a panic mid-pull; take the data anyway rather than
/// turning every later call into a panic.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl FilingReader {
    /// Pull up to [`BATCH`] filter-passing rows into `pending`, **without the GIL**.
    ///
    /// Returns `true` when the source is exhausted.  Rejected rows never leave Rust,
    /// which is the entire point of `rows(*prefixes)`.
    fn refill(&self) -> PyResult<bool> {
        let mut guard = lock(&self.inner);
        let Some(src) = guard.as_mut() else {
            return Err(closed_error());
        };
        let prefixes = lock(&self.prefixes).clone();
        let mut pending = lock(&self.pending);
        while pending.len() < BATCH {
            match src.next_row() {
                None => return Ok(true),
                Some(Ok(row)) if !matches_prefix(prefixes.as_ref(), &row.row_type) => continue,
                Some(item) => pending.push_back(item),
            }
        }
        Ok(false)
    }

    /// Drop the source and any rows already pulled.  Idempotent.
    fn shut(&self) {
        *lock(&self.inner) = None;
        lock(&self.pending).clear();
    }
}

#[pymethods]
impl FilingReader {
    /// The filing's `HDR` record — the same object every time.
    #[getter]
    fn header(&self, py: Python<'_>) -> Py<Header> {
        self.header.clone_ref(py)
    }

    /// The six normalized cover attributes — the same object every time.
    #[getter]
    fn cover(&self, py: Python<'_>) -> Py<Cover> {
        self.cover.clone_ref(py)
    }

    /// The full cover line as a `Row` — the same object every time.
    #[getter]
    fn cover_row(&self, py: Python<'_>) -> Py<Row> {
        self.cover_row.clone_ref(py)
    }

    /// The filing id: a path's file stem (`FEC-` stripped), `None` for bytes.
    #[getter]
    fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Shortcut for `reader.header.fec_version`.
    #[getter]
    fn fec_version(&self) -> &str {
        &self.version
    }

    /// The source's size in bytes, or `0` if it is not known.
    #[getter]
    fn source_length(&self) -> usize {
        self.source_length
    }

    /// Whether `close()` has been called.
    #[getter]
    fn closed(&self) -> bool {
        lock(&self.inner).is_none()
    }

    /// Only yield rows whose type starts with one of `prefixes` (case-insensitive).
    ///
    /// Returns the reader itself, so `for row in reader.rows("SA", "SB")` reads well.
    /// The filter applies to the single remaining pass: a second call replaces it, and
    /// `rows()` with no arguments clears it.
    #[pyo3(signature = (*prefixes))]
    fn rows<'py>(slf: Bound<'py, Self>, prefixes: Vec<String>) -> Bound<'py, Self> {
        let filter = if prefixes.is_empty() {
            None
        } else {
            Some(prefixes)
        };
        *lock(&slf.get().prefixes) = filter;
        slf
    }

    /// Drop the source.  Idempotent; iterating afterwards raises `ValueError`.
    fn close(&self) {
        self.shut();
    }

    fn __enter__(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        slf
    }

    #[pyo3(signature = (exc_type, exc_value, traceback, /))]
    fn __exit__(
        &self,
        exc_type: Option<&Bound<'_, PyAny>>,
        exc_value: Option<&Bound<'_, PyAny>>,
        traceback: Option<&Bound<'_, PyAny>>,
    ) {
        let (_, _, _) = (exc_type, exc_value, traceback);
        self.shut();
    }

    fn __iter__(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        slf
    }

    /// The next row, or `StopIteration` at end of file.
    ///
    /// An unmapped row type raises `MissingMappingError` but leaves the reader usable —
    /// the rows queued behind it are still in `pending`.  A CSV error is terminal: the
    /// underlying reader is poisoned, so the reader is closed before raising.
    fn __next__(slf: &Bound<'_, Self>) -> PyResult<Option<Py<Row>>> {
        let py = slf.py();
        let this = slf.get();
        loop {
            let queued = lock(&this.pending).pop_front();
            if let Some(item) = queued {
                return match item {
                    Ok(row) => match schema_for(py, &row.row_type, &this.version) {
                        Some(schema) => {
                            Ok(Some(Py::new(py, Row::new(schema, row.record, row.line))?))
                        }
                        None => Err(missing_mapping(py, &row.row_type, &this.version, row.line)),
                    },
                    Err(e) => {
                        this.shut();
                        Err(parse_error(e))
                    }
                };
            }
            // `&FilingReader` is `Send` (every field is behind a `Mutex`/`Py<T>`), which
            // is what `Ungil` asks for; no `PyRef`/`Bound` crosses into the closure.
            let exhausted = py.detach(|| this.refill())?;
            if exhausted && lock(&this.pending).is_empty() {
                return Ok(None);
            }
        }
    }

    fn __repr__(&self) -> String {
        let cover = self.cover.get();
        let id = match &self.id {
            Some(id) => format!("'{id}'"),
            None => "None".to_owned(),
        };
        format!(
            "FilingReader(id={}, form_type='{}', filer_id='{}')",
            id, cover.form_type, cover.filer_id
        )
    }
}

/// Turn a Python source into `(reader, source_length, id)`.
///
/// Ticket 14 accepts a path (`str`/`os.PathLike`) and `bytes`; ticket 15 adds branches
/// here for the rest of the buffer protocol and for binary file objects.
fn source_to_reader(
    source: &Bound<'_, PyAny>,
) -> PyResult<(Box<dyn Read + Send>, usize, Option<String>)> {
    // `bytes` first: `PathBuf` extraction goes through `os.fspath`, which accepts
    // `bytes` as a path, so checking the path branch first would swallow `bytes`.
    if let Ok(bytes) = source.cast::<PyBytes>() {
        let backed = PyBackedBytes::from(bytes.clone());
        let len = backed.len();
        return Ok((Box::new(Cursor::new(backed)), len, None));
    }
    if let Ok(path) = source.extract::<PathBuf>() {
        let file = std::fs::File::open(&path).map_err(|e| io_error(e, &path))?;
        let len = file.metadata().map_err(|e| io_error(e, &path))?.len() as usize;
        // Mirrors `fec_parser::Filing::from_path` + `from_reader`'s `FEC-` stripping.
        let id = path.file_stem().map(|stem| {
            let stem = stem.to_string_lossy();
            stem.strip_prefix("FEC-").unwrap_or(&stem).to_owned()
        });
        return Ok((Box::new(file), len, id));
    }
    Err(PyTypeError::new_err(format!(
        "source must be a path (str or os.PathLike) or bytes, not {}",
        source.get_type().name()?
    )))
}

/// Open a filing for streaming.
///
/// Parses the `HDR` and cover records eagerly; everything after them is pulled on
/// demand by iteration.
#[pyfunction]
#[pyo3(name = "open", signature = (source, /))]
pub fn open_filing(py: Python<'_>, source: &Bound<'_, PyAny>) -> PyResult<FilingReader> {
    let (reader, source_length, id) = source_to_reader(source)?;

    let filing_id = id.clone().unwrap_or_default();
    let filing = py
        .detach(move || fec_parser::Filing::from_reader(reader, filing_id, source_length))
        .map_err(parse_error)?;

    let header = Header {
        record_type: filing.header.record_type.clone(),
        ef_type: filing.header.ef_type.clone(),
        fec_version: filing.header.fec_version.clone(),
        software_name: filing.header.software_name.clone(),
        software_version: filing.header.software_version.clone(),
        report_id: filing.header.report_id.clone(),
        report_number: filing.header.report_number.clone(),
        comment: filing.header.comment.clone(),
    };
    let version = header.fec_version.clone();

    let cover = Cover {
        form_type: filing.cover.form_type.clone(),
        filer_id: filing.cover.filer_id.clone(),
        filer_name: filing.cover.filer_name.clone(),
        report_code: filing.cover.report_code.clone(),
        coverage_from_date: filing.cover.coverage_from_date,
        coverage_through_date: filing.cover.coverage_through_date,
    };

    // `from_reader` already failed if the cover's form type had no mapping
    // (`FilingCover::from_record` looks up its columns), so this cannot miss.
    let cover_schema = schema_for(py, &cover.form_type, &version).ok_or_else(|| {
        parse_error(format!(
            "no column mapping for cover form type '{}' in FEC version '{version}'",
            cover.form_type
        ))
    })?;
    // The cover record is always the filing's second line.
    let cover_row = Row::new(cover_schema, filing.cover.record.clone(), 2);

    Ok(FilingReader {
        inner: Mutex::new(Some(filing)),
        pending: Mutex::new(VecDeque::new()),
        prefixes: Mutex::new(None),
        header: Py::new(py, header)?,
        cover: Py::new(py, cover)?,
        cover_row: Py::new(py, cover_row)?,
        id,
        version,
        source_length,
    })
}

#[pyfunction]
pub fn fec_header(contents: &[u8]) -> PyResult<String> {
    let f = fec_parser::Filing::from_reader(contents, "123".to_string(), contents.len())
        .map_err(parse_error)?;
    Ok(f.header.fec_version)
}
