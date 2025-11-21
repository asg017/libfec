use pyo3::prelude::*;
use pyo3::wrap_pymodule;
use fec_parser::Filing;

// Parser module functions
#[pyfunction]
fn fec_header(contents: &[u8]) -> String {
  let f = Filing::from_reader(contents, "123".to_string(), contents.len()).unwrap();
  
   f.header.fec_version
}

/// Parser submodule for FEC file parsing
#[pymodule]
fn parser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fec_header, m)?)?;
    Ok(())
}

// Foo module functions
#[pyfunction]
fn bar() -> i32 {
    42
}

/// Foo submodule - example module
#[pymodule]
fn foo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(bar, m)?)?;
    Ok(())
}

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn fec_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add submodules
    m.add_wrapped(wrap_pymodule!(parser))?;
    m.add_wrapped(wrap_pymodule!(foo))?;
    Ok(())
}
