//! Turning a Python object into something the parser can `Read`.
//!
//! [`resolve`] is the one place that decides what `open()`, `read()`, `Filing()`
//! and `fec_header()` accept.  Resolution order, first match wins:
//!
//! 1. the **buffer protocol** (`bytes`, `bytearray`, `memoryview`, `mmap`, …) —
//!    read straight out of the exporter's memory, no copy;
//! 2. **`str` / `os.PathLike`** — a filesystem path, opened as a `File`;
//! 3. a **text-mode file** — rejected with a `TypeError` naming `'rb'`;
//! 4. anything with a **`read`** method — pulled [`CHUNK`] bytes at a time;
//! 5. anything else — `TypeError`.
//!
//! The buffer branch has to come first: `PathBuf` extraction goes through
//! `os.fspath`, which accepts `bytes` as a path, so the path branch would swallow
//! a `bytes` source.  `str` is not a buffer exporter and `pathlib.Path` has no
//! `read`, so nothing that means *path* is caught by 1 or 4.

use std::ffi::CStr;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Arc, Mutex, MutexGuard};

use pyo3::buffer::{PyBuffer, PyUntypedBuffer};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};

use crate::errors::io_error;

/// How many bytes one `read()` call on a Python file object asks for.
const CHUNK: usize = 64 * 1024;

/// A `Mutex` here is only ever held by this crate's own short critical sections,
/// so a poisoned lock means a panic mid-pull; take the data anyway rather than
/// turning every later call into a panic.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Where an exception raised by a Python `read()` waits to be re-raised.
///
/// The parser only ever hands back its own error types (`anyhow::Error`,
/// `FilingRowReadError`), and this crate turns those into `FecParseError` by
/// `Display`, which would lose the original Python exception.  pyo3 does recover a
/// `PyErr` wrapped in `io::Error` (`From<io::Error> for PyErr`,
/// `pyo3-0.29.2/src/err/impls.rs:46-51`), but nothing on our path performs that
/// conversion, so the adapter also stashes the `PyErr` here and the caller prefers
/// it over the wrapped error.
pub type ErrorSlot = Arc<Mutex<Option<PyErr>>>;

/// A Python source resolved into a reader, plus what could be learned about it.
pub struct SourceReader {
    /// `Box<dyn Read + Send>` (not just `Read`) so the whole `Filing` is `Send`
    /// and can be pulled inside `Python::detach`.
    pub reader: Box<dyn Read + Send>,
    /// The source's size in bytes, `0` when it cannot be known.
    pub length: usize,
    /// The filing id, where the source names itself (a path, or a file object's
    /// `.name`); `None` for anonymous sources like `bytes` or `BytesIO`.
    pub id: Option<String>,
    /// See [`ErrorSlot`]; always empty for sources that cannot run Python code.
    pub raised: ErrorSlot,
}

/// The exception a Python source raised, if any, else `fallback`.
pub fn raised_or(slot: &ErrorSlot, fallback: PyErr) -> PyErr {
    lock(slot).take().unwrap_or(fallback)
}

fn text_mode_error() -> PyErr {
    PyTypeError::new_err("file must be opened in binary mode, e.g. open(path, 'rb')")
}

/// A `Read` straight out of a Python buffer's memory — the reason to take a
/// `PyBuffer` at all.
struct BufferReader {
    /// C-contiguous, one byte per item; checked before this struct is built.
    buf: PyBuffer<u8>,
    /// Bytes already handed out.  Invariant: `pos <= buf.len_bytes()`.
    pos: usize,
}

