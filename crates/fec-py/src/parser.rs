//! `Filing(src)` / `Filing.from_id(id)`: a streaming reader over a filing.
//!
//! The HDR and cover records are parsed eagerly so `.header`/`.cover` are
//! available up front.  Every record after them is streamed by a [`Pass`]: each
//! access of `.itemizations` or `.itemization_records` starts one, which pulls
//! [`BATCH`] records at a time with the GIL released.

use std::collections::VecDeque;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::thread::ThreadId;

use fec_parser::itemizations::Itemization;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::errors::{missing_mapping, parse_error};
use crate::record::{schema_for, Record, Unknown};
use crate::source::{lock, lock_attached, raised_or, resolve, ErrorSlot, SourceReader};

/// The filing's `HDR` record.
#[pyclass(module = "libfec.parser", frozen)]
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

/// The parser's filing, over a type-erased reader.
///
/// `Box<dyn Read + Send>` (not just `Read`) so the whole `Filing` is `Send` and can
/// be pulled inside `Python::detach`.
type Stream = fec_parser::Filing<Box<dyn Read + Send>>;

/// A record pulled by [`Pass::refill`], typed already if the pass is typed and
/// the record has a class.
enum Pulled {
    Raw(fec_parser::FilingRow),
    Typed(Itemization),
}

type Pending = Result<Pulled, fec_parser::FilingRowReadError>;

/// How many records one `detach` pull queues up before handing the GIL back.
const BATCH: usize = 256;

fn closed_error() -> PyErr {
    PyValueError::new_err("I/O operation on closed filing")
}

/// One independent, single-pass walk over every record after the cover.
///
/// # Locking
///
/// Pulling a record can run arbitrary Python code (a file object's `read()`), so a
/// thread can hold `inner` while waiting to attach to the interpreter.  Two rules
/// keep that from deadlocking, and both are load-bearing:
///
/// 1. **Never block on one of these mutexes while attached.**  Every lock taken
///    with the GIL held goes through [`lock_attached`]; only code running inside
///    `py.detach` (i.e. [`Pass::refill`]) uses the plain [`lock`].
/// 2. **Lock order is `inner` → `pending`**, and `inner` is the only one ever
///    held across another.  `puller` is a leaf, held for a single assignment or
///    comparison and never across a call into Python.
struct Pass {
    /// `None` once shut.
    inner: Mutex<Option<Stream>>,
    /// Records pulled but not yet handed to Python.
    pending: Mutex<VecDeque<Pending>>,
    /// An exception raised by a Python `read()` mid-pull, to re-raise as itself.
    raised: ErrorSlot,
    /// The thread currently inside a pull, if any.
    ///
    /// `inner` is a plain, non-reentrant `Mutex`, so a pathological source whose
    /// `read()` calls `next()` or `close()` on the very pass reading it would
    /// deadlock against itself.  Recording the puller turns that into a clean
    /// `RuntimeError`.
    puller: Mutex<Option<ThreadId>>,
    /// Whether records are typed into `libfec.itemizations` classes.
    typed: bool,
    /// `header.fec_version`, needed for every `schema_for` lookup.
    version: Arc<str>,
    /// `header.name_delimiter`, for legacy combined names.
    name_delimiter: Option<Arc<str>>,
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

impl Pass {
    /// Whether this thread is already inside a pull on this pass.
    fn pulling_here(&self) -> bool {
        *lock(&self.puller) == Some(std::thread::current().id())
    }

    /// Reject a re-entrant call before it blocks on a lock this thread holds.
    fn check_not_reentrant(&self) -> PyResult<()> {
        if self.pulling_here() {
            return Err(PyRuntimeError::new_err(
                "filing is already being read on this thread: a source's read() \
                 must not call back into the Filing reading it",
            ));
        }
        Ok(())
    }

