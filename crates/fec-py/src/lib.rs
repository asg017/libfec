mod errors;
mod fecfile;
mod parser;
mod record;
mod rss;
mod source;

use pyo3::prelude::*;

/// Native extension backing the `libfec` package.
///
/// The public API lives in `python/libfec/{parser,fecfile}.py`, which
/// re-export from the submodules declared here. `run_cli` backs the `libfec`
/// command (`python/libfec/_cli.py`).
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
        use crate::parser::{fec_header, open_filing, FilingIterator, FilingReader, Header};
        #[pymodule_export]
        use crate::record::{record_from_parts, Record, Unknown};
    }

    #[pymodule]
    mod fecfile {
        #[pymodule_export]
        use crate::fecfile::{
            from_file, from_http, loads, parse_header, parse_line, print_example,
        };
    }

    /// The FEC e-filing RSS feed parser (`fec-rss`); `python/libfec/rss.py` adds `fetch()`.
    #[pymodule]
    mod rss {
        #[pymodule_export]
        use crate::rss::{feed_url, parse_feed, Feed, Item};
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
