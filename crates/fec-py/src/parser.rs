//! `open(src)` → [`FilingReader`]: a streaming, single-pass reader over a filing.
//!
//! The HDR and cover records are parsed eagerly so `.header`/`.cover` are
//! available before iteration; every itemization row after them is pulled lazily,
//! [`BATCH`] rows at a time, with the GIL released for the pull.

use std::collections::VecDeque;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::errors::{missing_mapping, parse_error};
use crate::row::{schema_for, Row};
use crate::source::{lock, lock_attached, raised_or, resolve, ErrorSlot, SourceReader};

/// The filing's `HDR` record.
#[pyclass(module = "libfec_parser.parser", frozen)]
pub struct Header {
    #[pyo3(get)]
    pub record_type: String,
    #[pyo3(get)]
    pub ef_type: String,
    #[pyo3(get)]
    pub fec_version: String,
    #[pyo3(get)]
    pub software_name: String,
    #[pyo3(get)]
    pub software_version: String,
    #[pyo3(get)]
    pub report_id: Option<String>,
    #[pyo3(get)]
    pub report_number: Option<String>,
    #[pyo3(get)]
    pub comment: Option<String>,
    /// How the header is written: `"hdr"`, `"legacy_block"` (electronic
    /// 1.x/2.x `/* Header` block) or `"paper"`.
    #[pyo3(get)]
    pub style: String,
    /// Body field delimiter: `"fs"` (0x1C) or `"comma"`.
    #[pyo3(get)]
    pub delimiter: String,
    /// Sub-delimiter of combined name fields (3.x–5.x and 1.x/2.x only).
    #[pyo3(get)]
    pub name_delimiter: Option<String>,
    /// Paper filings: FEC data-entry batch number.
    #[pyo3(get)]
    pub batch_number: Option<String>,
    /// Paper filings P2.6+: date the FEC received the filing.
    #[pyo3(get)]
    pub received_date: Option<String>,
    /// Whether this is FEC data entry of a paper filing.
    #[pyo3(get)]
    pub is_paper: bool,
    legacy_fields: Vec<(String, String)>,
    schedule_counts: Vec<(String, String)>,
}

impl From<&fec_parser::FilingHeader> for Header {
    fn from(h: &fec_parser::FilingHeader) -> Self {
        Header {
            record_type: h.record_type.clone(),
            ef_type: h.ef_type.clone(),
            fec_version: h.fec_version.clone(),
            software_name: h.software_name.clone(),
            software_version: h.software_version.clone(),
            report_id: h.report_id.clone(),
            report_number: h.report_number.clone(),
            comment: h.comment.clone(),
            style: h.style.as_str().to_owned(),
            delimiter: h.delimiter.as_str().to_owned(),
            name_delimiter: h.name_delimiter.clone(),
            batch_number: h.batch_number.clone(),
            received_date: h.received_date.clone(),
            is_paper: h.is_paper(),
            legacy_fields: pairs(&h.legacy_fields),
            schedule_counts: pairs(&h.schedule_counts),
        }
    }
}

fn pairs<'a>(m: impl IntoIterator<Item = (&'a String, &'a String)>) -> Vec<(String, String)> {
    m.into_iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

fn pairs_to_dict<'py>(py: Python<'py>, pairs: &[(String, String)]) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    for (k, v) in pairs {
        dict.set_item(k, v)?;
    }
    Ok(dict)
}

#[pymethods]
impl Header {
    fn __repr__(&self) -> String {
        format!(
            "Header(fec_version='{}', software_name='{}', software_version='{}')",
            self.fec_version, self.software_name, self.software_version
        )
    }

    /// `/* Header` block (electronic 1.x/2.x): every `key = value` line before
    /// `Schedule_Counts:`, keys as written, in file order. Empty otherwise.
    #[getter]
    fn legacy_fields<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        pairs_to_dict(py, &self.legacy_fields)
    }

    /// `/* Header` block: the `Schedule_Counts:` lines (row type -> count as
    /// written). Empty otherwise.
    #[getter]
    fn schedule_counts<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        pairs_to_dict(py, &self.schedule_counts)
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

    /// The six normalized attributes as a dict; for every column on the cover
    /// page use `cover_row`.
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
///
/// # Locking
///
/// Pulling a row can run arbitrary Python code (a file object's `read()`), so a
/// thread can hold `inner` while waiting to attach to the interpreter.  Two rules
/// keep that from deadlocking, and both are load-bearing:
///
/// 1. **Never block on one of these mutexes while attached.**  Every lock taken
///    with the GIL held goes through [`lock_attached`]; only code running inside
///    `py.detach` (i.e. [`FilingReader::refill`]) uses the plain [`lock`].
/// 2. **Lock order is `inner` → `prefixes` → `pending`**, and `inner` is the only
///    one ever held across another.  `puller` is a leaf, held for a single
///    assignment or comparison and never across a call into Python.
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
    /// The typed cover (`libfec_parser.covers.Form3X`, …), `None` for a form
    /// `fec-parser` has no struct for.
    cover_data: Option<Py<PyAny>>,
    id: Option<String>,
    /// `header.fec_version`, needed for every `schema_for` lookup.
    version: String,
    /// `header.name_delimiter`, handed to every `Row` for its `itemization`.
    name_delimiter: Option<Arc<str>>,
    source_length: usize,
    /// An exception raised by a Python `read()` mid-pull, to re-raise as itself.
    raised: ErrorSlot,
    /// The thread currently inside a pull, if any.
    ///
    /// `inner` is a plain, non-reentrant `Mutex`, so a pathological source whose
    /// `read()` calls `next()` or `close()` on the very reader reading it would
    /// deadlock against itself.  Recording the puller turns that into a clean
    /// `RuntimeError`.
    puller: Mutex<Option<ThreadId>>,
}

