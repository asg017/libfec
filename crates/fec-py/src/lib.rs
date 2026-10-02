mod fecfile;
mod parser;

use pyo3::prelude::*;

/// Native extension backing the `libfec_parser` package.
///
/// The public API lives in `python/libfec_parser/{parser,fecfile}.py`, which
/// re-export from the submodules declared here.
#[pymodule]
mod _native {
    use pyo3::prelude::*;

    #[pymodule]
    mod parser {
        #[pymodule_export]
        use crate::parser::{fec_header, Cover, Filing, Header, Itemization};
    }

    #[pymodule]
    mod fecfile {
        #[pymodule_export]
        use crate::fecfile::{
            from_file, from_http, loads, parse_header, parse_line, print_example,
        };
    }
}