    /// Pull up to [`BATCH`] records into `pending`, **without the GIL**.
    ///
    /// A typed pass types each record here too, so only the conversion to a
    /// Python object is left for the GIL.  Returns `true` when the source is
    /// exhausted.
    fn refill(&self) -> PyResult<bool> {
        self.check_not_reentrant()?;
        let mut guard = lock(&self.inner);
        let Some(src) = guard.as_mut() else {
            return Err(closed_error());
        };
        let _mark = PullMark::set(&self.puller);

        // Pull into a local queue rather than into `pending` directly: a batch can
        // mean hundreds of Python `read()` calls, and `pending` is what every other
        // thread's `next()` pops from.  Holding it throughout would make them all
        // wait for the whole batch (detached, so not a deadlock — just a stall).
        let mut batch = VecDeque::with_capacity(BATCH);
        let mut exhausted = false;
        while batch.len() < BATCH {
            let Some(item) = src.next_row() else {
                exhausted = true;
                break;
            };
            batch.push_back(item.map(|row| {
                let typed = self
                    .typed
                    .then(|| {
                        Itemization::from_record(
                            &row.record,
                            &self.version,
                            self.name_delimiter.as_deref(),
                        )
                    })
                    .flatten();
                match typed {
                    Some(item) => Pulled::Typed(item),
                    None => Pulled::Raw(row),
                }
            }));
        }
        // `inner` stays held across this append — it is what serializes two
        // concurrent refills, so their batches cannot interleave out of file order.
        lock(&self.pending).extend(batch);
        Ok(exhausted)
    }

    /// Drop the source and any records already pulled.  Idempotent.
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

    /// The next record as a Python object, or `None` at end of file.
    ///
    /// An unmapped row type raises `MissingMappingError` but leaves the pass usable —
    /// the records queued behind it are still in `pending`.  A CSV error is terminal:
    /// the underlying reader is poisoned, so the pass is shut before raising.
    fn next(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        loop {
            let queued = lock_attached(py, &self.pending).pop_front();
            if let Some(item) = queued {
                return match item {
                    Ok(Pulled::Typed(item)) => Ok(Some(
                        fec_parser::itemizations::python::itemization_to_py(py, item)?.unbind(),
                    )),
                    Ok(Pulled::Raw(row)) => {
                        let Some(schema) = schema_for(py, &row.row_type, &self.version) else {
                            return Err(missing_mapping(
                                py,
                                &row.row_type,
                                &self.version,
                                row.line,
                            ));
                        };
                        let record = Py::new(
                            py,
                            Record::new(schema, row.record, row.line, self.name_delimiter.clone()),
                        )?;
                        if self.typed {
                            Ok(Some(Py::new(py, Unknown::new(record))?.into_any()))
                        } else {
                            Ok(Some(record.into_any()))
                        }
                    }
                    Err(e) => {
                        // Not inside a pull here: `refill` has already returned.
                        self.shut(py);
                        // A `read()` that raised comes back here wrapped in a CSV
                        // error; hand back the exception the source actually raised.
                        Err(raised_or(&self.raised, parse_error(e)))
                    }
                };
            }
            let exhausted = py.detach(|| self.refill())?;
            if exhausted && lock_attached(py, &self.pending).is_empty() {
                return Ok(None);
            }
        }
    }
}

/// The iterator behind `Filing.itemizations` and
/// `Filing.itemization_records`: one pass over the filing.
#[pyclass(module = "libfec.parser", frozen)]
pub struct FilingIterator {
    pass: Arc<Pass>,
}

#[pymethods]
impl FilingIterator {
    fn __iter__(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        slf
    }

    fn __next__(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.pass.next(py)
    }

