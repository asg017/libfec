mod parser;
mod foo;
mod fecfile;

use pyo3::prelude::*;
use pyo3::wrap_pymodule;

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn libfec_parser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add submodules
    m.add_wrapped(wrap_pymodule!(parser::parser))?;
    m.add_wrapped(wrap_pymodule!(foo::foo))?;
    m.add_wrapped(wrap_pymodule!(fecfile::fecfile))?;
    Ok(())
}
