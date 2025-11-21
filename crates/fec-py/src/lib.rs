use pyo3::prelude::*;
use fec_parser::Filing;

#[pyfunction]
fn fec_header(contents: &[u8]) -> String {
  let f = Filing::from_reader(contents, "123".to_string(), contents.len()).unwrap();
  
   f.header.fec_version
}

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fec_header, m)?)?;
    Ok(())
}