    fn __repr__(&self) -> String {
        let kind = if self.pass.typed {
            "itemizations"
        } else {
            "itemization_records"
        };
        format!("<FilingIterator over {kind}>")
    }
}

/// A streaming reader over one filing.
///
/// `frozen`: every mutable field is behind a `Mutex` or an atomic, so `Bound::get()`
/// works and no runtime borrow flag is needed.
///
/// Lock order is `first` → `passes`, then a pass's own locks; `source` is a leaf.
/// Every lock here is taken with [`lock_attached`].
#[pyclass(module = "libfec.parser", frozen)]
pub struct Filing {
    /// The stream the constructor parsed the header and cover from, positioned at the
    /// first record after the cover; the first pass takes it, so it is not reparsed.
    first: Mutex<Option<(Stream, ErrorSlot)>>,
    /// The source the filing was opened from, for reopening; `None` for a file
    /// object (which cannot be reread) and once closed.
    source: Mutex<Option<Py<PyAny>>>,
    /// Every pass started so far that is still alive, so `close()` can shut them.
    passes: Mutex<Vec<Weak<Pass>>>,
    closed: AtomicBool,
    header: Py<Header>,
    /// The typed cover (`libfec.covers.Form3X`, …) or an `Unknown`.
    cover: Py<PyAny>,
    cover_record: Py<Record>,
    form_type: String,
    filer_id: String,
    filer_name: String,
    report_code: Option<String>,
    coverage_from_date: Option<jiff::civil::Date>,
    coverage_through_date: Option<jiff::civil::Date>,
    id: Option<String>,
    version: Arc<str>,
    name_delimiter: Option<Arc<str>>,
    source_length: usize,
}

impl Filing {
    /// The live passes, pruning the ones whose iterator has been dropped.
    fn live_passes(&self, py: Python<'_>) -> Vec<Arc<Pass>> {
        let mut passes = lock_attached(py, &self.passes);
        passes.retain(|p| p.strong_count() > 0);
        passes.iter().filter_map(Weak::upgrade).collect()
    }

    /// Start a pass: on the stream the constructor left behind if it is unused, else on
    /// a fresh parse of the source.
    fn start_pass(&self, py: Python<'_>, typed: bool) -> PyResult<FilingIterator> {
        if self.closed.load(Ordering::Acquire) {
            return Err(closed_error());
        }
        let first = lock_attached(py, &self.first).take();
        let (stream, raised) = match first {
            Some(first) => first,
            None => {
                let source = lock_attached(py, &self.source)
                    .as_ref()
                    .map(|s| s.clone_ref(py));
                let Some(source) = source else {
                    return Err(PyValueError::new_err(
                        "a filing opened from a file object can only be read once; \
                         pass a path or bytes to read it again",
                    ));
                };
                let SourceReader {
                    reader,
                    length,
                    raised,
                    ..
                } = resolve(source.bind(py))?;
                let filing_id = self.id.clone().unwrap_or_default();
                let stream = py
                    .detach(move || fec_parser::Filing::from_reader(reader, filing_id, length))
                    .map_err(|e| raised_or(&raised, parse_error(e)))?;
                (stream, raised)
            }
        };
        let pass = Arc::new(Pass {
            inner: Mutex::new(Some(stream)),
            pending: Mutex::new(VecDeque::new()),
            raised,
            puller: Mutex::new(None),
            typed,
            version: Arc::clone(&self.version),
            name_delimiter: self.name_delimiter.clone(),
        });
        lock_attached(py, &self.passes).push(Arc::downgrade(&pass));
        // A `close()` on another thread may have run since the check above and
        // missed this pass.
        if self.closed.load(Ordering::Acquire) {
            pass.shut(py);
        }
        Ok(FilingIterator { pass })
    }

    /// Drop the source, the unused first stream and every live pass.  Idempotent.
    fn shut(&self, py: Python<'_>) -> PyResult<()> {
        let passes = self.live_passes(py);
        for pass in &passes {
            pass.check_not_reentrant()?;
        }
        self.closed.store(true, Ordering::Release);
        // Taken out and dropped after the guards, as in `Pass::shut`.
        let first = lock_attached(py, &self.first).take();
        let source = lock_attached(py, &self.source).take();
        drop((first, source));
        for pass in passes {
            pass.shut(py);
        }
        Ok(())
    }
}

#[pymethods]
impl Filing {
    /// Open a filing for streaming.
    ///
    /// Parses the `HDR` and cover records eagerly; everything after them is
    /// pulled on demand by `itemizations` / `itemization_records`.
    #[new]
    #[pyo3(signature = (source, /))]
    fn new(py: Python<'_>, source: &Bound<'_, PyAny>) -> PyResult<Self> {
        open_source(py, source, None)
    }

