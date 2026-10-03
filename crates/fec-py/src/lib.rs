mod errors;
mod parser;
mod row;
mod source;

use pyo3::prelude::*;

/// Native extension backing the `libfec` package.
///
/// The public API lives in `python/libfec/{parser,fecfile}.py`; `fecfile.py`
/// is pure Python over `parser.py`'s `open()` and no longer has a Rust-side module
/// of its own (the old `src/fecfile.rs` compat layer was deleted in ticket 22).
/// `run_cli` backs the `libfec` command (`python/libfec/_cli.py`).
#[pymodule]
mod _native {
    use pyo3::prelude::*;

    /// Run the `libfec` CLI with `argv` (`argv[0]` is the program name); returns the exit code.
    ///
    /// Detached from the interpreter, so a long export doesn't hold the GIL.
    #[pyfunction]
    fn run_cli(py: Python<'_>, argv: Vec<String>) -> i32 {
        py.detach(|| libfec::run(argv))
    }

    #[pymodule]
    mod parser {
        #[pymodule_export]
        use crate::errors::{FecError, FecParseError};
        #[pymodule_export]
        use crate::parser::{fec_header, open_filing, Cover, FilingReader, Header};
        #[pymodule_export]
        use crate::row::{column_names, row_from_parts, Row};
    }

    /// The typed cover classes (`Form3X`, `Form1`, …), defined in `fec-parser`
    /// itself under its `python` feature.
    #[pymodule]
    mod covers {
        use pyo3::prelude::*;

        #[pymodule_init]
        fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
            fec_parser::covers::python::add_classes(m)
        }
    }

    /// The typed itemization classes (`ScheduleA`, …), defined in `fec-parser`
    /// itself under its `python` feature.
    #[pymodule]
    mod itemizations {
        use pyo3::prelude::*;

        #[pymodule_init]
        fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
            fec_parser::itemizations::python::add_classes(m)
        }
    }
}