impl Read for BufferReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let len = self.buf.len_bytes();
        debug_assert!(self.pos <= len, "BufferReader read past its buffer");
        // `saturating_sub`, not `-`: `pos <= len` is an invariant, but if it ever
        // stopped holding, wrapping here would hand `copy_nonoverlapping` a huge
        // length.  This way the worst case is a short read.
        let n = out.len().min(len.saturating_sub(self.pos));
        if n == 0 {
            return Ok(0);
        }
        // SAFETY: `buf` owns a `Py_buffer` view, so the exporter must keep the
        // memory alive, at a fixed address and at a fixed length, until the view is
        // released — which happens in `PyBuffer`'s `Drop`, i.e. no earlier than this
        // struct's own drop (CPython enforces the length half: `bytearray` raises
        // `BufferError` on a resize while a view is exported).  `len_bytes()` is
        // therefore constant, `pos <= len` is an invariant (`pos` starts at 0 and
        // grows by `n = min(out.len(), len - pos)`), so `[pos, pos + n)` is inside
        // the buffer, and `out` is at least `n` long and cannot overlap it.
        // NOT guaranteed by the buffer protocol: that the *contents* hold still.
        // Another thread can mutate a writable exporter (a `bytearray`, a writable
        // `mmap`) while we copy with the GIL released.  That is a data race, but a
        // bounded one: the address and length are pinned, so the worst case is torn
        // bytes in `out` and a `FecParseError` — never a read outside the buffer.
        unsafe {
            ptr::copy_nonoverlapping(
                self.buf.buf_ptr().cast::<u8>().add(self.pos),
                out.as_mut_ptr(),
                n,
            );
        }
        self.pos += n;
        Ok(n)
    }
}

/// A `Read` over any Python object with a `read(n)` method.
///
/// Runs inside `Python::detach`, so every call re-attaches; it asks for at most
/// [`CHUNK`] bytes, so a huge filing behind a file object is never pulled into
/// Python whole.
struct PyReadAdapter {
    obj: Py<PyAny>,
    raised: ErrorSlot,
}

impl PyReadAdapter {
    /// Remember `err` for the caller to re-raise, and wrap a copy of it for the
    /// parser's own error path.
    fn stash(&self, py: Python<'_>, err: PyErr) -> io::Error {
        let wrapped = err.clone_ref(py);
        *lock(&self.raised) = Some(err);
        io::Error::other(wrapped)
    }
}

impl Read for PyReadAdapter {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        let want = out.len().min(CHUNK);
        Python::attach(|py| {
            let data = match self.obj.bind(py).call_method1("read", (want,)) {
                Ok(data) => data,
                Err(e) => return Err(self.stash(py, e)),
            };
            let Ok(bytes) = data.cast::<PyBytes>() else {
                // A text-mode file hands back `str`; say the same thing the
                // up-front `io.TextIOBase` check says.
                let e = if data.is_instance_of::<PyString>() {
                    text_mode_error()
                } else {
                    match data.get_type().name() {
                        Ok(name) => {
                            PyTypeError::new_err(format!("read() must return bytes, not {name}"))
                        }
                        Err(e) => e,
                    }
                };
                return Err(self.stash(py, e));
            };
            let b = bytes.as_bytes();
            if b.len() > want {
                let e = PyValueError::new_err(format!(
                    "read({want}) returned {} bytes, more than asked for",
                    b.len()
                ));
                return Err(self.stash(py, e));
            }
            out[..b.len()].copy_from_slice(b);
            Ok(b.len())
        })
    }
}

/// The filing id for a path: its file stem, with a `FEC-` prefix stripped.
///
/// Mirrors `fec_parser::Filing::from_path` + `from_reader`'s own stripping.
fn id_from_path(path: &Path) -> Option<String> {
    path.file_stem().map(|stem| {
        let stem = stem.to_string_lossy();
        stem.strip_prefix("FEC-").unwrap_or(&stem).to_owned()
    })
}

/// `os.fstat(src.fileno()).st_size`, or `0` if the object has no usable `fileno`.
///
/// `BytesIO` and `urlopen()` responses raise from `fileno()`; that is not an error,
/// it just means the length is unknown.
fn length_from_fileno(source: &Bound<'_, PyAny>) -> usize {
    let py = source.py();
    let stat = source
        .call_method0("fileno")
        .and_then(|fd| py.import("os")?.call_method1("fstat", (fd,)));
    match stat {
        Ok(stat) => stat
            .getattr("st_size")
            .and_then(|size| size.extract::<usize>())
            .unwrap_or(0),
        Err(_) => 0,
    }
}