    /// Download a filing from docquery.fec.gov by its id (`1721696`,
    /// `"1721696"` or `"FEC-1721696"`).
    ///
    /// Stdlib only: `urllib` streams the download into an anonymous temporary
    /// file, which is memory-mapped, so the filing is never held in Python
    /// memory and can be read in as many passes as a path can.  HTTP and network
    /// errors propagate as `urllib.error` exceptions (`HTTPError` 404 for an
    /// unknown id).
    #[staticmethod]
    #[pyo3(signature = (filing_id, /, *, timeout = 60.0))]
    fn from_id(py: Python<'_>, filing_id: &Bound<'_, PyAny>, timeout: f64) -> PyResult<Self> {
        let id = normalize_filing_id(filing_id)?;
        let url = format!("https://docquery.fec.gov/dcdev/posted/{id}.fec");

        let urllib = py.import("urllib.request")?;
        let headers = PyDict::new(py);
        headers.set_item(
            "User-Agent",
            concat!("libfec-python/", env!("CARGO_PKG_VERSION")),
        )?;
        let request = urllib.call_method1("Request", (url, py.None(), headers))?;
        let kwargs = PyDict::new(py);
        kwargs.set_item("timeout", timeout)?;
        let response = urllib.call_method("urlopen", (request,), Some(&kwargs))?;

        let spooled = (|| -> PyResult<Bound<'_, PyAny>> {
            let tmp = py.import("tempfile")?.call_method0("TemporaryFile")?;
            py.import("shutil")?
                .call_method1("copyfileobj", (&response, &tmp, 1 << 20))?;
            tmp.call_method0("flush")?;
            Ok(tmp)
        })();
        response.call_method0("close")?;
        let tmp = spooled?;

        // `mmap` refuses an empty file; an empty download is just an unparseable
        // filing, so let the parser say so.
        let size: u64 = tmp.call_method0("tell")?.extract()?;
        let source = if size == 0 {
            pyo3::types::PyBytes::new(py, b"").into_any()
        } else {
            let mmap = py.import("mmap")?;
            let access = mmap.getattr("ACCESS_READ")?;
            let kwargs = PyDict::new(py);
            kwargs.set_item("access", access)?;
            mmap.call_method("mmap", (tmp.call_method0("fileno")?, 0), Some(&kwargs))?
        };
        // The mapping keeps the data alive; closing the file deletes its last name.
        tmp.call_method0("close")?;
        open_source(py, &source, Some(id))
    }

    /// The filing's `HDR` record — the same object every time.
    #[getter]
    fn header(&self, py: Python<'_>) -> Py<Header> {
        self.header.clone_ref(py)
    }

    /// The typed cover — `Form3X`, `Form1`, … from `libfec.covers` — or an
    /// `Unknown` for a form with no typed class.  The same object every time.
    #[getter]
    fn cover(&self, py: Python<'_>) -> Py<PyAny> {
        self.cover.clone_ref(py)
    }

    /// The cover line as a raw `Record` — the same object every time.
    #[getter]
    fn cover_record(&self, py: Python<'_>) -> Py<Record> {
        self.cover_record.clone_ref(py)
    }

    /// Every record after the cover, typed (`ScheduleA`, … from
    /// `libfec.itemizations`, or `Unknown`), in file order.
    ///
    /// Each access starts a new pass from the top of the filing.
    #[getter]
    fn itemizations(&self, py: Python<'_>) -> PyResult<FilingIterator> {
        self.start_pass(py, true)
    }

    /// Every record after the cover as a raw `Record`, in file order.
    ///
    /// Each access starts a new pass from the top of the filing.
    #[getter]
    fn itemization_records(&self, py: Python<'_>) -> PyResult<FilingIterator> {
        self.start_pass(py, false)
    }

    /// The cover's form type, e.g. `F3XN`.
    #[getter]
    fn form_type(&self) -> &str {
        &self.form_type
    }

    /// The filer's FEC id (committee or candidate), whatever the form calls it.
    #[getter]
    fn filer_id(&self) -> &str {
        &self.filer_id
    }

    /// The filer's name, whatever the form calls it.
    #[getter]
    fn filer_name(&self) -> &str {
        &self.filer_name
    }

    /// The report code (`Q1`, `M8`, …); `None` for a form that is not a periodic report.
    #[getter]
    fn report_code(&self) -> Option<&str> {
        self.report_code.as_deref()
    }