/// Marks `puller` for the duration of a pull, and clears it however the pull ends.
struct PullMark<'a>(&'a Mutex<Option<ThreadId>>);

impl<'a> PullMark<'a> {
    fn set(slot: &'a Mutex<Option<ThreadId>>) -> Self {
        *lock(slot) = Some(std::thread::current().id());
        Self(slot)
    }
}

impl Drop for PullMark<'_> {
    fn drop(&mut self) {
        *lock(self.0) = None;
    }
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

impl FilingReader {
    /// Whether this thread is already inside a pull on this reader.
    fn pulling_here(&self) -> bool {
        *lock(&self.puller) == Some(std::thread::current().id())
    }

    /// Reject a re-entrant call before it blocks on a lock this thread holds.
    fn check_not_reentrant(&self) -> PyResult<()> {
        if self.pulling_here() {
            return Err(PyRuntimeError::new_err(
                "reader is already being read on this thread: a source's read() \
                 must not call back into the FilingReader reading it",
            ));
        }
        Ok(())
    }

    /// Pull up to [`BATCH`] filter-passing rows into `pending`, **without the GIL**.
    ///
    /// Returns `true` when the source is exhausted.  Rejected rows never leave Rust,
    /// which is the entire point of `rows(*prefixes)`.
    fn refill(&self) -> PyResult<bool> {
        self.check_not_reentrant()?;
        let mut guard = lock(&self.inner);
        let Some(src) = guard.as_mut() else {
            return Err(closed_error());
        };
        let _mark = PullMark::set(&self.puller);
        let prefixes = lock(&self.prefixes).clone();

        // Pull into a local queue rather than into `pending` directly: a batch can
        // mean hundreds of Python `read()` calls, and `pending` is what every other
        // thread's `next()` pops from.  Holding it throughout would make them all
        // wait for the whole batch (detached, so not a deadlock — just a stall).
        let mut batch = VecDeque::with_capacity(BATCH);
        let mut exhausted = false;
        while batch.len() < BATCH {
            match src.next_row() {
                None => {
                    exhausted = true;
                    break;
                }
                Some(Ok(row)) if !matches_prefix(prefixes.as_ref(), &row.row_type) => continue,
                Some(item) => batch.push_back(item),
            }
        }
        // `inner` stays held across this append — it is what serializes two
        // concurrent refills, so their batches cannot interleave out of file order.
        lock(&self.pending).extend(batch);
        Ok(exhausted)
    }