/// A file object's `.name`, if it is a `str`, as a filing id.
///
/// So `open(builtins.open("1921705.fec", "rb")).id == "1921705"`.  A file opened
/// from a raw descriptor has an `int` name and gets `None`.
fn id_from_name(source: &Bound<'_, PyAny>) -> Option<String> {
    let name: String = source.getattr("name").ok()?.extract().ok()?;
    id_from_path(Path::new(&name))
}

/// Whether `source` is a text-mode file (`open(p)`, `io.StringIO`, …).
fn is_text_io(source: &Bound<'_, PyAny>) -> PyResult<bool> {
    let text_io_base = source.py().import("io")?.getattr("TextIOBase")?;
    source.is_instance(&text_io_base)
}

/// Read a buffer exporter without copying it, or — for the rare non-contiguous
/// buffer, such as `memoryview(b)[::2]` — gather it into a `Vec` first.
fn buffer_source(py: Python<'_>, buffer: PyUntypedBuffer) -> PyResult<SourceReader> {
    let length = buffer.len_bytes();
    // Captured before `into_typed` consumes the buffer, for the error message.
    let format = buffer.format().to_owned();
    let item_size = buffer.item_size();

    let buf = buffer
        .into_typed::<u8>()
        .map_err(|_| not_bytes_like(&format, item_size))?;
    let reader: Box<dyn Read + Send> = if buf.is_c_contiguous() {
        Box::new(BufferReader { buf, pos: 0 })
    } else {
        Box::new(Cursor::new(buf.to_vec(py)?))
    };
    Ok(SourceReader {
        reader,
        length,
        id: None,
        raised: ErrorSlot::default(),
    })
}

/// A buffer of something other than bytes — a NumPy float array, `array('i')`,
/// `memoryview(b).cast('I')`.  `PyBuffer::<u8>::get` rejects those (the format and
/// item size have to match `u8`), and so do we, with a type error rather than the
/// `BufferError` pyo3 raises.
fn not_bytes_like(format: &CStr, item_size: usize) -> PyErr {
    PyTypeError::new_err(format!(
        "source buffer must be bytes-like (one byte per item), \
         got format '{}' with {item_size}-byte items",
        format.to_string_lossy()
    ))
}

/// Pull from a Python file object `CHUNK` bytes at a time.
fn read_source(source: &Bound<'_, PyAny>) -> SourceReader {
    let raised = ErrorSlot::default();
    let adapter = PyReadAdapter {
        obj: source.clone().unbind(),
        raised: Arc::clone(&raised),
    };
    SourceReader {
        reader: Box::new(adapter),
        length: length_from_fileno(source),
        id: id_from_name(source),
        raised,
    }
}

/// Turn a Python source into a reader; see the module docs for the order.
pub fn resolve(source: &Bound<'_, PyAny>) -> PyResult<SourceReader> {
    if let Ok(buffer) = PyUntypedBuffer::get(source) {
        return buffer_source(source.py(), buffer);
    }
    if let Ok(path) = source.extract::<PathBuf>() {
        let file = std::fs::File::open(&path).map_err(|e| io_error(e, &path))?;
        let length = file.metadata().map_err(|e| io_error(e, &path))?.len() as usize;
        return Ok(SourceReader {
            reader: Box::new(file),
            length,
            id: id_from_path(&path),
            raised: ErrorSlot::default(),
        });
    }
    if is_text_io(source)? {
        return Err(text_mode_error());
    }
    if source.hasattr("read")? {
        return Ok(read_source(source));
    }
    Err(PyTypeError::new_err(format!(
        "source must be a path, a bytes-like object, or a binary file object; got {}",
        source.get_type().name()?
    )))
}