    /// The first day of the coverage period; `None` for a form without one.
    #[getter]
    fn coverage_from_date(&self) -> Option<jiff::civil::Date> {
        self.coverage_from_date
    }

    /// The last day of the coverage period; `None` for a form without one.
    #[getter]
    fn coverage_through_date(&self) -> Option<jiff::civil::Date> {
        self.coverage_through_date
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
    fn closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    /// Drop the source and stop every pass.  Idempotent; iterating afterwards
    /// raises `ValueError`.
    fn close(&self, py: Python<'_>) -> PyResult<()> {
        self.shut(py)
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
        self.shut(py)
    }

    fn __repr__(&self) -> String {
        let id = match &self.id {
            Some(id) => format!("'{id}'"),
            None => "None".to_owned(),
        };
        format!(
            "Filing(id={}, form_type='{}', filer_id='{}')",
            id, self.form_type, self.filer_id
        )
    }
}

/// Parse the `HDR` and cover records of `source`, leaving the rest for the passes.
///
/// `id_override` names a source that cannot name itself, such as the download
/// behind `Filing.from_id`.
fn open_source(
    py: Python<'_>,
    source: &Bound<'_, PyAny>,
    id_override: Option<String>,
) -> PyResult<Filing> {
    let SourceReader {
        reader,
        length: source_length,
        id,
        raised,
        reopenable,
    } = resolve(source)?;
    let id = id_override.or(id);

    let filing_id = id.clone().unwrap_or_default();
    let filing = py
        .detach(move || fec_parser::Filing::from_reader(reader, filing_id, source_length))
        .map_err(|e| raised_or(&raised, parse_error(e)))?;

    let header = Header::from(&filing.header);
    let version: Arc<str> = Arc::from(header.fec_version.as_str());
    let cover = &filing.cover;

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
    let cover_record = Py::new(
        py,
        Record::new(
            cover_schema,
            cover.record.clone(),
            2,
            name_delimiter.clone(),
        ),
    )?;
    let typed_cover = match &cover.cover_data {
        Some(c) => fec_parser::covers::python::cover_to_py(py, c)?.unbind(),
        None => Py::new(py, Unknown::new(cover_record.clone_ref(py)))?.into_any(),
    };

    Ok(Filing {
        form_type: cover.form_type.clone(),
        filer_id: cover.filer_id.clone(),
        filer_name: cover.filer_name.clone(),
        report_code: cover.report_code.clone(),
        coverage_from_date: cover.coverage_from_date,
        coverage_through_date: cover.coverage_through_date,
        first: Mutex::new(Some((filing, raised))),
        source: Mutex::new(reopenable.then(|| source.clone().unbind())),
        passes: Mutex::new(Vec::new()),
        closed: AtomicBool::new(false),
        header: Py::new(py, header)?,
        cover: typed_cover,
        cover_record,
        id,
        version,
        name_delimiter,
        source_length,
    })
}

/// `1721696`, `"1721696"` or `"FEC-1721696"` → `"1721696"`.
fn normalize_filing_id(filing_id: &Bound<'_, PyAny>) -> PyResult<String> {
    // An `int` goes through its `str()`, so a negative one fails the digit check
    // below like any other malformed id.
    let raw = if filing_id.is_instance_of::<pyo3::types::PyInt>() {
        filing_id.str()?.to_string()
    } else {
        filing_id.extract::<String>().map_err(|_| {
            pyo3::exceptions::PyTypeError::new_err(format!(
                "filing id must be an int or str, not {}",
                filing_id
                    .get_type()
                    .name()
                    .map(|n| n.to_string())
                    .unwrap_or_default()
            ))
        })?
    };
    let trimmed = raw.trim();
    let id = trimmed
        .strip_prefix("FEC-")
        .or_else(|| trimmed.strip_prefix("FEC"))
        .unwrap_or(trimmed);
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(PyValueError::new_err(format!(
            "not an FEC filing id: {raw:?}"
        )));
    }
    Ok(id.to_owned())
}

/// The `fec_version` of a filing, from any source `Filing()` accepts.
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