    /// Drop the source and any rows already pulled.  Idempotent.
    ///
    /// The caller must not be inside a pull on this thread; the public entry
    /// points check that first.
    fn shut(&self, py: Python<'_>) {
        // `take()`, then drop *after* the guard is gone: dropping the source
        // releases a `PyBuffer` or a `Py<PyAny>`, and the latter's final decref can
        // run a `__del__`, i.e. arbitrary Python, which must not happen while we
        // hold `inner`.  (`PyBuffer`'s own `Drop` is safe either way: it re-attaches
        // via `Python::try_attach`, which is a no-op on an already-attached thread —
        // `pyo3-0.29.2/src/internal/state.rs:86-91`.)
        let source = lock_attached(py, &self.inner).take();
        lock_attached(py, &self.pending).clear();
        drop(source);
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

    /// The typed cover record — `Form3X`, `Form1`, … from `libfec_parser.covers` —
    /// or `None` when `fec-parser` has no typed struct for the form.  The same
    /// object every time.
    #[getter]
    fn cover_data(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.cover_data.as_ref().map(|c| c.clone_ref(py))
    }

    /// The filing id: the file stem of a path, or of a file object's `name`
    /// (`FEC-` stripped); `None` when the source does not name itself.
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
    fn closed(&self, py: Python<'_>) -> bool {
        // Inside a pull on this thread we are the ones holding `inner`, and the
        // source is open by definition; answer without touching the lock.
        !self.pulling_here() && lock_attached(py, &self.inner).is_none()
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
        *lock_attached(slf.py(), &slf.get().prefixes) = filter;
        slf
    }

    /// Drop the source.  Idempotent; iterating afterwards raises `ValueError`.
    fn close(&self, py: Python<'_>) -> PyResult<()> {
        self.check_not_reentrant()?;
        self.shut(py);
        Ok(())
    }

    fn __enter__(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        slf
    }

    #[pyo3(signature = (exc_type, exc_value, traceback, /))]
    fn __exit__(
        &self,
        py: Python<'_>,
        exc_type: Option<&Bound<'_, PyAny>>,
        exc_value: Option<&Bound<'_, PyAny>>,
        traceback: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let (_, _, _) = (exc_type, exc_value, traceback);
        self.check_not_reentrant()?;
        self.shut(py);
        Ok(())
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
            let queued = lock_attached(py, &this.pending).pop_front();
            if let Some(item) = queued {
                return match item {
                    Ok(row) => match schema_for(py, &row.row_type, &this.version) {
                        Some(schema) => Ok(Some(Py::new(
                            py,
                            Row::new(schema, row.record, row.line, this.name_delimiter.clone()),
                        )?)),
                        None => Err(missing_mapping(py, &row.row_type, &this.version, row.line)),
                    },
                    Err(e) => {
                        // Not inside a pull here: `refill` has already returned.
                        this.shut(py);
                        // A `read()` that raised comes back here wrapped in a CSV
                        // error; hand back the exception the source actually raised.
                        Err(raised_or(&this.raised, parse_error(e)))
                    }
                };
            }
            // `&FilingReader` is `Send` (every field is behind a `Mutex`/`Py<T>`), which
            // is what `Ungil` asks for; no `PyRef`/`Bound` crosses into the closure.
            let exhausted = py.detach(|| this.refill())?;
            if exhausted && lock_attached(py, &this.pending).is_empty() {
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

/// Open a filing for streaming.
///
/// Parses the `HDR` and cover records eagerly; everything after them is pulled on
/// demand by iteration.
#[pyfunction]
#[pyo3(name = "open", signature = (source, /))]
pub fn open_filing(py: Python<'_>, source: &Bound<'_, PyAny>) -> PyResult<FilingReader> {
    let SourceReader {
        reader,
        length: source_length,
        id,
        raised,
    } = resolve(source)?;

    let filing_id = id.clone().unwrap_or_default();
    let filing = py
        .detach(move || fec_parser::Filing::from_reader(reader, filing_id, source_length))
        .map_err(|e| raised_or(&raised, parse_error(e)))?;

    let header = Header::from(&filing.header);
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
    let name_delimiter: Option<Arc<str>> = filing.header.name_delimiter.as_deref().map(Arc::from);
    let cover_row = Row::new(
        cover_schema,
        filing.cover.record.clone(),
        2,
        name_delimiter.clone(),
    );
    let cover_data = filing
        .cover
        .cover_data
        .as_ref()
        .map(|c| fec_parser::covers::python::cover_to_py(py, c).map(Bound::unbind))
        .transpose()?;

    Ok(FilingReader {
        inner: Mutex::new(Some(filing)),
        pending: Mutex::new(VecDeque::new()),
        prefixes: Mutex::new(None),
        header: Py::new(py, header)?,
        cover: Py::new(py, cover)?,
        cover_row: Py::new(py, cover_row)?,
        cover_data,
        id,
        version,
        name_delimiter,
        source_length,
        raised,
        puller: Mutex::new(None),
    })
}

/// The `fec_version` of a filing, from any source `open()` accepts.
///
/// Reads only the header — the `HDR` record — so this works even on a
/// filing whose cover has no column mapping, and never touches the rest of
/// the file.
#[pyfunction]
#[pyo3(signature = (source, /))]
pub fn fec_header(py: Python<'_>, source: &Bound<'_, PyAny>) -> PyResult<String> {
    let SourceReader { reader, raised, .. } = resolve(source)?;
    let fec_version = py
        .detach(move || -> Result<String, String> {
            // `read_header` reads only the header (the `HDR` record, or the
            // `/* Header` block of a 1.x/2.x filing), never the cover.
            fec_parser::read_header(reader)
                .map(|(header, _)| header.fec_version)
                .map_err(|e| format!("{e:#}"))
        })
        .map_err(|e| raised_or(&raised, parse_error(e)))?;
    Ok(fec_version)
}
